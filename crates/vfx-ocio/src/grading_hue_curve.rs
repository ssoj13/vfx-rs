//! Inline grading hue curve types and apply logic for vfx-ocio.
//!
//! Avoids cyclic dependency on vfx-ops by duplicating the needed types.
//! Reference: OCIO GradingHueCurveOpCPU.cpp

/// Control point on a hue curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HueControlPoint {
    pub hue: f32,
    pub value: f32,
}

impl HueControlPoint {
    #[inline]
    pub fn new(hue: f32, value: f32) -> Self {
        Self { hue, value }
    }
}

/// A hue-based curve with linear interpolation.
#[derive(Debug, Clone)]
pub struct HueCurve {
    pub points: Vec<HueControlPoint>,
}

impl HueCurve {
    pub fn new(mut points: Vec<HueControlPoint>) -> Self {
        points.sort_by(|a, b| a.hue.partial_cmp(&b.hue).unwrap());
        Self { points }
    }

    /// Evaluate curve at hue using linear interpolation with wrap-around.
    pub fn evaluate(&self, hue: f32) -> f32 {
        if self.points.is_empty() { return 0.0; }
        if self.points.len() == 1 { return self.points[0].value; }

        let h = hue.rem_euclid(1.0);
        let mut i1 = 0;
        for (i, p) in self.points.iter().enumerate() {
            if p.hue > h { break; }
            i1 = i;
        }
        let i2 = (i1 + 1) % self.points.len();
        let p1 = &self.points[i1];
        let p2 = &self.points[i2];

        let h1 = p1.hue;
        let mut h2 = p2.hue;
        let mut target = h;
        if h2 < h1 {
            h2 += 1.0;
            if target < h1 { target += 1.0; }
        }

        let span = h2 - h1;
        if span.abs() < 1e-6 { return p1.value; }
        let t = (target - h1) / span;
        p1.value + t * (p2.value - p1.value)
    }
}

/// Grading style (HSY variant selector).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GradingStyle {
    #[default]
    Log,
    Linear,
    Video,
}

/// Full set of 8 grading hue curves.
#[derive(Debug, Clone)]
pub struct GradingHueCurves {
    pub style: GradingStyle,
    pub hue_hue: HueCurve,
    pub hue_sat: HueCurve,
    pub hue_lum: HueCurve,
    pub lum_sat: HueCurve,
    pub sat_sat: HueCurve,
    pub lum_lum: HueCurve,
    pub sat_lum: HueCurve,
    pub hue_fx: HueCurve,
}

impl GradingHueCurves {
    /// Check if all curves are identity.
    pub fn is_identity(&self) -> bool {
        let diag = |c: &HueCurve| c.points.iter().all(|p| (p.value - p.hue).abs() < 1e-6);
        let horiz = |c: &HueCurve, v: f32| c.points.iter().all(|p| (p.value - v).abs() < 1e-6);
        diag(&self.hue_hue)
            && horiz(&self.hue_sat, 1.0)
            && horiz(&self.hue_lum, 1.0)
            && horiz(&self.lum_sat, 1.0)
            && diag(&self.sat_sat)
            && diag(&self.lum_lum)
            && horiz(&self.sat_lum, 1.0)
            && horiz(&self.hue_fx, 0.0)
    }
}

// --- Lin-Log constants (OCIO LogLinConstants) ---
mod lin_log {
    pub const XBRK: f32 = 0.0041318374739483946;
    pub const SHIFT: f32 = -0.000157849851665374;
    pub const M: f32 = 1.0 / (0.18 + SHIFT);
    pub const GAIN: f32 = 363.034608563;
    pub const OFFS: f32 = -7.0;
    pub const YBRK: f32 = -5.5;
    pub const BASE2: f32 = 1.4426950408889634; // 1/ln(2)
}

#[inline]
fn lin_to_log(lum: f32) -> f32 {
    if lum < lin_log::XBRK {
        lum * lin_log::GAIN + lin_log::OFFS
    } else {
        lin_log::BASE2 * ((lum + lin_log::SHIFT) * lin_log::M).ln()
    }
}

#[inline]
fn log_to_lin(lum: f32) -> f32 {
    if lum < lin_log::YBRK {
        (lum - lin_log::OFFS) / lin_log::GAIN
    } else {
        (2.0_f32).powf(lum) * (0.18 + lin_log::SHIFT) - lin_log::SHIFT
    }
}

// --- HSY conversion (simplified inline, matching OCIO RGB_TO_HSY) ---
// Uses Rec.709 luma weights.
const LUMA_R: f32 = 0.2126;
const LUMA_G: f32 = 0.7152;
const LUMA_B: f32 = 0.0722;

pub(crate) fn rgb_to_hsy(rgb: [f32; 3]) -> [f32; 3] {
    let (r, g, b) = (rgb[0], rgb[1], rgb[2]);
    let y = LUMA_R * r + LUMA_G * g + LUMA_B * b;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let chroma = max - min;

    let h = if chroma < 1e-10 {
        0.0
    } else if max == r {
        ((g - b) / chroma).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / chroma + 2.0) / 6.0
    } else {
        ((r - g) / chroma + 4.0) / 6.0
    };

    let s = if max < 1e-10 { 0.0 } else { chroma / max };

    [h, s, y]
}

pub(crate) fn hsy_to_rgb(hsy: [f32; 3]) -> [f32; 3] {
    let (h, s, y) = (hsy[0], hsy[1], hsy[2]);

    if s < 1e-10 {
        return [y, y, y];
    }

    // Reconstruct via HSV-like approach then scale to match luminance
    let h6 = (h * 6.0).rem_euclid(6.0);
    let hi = h6 as u32;
    let f = h6 - hi as f32;

    // Temporary HSV with V=1, S=s
    let (r, g, b) = match hi {
        0 => (1.0, f, 0.0),
        1 => (1.0 - f, 1.0, 0.0),
        2 => (0.0, 1.0, f),
        3 => (0.0, 1.0 - f, 1.0),
        4 => (f, 0.0, 1.0),
        _ => (1.0, 0.0, 1.0 - f),
    };

    // Apply saturation
    let r = 1.0 - s * (1.0 - r);
    let g = 1.0 - s * (1.0 - g);
    let b = 1.0 - s * (1.0 - b);

    // Scale to match target luminance
    let cur_y = LUMA_R * r + LUMA_G * g + LUMA_B * b;
    if cur_y > 1e-10 {
        let scale = y / cur_y;
        [r * scale, g * scale, b * scale]
    } else {
        [0.0, 0.0, 0.0]
    }
}

/// Apply grading hue curves forward.
pub fn apply_hue_curves_fwd(curves: &GradingHueCurves, rgb: &mut [f32; 3]) {
    if curves.is_identity() { return; }

    let is_linear = curves.style == GradingStyle::Linear;
    let mut hsy = rgb_to_hsy(*rgb);

    // LinLog for Linear style
    if is_linear { hsy[2] = lin_to_log(hsy[2]); }

    // HUE_SAT gain
    let hue_sat_gain = curves.hue_sat.evaluate(hsy[0]).max(0.0);
    // HUE_LUM gain
    let hue_lum_gain_raw = curves.hue_lum.evaluate(hsy[0]).max(0.0);
    // HUE_HUE curve
    hsy[0] = curves.hue_hue.evaluate(hsy[0]);
    // SAT_SAT curve
    hsy[1] = curves.sat_sat.evaluate(hsy[1]).max(0.0);
    // LUM_SAT gain
    let lum_sat_gain = curves.lum_sat.evaluate(hsy[2]).max(0.0);
    // Apply saturation gain
    hsy[1] *= lum_sat_gain * hue_sat_gain;
    // SAT_LUM gain
    let sat_lum_gain = curves.sat_lum.evaluate(hsy[1]).max(0.0);
    // LUM_LUM curve
    hsy[2] = curves.lum_lum.evaluate(hsy[2]);

    // LogLin for Linear style
    if is_linear { hsy[2] = log_to_lin(hsy[2]); }

    // Limit hue-lum gain at low saturation
    let hue_lum_gain = 1.0 - (1.0 - hue_lum_gain_raw) * hsy[1].min(1.0);

    // Apply luminance gain
    if is_linear {
        hsy[2] *= hue_lum_gain * sat_lum_gain;
    } else {
        hsy[2] += (hue_lum_gain + sat_lum_gain - 2.0) * 0.1;
    }

    // HUE_FX curve
    hsy[0] = hsy[0] - hsy[0].floor();
    hsy[0] += curves.hue_fx.evaluate(hsy[0]);

    *rgb = hsy_to_rgb(hsy);
}

/// Apply grading hue curves reverse (approximate inverse).
pub fn apply_hue_curves_rev(curves: &GradingHueCurves, rgb: &mut [f32; 3]) {
    if curves.is_identity() { return; }

    let is_linear = curves.style == GradingStyle::Linear;
    let mut hsy = rgb_to_hsy(*rgb);

    // Invert HUE_FX
    hsy[0] = eval_curve_rev_hue(&curves.hue_fx, hsy[0]);
    // Invert HUE_HUE
    hsy[0] = eval_curve_rev_hue(&curves.hue_hue, hsy[0]);

    hsy[0] = hsy[0] - hsy[0].floor();
    let hue_sat_gain = curves.hue_sat.evaluate(hsy[0]).max(0.0);
    let hue_lum_gain_raw = curves.hue_lum.evaluate(hsy[0]).max(0.0);

    hsy[1] = hsy[1].max(0.0);
    let sat_lum_gain = curves.sat_lum.evaluate(hsy[1]).max(0.0);
    let hue_lum_gain = 1.0 - (1.0 - hue_lum_gain_raw) * hsy[1].min(1.0);

    // Invert luminance gain
    let lum_gain = hue_lum_gain * sat_lum_gain;
    if is_linear {
        hsy[2] /= lum_gain.max(0.01);
    } else {
        hsy[2] -= (hue_lum_gain + sat_lum_gain - 2.0) * 0.1;
    }

    if is_linear { hsy[2] = lin_to_log(hsy[2]); }
    hsy[2] = eval_curve_rev(&curves.lum_lum, hsy[2]);

    let lum_sat_gain = curves.lum_sat.evaluate(hsy[2]).max(0.0);
    if is_linear { hsy[2] = log_to_lin(hsy[2]); }

    // Invert saturation gain
    hsy[1] /= (lum_sat_gain * hue_sat_gain).max(0.01);
    hsy[1] = eval_curve_rev(&curves.sat_sat, hsy[1]).max(0.0);

    *rgb = hsy_to_rgb(hsy);
}

/// Newton-Raphson curve inversion.
fn eval_curve_rev(curve: &HueCurve, target: f32) -> f32 {
    let mut x = target;
    for _ in 0..8 {
        let y = curve.evaluate(x);
        let error = y - target;
        if error.abs() < 1e-5 { break; }
        let dx = 0.001;
        let y2 = curve.evaluate(x + dx);
        let deriv = (y2 - y) / dx;
        if deriv.abs() > 1e-6 { x -= error / deriv; } else { x -= error * 0.5; }
    }
    x
}

/// Newton-Raphson curve inversion for periodic hue.
fn eval_curve_rev_hue(curve: &HueCurve, target: f32) -> f32 {
    let mut h = target;
    for _ in 0..8 {
        let mapped = curve.evaluate(h);
        let error = (mapped - target).rem_euclid(1.0);
        let error = if error > 0.5 { error - 1.0 } else { error };
        if error.abs() < 1e-5 { break; }
        h = (h - error * 0.5).rem_euclid(1.0);
    }
    h
}
