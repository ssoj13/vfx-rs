//! Range operation for clamping and remapping values.
//!
//! Reference: OCIO ops/range/RangeOpCPU.cpp
//!
//! The Range op can:
//! - Clamp values to a min/max range
//! - Scale and offset values (linear remapping)
//! - Combine both operations
//!
//! # Example
//!
//! ```rust
//! use vfx_ops::range::{Range, apply_range};
//!
//! // Simple clamp to [0, 1]
//! let range = Range::clamp(0.0, 1.0);
//! let mut pixel = [1.5, -0.2, 0.5];
//! apply_range(&range, &mut pixel);
//! assert_eq!(pixel, [1.0, 0.0, 0.5]);
//!
//! // Remap from [0, 1] to [0.1, 0.9]
//! let range = Range::new(0.0, 1.0, 0.1, 0.9);
//! let mut pixel = [0.0, 0.5, 1.0];
//! apply_range(&range, &mut pixel);
//! // 0.0 -> 0.1, 0.5 -> 0.5, 1.0 -> 0.9
//! ```

/// Range operation parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    /// Minimum input value (None = no min clamping on input).
    pub min_in: Option<f64>,
    /// Maximum input value (None = no max clamping on input).
    pub max_in: Option<f64>,
    /// Minimum output value (clamp floor).
    pub min_out: Option<f64>,
    /// Maximum output value (clamp ceiling).
    pub max_out: Option<f64>,
}

impl Range {
    /// Create a range with full parameters.
    ///
    /// This creates a linear remapping from [min_in, max_in] to [min_out, max_out]
    /// with clamping.
    pub fn new(min_in: f64, max_in: f64, min_out: f64, max_out: f64) -> Self {
        Self {
            min_in: Some(min_in),
            max_in: Some(max_in),
            min_out: Some(min_out),
            max_out: Some(max_out),
        }
    }

    /// Create a simple clamping range (no scaling).
    ///
    /// Values below `min` become `min`, values above `max` become `max`.
    pub fn clamp(min: f64, max: f64) -> Self {
        Self {
            min_in: None,
            max_in: None,
            min_out: Some(min),
            max_out: Some(max),
        }
    }

    /// Create a range with only minimum clamping.
    pub fn clamp_min(min: f64) -> Self {
        Self {
            min_in: None,
            max_in: None,
            min_out: Some(min),
            max_out: None,
        }
    }

    /// Create a range with only maximum clamping.
    pub fn clamp_max(max: f64) -> Self {
        Self {
            min_in: None,
            max_in: None,
            min_out: None,
            max_out: Some(max),
        }
    }

    /// Check if this range performs scaling (not just clamping).
    pub fn scales(&self) -> bool {
        match (self.min_in, self.max_in, self.min_out, self.max_out) {
            (Some(min_in), Some(max_in), Some(min_out), Some(max_out)) => {
                let in_range = max_in - min_in;
                let out_range = max_out - min_out;
                // Scales if ranges differ or there's an offset
                (in_range - out_range).abs() > 1e-9 || (min_in - min_out).abs() > 1e-9
            }
            _ => false,
        }
    }

    /// Calculate scale factor for remapping.
    pub fn scale(&self) -> f64 {
        match (self.min_in, self.max_in, self.min_out, self.max_out) {
            (Some(min_in), Some(max_in), Some(min_out), Some(max_out)) => {
                let in_range = max_in - min_in;
                if in_range.abs() < 1e-12 {
                    1.0
                } else {
                    (max_out - min_out) / in_range
                }
            }
            _ => 1.0,
        }
    }

    /// Calculate offset for remapping.
    pub fn offset(&self) -> f64 {
        match (self.min_in, self.min_out) {
            (Some(min_in), Some(min_out)) => min_out - min_in * self.scale(),
            _ => 0.0,
        }
    }

    /// Get the lower bound for clamping.
    pub fn lower_bound(&self) -> f64 {
        self.min_out.unwrap_or(f64::NEG_INFINITY)
    }

    /// Get the upper bound for clamping.
    pub fn upper_bound(&self) -> f64 {
        self.max_out.unwrap_or(f64::INFINITY)
    }

    /// Create inverse range (swap in/out).
    pub fn inverse(&self) -> Self {
        Self {
            min_in: self.min_out,
            max_in: self.max_out,
            min_out: self.min_in,
            max_out: self.max_in,
        }
    }
}

impl Default for Range {
    fn default() -> Self {
        Self::clamp(0.0, 1.0)
    }
}

// ============================================================================
// Apply functions
// ============================================================================

/// Apply range operation to a single RGB pixel (in-place).
#[inline]
pub fn apply_range(range: &Range, rgb: &mut [f32; 3]) {
    let scale = range.scale() as f32;
    let offset = range.offset() as f32;
    let lower = range.lower_bound() as f32;
    let upper = range.upper_bound() as f32;

    if range.scales() {
        // Scale and clamp
        for c in rgb.iter_mut() {
            let v = *c * scale + offset;
            // NaN becomes lower bound (OCIO behavior)
            *c = clamp_nan(v, lower, upper);
        }
    } else if range.min_out.is_some() && range.max_out.is_some() {
        // Clamp both
        for c in rgb.iter_mut() {
            *c = clamp_nan(*c, lower, upper);
        }
    } else if range.min_out.is_some() {
        // Clamp min only
        for c in rgb.iter_mut() {
            // NaN becomes lower bound
            *c = if c.is_nan() { lower } else { c.max(lower) };
        }
    } else if range.max_out.is_some() {
        // Clamp max only
        for c in rgb.iter_mut() {
            // NaN becomes upper bound
            *c = if c.is_nan() { upper } else { c.min(upper) };
        }
    }
}

/// Apply range operation to a single RGB pixel (f64 precision).
#[inline]
pub fn apply_range_f64(range: &Range, rgb: &mut [f64; 3]) {
    let scale = range.scale();
    let offset = range.offset();
    let lower = range.lower_bound();
    let upper = range.upper_bound();

    if range.scales() {
        for c in rgb.iter_mut() {
            let v = *c * scale + offset;
            *c = clamp_nan_f64(v, lower, upper);
        }
    } else if range.min_out.is_some() && range.max_out.is_some() {
        for c in rgb.iter_mut() {
            *c = clamp_nan_f64(*c, lower, upper);
        }
    } else if range.min_out.is_some() {
        for c in rgb.iter_mut() {
            *c = if c.is_nan() { lower } else { c.max(lower) };
        }
    } else if range.max_out.is_some() {
        for c in rgb.iter_mut() {
            *c = if c.is_nan() { upper } else { c.min(upper) };
        }
    }
}

/// Apply range to a buffer of RGBA pixels.
pub fn apply_range_rgba(range: &Range, pixels: &mut [f32]) {
    debug_assert!(pixels.len() % 4 == 0);

    let scale = range.scale() as f32;
    let offset = range.offset() as f32;
    let lower = range.lower_bound() as f32;
    let upper = range.upper_bound() as f32;

    for chunk in pixels.chunks_exact_mut(4) {
        if range.scales() {
            for c in &mut chunk[..3] {
                let v = *c * scale + offset;
                *c = clamp_nan(v, lower, upper);
            }
        } else if range.min_out.is_some() && range.max_out.is_some() {
            for c in &mut chunk[..3] {
                *c = clamp_nan(*c, lower, upper);
            }
        } else if range.min_out.is_some() {
            for c in &mut chunk[..3] {
                *c = if c.is_nan() { lower } else { c.max(lower) };
            }
        } else if range.max_out.is_some() {
            for c in &mut chunk[..3] {
                *c = if c.is_nan() { upper } else { c.min(upper) };
            }
        }
        // Alpha unchanged
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Clamp value to [lower, upper], treating NaN as lower bound.
#[inline]
fn clamp_nan(v: f32, lower: f32, upper: f32) -> f32 {
    if v.is_nan() {
        lower
    } else if v < lower {
        lower
    } else if v > upper {
        upper
    } else {
        v
    }
}

/// Clamp value to [lower, upper], treating NaN as lower bound (f64).
#[inline]
fn clamp_nan_f64(v: f64, lower: f64, upper: f64) -> f64 {
    if v.is_nan() {
        lower
    } else if v < lower {
        lower
    } else if v > upper {
        upper
    } else {
        v
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-6;

    #[test]
    fn test_simple_clamp() {
        let range = Range::clamp(0.0, 1.0);

        let mut rgb = [1.5_f32, -0.2, 0.5];
        apply_range(&range, &mut rgb);

        assert!((rgb[0] - 1.0).abs() < EPSILON);
        assert!((rgb[1] - 0.0).abs() < EPSILON);
        assert!((rgb[2] - 0.5).abs() < EPSILON);
    }

    #[test]
    fn test_clamp_min_only() {
        let range = Range::clamp_min(0.0);

        let mut rgb = [1.5_f32, -0.2, 0.5];
        apply_range(&range, &mut rgb);

        assert!((rgb[0] - 1.5).abs() < EPSILON); // unchanged
        assert!((rgb[1] - 0.0).abs() < EPSILON); // clamped
        assert!((rgb[2] - 0.5).abs() < EPSILON);
    }

    #[test]
    fn test_clamp_max_only() {
        let range = Range::clamp_max(1.0);

        let mut rgb = [1.5_f32, -0.2, 0.5];
        apply_range(&range, &mut rgb);

        assert!((rgb[0] - 1.0).abs() < EPSILON); // clamped
        assert!((rgb[1] - -0.2).abs() < EPSILON); // unchanged
        assert!((rgb[2] - 0.5).abs() < EPSILON);
    }

    #[test]
    fn test_remap() {
        // Remap [0, 1] -> [0.1, 0.9]
        let range = Range::new(0.0, 1.0, 0.1, 0.9);

        let mut rgb = [0.0_f32, 0.5, 1.0];
        apply_range(&range, &mut rgb);

        assert!((rgb[0] - 0.1).abs() < EPSILON);
        assert!((rgb[1] - 0.5).abs() < EPSILON);
        assert!((rgb[2] - 0.9).abs() < EPSILON);
    }

    #[test]
    fn test_remap_with_clamp() {
        // Remap [0, 1] -> [0.1, 0.9], input outside range
        let range = Range::new(0.0, 1.0, 0.1, 0.9);

        let mut rgb = [-0.5_f32, 1.5, 0.5];
        apply_range(&range, &mut rgb);

        // -0.5 -> -0.5 * 0.8 + 0.1 = -0.3, clamped to 0.1
        assert!((rgb[0] - 0.1).abs() < EPSILON);
        // 1.5 -> 1.5 * 0.8 + 0.1 = 1.3, clamped to 0.9
        assert!((rgb[1] - 0.9).abs() < EPSILON);
        assert!((rgb[2] - 0.5).abs() < EPSILON);
    }

    #[test]
    fn test_nan_handling() {
        let range = Range::clamp(0.0, 1.0);

        let mut rgb = [f32::NAN, 0.5, f32::NAN];
        apply_range(&range, &mut rgb);

        // NaN becomes lower bound
        assert!((rgb[0] - 0.0).abs() < EPSILON);
        assert!((rgb[1] - 0.5).abs() < EPSILON);
        assert!((rgb[2] - 0.0).abs() < EPSILON);
    }

    #[test]
    fn test_inverse() {
        let range = Range::new(0.0, 1.0, 0.1, 0.9);
        let inv = range.inverse();

        assert_eq!(inv.min_in, Some(0.1));
        assert_eq!(inv.max_in, Some(0.9));
        assert_eq!(inv.min_out, Some(0.0));
        assert_eq!(inv.max_out, Some(1.0));
    }

    #[test]
    fn test_scales() {
        // No scaling
        let range = Range::clamp(0.0, 1.0);
        assert!(!range.scales());

        // With scaling
        let range = Range::new(0.0, 1.0, 0.0, 0.5);
        assert!(range.scales());

        // Same in/out range = no scaling
        let range = Range::new(0.0, 1.0, 0.0, 1.0);
        assert!(!range.scales());
    }

    #[test]
    fn test_rgba_buffer() {
        let range = Range::clamp(0.0, 1.0);

        let mut pixels = [
            1.5, -0.2, 0.5, 0.8, // pixel 1, alpha 0.8
            0.3, 0.4, 1.2, 1.0,  // pixel 2, alpha 1.0
        ];

        apply_range_rgba(&range, &mut pixels);

        // Pixel 1
        assert!((pixels[0] - 1.0).abs() < EPSILON);
        assert!((pixels[1] - 0.0).abs() < EPSILON);
        assert!((pixels[2] - 0.5).abs() < EPSILON);
        assert!((pixels[3] - 0.8).abs() < EPSILON); // alpha unchanged

        // Pixel 2
        assert!((pixels[4] - 0.3).abs() < EPSILON);
        assert!((pixels[5] - 0.4).abs() < EPSILON);
        assert!((pixels[6] - 1.0).abs() < EPSILON);
        assert!((pixels[7] - 1.0).abs() < EPSILON); // alpha unchanged
    }
}
