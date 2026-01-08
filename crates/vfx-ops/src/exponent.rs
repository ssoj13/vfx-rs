//! ExponentOp - per-channel power function.
//!
//! Reference: OCIO ExponentOp.cpp and GammaOpCPU.cpp

/// How to handle negative input values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NegativeStyle {
    /// Clamp negative values to 0 before applying power (OCIO default).
    #[default]
    Clamp,
    /// Mirror: sign(x) * pow(|x|, exp) - preserves sign.
    Mirror,
    /// Pass through: negative values unchanged, only apply power to positive.
    PassThru,
}

/// Exponent operation with per-channel exponents.
///
/// Applies `out = pow(in, exp)` with configurable negative handling.
#[derive(Debug, Clone)]
pub struct ExponentOp {
    /// Exponent for red channel.
    pub red: f64,
    /// Exponent for green channel.
    pub green: f64,
    /// Exponent for blue channel.
    pub blue: f64,
    /// Exponent for alpha channel.
    pub alpha: f64,
    /// How to handle negative values.
    pub negative_style: NegativeStyle,
}

impl ExponentOp {
    /// Create with uniform exponent for RGB and 1.0 for alpha.
    pub fn uniform(exp: f64) -> Self {
        Self {
            red: exp,
            green: exp,
            blue: exp,
            alpha: 1.0,
            negative_style: NegativeStyle::Clamp,
        }
    }
    
    /// Create with uniform exponent for all channels.
    pub fn uniform_all(exp: f64) -> Self {
        Self {
            red: exp,
            green: exp,
            blue: exp,
            alpha: exp,
            negative_style: NegativeStyle::Clamp,
        }
    }
    
    /// Create with per-channel exponents.
    pub fn per_channel(red: f64, green: f64, blue: f64, alpha: f64) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
            negative_style: NegativeStyle::Clamp,
        }
    }
    
    /// Set negative handling style.
    pub fn with_negative_style(mut self, style: NegativeStyle) -> Self {
        self.negative_style = style;
        self
    }
    
    /// Check if this is identity (all exponents == 1.0).
    pub fn is_identity(&self) -> bool {
        (self.red - 1.0).abs() < 1e-9
            && (self.green - 1.0).abs() < 1e-9
            && (self.blue - 1.0).abs() < 1e-9
            && (self.alpha - 1.0).abs() < 1e-9
    }
    
    /// Get inverse operation.
    ///
    /// Returns None if any exponent is 0.
    pub fn inverse(&self) -> Option<Self> {
        if self.red.abs() < 1e-15
            || self.green.abs() < 1e-15
            || self.blue.abs() < 1e-15
            || self.alpha.abs() < 1e-15
        {
            return None;
        }
        
        Some(Self {
            red: 1.0 / self.red,
            green: 1.0 / self.green,
            blue: 1.0 / self.blue,
            alpha: 1.0 / self.alpha,
            negative_style: self.negative_style,
        })
    }
    
    /// Combine with another ExponentOp (multiply exponents).
    pub fn combine(&self, other: &ExponentOp) -> Self {
        Self {
            red: self.red * other.red,
            green: self.green * other.green,
            blue: self.blue * other.blue,
            alpha: self.alpha * other.alpha,
            negative_style: self.negative_style,
        }
    }
}

impl Default for ExponentOp {
    fn default() -> Self {
        Self {
            red: 1.0,
            green: 1.0,
            blue: 1.0,
            alpha: 1.0,
            negative_style: NegativeStyle::Clamp,
        }
    }
}

// ============================================================================
// Apply functions
// ============================================================================

/// Apply power with clamping (negative -> 0).
#[inline]
fn pow_clamp(x: f32, exp: f32) -> f32 {
    x.max(0.0).powf(exp)
}

/// Apply power with mirroring (preserves sign).
#[inline]
fn pow_mirror(x: f32, exp: f32) -> f32 {
    x.signum() * x.abs().powf(exp)
}

/// Apply power with pass-through (negative unchanged).
#[inline]
fn pow_passthru(x: f32, exp: f32) -> f32 {
    if x > 0.0 {
        x.powf(exp)
    } else {
        x
    }
}

/// Apply exponent operation to RGB values.
#[inline]
pub fn apply_exponent(op: &ExponentOp, rgb: &mut [f32; 3]) {
    let exp_r = op.red as f32;
    let exp_g = op.green as f32;
    let exp_b = op.blue as f32;
    
    match op.negative_style {
        NegativeStyle::Clamp => {
            rgb[0] = pow_clamp(rgb[0], exp_r);
            rgb[1] = pow_clamp(rgb[1], exp_g);
            rgb[2] = pow_clamp(rgb[2], exp_b);
        }
        NegativeStyle::Mirror => {
            rgb[0] = pow_mirror(rgb[0], exp_r);
            rgb[1] = pow_mirror(rgb[1], exp_g);
            rgb[2] = pow_mirror(rgb[2], exp_b);
        }
        NegativeStyle::PassThru => {
            rgb[0] = pow_passthru(rgb[0], exp_r);
            rgb[1] = pow_passthru(rgb[1], exp_g);
            rgb[2] = pow_passthru(rgb[2], exp_b);
        }
    }
}

/// Apply exponent operation to RGBA buffer.
pub fn apply_exponent_rgba(op: &ExponentOp, pixels: &mut [f32]) {
    let exp_r = op.red as f32;
    let exp_g = op.green as f32;
    let exp_b = op.blue as f32;
    let exp_a = op.alpha as f32;
    
    match op.negative_style {
        NegativeStyle::Clamp => {
            for chunk in pixels.chunks_exact_mut(4) {
                chunk[0] = pow_clamp(chunk[0], exp_r);
                chunk[1] = pow_clamp(chunk[1], exp_g);
                chunk[2] = pow_clamp(chunk[2], exp_b);
                chunk[3] = pow_clamp(chunk[3], exp_a);
            }
        }
        NegativeStyle::Mirror => {
            for chunk in pixels.chunks_exact_mut(4) {
                chunk[0] = pow_mirror(chunk[0], exp_r);
                chunk[1] = pow_mirror(chunk[1], exp_g);
                chunk[2] = pow_mirror(chunk[2], exp_b);
                chunk[3] = pow_mirror(chunk[3], exp_a);
            }
        }
        NegativeStyle::PassThru => {
            for chunk in pixels.chunks_exact_mut(4) {
                chunk[0] = pow_passthru(chunk[0], exp_r);
                chunk[1] = pow_passthru(chunk[1], exp_g);
                chunk[2] = pow_passthru(chunk[2], exp_b);
                chunk[3] = pow_passthru(chunk[3], exp_a);
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    const EPSILON: f32 = 1e-6;
    
    // ========================================================================
    // Basic exponent tests
    // ========================================================================
    
    #[test]
    fn test_identity() {
        let op = ExponentOp::default();
        assert!(op.is_identity());
        
        let mut rgb = [0.5, 0.25, 0.75];
        apply_exponent(&op, &mut rgb);
        assert!((rgb[0] - 0.5).abs() < EPSILON);
        assert!((rgb[1] - 0.25).abs() < EPSILON);
        assert!((rgb[2] - 0.75).abs() < EPSILON);
    }
    
    #[test]
    fn test_square() {
        let op = ExponentOp::uniform(2.0);
        
        let mut rgb = [0.5, 0.25, 0.75];
        apply_exponent(&op, &mut rgb);
        
        assert!((rgb[0] - 0.25).abs() < EPSILON);   // 0.5^2 = 0.25
        assert!((rgb[1] - 0.0625).abs() < EPSILON); // 0.25^2 = 0.0625
        assert!((rgb[2] - 0.5625).abs() < EPSILON); // 0.75^2 = 0.5625
    }
    
    #[test]
    fn test_sqrt() {
        let op = ExponentOp::uniform(0.5);
        
        let mut rgb = [0.25, 0.16, 0.64];
        apply_exponent(&op, &mut rgb);
        
        assert!((rgb[0] - 0.5).abs() < EPSILON);  // sqrt(0.25) = 0.5
        assert!((rgb[1] - 0.4).abs() < EPSILON);  // sqrt(0.16) = 0.4
        assert!((rgb[2] - 0.8).abs() < EPSILON);  // sqrt(0.64) = 0.8
    }
    
    #[test]
    fn test_per_channel() {
        let op = ExponentOp::per_channel(2.0, 0.5, 1.0, 1.0);
        
        let mut rgb = [0.5, 0.25, 0.75];
        apply_exponent(&op, &mut rgb);
        
        assert!((rgb[0] - 0.25).abs() < EPSILON);  // 0.5^2 = 0.25
        assert!((rgb[1] - 0.5).abs() < EPSILON);   // 0.25^0.5 = 0.5
        assert!((rgb[2] - 0.75).abs() < EPSILON);  // 0.75^1 = 0.75
    }
    
    // ========================================================================
    // Negative handling tests
    // ========================================================================
    
    #[test]
    fn test_clamp_negative() {
        let op = ExponentOp::uniform(2.0);
        
        let mut rgb = [-0.5, -0.25, 0.5];
        apply_exponent(&op, &mut rgb);
        
        assert!((rgb[0] - 0.0).abs() < EPSILON);  // clamped to 0
        assert!((rgb[1] - 0.0).abs() < EPSILON);  // clamped to 0
        assert!((rgb[2] - 0.25).abs() < EPSILON); // 0.5^2 = 0.25
    }
    
    #[test]
    fn test_mirror_negative() {
        let op = ExponentOp::uniform(2.0).with_negative_style(NegativeStyle::Mirror);
        
        let mut rgb = [-0.5, -0.25, 0.5];
        apply_exponent(&op, &mut rgb);
        
        assert!((rgb[0] - (-0.25)).abs() < EPSILON);  // -1 * 0.5^2 = -0.25
        assert!((rgb[1] - (-0.0625)).abs() < EPSILON); // -1 * 0.25^2 = -0.0625
        assert!((rgb[2] - 0.25).abs() < EPSILON);     // 0.5^2 = 0.25
    }
    
    #[test]
    fn test_passthru_negative() {
        let op = ExponentOp::uniform(2.0).with_negative_style(NegativeStyle::PassThru);
        
        let mut rgb = [-0.5, -0.25, 0.5];
        apply_exponent(&op, &mut rgb);
        
        assert!((rgb[0] - (-0.5)).abs() < EPSILON);   // unchanged
        assert!((rgb[1] - (-0.25)).abs() < EPSILON);  // unchanged
        assert!((rgb[2] - 0.25).abs() < EPSILON);     // 0.5^2 = 0.25
    }
    
    // ========================================================================
    // Roundtrip tests
    // ========================================================================
    
    #[test]
    fn test_roundtrip() {
        let op = ExponentOp::uniform(2.2);
        let inv = op.inverse().unwrap();
        
        let original = [0.2, 0.5, 0.8];
        let mut rgb = original;
        
        apply_exponent(&op, &mut rgb);
        apply_exponent(&inv, &mut rgb);
        
        assert!((rgb[0] - original[0]).abs() < EPSILON);
        assert!((rgb[1] - original[1]).abs() < EPSILON);
        assert!((rgb[2] - original[2]).abs() < EPSILON);
    }
    
    #[test]
    fn test_roundtrip_per_channel() {
        let op = ExponentOp::per_channel(1.8, 2.2, 2.6, 1.0);
        let inv = op.inverse().unwrap();
        
        let original = [0.3, 0.5, 0.7];
        let mut rgb = original;
        
        apply_exponent(&op, &mut rgb);
        apply_exponent(&inv, &mut rgb);
        
        assert!((rgb[0] - original[0]).abs() < EPSILON);
        assert!((rgb[1] - original[1]).abs() < EPSILON);
        assert!((rgb[2] - original[2]).abs() < EPSILON);
    }
    
    // ========================================================================
    // Combine tests
    // ========================================================================
    
    #[test]
    fn test_combine() {
        let op1 = ExponentOp::uniform(2.0);
        let op2 = ExponentOp::uniform(0.5);
        let combined = op1.combine(&op2);
        
        // 2.0 * 0.5 = 1.0 -> identity
        assert!(combined.is_identity());
    }
    
    #[test]
    fn test_combine_sequential() {
        let op1 = ExponentOp::uniform(2.0);
        let op2 = ExponentOp::uniform(3.0);
        let combined = op1.combine(&op2);
        
        // Applying combined should equal applying both sequentially
        let original = [0.5, 0.25, 0.75];
        
        let mut sequential = original;
        apply_exponent(&op1, &mut sequential);
        apply_exponent(&op2, &mut sequential);
        
        let mut combined_result = original;
        apply_exponent(&combined, &mut combined_result);
        
        assert!((sequential[0] - combined_result[0]).abs() < EPSILON);
        assert!((sequential[1] - combined_result[1]).abs() < EPSILON);
        assert!((sequential[2] - combined_result[2]).abs() < EPSILON);
    }
    
    // ========================================================================
    // Inverse edge cases
    // ========================================================================
    
    #[test]
    fn test_inverse_zero_fails() {
        let op = ExponentOp::per_channel(0.0, 2.0, 2.0, 1.0);
        assert!(op.inverse().is_none());
    }
    
    // ========================================================================
    // RGBA buffer test
    // ========================================================================
    
    #[test]
    fn test_rgba_buffer() {
        let op = ExponentOp::uniform_all(2.0);
        
        let mut pixels = [
            0.5, 0.25, 0.75, 1.0,  // pixel 1
            0.2, 0.4, 0.6, 0.8,   // pixel 2
        ];
        
        apply_exponent_rgba(&op, &mut pixels);
        
        // Pixel 1
        assert!((pixels[0] - 0.25).abs() < EPSILON);
        assert!((pixels[1] - 0.0625).abs() < EPSILON);
        assert!((pixels[2] - 0.5625).abs() < EPSILON);
        assert!((pixels[3] - 1.0).abs() < EPSILON);
        
        // Pixel 2
        assert!((pixels[4] - 0.04).abs() < EPSILON);
        assert!((pixels[5] - 0.16).abs() < EPSILON);
        assert!((pixels[6] - 0.36).abs() < EPSILON);
        assert!((pixels[7] - 0.64).abs() < EPSILON);
    }
    
    // ========================================================================
    // Edge value tests
    // ========================================================================
    
    #[test]
    fn test_zero() {
        let op = ExponentOp::uniform(2.0);
        
        let mut rgb = [0.0, 0.0, 0.0];
        apply_exponent(&op, &mut rgb);
        
        assert!((rgb[0]).abs() < EPSILON);
        assert!((rgb[1]).abs() < EPSILON);
        assert!((rgb[2]).abs() < EPSILON);
    }
    
    #[test]
    fn test_one() {
        let op = ExponentOp::uniform(2.0);
        
        let mut rgb = [1.0, 1.0, 1.0];
        apply_exponent(&op, &mut rgb);
        
        assert!((rgb[0] - 1.0).abs() < EPSILON);
        assert!((rgb[1] - 1.0).abs() < EPSILON);
        assert!((rgb[2] - 1.0).abs() < EPSILON);
    }
    
    #[test]
    fn test_large_values() {
        let op = ExponentOp::uniform(2.0);
        
        let mut rgb = [2.0, 3.0, 4.0];
        apply_exponent(&op, &mut rgb);
        
        assert!((rgb[0] - 4.0).abs() < EPSILON);
        assert!((rgb[1] - 9.0).abs() < EPSILON);
        assert!((rgb[2] - 16.0).abs() < EPSILON);
    }
    
    // ========================================================================
    // Gamma-like tests (sRGB-like curve)
    // ========================================================================
    
    #[test]
    fn test_srgb_gamma() {
        // sRGB gamma ~ 2.2
        let encode = ExponentOp::uniform(1.0 / 2.2);
        let decode = ExponentOp::uniform(2.2);
        
        // 18% grey should stay around 0.18 after encode->decode
        let original = [0.18, 0.18, 0.18];
        let mut rgb = original;
        
        apply_exponent(&encode, &mut rgb);
        apply_exponent(&decode, &mut rgb);
        
        assert!((rgb[0] - original[0]).abs() < EPSILON);
        assert!((rgb[1] - original[1]).abs() < EPSILON);
        assert!((rgb[2] - original[2]).abs() < EPSILON);
    }
}
