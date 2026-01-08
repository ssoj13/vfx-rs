//! ASC CDL (Color Decision List) operations.
//!
//! Implements the ASC CDL v1.2 specification for color correction.
//! CDL applies Slope, Offset, Power (SOP) and Saturation adjustments.
//!
//! # Formula
//!
//! ```text
//! out = clamp((in * slope + offset) ^ power)
//! ```
//!
//! Then saturation is applied using Rec.709 luminance weights.
//!
//! # Reference
//!
//! ASC CDL Transfer Functions and Interchange Syntax v1.2

/// CDL parameters for color correction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cdl {
    /// Slope (multiplicative factor) per channel [R, G, B]
    pub slope: [f32; 3],
    /// Offset (additive) per channel [R, G, B]
    pub offset: [f32; 3],
    /// Power (gamma exponent) per channel [R, G, B]
    pub power: [f32; 3],
    /// Saturation adjustment (1.0 = no change)
    pub saturation: f32,
}

impl Default for Cdl {
    fn default() -> Self {
        Self::identity()
    }
}

impl Cdl {
    /// Create identity CDL (no change).
    #[inline]
    pub fn identity() -> Self {
        Self {
            slope: [1.0, 1.0, 1.0],
            offset: [0.0, 0.0, 0.0],
            power: [1.0, 1.0, 1.0],
            saturation: 1.0,
        }
    }

    /// Create CDL with uniform values across channels.
    #[inline]
    pub fn uniform(slope: f32, offset: f32, power: f32, saturation: f32) -> Self {
        Self {
            slope: [slope, slope, slope],
            offset: [offset, offset, offset],
            power: [power, power, power],
            saturation,
        }
    }

    /// Check if this CDL is identity (no-op).
    #[inline]
    pub fn is_identity(&self) -> bool {
        self.slope == [1.0, 1.0, 1.0]
            && self.offset == [0.0, 0.0, 0.0]
            && self.power == [1.0, 1.0, 1.0]
            && (self.saturation - 1.0).abs() < 1e-6
    }

    /// Apply CDL to a single RGB pixel.
    /// 
    /// Uses ASC CDL v1.2 formula with clamping.
    #[inline]
    pub fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        // Apply SOP (Slope, Offset, Power)
        let r = apply_sop(rgb[0], self.slope[0], self.offset[0], self.power[0]);
        let g = apply_sop(rgb[1], self.slope[1], self.offset[1], self.power[1]);
        let b = apply_sop(rgb[2], self.slope[2], self.offset[2], self.power[2]);

        // Apply saturation
        if (self.saturation - 1.0).abs() > 1e-6 {
            apply_saturation([r, g, b], self.saturation)
        } else {
            [r, g, b]
        }
    }

    /// Apply CDL without clamping (extended range).
    #[inline]
    pub fn apply_no_clamp(&self, rgb: [f32; 3]) -> [f32; 3] {
        let r = apply_sop_no_clamp(rgb[0], self.slope[0], self.offset[0], self.power[0]);
        let g = apply_sop_no_clamp(rgb[1], self.slope[1], self.offset[1], self.power[1]);
        let b = apply_sop_no_clamp(rgb[2], self.slope[2], self.offset[2], self.power[2]);

        if (self.saturation - 1.0).abs() > 1e-6 {
            apply_saturation([r, g, b], self.saturation)
        } else {
            [r, g, b]
        }
    }

    /// Apply inverse CDL to a single RGB pixel.
    #[inline]
    pub fn apply_inverse(&self, rgb: [f32; 3]) -> [f32; 3] {
        // Inverse saturation first
        let [r, g, b] = if (self.saturation - 1.0).abs() > 1e-6 {
            apply_saturation(rgb, 1.0 / self.saturation)
        } else {
            rgb
        };

        // Inverse SOP: in = ((out ^ (1/power)) - offset) / slope
        [
            apply_sop_inverse(r, self.slope[0], self.offset[0], self.power[0]),
            apply_sop_inverse(g, self.slope[1], self.offset[1], self.power[1]),
            apply_sop_inverse(b, self.slope[2], self.offset[2], self.power[2]),
        ]
    }
}

/// Apply SOP with clamping (ASC CDL v1.2).
#[inline]
fn apply_sop(x: f32, slope: f32, offset: f32, power: f32) -> f32 {
    let v = (x * slope + offset).max(0.0);
    v.powf(power).clamp(0.0, 1.0)
}

/// Apply SOP without clamping (for HDR/scene-referred).
#[inline]
fn apply_sop_no_clamp(x: f32, slope: f32, offset: f32, power: f32) -> f32 {
    let v = x * slope + offset;
    if v >= 0.0 {
        v.powf(power)
    } else {
        // Mirror for negative values
        -(-v).powf(power)
    }
}

/// Apply inverse SOP.
#[inline]
fn apply_sop_inverse(x: f32, slope: f32, offset: f32, power: f32) -> f32 {
    if slope.abs() < 1e-10 {
        return 0.0;
    }
    let v = if x >= 0.0 {
        x.powf(1.0 / power)
    } else {
        -(-x).powf(1.0 / power)
    };
    (v - offset) / slope
}

/// Rec.709 luminance weights.
const LUM_R: f32 = 0.2126;
const LUM_G: f32 = 0.7152;
const LUM_B: f32 = 0.0722;

/// Apply saturation adjustment using Rec.709 luminance.
#[inline]
fn apply_saturation(rgb: [f32; 3], sat: f32) -> [f32; 3] {
    let lum = rgb[0] * LUM_R + rgb[1] * LUM_G + rgb[2] * LUM_B;
    [
        lum + (rgb[0] - lum) * sat,
        lum + (rgb[1] - lum) * sat,
        lum + (rgb[2] - lum) * sat,
    ]
}

/// Apply CDL to an image buffer in-place.
pub fn apply_cdl_inplace(buffer: &mut [f32], cdl: &Cdl) {
    if cdl.is_identity() {
        return;
    }

    // Process in chunks of 3 (RGB)
    for chunk in buffer.chunks_exact_mut(3) {
        let rgb = [chunk[0], chunk[1], chunk[2]];
        let result = cdl.apply(rgb);
        chunk[0] = result[0];
        chunk[1] = result[1];
        chunk[2] = result[2];
    }
}

/// Apply CDL to RGBA buffer in-place (alpha unchanged).
pub fn apply_cdl_rgba_inplace(buffer: &mut [f32], cdl: &Cdl) {
    if cdl.is_identity() {
        return;
    }

    for chunk in buffer.chunks_exact_mut(4) {
        let rgb = [chunk[0], chunk[1], chunk[2]];
        let result = cdl.apply(rgb);
        chunk[0] = result[0];
        chunk[1] = result[1];
        chunk[2] = result[2];
        // alpha unchanged
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    #[test]
    fn identity() {
        let cdl = Cdl::identity();
        assert!(cdl.is_identity());

        let rgb = [0.5, 0.3, 0.7];
        let result = cdl.apply(rgb);
        assert!((result[0] - rgb[0]).abs() < EPSILON);
        assert!((result[1] - rgb[1]).abs() < EPSILON);
        assert!((result[2] - rgb[2]).abs() < EPSILON);
    }

    #[test]
    fn slope_only() {
        let cdl = Cdl {
            slope: [2.0, 2.0, 2.0],
            offset: [0.0, 0.0, 0.0],
            power: [1.0, 1.0, 1.0],
            saturation: 1.0,
        };

        let rgb = [0.25, 0.25, 0.25];
        let result = cdl.apply(rgb);
        assert!((result[0] - 0.5).abs() < EPSILON);
        assert!((result[1] - 0.5).abs() < EPSILON);
        assert!((result[2] - 0.5).abs() < EPSILON);
    }

    #[test]
    fn offset_only() {
        let cdl = Cdl {
            slope: [1.0, 1.0, 1.0],
            offset: [0.1, 0.1, 0.1],
            power: [1.0, 1.0, 1.0],
            saturation: 1.0,
        };

        let rgb = [0.2, 0.2, 0.2];
        let result = cdl.apply(rgb);
        assert!((result[0] - 0.3).abs() < EPSILON);
    }

    #[test]
    fn power_only() {
        let cdl = Cdl {
            slope: [1.0, 1.0, 1.0],
            offset: [0.0, 0.0, 0.0],
            power: [2.0, 2.0, 2.0],
            saturation: 1.0,
        };

        let rgb = [0.5, 0.5, 0.5];
        let result = cdl.apply(rgb);
        assert!((result[0] - 0.25).abs() < EPSILON);
    }

    #[test]
    fn saturation() {
        let cdl = Cdl {
            slope: [1.0, 1.0, 1.0],
            offset: [0.0, 0.0, 0.0],
            power: [1.0, 1.0, 1.0],
            saturation: 0.0,  // desaturate completely
        };

        let rgb = [1.0, 0.0, 0.0];  // pure red
        let result = cdl.apply(rgb);
        
        // Should become grey (luminance of red)
        let lum = LUM_R;
        assert!((result[0] - lum).abs() < EPSILON);
        assert!((result[1] - lum).abs() < EPSILON);
        assert!((result[2] - lum).abs() < EPSILON);
    }

    #[test]
    fn roundtrip() {
        let cdl = Cdl {
            slope: [1.2, 0.9, 1.1],
            offset: [0.01, -0.02, 0.03],
            power: [1.1, 0.95, 1.05],
            saturation: 1.1,
        };

        let rgb = [0.3, 0.5, 0.4];
        let forward = cdl.apply_no_clamp(rgb);
        let inverse = cdl.apply_inverse(forward);

        assert!((inverse[0] - rgb[0]).abs() < 0.001, "R: {} vs {}", inverse[0], rgb[0]);
        assert!((inverse[1] - rgb[1]).abs() < 0.001, "G: {} vs {}", inverse[1], rgb[1]);
        assert!((inverse[2] - rgb[2]).abs() < 0.001, "B: {} vs {}", inverse[2], rgb[2]);
    }

    #[test]
    fn clamping() {
        let cdl = Cdl {
            slope: [2.0, 2.0, 2.0],
            offset: [0.0, 0.0, 0.0],
            power: [1.0, 1.0, 1.0],
            saturation: 1.0,
        };

        let rgb = [0.8, 0.8, 0.8];
        let result = cdl.apply(rgb);
        
        // 0.8 * 2 = 1.6, should clamp to 1.0
        assert!((result[0] - 1.0).abs() < EPSILON);
    }

    #[test]
    fn negative_handling() {
        let cdl = Cdl::uniform(1.0, -0.1, 2.0, 1.0);
        
        // Input that becomes negative after offset
        let rgb = [0.05, 0.05, 0.05];  // 0.05 - 0.1 = -0.05
        
        // Standard CDL clamps to 0 before power
        let result = cdl.apply(rgb);
        assert!((result[0]).abs() < EPSILON);  // clamped to 0
        
        // No-clamp version handles negative
        let result_nc = cdl.apply_no_clamp(rgb);
        // (-0.05)^2 with sign = -0.0025
        assert!(result_nc[0] < 0.0);
    }
}
