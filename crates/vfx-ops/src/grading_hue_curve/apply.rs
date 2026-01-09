//! Apply functions for GradingHueCurve operation.
//!
//! Converts RGB to HSL, applies hue-based adjustments, converts back.

use super::types::GradingHueCurves;

/// Convert RGB to HSL.
///
/// Returns (hue, saturation, lightness) where hue is 0-1.
#[inline]
pub fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) * 0.5;

    if (max - min).abs() < 1e-6 {
        // Achromatic
        return (0.0, 0.0, l);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if (max - r).abs() < 1e-6 {
        let mut h = (g - b) / d;
        if g < b {
            h += 6.0;
        }
        h / 6.0
    } else if (max - g).abs() < 1e-6 {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };

    (h, s, l)
}

/// Convert HSL to RGB.
#[inline]
pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s.abs() < 1e-6 {
        return (l, l, l);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

    (r, g, b)
}

#[inline]
fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }

    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 0.5 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

/// Apply grading hue curves to RGB pixel (forward).
///
/// # Arguments
/// * `curves` - The hue curve adjustments
/// * `rgb` - RGB pixel to modify in-place
pub fn apply_hue_curves_fwd(curves: &GradingHueCurves, rgb: &mut [f32; 3]) {
    // Skip if identity
    if curves.is_identity() {
        return;
    }

    let (h, s, l) = rgb_to_hsl(rgb[0], rgb[1], rgb[2]);

    // Apply Hue vs Hue (hue shift)
    let hue_shift = curves.hue_vs_hue.evaluate(h);
    let new_h = (h + hue_shift).rem_euclid(1.0);

    // Apply Hue vs Sat (saturation multiplier)
    let sat_mult = curves.hue_vs_sat.evaluate(h);
    let new_s = (s * sat_mult).clamp(0.0, 1.0);

    // Apply Hue vs Lum (luminance offset)
    let lum_offset = curves.hue_vs_lum.evaluate(h);
    let new_l = (l + lum_offset).clamp(0.0, 1.0);

    let (r, g, b) = hsl_to_rgb(new_h, new_s, new_l);
    rgb[0] = r;
    rgb[1] = g;
    rgb[2] = b;
}

/// Apply grading hue curves to RGB pixel (reverse).
///
/// Note: Reverse is approximate for hue curves due to non-linearity.
pub fn apply_hue_curves_rev(curves: &GradingHueCurves, rgb: &mut [f32; 3]) {
    if curves.is_identity() {
        return;
    }

    let (h, s, l) = rgb_to_hsl(rgb[0], rgb[1], rgb[2]);

    // Reverse Hue vs Lum
    let lum_offset = curves.hue_vs_lum.evaluate(h);
    let orig_l = (l - lum_offset).clamp(0.0, 1.0);

    // Reverse Hue vs Sat
    let sat_mult = curves.hue_vs_sat.evaluate(h);
    let orig_s = if sat_mult.abs() > 1e-6 {
        (s / sat_mult).clamp(0.0, 1.0)
    } else {
        s
    };

    // Reverse Hue vs Hue (iterative search)
    let orig_h = find_original_hue(curves, h);

    let (r, g, b) = hsl_to_rgb(orig_h, orig_s, orig_l);
    rgb[0] = r;
    rgb[1] = g;
    rgb[2] = b;
}

/// Find original hue that maps to target hue (Newton-Raphson style).
fn find_original_hue(curves: &GradingHueCurves, target_h: f32) -> f32 {
    let mut h = target_h;
    
    // Simple iterative refinement
    for _ in 0..8 {
        let shift = curves.hue_vs_hue.evaluate(h);
        let mapped = (h + shift).rem_euclid(1.0);
        let error = (mapped - target_h).rem_euclid(1.0);
        
        // Adjust error for wrap-around
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
        // Alpha (chunk[3]) preserved
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::{HueCurve, HueControlPoint};

    const EPSILON: f32 = 0.02;

    #[test]
    fn test_rgb_hsl_roundtrip() {
        let test_colors = [
            [1.0, 0.0, 0.0],  // Red
            [0.0, 1.0, 0.0],  // Green
            [0.0, 0.0, 1.0],  // Blue
            [0.5, 0.5, 0.5],  // Gray
            [0.8, 0.2, 0.4],  // Pink
        ];

        for rgb in test_colors {
            let (h, s, l) = rgb_to_hsl(rgb[0], rgb[1], rgb[2]);
            let (r, g, b) = hsl_to_rgb(h, s, l);
            
            assert!((r - rgb[0]).abs() < EPSILON, "Red mismatch: {} vs {}", r, rgb[0]);
            assert!((g - rgb[1]).abs() < EPSILON, "Green mismatch: {} vs {}", g, rgb[1]);
            assert!((b - rgb[2]).abs() < EPSILON, "Blue mismatch: {} vs {}", b, rgb[2]);
        }
    }

    #[test]
    fn test_identity_passthrough() {
        let curves = GradingHueCurves::identity();
        
        let mut rgb = [0.5_f32, 0.3, 0.7];
        let original = rgb;
        
        apply_hue_curves_fwd(&curves, &mut rgb);
        
        assert!((rgb[0] - original[0]).abs() < EPSILON);
        assert!((rgb[1] - original[1]).abs() < EPSILON);
        assert!((rgb[2] - original[2]).abs() < EPSILON);
    }

    #[test]
    fn test_hue_shift() {
        let mut curves = GradingHueCurves::identity();
        
        // Shift all hues by 0.33 (120 degrees - red becomes green)
        curves.hue_vs_hue = HueCurve::new(vec![
            HueControlPoint::new(0.0, 0.333),
            HueControlPoint::new(1.0, 0.333),
        ]);

        let mut rgb = [1.0_f32, 0.0, 0.0]; // Pure red
        apply_hue_curves_fwd(&curves, &mut rgb);
        
        // Should be close to green
        assert!(rgb[1] > rgb[0] && rgb[1] > rgb[2], "Red should shift to green");
    }

    #[test]
    fn test_selective_desaturation() {
        let mut curves = GradingHueCurves::identity();
        
        // Desaturate reds (hue ~0.0)
        curves.hue_vs_sat = HueCurve::new(vec![
            HueControlPoint::new(0.0, 0.0),    // Full desat at red
            HueControlPoint::new(0.1, 1.0),   // Normal elsewhere
            HueControlPoint::new(0.9, 1.0),
        ]);

        let mut rgb = [1.0_f32, 0.0, 0.0]; // Pure red
        apply_hue_curves_fwd(&curves, &mut rgb);
        
        // Should be desaturated (gray)
        let spread = (rgb[0] - rgb[1]).abs() + (rgb[1] - rgb[2]).abs();
        assert!(spread < 0.1, "Red should be desaturated, spread={spread}");
    }

    #[test]
    fn test_roundtrip() {
        let mut curves = GradingHueCurves::identity();
        
        curves.hue_vs_hue = HueCurve::new(vec![
            HueControlPoint::new(0.0, 0.1),
            HueControlPoint::new(0.5, -0.1),
            HueControlPoint::new(1.0, 0.1),
        ]);

        let original = [0.6_f32, 0.3, 0.5];
        let mut rgb = original;
        
        apply_hue_curves_fwd(&curves, &mut rgb);
        apply_hue_curves_rev(&curves, &mut rgb);
        
        // Should be close to original (approximate due to non-linearity)
        assert!((rgb[0] - original[0]).abs() < 0.05);
        assert!((rgb[1] - original[1]).abs() < 0.05);
        assert!((rgb[2] - original[2]).abs() < 0.05);
    }
}
