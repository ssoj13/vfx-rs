//! Apply functions for GradingHueCurve operation.
//!
//! Reference: OCIO GradingHueCurveOpCPU.cpp
//!
//! Uses HSY color space, applies 8 curve types, supports Lin-Log transforms.

use super::types::{GradingHueCurves, GradingStyle};
use crate::fixed_function::{rgb_to_hsy, hsy_to_rgb};

// Lin-Log constants for Linear style (OCIO LogLinConstants)
mod lin_log {
    pub const XBRK: f32 = 0.0041318374739483946;
    pub const SHIFT: f32 = -0.000157849851665374;
    pub const M: f32 = 1.0 / (0.18 + SHIFT);
    pub const GAIN: f32 = 363.034608563;
    pub const OFFS: f32 = -7.0;
    pub const YBRK: f32 = -5.5;
    pub const BASE2: f32 = 1.4426950408889634; // 1/ln(2)
}

/// Lin -> Log transform for Linear style (applied to luminance).
#[inline]
fn lin_to_log(lum: f32) -> f32 {
    if lum < lin_log::XBRK {
        lum * lin_log::GAIN + lin_log::OFFS
    } else {
        lin_log::BASE2 * ((lum + lin_log::SHIFT) * lin_log::M).ln()
    }
}

/// Log -> Lin transform for Linear style (applied to luminance).
#[inline]
fn log_to_lin(lum: f32) -> f32 {
    if lum < lin_log::YBRK {
        (lum - lin_log::OFFS) / lin_log::GAIN
    } else {
        (2.0_f32).powf(lum) * (0.18 + lin_log::SHIFT) - lin_log::SHIFT
    }
}

/// Apply grading hue curves to RGB pixel (forward).
///
/// Reference: OCIO GradingHueCurveFwdOpCPU::apply
///
/// # Algorithm
/// 1. RGB -> HSY (using style's HSY variant)
/// 2. LinLog (if Linear style) - transform luminance
/// 3. Evaluate curves and apply gains:
///    - HUE_SAT, HUE_LUM gains from hue
///    - HUE_HUE maps hue
///    - SAT_SAT maps saturation
///    - LUM_SAT gain from luminance
///    - Apply saturation gain (lumSat * hueSat)
///    - SAT_LUM gain from (modified) saturation
///    - LUM_LUM maps luminance
/// 4. LogLin (if Linear style)
/// 5. Apply luminance gain (with low-sat limiting)
/// 6. HUE_FX curve
/// 7. HSY -> RGB
pub fn apply_hue_curves_fwd(curves: &GradingHueCurves, rgb: &mut [f32; 3]) {
    if curves.is_identity() {
        return;
    }

    let variant = curves.style.hsy_variant();
    let is_linear = curves.style == GradingStyle::Linear;

    // RGB -> HSY
    let mut hsy = rgb_to_hsy(*rgb, variant);

    // LinLog for Linear style (on luminance channel)
    if is_linear {
        hsy[2] = lin_to_log(hsy[2]);
    }

    // HUE_SAT gain
    let hue_sat_gain = curves.hue_sat.evaluate(hsy[0]).max(0.0);

    // HUE_LUM gain
    let hue_lum_gain_raw = curves.hue_lum.evaluate(hsy[0]).max(0.0);

    // HUE_HUE curve (maps hue)
    hsy[0] = curves.hue_hue.evaluate(hsy[0]);

    // SAT_SAT curve (maps saturation)
    hsy[1] = curves.sat_sat.evaluate(hsy[1]).max(0.0);

    // LUM_SAT gain
    let lum_sat_gain = curves.lum_sat.evaluate(hsy[2]).max(0.0);

    // Apply saturation gain
    let sat_gain = lum_sat_gain * hue_sat_gain;
    hsy[1] *= sat_gain;

    // SAT_LUM gain (from modified saturation)
    let sat_lum_gain = curves.sat_lum.evaluate(hsy[1]).max(0.0);

    // LUM_LUM curve (maps luminance)
    hsy[2] = curves.lum_lum.evaluate(hsy[2]);

    // LogLin for Linear style
    if is_linear {
        hsy[2] = log_to_lin(hsy[2]);
    }

    // Limit hue-lum gain at low saturation (hue is noisy at low sat)
    let hue_lum_gain = 1.0 - (1.0 - hue_lum_gain_raw) * hsy[1].min(1.0);

    // Apply luminance gain
    if is_linear {
        hsy[2] *= hue_lum_gain * sat_lum_gain;
    } else {
        hsy[2] += (hue_lum_gain + sat_lum_gain - 2.0) * 0.1;
    }

    // HUE_FX curve
    hsy[0] = hsy[0] - hsy[0].floor(); // wrap to [0,1)
    hsy[0] += curves.hue_fx.evaluate(hsy[0]);

    // HSY -> RGB
    *rgb = hsy_to_rgb(hsy, variant);
}

/// Apply grading hue curves to RGB pixel (reverse/inverse).
///
/// Reference: OCIO GradingHueCurveRevOpCPU::apply
///
/// Note: Reverse is approximate due to curve non-invertibility.
pub fn apply_hue_curves_rev(curves: &GradingHueCurves, rgb: &mut [f32; 3]) {
    if curves.is_identity() {
        return;
    }

    let variant = curves.style.hsy_variant();
    let is_linear = curves.style == GradingStyle::Linear;

    // RGB -> HSY
    let mut hsy = rgb_to_hsy(*rgb, variant);

    // Invert HUE_FX
    hsy[0] = eval_curve_rev_hue(&curves.hue_fx, hsy[0]);

    // Invert HUE_HUE
    hsy[0] = eval_curve_rev_hue(&curves.hue_hue, hsy[0]);

    // Use inverted hue for HUE_SAT and HUE_LUM gains
    hsy[0] = hsy[0] - hsy[0].floor(); // wrap to [0,1)
    let hue_sat_gain = curves.hue_sat.evaluate(hsy[0]).max(0.0);
    let hue_lum_gain_raw = curves.hue_lum.evaluate(hsy[0]).max(0.0);

    // SAT_LUM gain from output saturation
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

    // LinLog for Linear style
    if is_linear {
        hsy[2] = lin_to_log(hsy[2]);
    }

    // Invert LUM_LUM
    hsy[2] = eval_curve_rev(&curves.lum_lum, hsy[2]);

    // LUM_SAT gain from inverted luminance
    let lum_sat_gain = curves.lum_sat.evaluate(hsy[2]).max(0.0);

    // LogLin for Linear style
    if is_linear {
        hsy[2] = log_to_lin(hsy[2]);
    }

    // Invert saturation gain
    let sat_gain = lum_sat_gain * hue_sat_gain;
    hsy[1] /= sat_gain.max(0.01);

    // Invert SAT_SAT
    hsy[1] = eval_curve_rev(&curves.sat_sat, hsy[1]).max(0.0);

    // HSY -> RGB
    *rgb = hsy_to_rgb(hsy, variant);
}

/// Evaluate curve in reverse (Newton-Raphson iteration).
fn eval_curve_rev(curve: &super::types::HueCurve, target: f32) -> f32 {
    let mut x = target;

    for _ in 0..8 {
        let y = curve.evaluate(x);
        let error = y - target;

        if error.abs() < 1e-5 {
            break;
        }

        // Approximate derivative
        let dx = 0.001;
        let y2 = curve.evaluate(x + dx);
        let deriv = (y2 - y) / dx;

        if deriv.abs() > 1e-6 {
            x -= error / deriv;
        } else {
            x -= error * 0.5;
        }
    }

    x
}

/// Evaluate curve in reverse for periodic hue (handles wrap-around).
fn eval_curve_rev_hue(curve: &super::types::HueCurve, target: f32) -> f32 {
    let mut h = target;

    for _ in 0..8 {
        let mapped = curve.evaluate(h);
        let error = (mapped - target).rem_euclid(1.0);
        let error = if error > 0.5 { error - 1.0 } else { error };

        if error.abs() < 1e-5 {
            break;
        }

        h = (h - error * 0.5).rem_euclid(1.0);
    }

    h
}

/// Apply grading hue curves to RGB buffer.
pub fn apply_hue_curves_rgb(curves: &GradingHueCurves, forward: bool, buffer: &mut [f32]) {
    let apply_fn = if forward { apply_hue_curves_fwd } else { apply_hue_curves_rev };

    for chunk in buffer.chunks_exact_mut(3) {
        let rgb: &mut [f32; 3] = chunk.try_into().unwrap();
        apply_fn(curves, rgb);
    }
}

/// Apply grading hue curves to RGBA buffer (alpha preserved).
pub fn apply_hue_curves_rgba(curves: &GradingHueCurves, forward: bool, buffer: &mut [f32]) {
    let apply_fn = if forward { apply_hue_curves_fwd } else { apply_hue_curves_rev };

    for chunk in buffer.chunks_exact_mut(4) {
        let rgb: &mut [f32; 3] = (&mut chunk[0..3]).try_into().unwrap();
        apply_fn(curves, rgb);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::{HueCurve, HueControlPoint};

    const EPSILON: f32 = 0.05;

    #[test]
    fn test_identity_passthrough() {
        let curves = GradingHueCurves::default();

        let mut rgb = [0.5_f32, 0.3, 0.7];
        let original = rgb;

        apply_hue_curves_fwd(&curves, &mut rgb);

        assert!((rgb[0] - original[0]).abs() < EPSILON, "R: {} vs {}", rgb[0], original[0]);
        assert!((rgb[1] - original[1]).abs() < EPSILON, "G: {} vs {}", rgb[1], original[1]);
        assert!((rgb[2] - original[2]).abs() < EPSILON, "B: {} vs {}", rgb[2], original[2]);
    }

    #[test]
    fn test_lin_log_roundtrip() {
        let values = [0.001, 0.01, 0.1, 0.18, 0.5, 1.0, 2.0];

        for &v in &values {
            let log = lin_to_log(v);
            let back = log_to_lin(log);
            assert!((back - v).abs() < 1e-4, "Lin-Log roundtrip failed: {} -> {} -> {}", v, log, back);
        }
    }

    #[test]
    fn test_hue_shift() {
        let mut curves = GradingHueCurves::default();

        // Shift all hues by ~0.33 (diagonal + offset)
        curves.hue_hue = HueCurve::new(vec![
            HueControlPoint::new(0.0, 0.333),
            HueControlPoint::new(0.5, 0.833),
            HueControlPoint::new(1.0, 1.333),
        ]);

        let mut rgb = [1.0_f32, 0.0, 0.0]; // Pure red
        apply_hue_curves_fwd(&curves, &mut rgb);

        // Red shifted toward green
        assert!(rgb[1] > rgb[0] * 0.5 || rgb[1] > rgb[2],
                "Hue shift didn't work: {:?}", rgb);
    }

    #[test]
    fn test_selective_desaturation() {
        let mut curves = GradingHueCurves::default();

        // Desaturate around hue=0 (reds/magentas in HSY)
        curves.hue_sat = HueCurve::new(vec![
            HueControlPoint::new(0.0, 0.0),
            HueControlPoint::new(0.1, 1.0),
            HueControlPoint::new(0.5, 1.0),
            HueControlPoint::new(0.9, 1.0),
        ]);

        // Use a saturated reddish color
        let mut rgb = [0.9_f32, 0.1, 0.3];
        let original = rgb;
        apply_hue_curves_fwd(&curves, &mut rgb);

        // Should be more desaturated (values closer together)
        let orig_spread = (original[0] - original[1]).abs() + (original[1] - original[2]).abs();
        let new_spread = (rgb[0] - rgb[1]).abs() + (rgb[1] - rgb[2]).abs();
        assert!(new_spread < orig_spread,
                "Expected desaturation, spread {} -> {}", orig_spread, new_spread);
    }

    #[test]
    fn test_roundtrip_approx() {
        let mut curves = GradingHueCurves::default();

        // Small adjustments for better invertibility
        curves.hue_hue = HueCurve::new(vec![
            HueControlPoint::new(0.0, 0.02),
            HueControlPoint::new(0.5, 0.52),
            HueControlPoint::new(1.0, 1.02),
        ]);

        let original = [0.5_f32, 0.4, 0.5];
        let mut rgb = original;

        apply_hue_curves_fwd(&curves, &mut rgb);
        apply_hue_curves_rev(&curves, &mut rgb);

        // Approximate roundtrip - inverse is inherently approximate for nonlinear curves
        // OCIO also documents this as approximate, not exact
        assert!((rgb[0] - original[0]).abs() < 0.2, "R roundtrip: {} vs {}", rgb[0], original[0]);
        assert!((rgb[1] - original[1]).abs() < 0.2, "G roundtrip: {} vs {}", rgb[1], original[1]);
        assert!((rgb[2] - original[2]).abs() < 0.2, "B roundtrip: {} vs {}", rgb[2], original[2]);
    }

    #[test]
    fn test_linear_style() {
        let curves = GradingHueCurves::identity(GradingStyle::Linear);

        let mut rgb = [0.5_f32, 0.3, 0.7];
        let original = rgb;

        apply_hue_curves_fwd(&curves, &mut rgb);

        // Identity should pass through
        assert!((rgb[0] - original[0]).abs() < EPSILON);
        assert!((rgb[1] - original[1]).abs() < EPSILON);
        assert!((rgb[2] - original[2]).abs() < EPSILON);
    }

    #[test]
    fn test_video_style() {
        let curves = GradingHueCurves::identity(GradingStyle::Video);

        let mut rgb = [0.5_f32, 0.3, 0.7];
        let original = rgb;

        apply_hue_curves_fwd(&curves, &mut rgb);

        assert!((rgb[0] - original[0]).abs() < EPSILON);
        assert!((rgb[1] - original[1]).abs() < EPSILON);
        assert!((rgb[2] - original[2]).abs() < EPSILON);
    }
}
