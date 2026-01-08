//! B-spline fitting for RGB curves.
//! 
//! Based on OCIO GradingBSplineCurve.cpp FitRGBSpline and AdjustRGBSlopes.

use super::types::ControlPoint;
use super::slopes::estimate_rgb_slopes;

/// Maximum number of knots allowed across all curves.
#[allow(dead_code)]
pub const MAX_NUM_KNOTS: usize = 120;
/// Maximum number of coefficients allowed across all curves.
#[allow(dead_code)]
pub const MAX_NUM_COEFS: usize = 360;

/// Precomputed spline data for efficient curve evaluation.
#[derive(Debug, Clone)]
pub struct SplineData {
    /// X-coordinates of segment boundaries (knots).
    pub knots: Vec<f32>,
    /// Quadratic coefficient A for each segment.
    pub coefs_a: Vec<f32>,
    /// Linear coefficient B for each segment.
    pub coefs_b: Vec<f32>,
    /// Constant coefficient C for each segment.
    pub coefs_c: Vec<f32>,
}

impl SplineData {
    /// Create empty spline data.
    pub fn new() -> Self {
        Self {
            knots: Vec::new(),
            coefs_a: Vec::new(),
            coefs_b: Vec::new(),
            coefs_c: Vec::new(),
        }
    }

    /// Number of segments in the spline.
    #[inline]
    pub fn num_segments(&self) -> usize {
        self.coefs_a.len()
    }

    /// Check if spline is empty (identity).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.knots.is_empty()
    }
}

impl Default for SplineData {
    fn default() -> Self {
        Self::new()
    }
}

/// Fit a B-spline to control points.
/// 
/// Returns the fitted spline data with knots and polynomial coefficients.
pub fn fit_rgb_spline(ctrl_pts: &[ControlPoint], user_slopes: &[f32]) -> SplineData {
    let n = ctrl_pts.len();
    if n < 2 {
        return SplineData::new();
    }
    
    // Use user slopes if non-default, otherwise estimate
    let slopes = if user_slopes.iter().any(|&s| s != 0.0) && user_slopes.len() == n {
        user_slopes.to_vec()
    } else {
        estimate_rgb_slopes(ctrl_pts)
    };
    
    // First pass: fit spline
    let (mut knots, mut coefs_a, mut coefs_b, mut coefs_c, mut slopes) = 
        fit_spline_internal(ctrl_pts, slopes);
    
    // Check if any adjustments are needed
    let adjustment_done = adjust_rgb_slopes(ctrl_pts, &mut slopes, &knots);
    
    if adjustment_done {
        // Refit with adjusted slopes
        let result = fit_spline_internal(ctrl_pts, slopes);
        knots = result.0;
        coefs_a = result.1;
        coefs_b = result.2;
        coefs_c = result.3;
    }
    
    SplineData {
        knots,
        coefs_a,
        coefs_b,
        coefs_c,
    }
}

/// Internal spline fitting function.
fn fit_spline_internal(
    ctrl_pts: &[ControlPoint],
    slopes: Vec<f32>,
) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {
    let n = ctrl_pts.len();
    
    let mut knots = Vec::with_capacity(n * 2);
    let mut coefs_a = Vec::with_capacity(n * 2);
    let mut coefs_b = Vec::with_capacity(n * 2);
    let mut coefs_c = Vec::with_capacity(n * 2);
    
    knots.push(ctrl_pts[0].x);
    
    for i in 0..n - 1 {
        let xi = ctrl_pts[i].x;
        let xi_pl1 = ctrl_pts[i + 1].x;
        let yi = ctrl_pts[i].y;
        let yi_pl1 = ctrl_pts[i + 1].y;
        let del_x = xi_pl1 - xi;
        let del_y = yi_pl1 - yi;
        let secant_slope = del_y / del_x;
        
        // Check if single quadratic is sufficient
        if (slopes[i] + slopes[i + 1] - 2.0 * secant_slope).abs() < 1e-6 {
            // Single segment: y = A*(x-x0)^2 + B*(x-x0) + C
            coefs_c.push(yi);
            coefs_b.push(slopes[i]);
            coefs_a.push(0.5 * (slopes[i + 1] - slopes[i]) / del_x);
        } else {
            // Need to split into two segments
            let ksi = calculate_ksi(ctrl_pts, &slopes, i);
            
            let s_bar = (2.0 * secant_slope - slopes[i + 1])
                + (slopes[i + 1] - slopes[i]) * (ksi - xi) / del_x;
            let eta = (s_bar - slopes[i]) / (ksi - xi);
            
            coefs_c.push(yi);
            coefs_b.push(slopes[i]);
            coefs_a.push(0.5 * eta);
            
            // Second segment coefficients
            let t = ksi - xi;
            let y_at_ksi = yi + slopes[i] * t + 0.5 * eta * t * t;
            coefs_c.push(y_at_ksi);
            coefs_b.push(s_bar);
            coefs_a.push(0.5 * (slopes[i + 1] - s_bar) / (xi_pl1 - ksi));
            
            knots.push(ksi);
        }
        
        knots.push(xi_pl1);
    }
    
    (knots, coefs_a, coefs_b, coefs_c, slopes)
}

/// Calculate the optimal split point (ksi) for a segment.
fn calculate_ksi(ctrl_pts: &[ControlPoint], slopes: &[f32], i: usize) -> f32 {
    let xi = ctrl_pts[i].x;
    let xi_pl1 = ctrl_pts[i + 1].x;
    let yi = ctrl_pts[i].y;
    let yi_pl1 = ctrl_pts[i + 1].y;
    let del_x = xi_pl1 - xi;
    let secant = (yi_pl1 - yi) / del_x;
    
    let aa = slopes[i] - secant;
    let bb = slopes[i + 1] - secant;
    
    if aa * bb >= 0.0 {
        // Same sign or zero - use midpoint
        (xi + xi_pl1) * 0.5
    } else {
        // Opposite signs - calculate optimal split
        if aa.abs() > bb.abs() {
            xi_pl1 + aa * del_x / (slopes[i + 1] - slopes[i])
        } else {
            xi + bb * del_x / (slopes[i + 1] - slopes[i])
        }
    }
}

/// Adjust slopes to ensure positive second derivative (no inflection).
fn adjust_rgb_slopes(
    ctrl_pts: &[ControlPoint],
    slopes: &mut [f32],
    knots: &[f32],
) -> bool {
    let mut adjustment_done = false;
    let n = knots.len();
    let mut i = 0;
    let mut j = 0;
    
    while j < n {
        if ctrl_pts[i].x != knots[j] {
            // This is a split point - check for negative s_bar
            let ksi = knots[j];
            let xi = ctrl_pts[i].x;
            let xi_pl1 = ctrl_pts[i + 1].x;
            let yi = ctrl_pts[i].y;
            let yi_pl1 = ctrl_pts[i + 1].y;
            
            let s_bar = (2.0 * (yi_pl1 - yi) - (ksi - xi) * slopes[i]
                - (xi_pl1 - ksi) * slopes[i + 1])
                / (xi_pl1 - xi);
            
            if s_bar < 0.0 {
                adjustment_done = true;
                let secant = (yi_pl1 - yi) / (xi_pl1 - xi);
                let blend_slope = ((ksi - xi) * slopes[i] + (xi_pl1 - ksi) * slopes[i + 1])
                    / (xi_pl1 - xi);
                let aim_slope = (0.01 * 0.5 * (slopes[i] + slopes[i + 1])).min(secant);
                let adjust = (2.0 * secant - aim_slope) / blend_slope;
                slopes[i] *= adjust;
                slopes[i + 1] *= adjust;
            }
            i += 1;
        }
        j += 1;
    }
    
    adjustment_done
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_identity_spline() {
        let pts = vec![
            ControlPoint::new(0.0, 0.0),
            ControlPoint::new(1.0, 1.0),
        ];
        let spline = fit_rgb_spline(&pts, &[0.0, 0.0]);
        assert!(!spline.is_empty());
        assert_eq!(spline.knots.len(), 2);
    }
    
    #[test]
    fn test_three_point_spline() {
        let pts = vec![
            ControlPoint::new(0.0, 0.0),
            ControlPoint::new(0.5, 0.6), // Above diagonal
            ControlPoint::new(1.0, 1.0),
        ];
        let spline = fit_rgb_spline(&pts, &[0.0, 0.0, 0.0]);
        assert!(!spline.is_empty());
        // Should have at least 3 knots (may have more if split needed)
        assert!(spline.knots.len() >= 3);
    }
}
