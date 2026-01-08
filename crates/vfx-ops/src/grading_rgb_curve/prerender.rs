//! Pre-rendered curve data for efficient evaluation.
//! 
//! Combines all four RGB curves into a single structure for batch processing.

use super::types::{GradingRGBCurves, RGBChannel, NUM_RGB_CURVES};
use super::spline::{SplineData, fit_rgb_spline};

/// Pre-rendered data for all RGB curves.
/// 
/// Stores fitted spline data for R, G, B, and Master curves.
#[derive(Debug, Clone)]
pub struct GradingRGBCurvePreRender {
    /// Fitted spline data for each curve.
    pub splines: [SplineData; NUM_RGB_CURVES],
    /// Local bypass flag (all curves are identity).
    pub local_bypass: bool,
}

impl GradingRGBCurvePreRender {
    /// Create pre-rendered data from RGB curves.
    pub fn new(curves: &GradingRGBCurves) -> Self {
        let mut splines = [
            SplineData::new(),
            SplineData::new(),
            SplineData::new(),
            SplineData::new(),
        ];
        
        let mut all_identity = true;
        
        for i in 0..NUM_RGB_CURVES {
            let curve = &curves.curves[i];
            
            if curve.is_identity() {
                // Leave spline empty for identity curves
                continue;
            }
            
            all_identity = false;
            splines[i] = fit_rgb_spline(&curve.control_points, &curve.slopes);
        }
        
        Self {
            splines,
            local_bypass: all_identity,
        }
    }

    /// Get spline data for a specific channel.
    #[inline]
    pub fn get(&self, channel: RGBChannel) -> &SplineData {
        &self.splines[channel as usize]
    }

    /// Check if all curves are identity (bypass).
    #[inline]
    pub fn is_bypass(&self) -> bool {
        self.local_bypass
    }
}

impl Default for GradingRGBCurvePreRender {
    fn default() -> Self {
        Self {
            splines: [
                SplineData::new(),
                SplineData::new(),
                SplineData::new(),
                SplineData::new(),
            ],
            local_bypass: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::{BSplineCurve, ControlPoint};
    
    #[test]
    fn test_identity_bypass() {
        let curves = GradingRGBCurves::identity();
        let pr = GradingRGBCurvePreRender::new(&curves);
        assert!(pr.is_bypass());
    }
    
    #[test]
    fn test_non_identity() {
        let mut curves = GradingRGBCurves::identity();
        
        // Modify red curve
        curves.curves[0] = BSplineCurve::new(vec![
            ControlPoint::new(0.0, 0.0),
            ControlPoint::new(0.5, 0.6),
            ControlPoint::new(1.0, 1.0),
        ]);
        
        let pr = GradingRGBCurvePreRender::new(&curves);
        assert!(!pr.is_bypass());
        assert!(!pr.splines[0].is_empty());
        assert!(pr.splines[1].is_empty()); // Green still identity
    }
}
