//! AllocationOp - LUT shaping for optimal value distribution.
//!
//! Reference: OCIO AllocationOp.cpp
//!
//! Used to prepare values before LUT sampling to ensure optimal distribution.
//! Two allocation types:
//! - Uniform: linear remapping from [min, max] to [0, 1]
//! - Lg2: log2 transformation then linear fit

/// Allocation type for LUT shaping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Allocation {
    /// Linear/uniform allocation: maps [min, max] to [0, 1].
    #[default]
    Uniform,
    /// Log2 allocation: log2(x + offset), then maps [min, max] to [0, 1].
    /// Default range is [-10, 6] (stops relative to 18% grey).
    Lg2,
}

/// Allocation operation parameters.
#[derive(Debug, Clone)]
pub struct AllocationOp {
    /// Allocation type.
    pub allocation: Allocation,
    /// Minimum value in source range (default: 0.0 for Uniform, -10.0 for Lg2).
    pub min: f64,
    /// Maximum value in source range (default: 1.0 for Uniform, 6.0 for Lg2).
    pub max: f64,
    /// Offset added before log2 (only used for Lg2, default: 0.0).
    pub offset: f64,
}

impl AllocationOp {
    /// Create uniform allocation with default [0, 1] range.
    pub fn uniform() -> Self {
        Self {
            allocation: Allocation::Uniform,
            min: 0.0,
            max: 1.0,
            offset: 0.0,
        }
    }
    
    /// Create uniform allocation with custom range.
    pub fn uniform_range(min: f64, max: f64) -> Self {
        Self {
            allocation: Allocation::Uniform,
            min,
            max,
            offset: 0.0,
        }
    }
    
    /// Create lg2 allocation with default [-10, 6] range.
    pub fn lg2() -> Self {
        Self {
            allocation: Allocation::Lg2,
            min: -10.0,
            max: 6.0,
            offset: 0.0,
        }
    }
    
    /// Create lg2 allocation with custom range.
    pub fn lg2_range(min: f64, max: f64) -> Self {
        Self {
            allocation: Allocation::Lg2,
            min,
            max,
            offset: 0.0,
        }
    }
    
    /// Create lg2 allocation with custom range and offset.
    pub fn lg2_full(min: f64, max: f64, offset: f64) -> Self {
        Self {
            allocation: Allocation::Lg2,
            min,
            max,
            offset,
        }
    }
    
    /// Check if this is identity (no transformation needed).
    pub fn is_identity(&self) -> bool {
        match self.allocation {
            Allocation::Uniform => {
                (self.min - 0.0).abs() < 1e-9 && (self.max - 1.0).abs() < 1e-9
            }
            Allocation::Lg2 => false, // Lg2 always transforms
        }
    }
}

impl Default for AllocationOp {
    fn default() -> Self {
        Self::uniform()
    }
}

// ============================================================================
// Apply functions
// ============================================================================

/// Apply allocation transform (forward: prepare for LUT).
#[inline]
pub fn apply_allocation_fwd(op: &AllocationOp, rgb: &mut [f32; 3]) {
    match op.allocation {
        Allocation::Uniform => {
            apply_fit_fwd(rgb, op.min as f32, op.max as f32);
        }
        Allocation::Lg2 => {
            apply_log2_fwd(rgb, op.offset as f32);
            apply_fit_fwd(rgb, op.min as f32, op.max as f32);
        }
    }
}

/// Apply allocation transform inverse (reverse: after LUT lookup).
#[inline]
pub fn apply_allocation_inv(op: &AllocationOp, rgb: &mut [f32; 3]) {
    match op.allocation {
        Allocation::Uniform => {
            apply_fit_inv(rgb, op.min as f32, op.max as f32);
        }
        Allocation::Lg2 => {
            apply_fit_inv(rgb, op.min as f32, op.max as f32);
            apply_log2_inv(rgb, op.offset as f32);
        }
    }
}

/// Apply allocation transform to RGBA buffer (forward).
pub fn apply_allocation_fwd_rgba(op: &AllocationOp, pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        apply_allocation_fwd(op, &mut rgb);
        chunk[0] = rgb[0];
        chunk[1] = rgb[1];
        chunk[2] = rgb[2];
        // Alpha unchanged
    }
}

/// Apply allocation transform to RGBA buffer (inverse).
pub fn apply_allocation_inv_rgba(op: &AllocationOp, pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        apply_allocation_inv(op, &mut rgb);
        chunk[0] = rgb[0];
        chunk[1] = rgb[1];
        chunk[2] = rgb[2];
        // Alpha unchanged
    }
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Fit/remap from [min, max] to [0, 1].
#[inline]
fn apply_fit_fwd(rgb: &mut [f32; 3], min: f32, max: f32) {
    let scale = 1.0 / (max - min);
    rgb[0] = (rgb[0] - min) * scale;
    rgb[1] = (rgb[1] - min) * scale;
    rgb[2] = (rgb[2] - min) * scale;
}

/// Fit/remap from [0, 1] to [min, max].
#[inline]
fn apply_fit_inv(rgb: &mut [f32; 3], min: f32, max: f32) {
    let scale = max - min;
    rgb[0] = rgb[0] * scale + min;
    rgb[1] = rgb[1] * scale + min;
    rgb[2] = rgb[2] * scale + min;
}

/// Apply log2 transform (forward).
/// out = log2(in + offset)
#[inline]
fn apply_log2_fwd(rgb: &mut [f32; 3], offset: f32) {
    rgb[0] = (rgb[0] + offset).max(f32::MIN_POSITIVE).log2();
    rgb[1] = (rgb[1] + offset).max(f32::MIN_POSITIVE).log2();
    rgb[2] = (rgb[2] + offset).max(f32::MIN_POSITIVE).log2();
}

/// Apply log2 transform inverse.
/// out = 2^in - offset
#[inline]
fn apply_log2_inv(rgb: &mut [f32; 3], offset: f32) {
    rgb[0] = 2.0_f32.powf(rgb[0]) - offset;
    rgb[1] = 2.0_f32.powf(rgb[1]) - offset;
    rgb[2] = 2.0_f32.powf(rgb[2]) - offset;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    const EPSILON: f32 = 1e-5;
    
    // ========================================================================
    // Uniform allocation tests
    // ========================================================================
    
    #[test]
    fn test_uniform_identity() {
        let op = AllocationOp::uniform();
        assert!(op.is_identity());
        
        let original = [0.5, 0.25, 0.75];
        let mut rgb = original;
        
        apply_allocation_fwd(&op, &mut rgb);
        
        // [0,1] -> [0,1] should be identity
        assert!((rgb[0] - original[0]).abs() < EPSILON);
        assert!((rgb[1] - original[1]).abs() < EPSILON);
        assert!((rgb[2] - original[2]).abs() < EPSILON);
    }
    
    #[test]
    fn test_uniform_range() {
        let op = AllocationOp::uniform_range(0.0, 2.0);
        
        let mut rgb = [0.0, 1.0, 2.0];
        apply_allocation_fwd(&op, &mut rgb);
        
        // [0, 2] -> [0, 1]
        assert!((rgb[0] - 0.0).abs() < EPSILON);  // 0/2 = 0
        assert!((rgb[1] - 0.5).abs() < EPSILON);  // 1/2 = 0.5
        assert!((rgb[2] - 1.0).abs() < EPSILON);  // 2/2 = 1
    }
    
    #[test]
    fn test_uniform_roundtrip() {
        let op = AllocationOp::uniform_range(-1.0, 3.0);
        
        let original = [0.5, 1.0, 2.0];
        let mut rgb = original;
        
        apply_allocation_fwd(&op, &mut rgb);
        apply_allocation_inv(&op, &mut rgb);
        
        assert!((rgb[0] - original[0]).abs() < EPSILON);
        assert!((rgb[1] - original[1]).abs() < EPSILON);
        assert!((rgb[2] - original[2]).abs() < EPSILON);
    }
    
    // ========================================================================
    // Lg2 allocation tests
    // ========================================================================
    
    #[test]
    fn test_lg2_default_range() {
        let op = AllocationOp::lg2();
        
        // 18% grey is approximately 2^-2.47 = 0.18
        // log2(0.18) ≈ -2.47
        // Fit [-10, 6] -> [0, 1]: (-2.47 - (-10)) / 16 ≈ 0.47
        
        let mut rgb = [0.18, 0.18, 0.18];
        apply_allocation_fwd(&op, &mut rgb);
        
        // Should be roughly in middle of [0, 1] range
        assert!(rgb[0] > 0.4 && rgb[0] < 0.6);
    }
    
    #[test]
    fn test_lg2_roundtrip() {
        let op = AllocationOp::lg2();
        
        let original = [0.18, 0.5, 1.0];
        let mut rgb = original;
        
        apply_allocation_fwd(&op, &mut rgb);
        apply_allocation_inv(&op, &mut rgb);
        
        assert!((rgb[0] - original[0]).abs() < EPSILON);
        assert!((rgb[1] - original[1]).abs() < EPSILON);
        assert!((rgb[2] - original[2]).abs() < EPSILON);
    }
    
    #[test]
    fn test_lg2_with_offset() {
        let op = AllocationOp::lg2_full(-10.0, 6.0, 0.01);
        
        // With offset, should handle values down to -0.01
        let original = [0.18, 0.5, 1.0];
        let mut rgb = original;
        
        apply_allocation_fwd(&op, &mut rgb);
        apply_allocation_inv(&op, &mut rgb);
        
        assert!((rgb[0] - original[0]).abs() < EPSILON);
        assert!((rgb[1] - original[1]).abs() < EPSILON);
        assert!((rgb[2] - original[2]).abs() < EPSILON);
    }
    
    #[test]
    fn test_lg2_custom_range() {
        let op = AllocationOp::lg2_range(-8.0, 4.0);
        
        // 12 stops range instead of default 16
        let original = [0.18, 0.5, 1.0];
        let mut rgb = original;
        
        apply_allocation_fwd(&op, &mut rgb);
        apply_allocation_inv(&op, &mut rgb);
        
        assert!((rgb[0] - original[0]).abs() < EPSILON);
        assert!((rgb[1] - original[1]).abs() < EPSILON);
        assert!((rgb[2] - original[2]).abs() < EPSILON);
    }
    
    // ========================================================================
    // Known values tests
    // ========================================================================
    
    #[test]
    fn test_lg2_known_values() {
        let op = AllocationOp::lg2_range(-10.0, 6.0);
        
        // 2^0 = 1.0 -> log2 = 0 -> fit: (0 - (-10)) / 16 = 10/16 = 0.625
        let mut rgb = [1.0, 1.0, 1.0];
        apply_allocation_fwd(&op, &mut rgb);
        assert!((rgb[0] - 0.625).abs() < EPSILON);
        
        // 2^6 = 64 -> log2 = 6 -> fit: (6 - (-10)) / 16 = 16/16 = 1.0
        let mut rgb = [64.0, 64.0, 64.0];
        apply_allocation_fwd(&op, &mut rgb);
        assert!((rgb[0] - 1.0).abs() < EPSILON);
    }
    
    #[test]
    fn test_uniform_known_values() {
        let op = AllocationOp::uniform_range(-0.5, 1.5);
        
        // -0.5 -> 0, 0.5 -> 0.5, 1.5 -> 1.0
        let mut rgb = [-0.5, 0.5, 1.5];
        apply_allocation_fwd(&op, &mut rgb);
        
        assert!((rgb[0] - 0.0).abs() < EPSILON);
        assert!((rgb[1] - 0.5).abs() < EPSILON);
        assert!((rgb[2] - 1.0).abs() < EPSILON);
    }
    
    // ========================================================================
    // RGBA buffer tests
    // ========================================================================
    
    #[test]
    fn test_rgba_buffer_uniform() {
        let op = AllocationOp::uniform_range(0.0, 2.0);
        
        let mut pixels = [
            0.0, 1.0, 2.0, 1.0,  // pixel 1
            0.5, 1.5, 0.25, 0.8, // pixel 2
        ];
        
        apply_allocation_fwd_rgba(&op, &mut pixels);
        
        // Pixel 1 RGB
        assert!((pixels[0] - 0.0).abs() < EPSILON);
        assert!((pixels[1] - 0.5).abs() < EPSILON);
        assert!((pixels[2] - 1.0).abs() < EPSILON);
        assert!((pixels[3] - 1.0).abs() < EPSILON); // alpha unchanged
        
        // Pixel 2 RGB
        assert!((pixels[4] - 0.25).abs() < EPSILON);
        assert!((pixels[5] - 0.75).abs() < EPSILON);
        assert!((pixels[6] - 0.125).abs() < EPSILON);
        assert!((pixels[7] - 0.8).abs() < EPSILON); // alpha unchanged
    }
    
    #[test]
    fn test_rgba_buffer_lg2_roundtrip() {
        let op = AllocationOp::lg2();
        
        let original = [
            0.18, 0.5, 1.0, 1.0,
            0.25, 0.75, 0.5, 0.5,
        ];
        let mut pixels = original;
        
        apply_allocation_fwd_rgba(&op, &mut pixels);
        apply_allocation_inv_rgba(&op, &mut pixels);
        
        for i in 0..8 {
            assert!(
                (pixels[i] - original[i]).abs() < EPSILON,
                "Mismatch at index {i}: {} vs {}", pixels[i], original[i]
            );
        }
    }
    
    // ========================================================================
    // Edge cases
    // ========================================================================
    
    #[test]
    fn test_lg2_very_small_values() {
        let op = AllocationOp::lg2();
        
        // Very small values should still work (clamped to MIN_POSITIVE)
        let mut rgb = [1e-10, 1e-20, 0.0];
        apply_allocation_fwd(&op, &mut rgb);
        
        // Should all be finite
        assert!(rgb[0].is_finite());
        assert!(rgb[1].is_finite());
        assert!(rgb[2].is_finite());
    }
    
    #[test]
    fn test_uniform_negative_range() {
        let op = AllocationOp::uniform_range(-1.0, 0.0);
        
        let mut rgb = [-1.0, -0.5, 0.0];
        apply_allocation_fwd(&op, &mut rgb);
        
        assert!((rgb[0] - 0.0).abs() < EPSILON);
        assert!((rgb[1] - 0.5).abs() < EPSILON);
        assert!((rgb[2] - 1.0).abs() < EPSILON);
    }
}
