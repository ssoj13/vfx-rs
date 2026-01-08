//! Slope estimation for RGB B-spline curves.
//! 
//! Based on OCIO GradingBSplineCurve.cpp EstimateRGBSlopes.

use super::types::ControlPoint;

/// Estimate slopes at control points for an RGB curve.
/// 
/// Uses weighted averaging of secant slopes to produce smooth curves
/// while preserving monotonicity.
pub fn estimate_rgb_slopes(ctrl_pts: &[ControlPoint]) -> Vec<f32> {
    let n = ctrl_pts.len();
    if n < 2 {
        return vec![];
    }
    
    // Calculate secant slopes and segment lengths
    let mut secant_slope = Vec::with_capacity(n - 1);
    let mut secant_len = Vec::with_capacity(n - 1);
    
    for i in 0..n - 1 {
        let del_x = ctrl_pts[i + 1].x - ctrl_pts[i].x;
        let del_y = ctrl_pts[i + 1].y - ctrl_pts[i].y;
        secant_slope.push(del_y / del_x);
        secant_len.push((del_x * del_x + del_y * del_y).sqrt());
    }
    
    // Special case: only 2 points - constant slope
    if n == 2 {
        return vec![secant_slope[0], secant_slope[0]];
    }
    
    // Merge segments with equal slopes
    let mut i = 0;
    while i < n - 1 {
        let mut j = i;
        let mut dl = secant_len[i];
        while j < n - 2 && (secant_slope[j + 1] - secant_slope[j]).abs() < 1e-6 {
            dl += secant_len[j + 1];
            j += 1;
        }
        for k in i..=j {
            secant_len[k] = dl;
        }
        if j >= n - 3 {
            break;
        }
        i = j + 1;
    }
    
    // Calculate interior slopes using weighted average
    let mut slopes = Vec::with_capacity(n);
    slopes.push(0.0); // placeholder for first slope
    
    for k in 1..n - 1 {
        let s = (secant_len[k] * secant_slope[k] + secant_len[k - 1] * secant_slope[k - 1])
            / (secant_len[k] + secant_len[k - 1]);
        slopes.push(s);
    }
    
    // End slopes: extrapolate from interior
    let min_slope: f32 = 0.01;
    let last_slope = min_slope.max(0.5 * (3.0 * secant_slope[n - 2] - slopes[n - 2]));
    slopes.push(last_slope);
    slopes[0] = min_slope.max(0.5 * (3.0 * secant_slope[0] - slopes[1]));
    
    slopes
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_two_points() {
        let pts = vec![
            ControlPoint::new(0.0, 0.0),
            ControlPoint::new(1.0, 1.0),
        ];
        let slopes = estimate_rgb_slopes(&pts);
        assert_eq!(slopes.len(), 2);
        assert!((slopes[0] - 1.0).abs() < 1e-6);
        assert!((slopes[1] - 1.0).abs() < 1e-6);
    }
    
    #[test]
    fn test_three_points_linear() {
        let pts = vec![
            ControlPoint::new(0.0, 0.0),
            ControlPoint::new(0.5, 0.5),
            ControlPoint::new(1.0, 1.0),
        ];
        let slopes = estimate_rgb_slopes(&pts);
        assert_eq!(slopes.len(), 3);
        // All slopes should be ~1.0 for linear curve
        for s in &slopes {
            assert!((*s - 1.0).abs() < 0.1);
        }
    }
}
