//! Curve evaluation for RGB B-spline curves.
//! 
//! Based on OCIO GradingBSplineCurve.cpp evalCurve and evalCurveRev.

use super::spline::SplineData;

/// Evaluate a fitted spline at a given x value.
/// 
/// If the curve is empty (identity), returns `identity_x`.
pub fn eval_curve(spline: &SplineData, x: f32, identity_x: f32) -> f32 {
    if spline.is_empty() {
        return identity_x;
    }
    
    let num_knots = spline.knots.len();
    let num_segs = spline.num_segments();
    let kn_start = spline.knots[0];
    let kn_end = spline.knots[num_knots - 1];
    
    if x <= kn_start {
        // Extrapolate below curve start
        let b = spline.coefs_b[0];
        let c = spline.coefs_c[0];
        return (x - kn_start) * b + c;
    }
    
    if x >= kn_end {
        // Extrapolate above curve end
        let a = spline.coefs_a[num_segs - 1];
        let b = spline.coefs_b[num_segs - 1];
        let c = spline.coefs_c[num_segs - 1];
        let kn = spline.knots[num_knots - 2];
        let t = kn_end - kn;
        let slope = 2.0 * a * t + b;
        let offs = (a * t + b) * t + c;
        return (x - kn_end) * slope + offs;
    }
    
    // Find the segment containing x
    let mut seg = 0;
    for i in 0..num_knots - 1 {
        if x < spline.knots[i + 1] {
            seg = i;
            break;
        }
    }
    
    // Evaluate quadratic polynomial
    let a = spline.coefs_a[seg];
    let b = spline.coefs_b[seg];
    let c = spline.coefs_c[seg];
    let kn = spline.knots[seg];
    let t = x - kn;
    
    (a * t + b) * t + c
}

/// Reverse-evaluate a fitted spline to find x given y.
/// 
/// Uses the quadratic formula to invert the polynomial.
/// Only works for monotonic curves.
pub fn eval_curve_rev(spline: &SplineData, y: f32) -> f32 {
    if spline.is_empty() {
        return y;
    }
    
    let num_knots = spline.knots.len();
    let num_segs = spline.num_segments();
    let kn_start = spline.knots[0];
    let kn_end = spline.knots[num_knots - 1];
    
    // Calculate y values at curve boundaries
    let kn_start_y = spline.coefs_c[0];
    let kn_end_y = {
        let a = spline.coefs_a[num_segs - 1];
        let b = spline.coefs_b[num_segs - 1];
        let c = spline.coefs_c[num_segs - 1];
        let kn = spline.knots[num_knots - 2];
        let t = kn_end - kn;
        (a * t + b) * t + c
    };
    
    if y <= kn_start_y {
        // Extrapolate below curve start
        let b = spline.coefs_b[0];
        let c = spline.coefs_c[0];
        if b.abs() < 1e-5 {
            return kn_start;
        }
        return (y - c) / b + kn_start;
    }
    
    if y >= kn_end_y {
        // Extrapolate above curve end
        let a = spline.coefs_a[num_segs - 1];
        let b = spline.coefs_b[num_segs - 1];
        let c = spline.coefs_c[num_segs - 1];
        let kn = spline.knots[num_knots - 2];
        let t = kn_end - kn;
        let slope = 2.0 * a * t + b;
        let offs = (a * t + b) * t + c;
        if slope.abs() < 1e-5 {
            return kn_end;
        }
        return (y - offs) / slope + kn_end;
    }
    
    // Find segment by searching y values at knots
    // For monotonic curve, y values at segment starts are coefs_c values
    let mut seg = 0;
    for i in 0..num_segs - 1 {
        if y < spline.coefs_c[i + 1] {
            seg = i;
            break;
        }
        seg = i + 1;
    }
    
    // Invert quadratic: a*t^2 + b*t + (c - y) = 0
    let a = spline.coefs_a[seg];
    let b = spline.coefs_b[seg];
    let c = spline.coefs_c[seg];
    let kn = spline.knots[seg];
    let c0 = c - y;
    
    // Use numerically stable quadratic formula
    let discrim = (b * b - 4.0 * a * c0).sqrt();
    kn + (-2.0 * c0) / (discrim + b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::ControlPoint;
    use super::super::spline::fit_rgb_spline;
    
    #[test]
    fn test_identity_eval() {
        let pts = vec![
            ControlPoint::new(0.0, 0.0),
            ControlPoint::new(1.0, 1.0),
        ];
        let spline = fit_rgb_spline(&pts, &[0.0, 0.0]);
        
        // Test several points
        let test_vals = [0.0, 0.25, 0.5, 0.75, 1.0];
        for &x in &test_vals {
            let y = eval_curve(&spline, x, x);
            assert!((y - x).abs() < 0.01, "Identity curve failed at x={x}: got y={y}");
        }
    }
    
    #[test]
    fn test_eval_roundtrip() {
        let pts = vec![
            ControlPoint::new(0.0, 0.0),
            ControlPoint::new(0.5, 0.6),
            ControlPoint::new(1.0, 1.0),
        ];
        let spline = fit_rgb_spline(&pts, &[0.0, 0.0, 0.0]);
        
        let test_vals = [0.1, 0.3, 0.5, 0.7, 0.9];
        for &x in &test_vals {
            let y = eval_curve(&spline, x, x);
            let x_rev = eval_curve_rev(&spline, y);
            assert!(
                (x - x_rev).abs() < 0.01,
                "Roundtrip failed at x={x}: y={y}, x_rev={x_rev}"
            );
        }
    }
    
    #[test]
    fn test_extrapolation() {
        let pts = vec![
            ControlPoint::new(0.0, 0.0),
            ControlPoint::new(1.0, 1.0),
        ];
        let spline = fit_rgb_spline(&pts, &[0.0, 0.0]);
        
        // Below range
        let y = eval_curve(&spline, -0.5, -0.5);
        assert!(y < 0.0, "Should extrapolate below: got {y}");
        
        // Above range
        let y = eval_curve(&spline, 1.5, 1.5);
        assert!(y > 1.0, "Should extrapolate above: got {y}");
    }
}
