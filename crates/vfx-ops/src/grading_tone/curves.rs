//! Curve evaluation functions for GradingTone.
//!
//! Reference: OCIO ops/gradingtone/GradingToneOpCPU.cpp
//!
//! Each zone uses spline/bezier curves with precomputed control points.
//! Forward and reverse evaluation are both supported.

use super::prerender::GradingTonePreRender;
use super::types::{GradingTone, RGBMChannel};

// ============================================================================
// Midtones: 6-point spline
// ============================================================================

/// Evaluate midtone curve forward (single channel).
#[inline]
pub fn mids_fwd_single(
    pr: &GradingTonePreRender,
    tone: &GradingTone,
    channel: RGBMChannel,
    val: f32,
) -> f32 {
    let ch = channel as usize;
    let mid_adj = (tone.midtones.get(channel) as f32).clamp(0.01, 1.99);

    if mid_adj == 1.0 {
        return val;
    }

    let x0 = pr.mid_x[ch][0];
    let x1 = pr.mid_x[ch][1];
    let x2 = pr.mid_x[ch][2];
    let x3 = pr.mid_x[ch][3];
    let x4 = pr.mid_x[ch][4];
    let x5 = pr.mid_x[ch][5];
    let y0 = pr.mid_y[ch][0];
    let y1 = pr.mid_y[ch][1];
    let y2 = pr.mid_y[ch][2];
    let y3 = pr.mid_y[ch][3];
    let y4 = pr.mid_y[ch][4];
    let y5 = pr.mid_y[ch][5];
    let m0 = pr.mid_m[ch][0];
    let m1 = pr.mid_m[ch][1];
    let m2 = pr.mid_m[ch][2];
    let m3 = pr.mid_m[ch][3];
    let m4 = pr.mid_m[ch][4];
    let m5 = pr.mid_m[ch][5];

    let t = val;

    // Quadratic segments
    let tl = (t - x0) / (x1 - x0);
    let tm = (t - x1) / (x2 - x1);
    let tr = (t - x2) / (x3 - x2);
    let tr2 = (t - x3) / (x4 - x3);
    let tr3 = (t - x4) / (x5 - x4);

    let fl = tl * (x1 - x0) * (tl * 0.5 * (m1 - m0) + m0) + y0;
    let fm = tm * (x2 - x1) * (tm * 0.5 * (m2 - m1) + m1) + y1;
    let fr = tr * (x3 - x2) * (tr * 0.5 * (m3 - m2) + m2) + y2;
    let fr2 = tr2 * (x4 - x3) * (tr2 * 0.5 * (m4 - m3) + m3) + y3;
    let fr3 = tr3 * (x5 - x4) * (tr3 * 0.5 * (m5 - m4) + m4) + y4;

    let mut res = if t < x1 { fl } else { fm };
    if t > x2 {
        res = fr;
    }
    if t > x3 {
        res = fr2;
    }
    if t > x4 {
        res = fr3;
    }
    if t < x0 {
        res = y0 + (t - x0) * m0;
    }
    if t > x5 {
        res = y5 + (t - x5) * m5;
    }

    res
}

/// Evaluate midtone curve forward (RGB channels, for Master).
#[inline]
pub fn mids_fwd_rgb(pr: &GradingTonePreRender, tone: &GradingTone, rgb: &mut [f32; 3]) {
    let ch = RGBMChannel::M as usize;
    let mid_adj = (tone.midtones.master as f32).clamp(0.01, 1.99);

    if mid_adj == 1.0 {
        return;
    }

    let x0 = pr.mid_x[ch][0];
    let x1 = pr.mid_x[ch][1];
    let x2 = pr.mid_x[ch][2];
    let x3 = pr.mid_x[ch][3];
    let x4 = pr.mid_x[ch][4];
    let x5 = pr.mid_x[ch][5];
    let y0 = pr.mid_y[ch][0];
    let y1 = pr.mid_y[ch][1];
    let y2 = pr.mid_y[ch][2];
    let y3 = pr.mid_y[ch][3];
    let y4 = pr.mid_y[ch][4];
    let y5 = pr.mid_y[ch][5];
    let m0 = pr.mid_m[ch][0];
    let m1 = pr.mid_m[ch][1];
    let m2 = pr.mid_m[ch][2];
    let m3 = pr.mid_m[ch][3];
    let m4 = pr.mid_m[ch][4];
    let m5 = pr.mid_m[ch][5];

    for c in rgb.iter_mut() {
        let t = *c;

        let tl = (t - x0) / (x1 - x0);
        let tm = (t - x1) / (x2 - x1);
        let tr = (t - x2) / (x3 - x2);
        let tr2 = (t - x3) / (x4 - x3);
        let tr3 = (t - x4) / (x5 - x4);

        let fl = tl * (x1 - x0) * (tl * 0.5 * (m1 - m0) + m0) + y0;
        let fm = tm * (x2 - x1) * (tm * 0.5 * (m2 - m1) + m1) + y1;
        let fr = tr * (x3 - x2) * (tr * 0.5 * (m3 - m2) + m2) + y2;
        let fr2 = tr2 * (x4 - x3) * (tr2 * 0.5 * (m4 - m3) + m3) + y3;
        let fr3 = tr3 * (x5 - x4) * (tr3 * 0.5 * (m5 - m4) + m4) + y4;

        let fr4 = (t - x0) * m0 + y0;
        let fr5 = (t - x5) * m5 + y5;

        let mut res = if t < x1 { fl } else { fm };
        if t > x2 {
            res = fr;
        }
        if t > x3 {
            res = fr2;
        }
        if t > x4 {
            res = fr3;
        }
        if t < x0 {
            res = fr4;
        }
        if t > x5 {
            res = fr5;
        }

        *c = res;
    }
}

/// Evaluate midtone curve reverse (single channel).
#[inline]
pub fn mids_rev_single(
    pr: &GradingTonePreRender,
    tone: &GradingTone,
    channel: RGBMChannel,
    val: f32,
) -> f32 {
    let ch = channel as usize;
    let mid_adj = (tone.midtones.get(channel) as f32).clamp(0.01, 1.99);

    if mid_adj == 1.0 {
        return val;
    }

    let x0 = pr.mid_x[ch][0];
    let x1 = pr.mid_x[ch][1];
    let x2 = pr.mid_x[ch][2];
    let x3 = pr.mid_x[ch][3];
    let x4 = pr.mid_x[ch][4];
    let x5 = pr.mid_x[ch][5];
    let y0 = pr.mid_y[ch][0];
    let y1 = pr.mid_y[ch][1];
    let y2 = pr.mid_y[ch][2];
    let y3 = pr.mid_y[ch][3];
    let y4 = pr.mid_y[ch][4];
    let y5 = pr.mid_y[ch][5];
    let m0 = pr.mid_m[ch][0];
    let m1 = pr.mid_m[ch][1];
    let m2 = pr.mid_m[ch][2];
    let m3 = pr.mid_m[ch][3];
    let m4 = pr.mid_m[ch][4];
    let m5 = pr.mid_m[ch][5];

    let t = val;

    // Solve quadratic for each segment
    if t >= y5 {
        return x0 + (t - y0) / m0;
    } else if t >= y4 {
        let c = y4 - t;
        let b = m4 * (x5 - x4);
        let a = 0.5 * (m5 - m4) * (x5 - x4);
        let discrim = (b * b - 4.0 * a * c).sqrt();
        let tmp = (2.0 * c) / (-discrim - b);
        return tmp * (x5 - x4) + x4;
    } else if t >= y3 {
        let c = y3 - t;
        let b = m3 * (x4 - x3);
        let a = 0.5 * (m4 - m3) * (x4 - x3);
        let discrim = (b * b - 4.0 * a * c).sqrt();
        let tmp = (2.0 * c) / (-discrim - b);
        return tmp * (x4 - x3) + x3;
    } else if t >= y2 {
        let c = y2 - t;
        let b = m2 * (x3 - x2);
        let a = 0.5 * (m3 - m2) * (x3 - x2);
        let discrim = (b * b - 4.0 * a * c).sqrt();
        let tmp = (2.0 * c) / (-discrim - b);
        return tmp * (x3 - x2) + x2;
    } else if t >= y1 {
        let c = y1 - t;
        let b = m1 * (x2 - x1);
        let a = 0.5 * (m2 - m1) * (x2 - x1);
        let discrim = (b * b - 4.0 * a * c).sqrt();
        let tmp = (2.0 * c) / (-discrim - b);
        return tmp * (x2 - x1) + x1;
    } else if t >= y0 {
        let c = y0 - t;
        let b = m0 * (x1 - x0);
        let a = 0.5 * (m1 - m0) * (x1 - x0);
        let discrim = (b * b - 4.0 * a * c).sqrt();
        let tmp = (2.0 * c) / (-discrim - b);
        return tmp * (x1 - x0) + x0;
    }

    x0 + (t - y0) / m0
}

/// Evaluate midtone curve reverse (RGB channels, for Master).
#[inline]
pub fn mids_rev_rgb(pr: &GradingTonePreRender, tone: &GradingTone, rgb: &mut [f32; 3]) {
    let ch = RGBMChannel::M as usize;
    let mid_adj = (tone.midtones.master as f32).clamp(0.01, 1.99);

    if mid_adj == 1.0 {
        return;
    }

    let x0 = pr.mid_x[ch][0];
    let x1 = pr.mid_x[ch][1];
    let x2 = pr.mid_x[ch][2];
    let x3 = pr.mid_x[ch][3];
    let x4 = pr.mid_x[ch][4];
    let x5 = pr.mid_x[ch][5];
    let y0 = pr.mid_y[ch][0];
    let y1 = pr.mid_y[ch][1];
    let y2 = pr.mid_y[ch][2];
    let y3 = pr.mid_y[ch][3];
    let y4 = pr.mid_y[ch][4];
    let y5 = pr.mid_y[ch][5];
    let m0 = pr.mid_m[ch][0];
    let m1 = pr.mid_m[ch][1];
    let m2 = pr.mid_m[ch][2];
    let m3 = pr.mid_m[ch][3];
    let m4 = pr.mid_m[ch][4];
    let m5 = pr.mid_m[ch][5];

    for c in rgb.iter_mut() {
        let t = *c;

        let out_l0 = x5 + (t - y5) / m5;
        let out_r3 = solve_quadratic(t, y4, m4, m5, x4, x5);
        let out_r2 = solve_quadratic(t, y3, m3, m4, x3, x4);
        let out_r = solve_quadratic(t, y2, m2, m3, x2, x3);
        let out_m = solve_quadratic(t, y1, m1, m2, x1, x2);
        let out_l = solve_quadratic(t, y0, m0, m1, x0, x1);
        let out_r4 = x0 + (t - y0) / m0;

        let mut res = if t < y1 { out_l } else { out_m };
        if t > y2 {
            res = out_r;
        }
        if t > y3 {
            res = out_r2;
        }
        if t > y4 {
            res = out_r3;
        }
        if t < y0 {
            res = out_r4;
        }
        if t > y5 {
            res = out_l0;
        }

        *c = res;
    }
}

/// Solve quadratic for segment inversion.
#[inline]
fn solve_quadratic(t: f32, y_start: f32, m_start: f32, m_end: f32, x_start: f32, x_end: f32) -> f32 {
    let c = y_start - t;
    let b = m_start * (x_end - x_start);
    let a = 0.5 * (m_end - m_start) * (x_end - x_start);
    let discrim = (b * b - 4.0 * a * c).sqrt();
    let tmp = (2.0 * c) / (-b - discrim);
    tmp * (x_end - x_start) + x_start
}

// ============================================================================
// Highlights/Shadows: 3-point faux-cubic (two quadratic Beziers)
// ============================================================================

/// Evaluate highlight/shadow curve forward (single channel).
#[inline]
pub fn hs_fwd_single(
    pr: &GradingTonePreRender,
    tone: &GradingTone,
    channel: RGBMChannel,
    is_shadow: bool,
    val: f32,
) -> f32 {
    let zone = if is_shadow {
        &tone.shadows
    } else {
        &tone.highlights
    };
    let mut zone_val = (zone.get(channel) as f32).clamp(0.01, 1.99);

    if !is_shadow {
        zone_val = 2.0 - zone_val;
    }

    if zone_val == 1.0 {
        return val;
    }

    let hs_idx = if is_shadow { 1 } else { 0 };
    let ch = channel as usize;

    let x0 = pr.hs_x[hs_idx][ch][0];
    let x1 = pr.hs_x[hs_idx][ch][1];
    let x2 = pr.hs_x[hs_idx][ch][2];
    let y0 = pr.hs_y[hs_idx][ch][0];
    let y1 = pr.hs_y[hs_idx][ch][1];
    let y2 = pr.hs_y[hs_idx][ch][2];
    let m0 = pr.hs_m[hs_idx][ch][0];
    let m2 = pr.hs_m[hs_idx][ch][1];

    if zone_val < 1.0 {
        compute_hs_fwd(val, x0, x1, x2, y0, y1, y2, m0, m2)
    } else {
        compute_hs_rev(val, x0, x1, x2, y0, y1, y2, m0, m2)
    }
}

/// Evaluate highlight/shadow curve forward (RGB channels, for Master).
#[inline]
pub fn hs_fwd_rgb(
    pr: &GradingTonePreRender,
    tone: &GradingTone,
    is_shadow: bool,
    rgb: &mut [f32; 3],
) {
    let zone = if is_shadow {
        &tone.shadows
    } else {
        &tone.highlights
    };
    let mut zone_val = (zone.master as f32).clamp(0.01, 1.99);

    if !is_shadow {
        zone_val = 2.0 - zone_val;
    }

    if zone_val == 1.0 {
        return;
    }

    let hs_idx = if is_shadow { 1 } else { 0 };
    let ch = RGBMChannel::M as usize;

    let x0 = pr.hs_x[hs_idx][ch][0];
    let x1 = pr.hs_x[hs_idx][ch][1];
    let x2 = pr.hs_x[hs_idx][ch][2];
    let y0 = pr.hs_y[hs_idx][ch][0];
    let y1 = pr.hs_y[hs_idx][ch][1];
    let y2 = pr.hs_y[hs_idx][ch][2];
    let m0 = pr.hs_m[hs_idx][ch][0];
    let m2 = pr.hs_m[hs_idx][ch][1];

    for c in rgb.iter_mut() {
        *c = if zone_val < 1.0 {
            compute_hs_fwd(*c, x0, x1, x2, y0, y1, y2, m0, m2)
        } else {
            compute_hs_rev(*c, x0, x1, x2, y0, y1, y2, m0, m2)
        };
    }
}

/// Evaluate highlight/shadow curve reverse (single channel).
#[inline]
pub fn hs_rev_single(
    pr: &GradingTonePreRender,
    tone: &GradingTone,
    channel: RGBMChannel,
    is_shadow: bool,
    val: f32,
) -> f32 {
    let zone = if is_shadow {
        &tone.shadows
    } else {
        &tone.highlights
    };
    let mut zone_val = (zone.get(channel) as f32).clamp(0.01, 1.99);

    if !is_shadow {
        zone_val = 2.0 - zone_val;
    }

    if zone_val == 1.0 {
        return val;
    }

    let hs_idx = if is_shadow { 1 } else { 0 };
    let ch = channel as usize;

    let x0 = pr.hs_x[hs_idx][ch][0];
    let x1 = pr.hs_x[hs_idx][ch][1];
    let x2 = pr.hs_x[hs_idx][ch][2];
    let y0 = pr.hs_y[hs_idx][ch][0];
    let y1 = pr.hs_y[hs_idx][ch][1];
    let y2 = pr.hs_y[hs_idx][ch][2];
    let m0 = pr.hs_m[hs_idx][ch][0];
    let m2 = pr.hs_m[hs_idx][ch][1];

    // Reverse of forward
    if zone_val < 1.0 {
        compute_hs_rev(val, x0, x1, x2, y0, y1, y2, m0, m2)
    } else {
        compute_hs_fwd(val, x0, x1, x2, y0, y1, y2, m0, m2)
    }
}

/// Evaluate highlight/shadow curve reverse (RGB channels, for Master).
#[inline]
pub fn hs_rev_rgb(
    pr: &GradingTonePreRender,
    tone: &GradingTone,
    is_shadow: bool,
    rgb: &mut [f32; 3],
) {
    let zone = if is_shadow {
        &tone.shadows
    } else {
        &tone.highlights
    };
    let mut zone_val = (zone.master as f32).clamp(0.01, 1.99);

    if !is_shadow {
        zone_val = 2.0 - zone_val;
    }

    if zone_val == 1.0 {
        return;
    }

    let hs_idx = if is_shadow { 1 } else { 0 };
    let ch = RGBMChannel::M as usize;

    let x0 = pr.hs_x[hs_idx][ch][0];
    let x1 = pr.hs_x[hs_idx][ch][1];
    let x2 = pr.hs_x[hs_idx][ch][2];
    let y0 = pr.hs_y[hs_idx][ch][0];
    let y1 = pr.hs_y[hs_idx][ch][1];
    let y2 = pr.hs_y[hs_idx][ch][2];
    let m0 = pr.hs_m[hs_idx][ch][0];
    let m2 = pr.hs_m[hs_idx][ch][1];

    for c in rgb.iter_mut() {
        *c = if zone_val < 1.0 {
            compute_hs_rev(*c, x0, x1, x2, y0, y1, y2, m0, m2)
        } else {
            compute_hs_fwd(*c, x0, x1, x2, y0, y1, y2, m0, m2)
        };
    }
}

/// Forward evaluation of faux-cubic highlight/shadow curve.
#[inline]
fn compute_hs_fwd(
    t: f32,
    x0: f32,
    x1: f32,
    x2: f32,
    y0: f32,
    y1: f32,
    y2: f32,
    m0: f32,
    m2: f32,
) -> f32 {
    let tl = (t - x0) / (x1 - x0);
    let tr = (t - x1) / (x2 - x1);
    let fl = y0 * (1.0 - tl * tl) + y1 * tl * tl + m0 * (1.0 - tl) * tl * (x1 - x0);
    let fr = y1 * (1.0 - tr) * (1.0 - tr) + y2 * (2.0 - tr) * tr + m2 * (tr - 1.0) * tr * (x2 - x1);

    let mut res = if t < x1 { fl } else { fr };
    if t < x0 {
        res = (t - x0) * m0 + y0;
    }
    if t > x2 {
        res = (t - x2) * m2 + y2;
    }
    res
}

/// Reverse evaluation of faux-cubic highlight/shadow curve.
#[inline]
fn compute_hs_rev(
    t: f32,
    x0: f32,
    x1: f32,
    x2: f32,
    y0: f32,
    y1: f32,
    y2: f32,
    m0: f32,
    m2: f32,
) -> f32 {
    let bl = m0 * (x1 - x0);
    let al = y1 - y0 - m0 * (x1 - x0);
    let cl = y0 - t;
    let discrim_l = (bl * bl - 4.0 * al * cl).sqrt();
    let out_l = (-2.0 * cl) / (discrim_l + bl) * (x1 - x0) + x0;

    let br = 2.0 * y2 - 2.0 * y1 - m2 * (x2 - x1);
    let ar = y1 - y2 + m2 * (x2 - x1);
    let cr = y1 - t;
    let discrim_r = (br * br - 4.0 * ar * cr).sqrt();
    let out_r = (-2.0 * cr) / (discrim_r + br) * (x2 - x1) + x1;

    let mut res = if t < y1 { out_l } else { out_r };
    if t < y0 {
        res = (t - y0) / m0 + x0;
    }
    if t > y2 {
        res = (t - y2) / m2 + x2;
    }
    res
}

// ============================================================================
// Whites/Blacks: 2-point curves with gain
// ============================================================================

/// Evaluate white/black curve forward (single channel).
#[inline]
pub fn wb_fwd_single(
    pr: &GradingTonePreRender,
    tone: &GradingTone,
    channel: RGBMChannel,
    is_black: bool,
    val: f32,
) -> f32 {
    let zone = if is_black { &tone.blacks } else { &tone.whites };
    let zone_val = (zone.get(channel) as f32).clamp(0.01, 1.99);

    let mtest = if !is_black { zone_val } else { 2.0 - zone_val };

    if mtest == 1.0 {
        return val;
    }

    let wb_idx = if is_black { 1 } else { 0 };
    let ch = channel as usize;

    let x0 = pr.wb_x[wb_idx][ch][0];
    let x1 = pr.wb_x[wb_idx][ch][1];
    let y0 = pr.wb_y[wb_idx][ch][0];
    let y1 = pr.wb_y[wb_idx][ch][1];
    let m0 = pr.wb_m[wb_idx][ch][0];
    let m1 = pr.wb_m[wb_idx][ch][1];
    let gain = pr.wb_gain[wb_idx][ch];

    compute_wb_fwd(val, is_black, zone_val, x0, x1, y0, y1, m0, m1, gain)
}

/// Evaluate white/black curve forward (RGB channels, for Master).
#[inline]
pub fn wb_fwd_rgb(
    pr: &GradingTonePreRender,
    tone: &GradingTone,
    is_black: bool,
    rgb: &mut [f32; 3],
) {
    let zone = if is_black { &tone.blacks } else { &tone.whites };
    let zone_val = (zone.master as f32).clamp(0.01, 1.99);

    let mtest = if !is_black { zone_val } else { 2.0 - zone_val };

    if mtest == 1.0 {
        return;
    }

    let wb_idx = if is_black { 1 } else { 0 };
    let ch = RGBMChannel::M as usize;

    let x0 = pr.wb_x[wb_idx][ch][0];
    let x1 = pr.wb_x[wb_idx][ch][1];
    let y0 = pr.wb_y[wb_idx][ch][0];
    let y1 = pr.wb_y[wb_idx][ch][1];
    let m0 = pr.wb_m[wb_idx][ch][0];
    let m1 = pr.wb_m[wb_idx][ch][1];
    let gain = pr.wb_gain[wb_idx][ch];

    for c in rgb.iter_mut() {
        *c = compute_wb_fwd(*c, is_black, zone_val, x0, x1, y0, y1, m0, m1, gain);
    }
}

/// Evaluate white/black curve reverse (single channel).
#[inline]
pub fn wb_rev_single(
    pr: &GradingTonePreRender,
    tone: &GradingTone,
    channel: RGBMChannel,
    is_black: bool,
    val: f32,
) -> f32 {
    let zone = if is_black { &tone.blacks } else { &tone.whites };
    let zone_val = (zone.get(channel) as f32).clamp(0.01, 1.99);

    let mtest = if !is_black { zone_val } else { 2.0 - zone_val };

    if mtest == 1.0 {
        return val;
    }

    let wb_idx = if is_black { 1 } else { 0 };
    let ch = channel as usize;

    let x0 = pr.wb_x[wb_idx][ch][0];
    let x1 = pr.wb_x[wb_idx][ch][1];
    let y0 = pr.wb_y[wb_idx][ch][0];
    let y1 = pr.wb_y[wb_idx][ch][1];
    let m0 = pr.wb_m[wb_idx][ch][0];
    let m1 = pr.wb_m[wb_idx][ch][1];
    let gain = pr.wb_gain[wb_idx][ch];

    compute_wb_rev(val, is_black, zone_val, x0, x1, y0, y1, m0, m1, gain)
}

/// Evaluate white/black curve reverse (RGB channels, for Master).
#[inline]
pub fn wb_rev_rgb(
    pr: &GradingTonePreRender,
    tone: &GradingTone,
    is_black: bool,
    rgb: &mut [f32; 3],
) {
    let zone = if is_black { &tone.blacks } else { &tone.whites };
    let zone_val = (zone.master as f32).clamp(0.01, 1.99);

    let mtest = if !is_black { zone_val } else { 2.0 - zone_val };

    if mtest == 1.0 {
        return;
    }

    let wb_idx = if is_black { 1 } else { 0 };
    let ch = RGBMChannel::M as usize;

    let x0 = pr.wb_x[wb_idx][ch][0];
    let x1 = pr.wb_x[wb_idx][ch][1];
    let y0 = pr.wb_y[wb_idx][ch][0];
    let y1 = pr.wb_y[wb_idx][ch][1];
    let m0 = pr.wb_m[wb_idx][ch][0];
    let m1 = pr.wb_m[wb_idx][ch][1];
    let gain = pr.wb_gain[wb_idx][ch];

    for c in rgb.iter_mut() {
        *c = compute_wb_rev(*c, is_black, zone_val, x0, x1, y0, y1, m0, m1, gain);
    }
}

/// Forward white/black curve computation.
#[inline]
fn compute_wb_fwd(
    mut t: f32,
    is_black: bool,
    val: f32,
    x0: f32,
    x1: f32,
    y0: f32,
    y1: f32,
    m0: f32,
    m1: f32,
    gain: f32,
) -> f32 {
    let mtest = if !is_black { val } else { 2.0 - val };

    if mtest < 1.0 {
        // Slope is decreasing
        let tlocal = (t - x0) / (x1 - x0);
        let mut res = tlocal * (x1 - x0) * (tlocal * 0.5 * (m1 - m0) + m0) + y0;
        if t < x0 {
            res = y0 + (t - x0) * m0;
        }
        if t > x1 {
            res = y1 + (t - x1) * m1;
        }
        res
    } else if mtest > 1.0 {
        // Slope is increasing
        t = if !is_black {
            (t - x0) * gain + x0
        } else {
            (t - x1) * gain + x1
        };

        let a = 0.5 * (m1 - m0) * (x1 - x0);
        let b = m0 * (x1 - x0);
        let c = y0 - t;
        let discrim = (b * b - 4.0 * a * c).sqrt();
        let tmp = (-2.0 * c) / (discrim + b);
        let mut res = tmp * (x1 - x0) + x0;

        if t < y0 {
            res = x0 + (t - y0) / m0;
        }

        if !is_black {
            res = (res - x0) / gain + x0;
            // Quadratic extrapolation for HDR
            let new_y1 = (x1 - x0) / gain + x0;
            let xd = x0 + (x1 - x0) * 0.99;
            let mut md = m0 + (xd - x0) * (m1 - m0) / (x1 - x0);
            md = 1.0 / md;
            let aa = 0.5 * (1.0 / m1 - md) / (x1 - xd);
            let bb = 1.0 / m1 - 2.0 * aa * x1;
            let cc = new_y1 - bb * x1 - aa * x1 * x1;
            let t_orig = (t - x0) / gain + x0;

            if t_orig > x1 {
                res = (aa * t_orig + bb) * t_orig + cc;
            }
        } else {
            if t > y1 {
                res = x1 + (t - y1) / m1;
            }
            res = (res - x1) / gain + x1;
        }

        res
    } else {
        t
    }
}

/// Reverse white/black curve computation.
#[inline]
fn compute_wb_rev(
    mut t: f32,
    is_black: bool,
    val: f32,
    x0: f32,
    x1: f32,
    y0: f32,
    y1: f32,
    m0: f32,
    m1: f32,
    gain: f32,
) -> f32 {
    let mtest = if !is_black { val } else { 2.0 - val };

    if mtest < 1.0 {
        // Slope is decreasing - reverse
        let a = 0.5 * (m1 - m0) * (x1 - x0);
        let b = m0 * (x1 - x0);
        let c = y0 - t;
        let discrim = (b * b - 4.0 * a * c).sqrt();
        let tmp = (-2.0 * c) / (discrim + b);
        let mut res = tmp * (x1 - x0) + x0;

        if t < y0 {
            res = x0 + (t - y0) / m0;
        }
        if t > y1 {
            res = x1 + (t - y1) / m1;
        }
        res
    } else if mtest > 1.0 {
        // Slope is increasing - reverse
        t = if !is_black {
            (t - x0) * gain + x0
        } else {
            (t - x1) * gain + x1
        };

        let tlocal = (t - x0) / (x1 - x0);
        let mut res = tlocal * (x1 - x0) * (tlocal * 0.5 * (m1 - m0) + m0) + y0;

        if t < x0 {
            res = y0 + (t - x0) * m0;
        }

        if !is_black {
            res = (res - x0) / gain + x0;
            // Quadratic extrapolation for HDR
            let new_y1 = (x1 - x0) / gain + x0;
            let xd = x0 + (x1 - x0) * 0.99;
            let mut md = m0 + (xd - x0) * (m1 - m0) / (x1 - x0);
            md = 1.0 / md;
            let aa = 0.5 * (1.0 / m1 - md) / (x1 - xd);
            let bb = 1.0 / m1 - 2.0 * aa * x1;
            let cc = new_y1 - bb * x1 - aa * x1 * x1;
            let t_orig = (t - x0) / gain + x0;

            let brk = (aa * x1 + bb) * x1 + cc;
            if t_orig > brk {
                let c = cc - t_orig;
                let discrim = (bb * bb - 4.0 * aa * c).sqrt();
                res = (-2.0 * c) / (discrim + bb);
            }
        } else {
            if t > x1 {
                res = y1 + (t - x1) * m1;
            }
            res = (res - x1) / gain + x1;
        }

        res
    } else {
        t
    }
}

// ============================================================================
// S-Contrast: quadratic segments at top and bottom
// ============================================================================

/// Evaluate s-contrast forward.
#[inline]
pub fn scontrast_fwd(pr: &GradingTonePreRender, tone: &GradingTone, rgb: &mut [f32; 3]) {
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

    let pivot = pr.pivot;

    for c in rgb.iter_mut() {
        let t = *c;
        let mut out_color = (t - pivot) * contrast + pivot;

        // Top end
        {
            let x1 = pr.sc_x[0][1];
            let x2 = pr.sc_x[0][2];
            let y1 = pr.sc_y[0][1];
            let y2 = pr.sc_y[0][2];
            let m0 = pr.sc_m[0][0];
            let m3 = pr.sc_m[0][1];

            let tr = (t - x1) / (x2 - x1);
            let res = tr * (x2 - x1) * (tr * 0.5 * (m3 - m0) + m0) + y1;

            if t > x1 {
                out_color = res;
            }
            if t > x2 {
                out_color = y2 + (t - x2) * m3;
            }
        }

        // Bottom end
        {
            let x1 = pr.sc_x[1][1];
            let x2 = pr.sc_x[1][2];
            let y1 = pr.sc_y[1][1];
            let m0 = pr.sc_m[1][0];
            let m3 = pr.sc_m[1][1];

            let tr = (t - x1) / (x2 - x1);
            let res = tr * (x2 - x1) * (tr * 0.5 * (m3 - m0) + m0) + y1;

            if t < x2 {
                out_color = res;
            }
            if t < x1 {
                out_color = y1 + (t - x1) * m0;
            }
        }

        *c = out_color;
    }
}

/// Evaluate s-contrast reverse.
#[inline]
pub fn scontrast_rev(pr: &GradingTonePreRender, tone: &GradingTone, rgb: &mut [f32; 3]) {
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

    let pivot = pr.pivot;

    for c in rgb.iter_mut() {
        let t = *c;
        let mut out_color = (t - pivot) / contrast + pivot;

        // Top end
        {
            let x1 = pr.sc_x[0][1];
            let x2 = pr.sc_x[0][2];
            let y1 = pr.sc_y[0][1];
            let y2 = pr.sc_y[0][2];
            let m0 = pr.sc_m[0][0];
            let m3 = pr.sc_m[0][1];

            let b = m0 * (x2 - x1);
            let a = (m3 - m0) * 0.5 * (x2 - x1);
            let c_coeff = y1 - t;
            let discrim = (b * b - 4.0 * a * c_coeff).sqrt();
            let res = (x2 - x1) * (-2.0 * c_coeff) / (discrim + b) + x1;

            if t > y1 {
                out_color = res;
            }
            if t > y2 {
                out_color = x2 + (t - y2) / m3;
            }
        }

        // Bottom end
        {
            let x1 = pr.sc_x[1][1];
            let x2 = pr.sc_x[1][2];
            let y1 = pr.sc_y[1][1];
            let y2 = pr.sc_y[1][2];
            let m0 = pr.sc_m[1][0];
            let m3 = pr.sc_m[1][1];

            let b = m0 * (x2 - x1);
            let a = (m3 - m0) * 0.5 * (x2 - x1);
            let c_coeff = y1 - t;
            let discrim = (b * b - 4.0 * a * c_coeff).sqrt();
            let res = (x2 - x1) * (-2.0 * c_coeff) / (discrim + b) + x1;

            if t < y2 {
                out_color = res;
            }
            if t < y1 {
                out_color = x1 + (t - y1) / m0;
            }
        }

        *c = out_color;
    }
}
