//! Pre-computed values for GradingTone rendering.
//!
//! Reference: OCIO ops/gradingtone/GradingTone.cpp

use super::types::{GradingTone, GradingRGBMSW, RGBMChannel};
use crate::GradingStyle;

/// Style-dependent constants for grading.
#[derive(Debug, Clone, Copy)]
pub struct StyleParams {
    pub top: f32,
    pub top_sc: f32,
    pub bottom: f32,
    pub pivot: f32,
}

impl StyleParams {
    pub fn from_style(style: GradingStyle) -> Self {
        match style {
            GradingStyle::Log => Self {
                top: 1.0,
                top_sc: 1.0,
                bottom: 0.0,
                pivot: 0.4,
            },
            GradingStyle::Linear => Self {
                top: 7.5,
                top_sc: 6.5,
                bottom: -5.5, // breakpoint of lin-to-log
                pivot: 0.0,
            },
            GradingStyle::Video => Self {
                top: 1.0,
                top_sc: 1.0,
                bottom: 0.0,
                pivot: 0.4, // 0.18 -> ~0.39 in video
            },
        }
    }
}

/// Pre-computed curve data for GradingTone.
///
/// Contains control points and slopes for efficient curve evaluation.
/// These values are computed once from `GradingTone` parameters and
/// reused for each pixel.
#[derive(Debug, Clone)]
#[allow(missing_docs)] // Internal curve data
pub struct GradingTonePreRender {
    /// Grading style.
    pub style: GradingStyle,
    /// Top of curve range.
    pub top: f32,
    /// Top for s-contrast.
    pub top_sc: f32,
    /// Bottom of curve range.
    pub bottom: f32,
    /// Pivot point for s-contrast.
    pub pivot: f32,

    // Computed zone boundaries
    pub highlights_start: f64,
    pub highlights_width: f64,
    pub shadows_start: f64,
    pub shadows_width: f64,
    pub whites_start: f64,
    pub whites_width: f64,
    pub blacks_start: f64,
    pub blacks_width: f64,

    // Midtones: 6-point spline [channel][point]
    pub mid_x: [[f32; 6]; 4],
    pub mid_y: [[f32; 6]; 4],
    pub mid_m: [[f32; 6]; 4],

    // Highlights/Shadows: [is_shadow][channel][point]
    pub hs_x: [[[f32; 3]; 4]; 2],
    pub hs_y: [[[f32; 3]; 4]; 2],
    pub hs_m: [[[f32; 2]; 4]; 2],

    // Whites/Blacks: [is_black][channel][point]
    pub wb_x: [[[f32; 2]; 4]; 2],
    pub wb_y: [[[f32; 2]; 4]; 2],
    pub wb_m: [[[f32; 2]; 4]; 2],
    pub wb_gain: [[f32; 4]; 2],

    // S-Contrast: [top=0/bottom=1][point]
    pub sc_x: [[f32; 4]; 2],
    pub sc_y: [[f32; 4]; 2],
    pub sc_m: [[f32; 2]; 2],

    /// True if this is an identity transform (no effect).
    pub local_bypass: bool,
}

impl GradingTonePreRender {
    /// Create pre-render data from GradingTone parameters.
    pub fn new(style: GradingStyle, tone: &GradingTone) -> Self {
        let params = StyleParams::from_style(style);
        let mut pr = Self {
            style,
            top: params.top,
            top_sc: params.top_sc,
            bottom: params.bottom,
            pivot: params.pivot,
            highlights_start: 0.0,
            highlights_width: 0.0,
            shadows_start: 0.0,
            shadows_width: 0.0,
            whites_start: 0.0,
            whites_width: 0.0,
            blacks_start: 0.0,
            blacks_width: 0.0,
            mid_x: [[0.0; 6]; 4],
            mid_y: [[0.0; 6]; 4],
            mid_m: [[1.0; 6]; 4],
            hs_x: [[[0.0; 3]; 4]; 2],
            hs_y: [[[0.0; 3]; 4]; 2],
            hs_m: [[[1.0; 2]; 4]; 2],
            wb_x: [[[0.0; 2]; 4]; 2],
            wb_y: [[[0.0; 2]; 4]; 2],
            wb_m: [[[1.0; 2]; 4]; 2],
            wb_gain: [[1.0; 4]; 2],
            sc_x: [[0.0; 4]; 2],
            sc_y: [[0.0; 4]; 2],
            sc_m: [[1.0; 2]; 2],
            local_bypass: tone.is_identity(),
        };

        if !pr.local_bypass {
            pr.update(tone);
        }

        pr
    }

    /// Update pre-render values from GradingTone.
    pub fn update(&mut self, tone: &GradingTone) {
        self.local_bypass = tone.is_identity();
        if self.local_bypass {
            return;
        }

        // Compute zone boundaries (highlights affect whites, shadows affect blacks)
        self.compute_zone_boundaries(tone);

        // Compute curve data for each zone
        self.mids_precompute(tone);
        self.highlight_shadow_precompute(tone);
        self.white_black_precompute(tone);
        self.scontrast_precompute(tone);
    }

    /// Compute zone boundaries based on highlight/shadow settings.
    fn compute_zone_boundaries(&mut self, tone: &GradingTone) {
        // Highlights affect whites
        {
            let master = tone.highlights.master;
            let start = tone.highlights.start;
            let pivot = tone.highlights.width;
            let startw = tone.whites.start;
            let widthw = tone.whites.width;

            self.highlights_start = if start > pivot - 0.01 {
                pivot - 0.01
            } else {
                start
            };
            self.highlights_width = pivot;

            let new_start = highlight_fwd_eval(
                startw,
                self.highlights_start,
                self.highlights_width,
                master,
            );
            let new_end = highlight_fwd_eval(
                startw + widthw,
                self.highlights_start,
                self.highlights_width,
                master,
            );
            self.whites_start = new_start;
            self.whites_width = new_end - new_start;
        }

        // Shadows affect blacks
        {
            let master = tone.shadows.master;
            let start = tone.shadows.start;
            let pivot = tone.shadows.width;
            let startb = tone.blacks.start;
            let widthb = tone.blacks.width;

            self.shadows_start = if start < pivot + 0.01 {
                pivot + 0.01
            } else {
                start
            };
            self.shadows_width = pivot;

            let new_start = shadow_fwd_eval(
                startb,
                self.shadows_width,
                self.shadows_start,
                master,
            );
            let new_end = shadow_fwd_eval(
                startb - widthb,
                self.shadows_width,
                self.shadows_start,
                master,
            );
            self.blacks_start = new_start;
            self.blacks_width = new_start - new_end;
        }
    }

    /// Precompute midtones spline.
    fn mids_precompute(&mut self, tone: &GradingTone) {
        const HALO: f32 = 0.4;

        for channel in RGBMChannel::ALL {
            let ch = channel as usize;

            let mid_adj = get_channel_clamped(&tone.midtones, channel, 0.01, 1.99);

            if mid_adj == 1.0 {
                continue;
            }

            let x0 = self.bottom;
            let x5 = self.top;

            let max_width = (x5 - x0) * 0.95;
            let width = (tone.midtones.width as f32).clamp(0.01, max_width);
            let min_cent = x0 + width * 0.51;
            let max_cent = x5 - width * 0.51;
            let center = (tone.midtones.start as f32).clamp(min_cent, max_cent);

            let x1 = center - width * 0.5;
            let x4 = x1 + width;
            let x2 = x1 + (x4 - x1) * 0.25;
            let x3 = x1 + (x4 - x1) * 0.75;

            let y0 = x0;
            let m0 = 1.0_f32;
            let m5 = 1.0_f32;

            const MIN_SLOPE: f32 = 0.1;

            let mut mid_adj = mid_adj - 1.0;
            mid_adj *= 1.0 - MIN_SLOPE;

            let m2 = 1.0 + mid_adj;
            let m3 = 1.0 - mid_adj;
            let mut m1 = 1.0 + mid_adj * HALO;
            let mut m4 = 1.0 - mid_adj * HALO;

            // Area-preserving constraint
            if center <= (x5 + x0) * 0.5 {
                let area = (x1 - x0) * (m1 - m0) * 0.5
                    + (x2 - x1) * ((m1 - m0) + (m2 - m1) * 0.5)
                    + (center - x2) * (m2 - m0) * 0.5;
                m4 = (-0.5 * (x5 - x4) * m5
                    + (x4 - x3) * (0.5 * m3 - m5)
                    + (x3 - center) * (m3 - m5) * 0.5
                    + area)
                    / (-0.5 * (x5 - x3));
            } else {
                let area = (x5 - x4) * (m4 - m5) * 0.5
                    + (x4 - x3) * ((m4 - m5) + (m3 - m4) * 0.5)
                    + (x3 - center) * (m3 - m5) * 0.5;
                m1 = (-0.5 * (x1 - x0) * m0
                    + (x2 - x1) * (0.5 * m2 - m0)
                    + (center - x2) * (m2 - m0) * 0.5
                    + area)
                    / (-0.5 * (x2 - x0));
            }

            let y1 = y0 + (m0 + m1) * (x1 - x0) * 0.5;
            let y2 = y1 + (m1 + m2) * (x2 - x1) * 0.5;
            let y3 = y2 + (m2 + m3) * (x3 - x2) * 0.5;
            let y4 = y3 + (m3 + m4) * (x4 - x3) * 0.5;
            let y5 = y4 + (m4 + m5) * (x5 - x4) * 0.5;

            self.mid_x[ch] = [x0, x1, x2, x3, x4, x5];
            self.mid_y[ch] = [y0, y1, y2, y3, y4, y5];
            self.mid_m[ch] = [m0, m1, m2, m3, m4, m5];
        }
    }

    /// Precompute highlight/shadow curves.
    fn highlight_shadow_precompute(&mut self, tone: &GradingTone) {
        for is_shadow in [false, true] {
            let hs_idx = if is_shadow { 1 } else { 0 };

            for channel in RGBMChannel::ALL {
                let ch = channel as usize;

                let zone = if is_shadow {
                    &tone.shadows
                } else {
                    &tone.highlights
                };
                let mut val = get_channel_clamped(zone, channel, 0.01, 1.99);

                if !is_shadow {
                    val = 2.0 - val;
                }

                if val == 1.0 {
                    continue;
                }

                let start = if is_shadow {
                    self.shadows_start as f32
                } else {
                    self.highlights_start as f32
                };
                let pivot = if is_shadow {
                    self.shadows_width as f32
                } else {
                    self.highlights_width as f32
                };

                let x0 = if is_shadow { pivot } else { start };
                let x2 = if is_shadow { start } else { pivot };
                let y0 = x0;
                let y2 = x2;
                let x1 = x0 + (x2 - x0) * 0.5;

                let (m0, m2, y1) = if val < 1.0 {
                    let m0 = if is_shadow { val.max(0.01) } else { 1.0 };
                    let m2 = if is_shadow { 1.0 } else { val.max(0.01) };
                    let y1 = (0.5 / (x2 - x0))
                        * ((2.0 * y0 + m0 * (x1 - x0)) * (x2 - x1)
                            + (2.0 * y2 - m2 * (x2 - x1)) * (x1 - x0));
                    (m0, m2, y1)
                } else {
                    let m0 = if is_shadow { (2.0 - val).max(0.01) } else { 1.0 };
                    let m2 = if is_shadow { 1.0 } else { (2.0 - val).max(0.01) };
                    let y1 = (0.5 / ((x2 - x1) + (x1 - x0)))
                        * ((2.0 * y0 + m0 * (x1 - x0)) * (x2 - x1)
                            + (2.0 * y2 - m2 * (x2 - x1)) * (x1 - x0));
                    (m0, m2, y1)
                };

                self.hs_x[hs_idx][ch] = [x0, x1, x2];
                self.hs_y[hs_idx][ch] = [y0, y1, y2];
                self.hs_m[hs_idx][ch] = [m0, m2];
            }
        }
    }

    /// Precompute white/black curves.
    fn white_black_precompute(&mut self, tone: &GradingTone) {
        for is_black in [false, true] {
            let wb_idx = if is_black { 1 } else { 0 };

            for channel in RGBMChannel::ALL {
                let ch = channel as usize;

                let zone = if is_black { &tone.blacks } else { &tone.whites };
                let val = get_channel_clamped(zone, channel, 0.01, 1.99);

                let start = if is_black {
                    self.blacks_start as f32
                } else {
                    self.whites_start as f32
                };
                let width = if is_black {
                    self.blacks_width as f32
                } else {
                    self.whites_width as f32
                };

                let x0 = if !is_black { start } else { start - width };
                let x1 = if !is_black { x0 + width } else { start };

                let mtest = if !is_black { val } else { 2.0 - val };

                let (m0, m1, y0, y1, gain) = if mtest < 1.0 {
                    // Slope is decreasing
                    if !is_black {
                        let m0 = 1.0_f32;
                        let m1 = val.max(0.01);
                        let y0 = x0;
                        let y1 = y0 + (m0 + m1) * (x1 - x0) * 0.5;
                        (m0, m1, y0, y1, 1.0)
                    } else {
                        let m0 = (2.0 - val).max(0.01);
                        let m1 = 1.0_f32;
                        let y1 = x1;
                        let y0 = y1 - (m0 + m1) * (x1 - x0) * 0.5;
                        (m0, m1, y0, y1, 1.0)
                    }
                } else if mtest > 1.0 {
                    // Slope is increasing
                    if !is_black {
                        let m0 = 1.0_f32;
                        let m1 = (2.0 - val).max(0.01);
                        let y0 = x0;
                        let y1 = 0.0; // won't be used
                        let gain = (m0 + m1) * 0.5;
                        (m0, m1, y0, y1, gain)
                    } else {
                        let m0 = val.max(0.01);
                        let m1 = 1.0_f32;
                        let y1 = x1;
                        let y0 = y1 - (m0 + m1) * (x1 - x0) * 0.5;
                        let gain = (m0 + m1) * 0.5;
                        (m0, m1, y0, y1, gain)
                    }
                } else {
                    (1.0, 1.0, x0, x1, 1.0)
                };

                self.wb_x[wb_idx][ch] = [x0, x1];
                self.wb_y[wb_idx][ch] = [y0, y1];
                self.wb_m[wb_idx][ch] = [m0, m1];
                self.wb_gain[wb_idx][ch] = gain;
            }
        }
    }

    /// Precompute s-contrast curves.
    fn scontrast_precompute(&mut self, tone: &GradingTone) {
        let contrast = tone.s_contrast as f32;
        if contrast == 1.0 {
            return;
        }

        // Limit the range to prevent reversals
        let contrast = if contrast > 1.0 {
            1.0 / (1.8125 - 0.8125 * contrast.min(1.99))
        } else {
            0.28125 + 0.71875 * contrast.max(0.01)
        };

        // Top end (index 0)
        {
            let x3 = self.top_sc;
            let y3 = self.top_sc;
            let y0 = self.pivot + (y3 - self.pivot) * 0.25;
            let m0 = contrast;
            let x0 = self.pivot + (y0 - self.pivot) / m0;
            let min_width = (x3 - x0) * 0.3;
            let mut m3 = 1.0 / m0;

            let center = (y3 - y0 - m3 * x3 + m0 * x0) / (m0 - m3);
            let mut x1 = x0;
            let mut x2 = 2.0 * center - x1;

            if x2 > x3 {
                x2 = x3;
                x1 = 2.0 * center - x2;
            } else if (x2 - x1) < min_width {
                x2 = x1 + min_width;
                let new_center = (x2 + x1) * 0.5;
                m3 = (y3 - y0 + m0 * x0 - new_center * m0) / (x3 - new_center);
            }

            let y1 = y0;
            let y2 = y1 + (m0 + m3) * (x2 - x1) * 0.5;

            self.sc_x[0] = [x0, x1, x2, x3];
            self.sc_y[0] = [y0, y1, y2, y3];
            self.sc_m[0] = [m0, m3];
        }

        // Bottom end (index 1)
        {
            let x0 = self.bottom;
            let y0 = self.bottom;
            let y3 = self.pivot - (self.pivot - y0) * 0.25;
            let m3 = contrast;
            let x3 = self.pivot - (self.pivot - y3) / m3;
            let min_width = (x3 - x0) * 0.3;
            let mut m0 = 1.0 / m3;

            let center = (y3 - y0 - m3 * x3 + m0 * x0) / (m0 - m3);
            let mut x2 = x3;
            let mut x1 = 2.0 * center - x2;

            if x1 < x0 {
                x1 = x0;
                x2 = 2.0 * center - x1;
            } else if (x2 - x1) < min_width {
                x1 = x2 - min_width;
                let new_center = (x2 + x1) * 0.5;
                m0 = (y3 - y0 - m3 * x3 + new_center * m3) / (new_center - x0);
            }

            let y2 = y3;
            let y1 = y2 - (m0 + m3) * (x2 - x1) * 0.5;

            self.sc_x[1] = [x0, x1, x2, x3];
            self.sc_y[1] = [y0, y1, y2, y3];
            self.sc_m[1] = [m0, m3];
        }
    }
}

// ============================================================================
// Helper functions for zone boundary computation
// ============================================================================

/// Faux-cubic forward evaluation (two quadratic Bezier segments).
fn faux_cubic_fwd_eval(
    t: f64,
    x0: f64,
    x2: f64,
    y0: f64,
    y2: f64,
    m0: f64,
    m2: f64,
    x1: f64,
) -> f64 {
    let y1 = (0.5 / ((x2 - x1) + (x1 - x0)))
        * ((2.0 * y0 + m0 * (x1 - x0)) * (x2 - x1) + (2.0 * y2 - m2 * (x2 - x1)) * (x1 - x0));

    let tl = (t - x0) / (x1 - x0);
    let tr = (t - x1) / (x2 - x1);
    let fl = y0 * (1.0 - tl * tl) + y1 * tl * tl + m0 * (1.0 - tl) * tl * (x1 - x0);
    let fr = y1 * (1.0 - tr) * (1.0 - tr) + y2 * (2.0 - tr) * tr + m2 * (tr - 1.0) * tr * (x2 - x1);

    let mut res = if t < x1 { fl } else { fr };
    if t < x0 {
        res = y0 + (t - x0) * m0;
    }
    if t > x2 {
        res = y2 + (t - x2) * m2;
    }
    res
}

/// Faux-cubic reverse evaluation.
fn faux_cubic_rev_eval(
    t: f64,
    x0: f64,
    x2: f64,
    y0: f64,
    y2: f64,
    m0: f64,
    m2: f64,
    x1: f64,
) -> f64 {
    let y1 = (0.5 / ((x2 - x1) + (x1 - x0)))
        * ((2.0 * y0 + m0 * (x1 - x0)) * (x2 - x1) + (2.0 * y2 - m2 * (x2 - x1)) * (x1 - x0));

    let cl = y0 - t;
    let bl = m0 * (x1 - x0);
    let al = y1 - y0 - m0 * (x1 - x0);
    let discrim_l = (bl * bl - 4.0 * al * cl).sqrt();
    let tmp_l = (2.0 * cl) / (-discrim_l - bl);
    let out_l = tmp_l * (x1 - x0) + x0;

    let cr = y1 - t;
    let br = 2.0 * y2 - 2.0 * y1 - m2 * (x2 - x1);
    let ar = y1 - y2 + m2 * (x2 - x1);
    let discrim_r = (br * br - 4.0 * ar * cr).sqrt();
    let tmp_r = (2.0 * cr) / (-discrim_r - br);
    let out_r = tmp_r * (x2 - x1) + x1;

    let mut res = if t < y1 { out_l } else { out_r };
    if t < y0 {
        res = x0 + (t - y0) / m0;
    }
    if t > y2 {
        res = x2 + (t - y2) / m2;
    }
    res
}

/// Evaluate highlight curve at given input.
fn highlight_fwd_eval(t: f64, start: f64, pivot: f64, val: f64) -> f64 {
    let x0 = start;
    let x2 = pivot;
    let y0 = x0;
    let y2 = x2;
    let m0 = 1.0;
    let x1 = x0 + (x2 - x0) * 0.5;
    let val = 2.0 - val;

    if val <= 1.0 {
        let m2 = val.max(0.01);
        faux_cubic_fwd_eval(t, x0, x2, y0, y2, m0, m2, x1)
    } else {
        let m2 = (2.0 - val).max(0.01);
        faux_cubic_rev_eval(t, x0, x2, y0, y2, m0, m2, x1)
    }
}

/// Evaluate shadow curve at given input.
fn shadow_fwd_eval(t: f64, start: f64, pivot: f64, val: f64) -> f64 {
    let x0 = start;
    let x2 = pivot;
    let y0 = x0;
    let y2 = x2;
    let m2 = 1.0;
    let x1 = x0 + (x2 - x0) * 0.5;

    if val <= 1.0 {
        let m0 = val.max(0.01);
        faux_cubic_fwd_eval(t, x0, x2, y0, y2, m0, m2, x1)
    } else {
        let m0 = (2.0 - val).max(0.01);
        faux_cubic_rev_eval(t, x0, x2, y0, y2, m0, m2, x1)
    }
}

/// Get channel value as f32, clamped.
#[inline]
fn get_channel_clamped(zone: &GradingRGBMSW, channel: RGBMChannel, min: f32, max: f32) -> f32 {
    (zone.get(channel) as f32).clamp(min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prerender_identity() {
        let tone = GradingTone::new(GradingStyle::Log);
        let pr = GradingTonePreRender::new(GradingStyle::Log, &tone);
        assert!(pr.local_bypass);
    }

    #[test]
    fn test_prerender_midtones() {
        let mut tone = GradingTone::new(GradingStyle::Log);
        tone.midtones.master = 1.5;

        let pr = GradingTonePreRender::new(GradingStyle::Log, &tone);
        assert!(!pr.local_bypass);

        // Check that midtone spline was computed for master channel
        let m = RGBMChannel::M as usize;
        assert!(pr.mid_x[m][0] < pr.mid_x[m][5]);
    }

    #[test]
    fn test_style_params() {
        let log = StyleParams::from_style(GradingStyle::Log);
        assert_eq!(log.pivot, 0.4);

        let lin = StyleParams::from_style(GradingStyle::Linear);
        assert_eq!(lin.pivot, 0.0);
        assert_eq!(lin.bottom, -5.5);

        let vid = StyleParams::from_style(GradingStyle::Video);
        assert_eq!(vid.pivot, 0.4);
    }
}
