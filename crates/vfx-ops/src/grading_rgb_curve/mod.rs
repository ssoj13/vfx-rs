//! GradingRGBCurve operation - user-adjustable RGB curves with B-spline interpolation.
//!
//! This module implements OCIO-compatible RGB curve grading with:
//! - Per-channel curves (Red, Green, Blue) plus Master curve
//! - B-spline interpolation for smooth curves
//! - Automatic slope estimation at control points
//! - Forward and inverse (reverse) evaluation
//! - Lin-Log conversion for LINEAR grading style
//!
//! # Algorithm
//!
//! The RGB curve system uses piecewise quadratic polynomials for smooth interpolation:
//!
//! 1. **Control Points**: User specifies (x, y) points on the curve
//! 2. **Slope Estimation**: Slopes at each control point are estimated using
//!    weighted averaging of adjacent secant slopes
//! 3. **Spline Fitting**: For each segment, if a single quadratic is insufficient
//!    (slopes don't match 2× secant), the segment is split at an optimal point
//! 4. **Evaluation**: `y = A*(x-x₀)² + B*(x-x₀) + C` for the containing segment
//! 5. **Inversion**: Uses the quadratic formula for monotonic curves
//!
//! # Example
//!
//! ```
//! use vfx_ops::grading_rgb_curve::*;
//!
//! // Create curves with a lifted master curve
//! let mut curves = GradingRGBCurves::identity();
//! curves.curves[3] = BSplineCurve::new(vec![
//!     ControlPoint::new(0.0, 0.1),   // Lift blacks
//!     ControlPoint::new(0.5, 0.55),  // Slight midtone boost
//!     ControlPoint::new(1.0, 1.0),   // Preserve whites
//! ]);
//!
//! // Pre-render for efficient evaluation
//! let pr = GradingRGBCurvePreRender::new(&curves);
//!
//! // Apply to a pixel
//! let mut rgb = [0.18_f32, 0.18, 0.18];
//! apply_grading_rgb_curve(GradingStyle::Log, &pr, true, &mut rgb);
//! ```
//!
//! # Reference
//!
//! Based on OCIO `ops/gradingrgbcurve/` implementation.

mod types;
mod slopes;
mod spline;
mod eval;
mod prerender;
mod apply;

// Re-export public API
pub use types::{ControlPoint, BSplineCurve, GradingRGBCurves, RGBChannel, NUM_RGB_CURVES};
pub use spline::SplineData;
pub use prerender::GradingRGBCurvePreRender;
pub use apply::{
    GradingStyle,
    apply_grading_rgb_curve,
    apply_grading_rgb_curve_rgba,
    apply_rgb_curves_fwd,
    apply_rgb_curves_rev,
};
pub use eval::{eval_curve, eval_curve_rev};

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-3;

    // ========================================================================
    // Identity tests
    // ========================================================================

    #[test]
    fn test_identity_is_bypass() {
        let curves = GradingRGBCurves::identity();
        assert!(curves.is_identity());
        
        let pr = GradingRGBCurvePreRender::new(&curves);
        assert!(pr.is_bypass());
    }

    #[test]
    fn test_identity_passthrough() {
        let curves = GradingRGBCurves::identity();
        let pr = GradingRGBCurvePreRender::new(&curves);

        let test_vals = [0.0, 0.1, 0.18, 0.5, 0.9, 1.0];
        for &v in &test_vals {
            let mut rgb = [v, v, v];
            apply_grading_rgb_curve(GradingStyle::Log, &pr, true, &mut rgb);
            
            for i in 0..3 {
                assert!(
                    (rgb[i] - v).abs() < EPSILON,
                    "Identity failed for v={v}: got rgb[{i}]={}", rgb[i]
                );
            }
        }
    }

    // ========================================================================
    // Curve modification tests
    // ========================================================================

    #[test]
    fn test_master_curve_affects_all_channels() {
        let mut curves = GradingRGBCurves::identity();
        
        // Master curve: lift shadows
        curves.curves[RGBChannel::Master as usize] = BSplineCurve::new(vec![
            ControlPoint::new(0.0, 0.1),
            ControlPoint::new(1.0, 1.0),
        ]);

        let pr = GradingRGBCurvePreRender::new(&curves);
        
        let mut rgb = [0.0_f32, 0.0, 0.0];
        apply_grading_rgb_curve(GradingStyle::Log, &pr, true, &mut rgb);
        
        // All channels should be lifted to ~0.1
        for i in 0..3 {
            assert!(
                (rgb[i] - 0.1).abs() < 0.05,
                "Master curve should lift channel {i}: got {}", rgb[i]
            );
        }
    }

    #[test]
    fn test_per_channel_independence() {
        let mut curves = GradingRGBCurves::identity();
        
        // Only modify red channel
        curves.curves[RGBChannel::Red as usize] = BSplineCurve::new(vec![
            ControlPoint::new(0.0, 0.0),
            ControlPoint::new(0.5, 0.7),  // Lift midtones
            ControlPoint::new(1.0, 1.0),
        ]);

        let pr = GradingRGBCurvePreRender::new(&curves);
        
        let mut rgb = [0.5_f32, 0.5, 0.5];
        apply_grading_rgb_curve(GradingStyle::Log, &pr, true, &mut rgb);
        
        // Red should be lifted, G and B unchanged
        assert!(rgb[0] > 0.6, "Red should be lifted: got {}", rgb[0]);
        assert!((rgb[1] - 0.5).abs() < EPSILON, "Green should be unchanged: got {}", rgb[1]);
        assert!((rgb[2] - 0.5).abs() < EPSILON, "Blue should be unchanged: got {}", rgb[2]);
    }

    // ========================================================================
    // Roundtrip tests
    // ========================================================================

    #[test]
    fn test_roundtrip_log_style() {
        let mut curves = GradingRGBCurves::identity();
        
        curves.curves[RGBChannel::Master as usize] = BSplineCurve::new(vec![
            ControlPoint::new(0.0, 0.05),
            ControlPoint::new(0.3, 0.4),
            ControlPoint::new(0.7, 0.75),
            ControlPoint::new(1.0, 1.0),
        ]);

        let pr = GradingRGBCurvePreRender::new(&curves);
        
        let test_vals = [0.1, 0.18, 0.3, 0.5, 0.7, 0.9];
        for &v in &test_vals {
            let original = [v, v, v];
            let mut rgb = original;
            
            // Forward
            apply_grading_rgb_curve(GradingStyle::Log, &pr, true, &mut rgb);
            // Reverse
            apply_grading_rgb_curve(GradingStyle::Log, &pr, false, &mut rgb);
            
            for i in 0..3 {
                assert!(
                    (rgb[i] - original[i]).abs() < EPSILON,
                    "Roundtrip failed for v={v}: original={}, got rgb[{i}]={}",
                    original[i], rgb[i]
                );
            }
        }
    }

    #[test]
    fn test_roundtrip_linear_style() {
        let mut curves = GradingRGBCurves::identity();
        
        // Curve in log space
        curves.curves[RGBChannel::Master as usize] = BSplineCurve::new(vec![
            ControlPoint::new(-6.0, -5.5),  // Lift shadows
            ControlPoint::new(0.0, 0.1),
            ControlPoint::new(6.0, 6.0),
        ]);

        let pr = GradingRGBCurvePreRender::new(&curves);
        
        let test_vals = [0.01, 0.05, 0.18, 0.5, 1.0];
        for &v in &test_vals {
            let original = [v, v, v];
            let mut rgb = original;
            
            // Forward (linear→log→curve→log→linear)
            apply_grading_rgb_curve(GradingStyle::Linear, &pr, true, &mut rgb);
            // Reverse
            apply_grading_rgb_curve(GradingStyle::Linear, &pr, false, &mut rgb);
            
            for i in 0..3 {
                let diff = (rgb[i] - original[i]).abs();
                let rel_diff = diff / original[i].max(0.001);
                assert!(
                    rel_diff < 0.05,
                    "Linear roundtrip failed for v={v}: original={}, got rgb[{i}]={}",
                    original[i], rgb[i]
                );
            }
        }
    }

    // ========================================================================
    // Monotonicity tests
    // ========================================================================

    #[test]
    fn test_monotonic_curve() {
        let mut curves = GradingRGBCurves::identity();
        
        curves.curves[RGBChannel::Master as usize] = BSplineCurve::new(vec![
            ControlPoint::new(0.0, 0.0),
            ControlPoint::new(0.25, 0.3),
            ControlPoint::new(0.5, 0.55),
            ControlPoint::new(0.75, 0.8),
            ControlPoint::new(1.0, 1.0),
        ]);

        let pr = GradingRGBCurvePreRender::new(&curves);
        
        let mut prev_y = -1.0_f32;
        for i in 0..100 {
            let x = i as f32 / 99.0;
            let mut rgb = [x, x, x];
            apply_grading_rgb_curve(GradingStyle::Log, &pr, true, &mut rgb);
            
            assert!(
                rgb[0] >= prev_y - EPSILON,
                "Curve not monotonic at x={x}: prev={prev_y}, current={}",
                rgb[0]
            );
            prev_y = rgb[0];
        }
    }

    // ========================================================================
    // S-curve test
    // ========================================================================

    #[test]
    fn test_s_curve_contrast() {
        let mut curves = GradingRGBCurves::identity();
        
        // S-curve: darken shadows, lift highlights
        curves.curves[RGBChannel::Master as usize] = BSplineCurve::new(vec![
            ControlPoint::new(0.0, 0.0),
            ControlPoint::new(0.25, 0.15),  // Push shadows down
            ControlPoint::new(0.5, 0.5),    // Keep midpoint
            ControlPoint::new(0.75, 0.85),  // Push highlights up
            ControlPoint::new(1.0, 1.0),
        ]);

        let pr = GradingRGBCurvePreRender::new(&curves);
        
        // Check shadow compression
        let mut shadow = [0.25_f32, 0.25, 0.25];
        apply_grading_rgb_curve(GradingStyle::Log, &pr, true, &mut shadow);
        assert!(shadow[0] < 0.25, "S-curve should compress shadows: got {}", shadow[0]);
        
        // Check midpoint preservation
        let mut mid = [0.5_f32, 0.5, 0.5];
        apply_grading_rgb_curve(GradingStyle::Log, &pr, true, &mut mid);
        assert!((mid[0] - 0.5).abs() < 0.05, "S-curve should preserve midpoint: got {}", mid[0]);
        
        // Check highlight expansion
        let mut highlight = [0.75_f32, 0.75, 0.75];
        apply_grading_rgb_curve(GradingStyle::Log, &pr, true, &mut highlight);
        assert!(highlight[0] > 0.75, "S-curve should expand highlights: got {}", highlight[0]);
    }

    // ========================================================================
    // Buffer processing test
    // ========================================================================

    #[test]
    fn test_rgba_buffer_processing() {
        let mut curves = GradingRGBCurves::identity();
        curves.curves[RGBChannel::Master as usize] = BSplineCurve::new(vec![
            ControlPoint::new(0.0, 0.1),
            ControlPoint::new(1.0, 1.0),
        ]);

        let pr = GradingRGBCurvePreRender::new(&curves);
        
        // 3 pixels with different alphas
        let mut pixels = vec![
            0.0, 0.0, 0.0, 1.0,   // Black, full alpha
            0.5, 0.5, 0.5, 0.5,   // Gray, half alpha
            1.0, 1.0, 1.0, 0.0,   // White, zero alpha
        ];
        
        apply_grading_rgb_curve_rgba(GradingStyle::Log, &pr, true, &mut pixels);
        
        // Check alpha is preserved
        assert!((pixels[3] - 1.0).abs() < EPSILON, "Alpha 1 preserved");
        assert!((pixels[7] - 0.5).abs() < EPSILON, "Alpha 2 preserved");
        assert!((pixels[11] - 0.0).abs() < EPSILON, "Alpha 3 preserved");
        
        // Check blacks are lifted
        assert!(pixels[0] > 0.05, "Black lifted");
        
        // Check whites unchanged
        assert!((pixels[8] - 1.0).abs() < EPSILON, "White unchanged");
    }

    // ========================================================================
    // Control point validation
    // ========================================================================

    #[test]
    fn test_curve_validation() {
        // Valid curve
        let curve = BSplineCurve::new(vec![
            ControlPoint::new(0.0, 0.0),
            ControlPoint::new(1.0, 1.0),
        ]);
        assert!(curve.validate().is_ok());
        
        // Invalid: only 1 point
        let curve = BSplineCurve::new(vec![ControlPoint::new(0.0, 0.0)]);
        assert!(curve.validate().is_err());
    }

    #[test]
    fn test_identity_detection() {
        let identity = BSplineCurve::identity();
        assert!(identity.is_identity());
        
        let non_identity = BSplineCurve::new(vec![
            ControlPoint::new(0.0, 0.1),  // Off diagonal
            ControlPoint::new(1.0, 1.0),
        ]);
        assert!(!non_identity.is_identity());
    }
}
