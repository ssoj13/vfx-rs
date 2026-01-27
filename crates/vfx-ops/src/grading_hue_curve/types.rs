//! Type definitions for GradingHueCurve operation.
//!
//! Reference: OCIO GradingHueCurve.cpp, GradingHueCurveOpCPU.cpp
//!
//! Provides 8 curve types for hue-based color correction:
//! - HUE_HUE: shift hue based on input hue (diagonal identity)
//! - HUE_SAT: adjust saturation based on input hue (horizontal at 1.0)
//! - HUE_LUM: adjust luminance based on input hue (horizontal at 1.0)
//! - LUM_SAT: adjust saturation based on luminance (horizontal at 1.0)
//! - SAT_SAT: adjust saturation based on saturation (diagonal identity)
//! - LUM_LUM: adjust luminance based on luminance (diagonal identity)
//! - SAT_LUM: adjust luminance based on saturation (horizontal at 1.0)
//! - HUE_FX: special effects hue shift (horizontal at 0.0)
//!
//! Uses HSY color space (from fixed_function) with three styles:
//! - Log: for log-encoded footage
//! - Linear: for scene-linear footage (with Lin-Log transforms)
//! - Video: for display-referred footage

use crate::fixed_function::HsyVariant;

/// Curve type for GradingHueCurve.
/// 
/// Reference: OCIO OpenColorTypes.h HueCurveType
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum HueCurveType {
    /// Map input hue to output hue (diagonal identity).
    #[default]
    HueHue = 0,
    /// Adjust saturation as a function of hue (value of 1.0 is identity).
    HueSat = 1,
    /// Adjust luminance as a function of hue (value of 1.0 is identity).
    HueLum = 2,
    /// Adjust saturation as a function of luminance (value of 1.0 is identity).
    LumSat = 3,
    /// Adjust saturation as a function of saturation (diagonal identity).
    SatSat = 4,
    /// Adjust luminance as a function of luminance (diagonal identity).
    LumLum = 5,
    /// Adjust luminance as a function of saturation (value of 1.0 is identity).
    SatLum = 6,
    /// Map input hue to delta output hue (value of 0.0 is identity).
    HueFx = 7,
}

impl HueCurveType {
    /// Total number of curve types.
    pub const COUNT: usize = 8;
    
    /// All curve types in order.
    pub const ALL: [HueCurveType; 8] = [
        HueCurveType::HueHue,
        HueCurveType::HueSat,
        HueCurveType::HueLum,
        HueCurveType::LumSat,
        HueCurveType::SatSat,
        HueCurveType::LumLum,
        HueCurveType::SatLum,
        HueCurveType::HueFx,
    ];
}

/// Grading style determines HSY variant and Lin-Log transforms.
/// 
/// Reference: OCIO GradingStyle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GradingStyle {
    /// For log-encoded footage.
    #[default]
    Log,
    /// For scene-linear footage (uses Lin-Log transforms).
    Linear,
    /// For display-referred (video) footage.
    Video,
}

impl GradingStyle {
    /// Get the corresponding HSY variant.
    #[inline]
    pub fn hsy_variant(self) -> HsyVariant {
        match self {
            GradingStyle::Log => HsyVariant::Log,
            GradingStyle::Linear => HsyVariant::Lin,
            GradingStyle::Video => HsyVariant::Vid,
        }
    }
}

/// Control point on a curve.
///
/// X is the input value, Y is the output adjustment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HueControlPoint {
    /// Input position (interpretation depends on curve type).
    pub hue: f32,
    /// Output adjustment value (meaning depends on curve type).
    pub value: f32,
}

impl HueControlPoint {
    /// Create a new control point.
    #[inline]
    pub fn new(hue: f32, value: f32) -> Self {
        Self { hue, value }
    }
}

/// A single hue-based curve with control points.
///
/// The curve wraps around (hue 1.0 connects to hue 0.0).
#[derive(Debug, Clone)]
pub struct HueCurve {
    /// Control points sorted by hue.
    pub points: Vec<HueControlPoint>,
}

impl Default for HueCurve {
    fn default() -> Self {
        Self::identity()
    }
}

impl HueCurve {
    /// Create identity curve (no adjustment).
    pub fn identity() -> Self {
        Self {
            points: vec![
                HueControlPoint::new(0.0, 0.0),
                HueControlPoint::new(1.0, 0.0),
            ],
        }
    }

    /// Create curve from control points.
    pub fn new(mut points: Vec<HueControlPoint>) -> Self {
        // Sort by hue
        points.sort_by(|a, b| a.hue.partial_cmp(&b.hue).unwrap());
        Self { points }
    }

    /// Check if curve is identity (all values zero).
    pub fn is_identity(&self) -> bool {
        self.points.iter().all(|p| p.value.abs() < 1e-6)
    }

    /// Evaluate curve at given hue using linear interpolation.
    ///
    /// Hue wraps around (0.0 = 1.0).
    pub fn evaluate(&self, hue: f32) -> f32 {
        if self.points.is_empty() {
            return 0.0;
        }
        if self.points.len() == 1 {
            return self.points[0].value;
        }

        // Normalize hue to 0-1
        let h = hue.rem_euclid(1.0);

        // Find surrounding control points
        let mut i1 = 0;
        for (i, p) in self.points.iter().enumerate() {
            if p.hue > h {
                break;
            }
            i1 = i;
        }
        
        let i2 = (i1 + 1) % self.points.len();
        let p1 = &self.points[i1];
        let p2 = &self.points[i2];

        // Handle wrap-around
        let h1 = p1.hue;
        let mut h2 = p2.hue;
        let mut target = h;

        if h2 < h1 {
            // Wrap around case
            h2 += 1.0;
            if target < h1 {
                target += 1.0;
            }
        }

        // Linear interpolation
        let span = h2 - h1;
        if span.abs() < 1e-6 {
            return p1.value;
        }

        let t = (target - h1) / span;
        p1.value + t * (p2.value - p1.value)
    }
}

/// Full set of 8 grading hue curves.
/// 
/// Reference: OCIO GradingHueCurve
#[derive(Debug, Clone)]
pub struct GradingHueCurves {
    /// Grading style (Log, Linear, Video).
    pub style: GradingStyle,
    /// HUE_HUE: Map hue to hue (diagonal identity).
    pub hue_hue: HueCurve,
    /// HUE_SAT: Adjust saturation based on hue (horizontal at 1.0).
    pub hue_sat: HueCurve,
    /// HUE_LUM: Adjust luminance based on hue (horizontal at 1.0).
    pub hue_lum: HueCurve,
    /// LUM_SAT: Adjust saturation based on luminance (horizontal at 1.0).
    pub lum_sat: HueCurve,
    /// SAT_SAT: Adjust saturation based on saturation (diagonal identity).
    pub sat_sat: HueCurve,
    /// LUM_LUM: Adjust luminance based on luminance (diagonal identity).
    pub lum_lum: HueCurve,
    /// SAT_LUM: Adjust luminance based on saturation (horizontal at 1.0).
    pub sat_lum: HueCurve,
    /// HUE_FX: Special effects hue shift (horizontal at 0.0).
    pub hue_fx: HueCurve,
}

impl Default for GradingHueCurves {
    fn default() -> Self {
        Self::identity(GradingStyle::Log)
    }
}

impl GradingHueCurves {
    /// Create identity curves for the given style.
    /// 
    /// Reference: OCIO GradingHueCurve.cpp DefaultCurves
    pub fn identity(style: GradingStyle) -> Self {
        let is_linear = style == GradingStyle::Linear;
        
        // Periodic curves use 6 points at hue intervals
        let hue_6pts = |v: f32| vec![
            HueControlPoint::new(0.0, v),
            HueControlPoint::new(1.0/6.0, v),
            HueControlPoint::new(2.0/6.0, v),
            HueControlPoint::new(0.5, v),
            HueControlPoint::new(4.0/6.0, v),
            HueControlPoint::new(5.0/6.0, v),
        ];
        
        // Diagonal curves (hue_hue)
        let hue_hue_pts = vec![
            HueControlPoint::new(0.0, 0.0),
            HueControlPoint::new(1.0/6.0, 1.0/6.0),
            HueControlPoint::new(2.0/6.0, 2.0/6.0),
            HueControlPoint::new(0.5, 0.5),
            HueControlPoint::new(4.0/6.0, 4.0/6.0),
            HueControlPoint::new(5.0/6.0, 5.0/6.0),
        ];
        
        // LUM curves differ for Linear style (range -7 to 7 vs 0 to 1)
        let lum_sat_pts = if is_linear {
            vec![
                HueControlPoint::new(-7.0, 1.0),
                HueControlPoint::new(0.0, 1.0),
                HueControlPoint::new(7.0, 1.0),
            ]
        } else {
            vec![
                HueControlPoint::new(0.0, 1.0),
                HueControlPoint::new(0.5, 1.0),
                HueControlPoint::new(1.0, 1.0),
            ]
        };
        
        // SAT curves
        let sat_sat_pts = vec![
            HueControlPoint::new(0.0, 0.0),
            HueControlPoint::new(0.5, 0.5),
            HueControlPoint::new(1.0, 1.0),
        ];
        
        let sat_lum_pts = vec![
            HueControlPoint::new(0.0, 1.0),
            HueControlPoint::new(0.5, 1.0),
            HueControlPoint::new(1.0, 1.0),
        ];
        
        // LUM_LUM differs for Linear style
        let lum_lum_pts = if is_linear {
            vec![
                HueControlPoint::new(-7.0, -7.0),
                HueControlPoint::new(0.0, 0.0),
                HueControlPoint::new(7.0, 7.0),
            ]
        } else {
            vec![
                HueControlPoint::new(0.0, 0.0),
                HueControlPoint::new(0.5, 0.5),
                HueControlPoint::new(1.0, 1.0),
            ]
        };
        
        Self {
            style,
            hue_hue: HueCurve::new(hue_hue_pts),
            hue_sat: HueCurve::new(hue_6pts(1.0)),
            hue_lum: HueCurve::new(hue_6pts(1.0)),
            lum_sat: HueCurve::new(lum_sat_pts),
            sat_sat: HueCurve::new(sat_sat_pts),
            lum_lum: HueCurve::new(lum_lum_pts),
            sat_lum: HueCurve::new(sat_lum_pts),
            hue_fx: HueCurve::new(hue_6pts(0.0)),
        }
    }

    /// Get curve by type.
    pub fn get(&self, curve_type: HueCurveType) -> &HueCurve {
        match curve_type {
            HueCurveType::HueHue => &self.hue_hue,
            HueCurveType::HueSat => &self.hue_sat,
            HueCurveType::HueLum => &self.hue_lum,
            HueCurveType::LumSat => &self.lum_sat,
            HueCurveType::SatSat => &self.sat_sat,
            HueCurveType::LumLum => &self.lum_lum,
            HueCurveType::SatLum => &self.sat_lum,
            HueCurveType::HueFx => &self.hue_fx,
        }
    }
    
    /// Get mutable curve by type.
    pub fn get_mut(&mut self, curve_type: HueCurveType) -> &mut HueCurve {
        match curve_type {
            HueCurveType::HueHue => &mut self.hue_hue,
            HueCurveType::HueSat => &mut self.hue_sat,
            HueCurveType::HueLum => &mut self.hue_lum,
            HueCurveType::LumSat => &mut self.lum_sat,
            HueCurveType::SatSat => &mut self.sat_sat,
            HueCurveType::LumLum => &mut self.lum_lum,
            HueCurveType::SatLum => &mut self.sat_lum,
            HueCurveType::HueFx => &mut self.hue_fx,
        }
    }

    /// Check if all curves are identity.
    pub fn is_identity(&self) -> bool {
        self.is_hue_hue_identity()
            && self.is_horizontal_identity(&self.hue_sat, 1.0)
            && self.is_horizontal_identity(&self.hue_lum, 1.0)
            && self.is_horizontal_identity(&self.lum_sat, 1.0)
            && self.is_sat_sat_identity()
            && self.is_lum_lum_identity()
            && self.is_horizontal_identity(&self.sat_lum, 1.0)
            && self.is_horizontal_identity(&self.hue_fx, 0.0)
    }
    
    fn is_hue_hue_identity(&self) -> bool {
        // Diagonal: value == hue at all points
        self.hue_hue.points.iter().all(|p| (p.value - p.hue).abs() < 1e-6)
    }
    
    fn is_sat_sat_identity(&self) -> bool {
        // Diagonal: value == hue at all points
        self.sat_sat.points.iter().all(|p| (p.value - p.hue).abs() < 1e-6)
    }
    
    fn is_lum_lum_identity(&self) -> bool {
        // Diagonal: value == hue at all points
        self.lum_lum.points.iter().all(|p| (p.value - p.hue).abs() < 1e-6)
    }
    
    fn is_horizontal_identity(&self, curve: &HueCurve, target: f32) -> bool {
        curve.points.iter().all(|p| (p.value - target).abs() < 1e-6)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_log() {
        let curves = GradingHueCurves::identity(GradingStyle::Log);
        assert!(curves.is_identity());
        assert_eq!(curves.style, GradingStyle::Log);
    }
    
    #[test]
    fn test_identity_linear() {
        let curves = GradingHueCurves::identity(GradingStyle::Linear);
        assert!(curves.is_identity());
        assert_eq!(curves.style, GradingStyle::Linear);
        // Linear style has different LUM ranges
        assert!(curves.lum_sat.points[0].hue < 0.0); // -7.0
    }

    #[test]
    fn test_hue_curve_eval() {
        let curve = HueCurve::new(vec![
            HueControlPoint::new(0.0, 0.0),
            HueControlPoint::new(0.5, 1.0),
            HueControlPoint::new(1.0, 0.0),
        ]);

        assert!((curve.evaluate(0.0) - 0.0).abs() < 0.01);
        assert!((curve.evaluate(0.25) - 0.5).abs() < 0.01);
        assert!((curve.evaluate(0.5) - 1.0).abs() < 0.01);
        assert!((curve.evaluate(0.75) - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_hue_wrap() {
        let curve = HueCurve::new(vec![
            HueControlPoint::new(0.0, 1.0),
            HueControlPoint::new(0.5, 0.0),
        ]);

        // Test wrap-around interpolation
        assert!((curve.evaluate(0.75) - 0.5).abs() < 0.01);
        assert!((curve.evaluate(1.25) - 0.5).abs() < 0.01); // Same as 0.25
    }
    
    #[test]
    fn test_curve_types() {
        assert_eq!(HueCurveType::COUNT, 8);
        assert_eq!(HueCurveType::HueHue as u8, 0);
        assert_eq!(HueCurveType::HueFx as u8, 7);
    }
    
    #[test]
    fn test_get_curve() {
        let curves = GradingHueCurves::identity(GradingStyle::Log);
        let hue_hue = curves.get(HueCurveType::HueHue);
        assert_eq!(hue_hue.points.len(), 6);
    }
}
