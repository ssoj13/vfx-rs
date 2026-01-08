//! Apply functions for GradingRGBCurve operations.

use super::prerender::GradingRGBCurvePreRender;
use super::eval::{eval_curve, eval_curve_rev};
use super::types::RGBChannel;

/// Grading style for RGB curves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GradingStyle {
    /// Log-encoded input (no Lin-Log conversion).
    #[default]
    Log,
    /// Linear input (apply Lin-Log conversion).
    Linear,
    /// Video input (no Lin-Log conversion).
    Video,
}

// Lin-Log constants (same as GradingTone)
mod linlog {
    pub const XBRK: f32 = 0.0041318374739483946;
    pub const SHIFT: f32 = -0.000157849851665374;
    pub const M: f32 = 5.560976; // 1.0 / (0.18 + SHIFT)
    pub const GAIN: f32 = 363.034608563;
    pub const OFFS: f32 = -7.0;
    pub const YBRK: f32 = -5.5;
    pub const BASE2: f32 = 1.4426950408889634; // 1/ln(2)
}

/// Convert linear to log space.
#[inline]
fn lin_to_log(x: f32) -> f32 {
    if x < linlog::XBRK {
        x * linlog::GAIN + linlog::OFFS
    } else {
        linlog::BASE2 * ((x + linlog::SHIFT) * linlog::M).ln()
    }
}

/// Convert log to linear space.
#[inline]
fn log_to_lin(x: f32) -> f32 {
    if x < linlog::YBRK {
        (x - linlog::OFFS) / linlog::GAIN
    } else {
        2.0_f32.powf(x) * (0.18 + linlog::SHIFT) - linlog::SHIFT
    }
}

/// Apply RGB curves forward to a single RGB pixel.
/// 
/// Order: R curve → G curve → B curve → Master curve
#[inline]
pub fn apply_rgb_curves_fwd(pr: &GradingRGBCurvePreRender, rgb: &mut [f32; 3]) {
    // Apply per-channel curves
    rgb[0] = eval_curve(pr.get(RGBChannel::Red), rgb[0], rgb[0]);
    rgb[1] = eval_curve(pr.get(RGBChannel::Green), rgb[1], rgb[1]);
    rgb[2] = eval_curve(pr.get(RGBChannel::Blue), rgb[2], rgb[2]);
    
    // Apply master curve to all channels
    rgb[0] = eval_curve(pr.get(RGBChannel::Master), rgb[0], rgb[0]);
    rgb[1] = eval_curve(pr.get(RGBChannel::Master), rgb[1], rgb[1]);
    rgb[2] = eval_curve(pr.get(RGBChannel::Master), rgb[2], rgb[2]);
}

/// Apply RGB curves reverse to a single RGB pixel.
/// 
/// Order: Master curve⁻¹ → R curve⁻¹ → G curve⁻¹ → B curve⁻¹
#[inline]
pub fn apply_rgb_curves_rev(pr: &GradingRGBCurvePreRender, rgb: &mut [f32; 3]) {
    // Reverse master curve first
    rgb[0] = eval_curve_rev(pr.get(RGBChannel::Master), rgb[0]);
    rgb[1] = eval_curve_rev(pr.get(RGBChannel::Master), rgb[1]);
    rgb[2] = eval_curve_rev(pr.get(RGBChannel::Master), rgb[2]);
    
    // Reverse per-channel curves
    rgb[0] = eval_curve_rev(pr.get(RGBChannel::Red), rgb[0]);
    rgb[1] = eval_curve_rev(pr.get(RGBChannel::Green), rgb[1]);
    rgb[2] = eval_curve_rev(pr.get(RGBChannel::Blue), rgb[2]);
}

/// Apply grading RGB curves to a single pixel.
/// 
/// Handles Lin-Log conversion for LINEAR style.
pub fn apply_grading_rgb_curve(
    style: GradingStyle,
    pr: &GradingRGBCurvePreRender,
    forward: bool,
    rgb: &mut [f32; 3],
) {
    if pr.is_bypass() {
        return;
    }
    
    let use_linlog = style == GradingStyle::Linear;
    
    if use_linlog {
        // Convert to log space
        rgb[0] = lin_to_log(rgb[0]);
        rgb[1] = lin_to_log(rgb[1]);
        rgb[2] = lin_to_log(rgb[2]);
    }
    
    if forward {
        apply_rgb_curves_fwd(pr, rgb);
    } else {
        apply_rgb_curves_rev(pr, rgb);
    }
    
    if use_linlog {
        // Convert back to linear space
        rgb[0] = log_to_lin(rgb[0]);
        rgb[1] = log_to_lin(rgb[1]);
        rgb[2] = log_to_lin(rgb[2]);
    }
}

/// Apply grading RGB curves to an RGBA buffer.
/// 
/// Alpha channel is passed through unchanged.
pub fn apply_grading_rgb_curve_rgba(
    style: GradingStyle,
    pr: &GradingRGBCurvePreRender,
    forward: bool,
    pixels: &mut [f32],
) {
    if pr.is_bypass() {
        return;
    }
    
    for chunk in pixels.chunks_exact_mut(4) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        apply_grading_rgb_curve(style, pr, forward, &mut rgb);
        chunk[0] = rgb[0];
        chunk[1] = rgb[1];
        chunk[2] = rgb[2];
        // Alpha unchanged
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::{GradingRGBCurves, BSplineCurve, ControlPoint};
    use super::super::prerender::GradingRGBCurvePreRender;
    
    #[test]
    fn test_identity_passthrough() {
        let curves = GradingRGBCurves::identity();
        let pr = GradingRGBCurvePreRender::new(&curves);
        
        let mut rgb = [0.18_f32, 0.5, 0.8];
        let original = rgb.clone();
        
        apply_grading_rgb_curve(GradingStyle::Log, &pr, true, &mut rgb);
        
        assert_eq!(rgb, original);
    }
    
    #[test]
    fn test_forward_reverse_roundtrip() {
        let mut curves = GradingRGBCurves::identity();
        
        // Create a non-identity curve (lift shadows)
        curves.curves[3] = BSplineCurve::new(vec![
            ControlPoint::new(0.0, 0.1),
            ControlPoint::new(0.5, 0.55),
            ControlPoint::new(1.0, 1.0),
        ]);
        
        let pr = GradingRGBCurvePreRender::new(&curves);
        
        let original = [0.18_f32, 0.5, 0.8];
        let mut rgb = original.clone();
        
        // Forward
        apply_grading_rgb_curve(GradingStyle::Log, &pr, true, &mut rgb);
        
        // Reverse
        apply_grading_rgb_curve(GradingStyle::Log, &pr, false, &mut rgb);
        
        for i in 0..3 {
            assert!(
                (rgb[i] - original[i]).abs() < 0.01,
                "Channel {i} roundtrip failed: original={}, got={}",
                original[i], rgb[i]
            );
        }
    }
    
    #[test]
    fn test_linear_style_linlog() {
        let mut curves = GradingRGBCurves::identity();
        
        // Create a simple gain curve
        curves.curves[3] = BSplineCurve::new(vec![
            ControlPoint::new(-8.0, -8.0),
            ControlPoint::new(0.0, 0.1),  // Lift at 0
            ControlPoint::new(8.0, 8.0),
        ]);
        
        let pr = GradingRGBCurvePreRender::new(&curves);
        
        let mut rgb = [0.18_f32, 0.18, 0.18];
        
        apply_grading_rgb_curve(GradingStyle::Linear, &pr, true, &mut rgb);
        
        // Values should be modified (lifted)
        assert!(rgb[0] > 0.18, "Linear style should lift midtones");
    }
    
    #[test]
    fn test_rgba_buffer() {
        let mut curves = GradingRGBCurves::identity();
        curves.curves[0] = BSplineCurve::new(vec![
            ControlPoint::new(0.0, 0.0),
            ControlPoint::new(0.5, 0.6),
            ControlPoint::new(1.0, 1.0),
        ]);
        
        let pr = GradingRGBCurvePreRender::new(&curves);
        
        let mut pixels = vec![0.5, 0.5, 0.5, 1.0, 0.3, 0.3, 0.3, 0.8];
        
        apply_grading_rgb_curve_rgba(GradingStyle::Log, &pr, true, &mut pixels);
        
        // Red channel should be modified, others unchanged
        assert!(pixels[0] > 0.5, "Red should be lifted");
        assert!((pixels[1] - 0.5).abs() < 0.01, "Green unchanged");
        assert!((pixels[3] - 1.0).abs() < 0.001, "Alpha unchanged");
    }
}
