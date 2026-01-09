//! Type definitions for GradingHueCurve operation.
//!
//! Provides "Hue vs X" curve adjustments common in color correction:
//! - Hue vs Hue: shift hue based on input hue
//! - Hue vs Sat: adjust saturation based on input hue
//! - Hue vs Lum: adjust luminance based on input hue

/// Control point on a hue curve.
///
/// X is the input hue (0.0-1.0, wrapping), Y is the adjustment value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HueControlPoint {
    /// Input hue position (0.0-1.0, where 0=red, 0.33=green, 0.67=blue).
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

/// Collection of hue curves for grading.
#[derive(Debug, Clone)]
pub struct GradingHueCurves {
    /// Hue vs Hue curve (hue shift based on input hue).
    pub hue_vs_hue: HueCurve,
    /// Hue vs Saturation curve (sat multiplier based on input hue).
    pub hue_vs_sat: HueCurve,
    /// Hue vs Luminance curve (lum offset based on input hue).
    pub hue_vs_lum: HueCurve,
}

impl Default for GradingHueCurves {
    fn default() -> Self {
        Self::identity()
    }
}

impl GradingHueCurves {
    /// Create identity curves (no adjustment).
    pub fn identity() -> Self {
        Self {
            hue_vs_hue: HueCurve::identity(),
            hue_vs_sat: HueCurve::new(vec![
                HueControlPoint::new(0.0, 1.0),
                HueControlPoint::new(1.0, 1.0),
            ]),
            hue_vs_lum: HueCurve::identity(),
        }
    }

    /// Check if all curves are identity.
    pub fn is_identity(&self) -> bool {
        self.hue_vs_hue.is_identity()
            && self.hue_vs_sat.points.iter().all(|p| (p.value - 1.0).abs() < 1e-6)
            && self.hue_vs_lum.is_identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity() {
        let curves = GradingHueCurves::identity();
        assert!(curves.is_identity());
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
}
