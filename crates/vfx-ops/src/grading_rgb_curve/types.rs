//! Core types for GradingRGBCurve operations.

/// A single control point on a curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlPoint {
    /// X coordinate (input value).
    pub x: f32,
    /// Y coordinate (output value).
    pub y: f32,
}

impl ControlPoint {
    /// Create a new control point.
    #[inline]
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl Default for ControlPoint {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// RGB curve channel index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum RGBChannel {
    /// Red channel curve.
    Red = 0,
    /// Green channel curve.
    Green = 1,
    /// Blue channel curve.
    Blue = 2,
    /// Master curve (applied to all channels).
    Master = 3,
}

/// Number of curves in an RGB curve set.
pub const NUM_RGB_CURVES: usize = 4;

/// A B-spline curve with control points and optional user-specified slopes.
#[derive(Debug, Clone)]
pub struct BSplineCurve {
    /// Control points defining the curve shape.
    pub control_points: Vec<ControlPoint>,
    /// Optional slopes at each control point (0.0 = auto-estimate).
    pub slopes: Vec<f32>,
}

impl BSplineCurve {
    /// Create a new curve with given control points.
    pub fn new(control_points: Vec<ControlPoint>) -> Self {
        let n = control_points.len();
        Self {
            control_points,
            slopes: vec![0.0; n],
        }
    }

    /// Create an identity curve (y = x).
    pub fn identity() -> Self {
        Self::new(vec![
            ControlPoint::new(0.0, 0.0),
            ControlPoint::new(1.0, 1.0),
        ])
    }

    /// Check if all slopes are default (zero).
    pub fn slopes_are_default(&self) -> bool {
        self.slopes.iter().all(|&s| s == 0.0)
    }

    /// Check if curve is identity (all points on y=x line).
    pub fn is_identity(&self) -> bool {
        if !self.slopes_are_default() {
            return false;
        }
        self.control_points.iter().all(|p| (p.x - p.y).abs() < 1e-6)
    }

    /// Validate the curve constraints.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.control_points.len() < 2 {
            return Err("Curve must have at least 2 control points");
        }
        if self.control_points.len() != self.slopes.len() {
            return Err("Slopes array must match control points length");
        }
        
        // Check x-coordinates are non-decreasing
        let mut last_x = f32::NEG_INFINITY;
        for p in &self.control_points {
            if p.x < last_x {
                return Err("X coordinates must be non-decreasing");
            }
            last_x = p.x;
        }
        
        // For monotonic curves, check y-coordinates are non-decreasing
        let mut last_y = f32::NEG_INFINITY;
        for p in &self.control_points {
            if p.y < last_y {
                return Err("Y coordinates must be non-decreasing for monotonic curves");
            }
            last_y = p.y;
        }
        
        Ok(())
    }
}

impl Default for BSplineCurve {
    fn default() -> Self {
        Self::identity()
    }
}

/// A set of RGB curves (Red, Green, Blue, Master).
#[derive(Debug, Clone)]
pub struct GradingRGBCurves {
    /// Individual curves for R, G, B, Master.
    pub curves: [BSplineCurve; NUM_RGB_CURVES],
}

impl GradingRGBCurves {
    /// Create a new set of curves.
    pub fn new(curves: [BSplineCurve; NUM_RGB_CURVES]) -> Self {
        Self { curves }
    }

    /// Create identity curves (all y = x).
    pub fn identity() -> Self {
        Self {
            curves: [
                BSplineCurve::identity(),
                BSplineCurve::identity(),
                BSplineCurve::identity(),
                BSplineCurve::identity(),
            ],
        }
    }

    /// Check if all curves are identity.
    pub fn is_identity(&self) -> bool {
        self.curves.iter().all(|c| c.is_identity())
    }

    /// Get curve by channel.
    #[inline]
    pub fn get(&self, channel: RGBChannel) -> &BSplineCurve {
        &self.curves[channel as usize]
    }

    /// Get mutable curve by channel.
    #[inline]
    pub fn get_mut(&mut self, channel: RGBChannel) -> &mut BSplineCurve {
        &mut self.curves[channel as usize]
    }
}

impl Default for GradingRGBCurves {
    fn default() -> Self {
        Self::identity()
    }
}
