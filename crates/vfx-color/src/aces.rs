//! ACES (Academy Color Encoding System) transforms.
//!
//! Provides simplified ACES RRT (Reference Rendering Transform) and
//! ODT (Output Device Transform) implementations.
//!
//! # ACES Pipeline
//!
//! ```text
//! Input -> IDT -> ACES AP0 -> RRT -> OCES -> ODT -> Display
//!                    |                         |
//!              ACEScg (AP1)              sRGB/Rec.709
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use vfx_color::aces::{rrt_odt, acescg_to_srgb};
//!
//! // Apply RRT+ODT to ACEScg values
//! let display_rgb = rrt_odt(acescg_r, acescg_g, acescg_b);
//! ```

use vfx_primaries::{
    ACES_AP0, ACES_AP1, SRGB,
    rgb_to_rgb_matrix, rgb_to_xyz_matrix, xyz_to_rgb_matrix,
};
use vfx_math::{Mat3, Vec3, adapt_matrix, BRADFORD, D60, D65};

// ============================================================================
// Pre-computed ACES Matrices
// ============================================================================

/// ACEScg (AP1) to sRGB matrix with D60->D65 chromatic adaptation.
pub fn acescg_to_srgb_matrix() -> Mat3 {
    let ap1_to_xyz = rgb_to_xyz_matrix(&ACES_AP1);
    let adapt = adapt_matrix(BRADFORD, D60, D65);
    let xyz_to_srgb = xyz_to_rgb_matrix(&SRGB);
    xyz_to_srgb * adapt * ap1_to_xyz
}

/// sRGB to ACEScg (AP1) matrix with D65->D60 chromatic adaptation.
pub fn srgb_to_acescg_matrix() -> Mat3 {
    let srgb_to_xyz = rgb_to_xyz_matrix(&SRGB);
    let adapt = adapt_matrix(BRADFORD, D65, D60);
    let xyz_to_ap1 = xyz_to_rgb_matrix(&ACES_AP1);
    xyz_to_ap1 * adapt * srgb_to_xyz
}

/// ACES AP0 to ACEScg (AP1) matrix.
pub fn ap0_to_ap1_matrix() -> Mat3 {
    rgb_to_rgb_matrix(&ACES_AP0, &ACES_AP1)
}

/// ACEScg (AP1) to ACES AP0 matrix.
pub fn ap1_to_ap0_matrix() -> Mat3 {
    rgb_to_rgb_matrix(&ACES_AP1, &ACES_AP0)
}

// ============================================================================
// ACES RRT (Reference Rendering Transform) - Simplified Filmic Tonemap
// ============================================================================

/// ACES RRT parameters for the simplified filmic curve.
#[derive(Debug, Clone, Copy)]
pub struct RrtParams {
    /// Shoulder strength
    pub a: f32,
    /// Linear section strength  
    pub b: f32,
    /// Linear angle
    pub c: f32,
    /// Toe strength
    pub d: f32,
    /// Toe numerator
    pub e: f32,
    /// Toe denominator
    pub f: f32,
    /// White point (where curve reaches 1.0)
    pub white: f32,
}

impl Default for RrtParams {
    /// Default ACES-like filmic curve parameters.
    fn default() -> Self {
        // Based on ACES/Narkowicz fitted curve
        Self {
            a: 2.51,
            b: 0.03,
            c: 2.43,
            d: 0.59,
            e: 0.14,
            f: 0.14,
            white: 1.0,
        }
    }
}

impl RrtParams {
    /// Stephen Hill's fitted ACES curve.
    pub fn aces_fitted() -> Self {
        Self {
            a: 2.51,
            b: 0.03,
            c: 2.43,
            d: 0.59,
            e: 0.14,
            f: 0.14,
            white: 1.0,
        }
    }
    
    /// Higher contrast ACES curve.
    pub fn aces_high_contrast() -> Self {
        Self {
            a: 2.80,
            b: 0.04,
            c: 2.90,
            d: 0.55,
            e: 0.10,
            f: 0.10,
            white: 1.0,
        }
    }
}

/// Apply ACES RRT filmic tonemap to a single channel.
/// 
/// Uses the simplified ACES curve: `(x*(a*x+b))/(x*(c*x+d)+e)`
#[inline]
pub fn rrt_tonemap(x: f32, params: &RrtParams) -> f32 {
    let x = x.max(0.0);
    let num = x * (params.a * x + params.b);
    let den = x * (params.c * x + params.d) + params.e;
    (num / den).clamp(0.0, 1.0)
}

/// Apply ACES RRT to RGB values.
pub fn rrt(r: f32, g: f32, b: f32, params: &RrtParams) -> (f32, f32, f32) {
    (
        rrt_tonemap(r, params),
        rrt_tonemap(g, params),
        rrt_tonemap(b, params),
    )
}

/// Apply ACES RRT with default parameters.
pub fn rrt_default(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    rrt(r, g, b, &RrtParams::default())
}

// ============================================================================
// Combined RRT+ODT
// ============================================================================

/// Apply combined ACES RRT+ODT: ACEScg -> sRGB display.
/// 
/// This performs:
/// 1. ACES RRT tonemap (filmic curve)
/// 2. ACEScg to sRGB color space conversion (with chromatic adaptation)
/// 3. sRGB gamma encoding
pub fn rrt_odt_srgb(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let params = RrtParams::default();
    
    // Apply RRT tonemap
    let (r, g, b) = rrt(r, g, b, &params);
    
    // Convert AP1 -> sRGB
    let matrix = acescg_to_srgb_matrix();
    let rgb = matrix * Vec3::new(r, g, b);
    
    // Apply sRGB gamma
    let r = vfx_transfer::srgb::oetf(rgb.x.clamp(0.0, 1.0));
    let g = vfx_transfer::srgb::oetf(rgb.y.clamp(0.0, 1.0));
    let b = vfx_transfer::srgb::oetf(rgb.z.clamp(0.0, 1.0));
    
    (r, g, b)
}

/// Apply combined ACES RRT+ODT: ACEScg -> Rec.709 display (video).
pub fn rrt_odt_rec709(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let params = RrtParams::default();
    
    // Apply RRT tonemap
    let (r, g, b) = rrt(r, g, b, &params);
    
    // Convert AP1 -> sRGB (same primaries as Rec.709)
    let matrix = acescg_to_srgb_matrix();
    let rgb = matrix * Vec3::new(r, g, b);
    
    // Apply Rec.709 gamma (BT.1886)
    let r = vfx_transfer::rec709::oetf(rgb.x.clamp(0.0, 1.0));
    let g = vfx_transfer::rec709::oetf(rgb.y.clamp(0.0, 1.0));
    let b = vfx_transfer::rec709::oetf(rgb.z.clamp(0.0, 1.0));
    
    (r, g, b)
}

// ============================================================================
// IDT (Input Device Transform) helpers
// ============================================================================

/// Convert linear sRGB to ACEScg.
pub fn srgb_to_acescg(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let matrix = srgb_to_acescg_matrix();
    let rgb = matrix * Vec3::new(r, g, b);
    (rgb.x, rgb.y, rgb.z)
}

/// Convert ACEScg to linear sRGB.
pub fn acescg_to_srgb(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let matrix = acescg_to_srgb_matrix();
    let rgb = matrix * Vec3::new(r, g, b);
    (rgb.x, rgb.y, rgb.z)
}

// ============================================================================
// Image Processing
// ============================================================================

/// Apply ACES RRT+ODT to an image buffer.
/// 
/// Input: ACEScg linear (3 or 4 channels)
/// Output: sRGB gamma-encoded
pub fn apply_rrt_odt_srgb(data: &[f32], channels: usize) -> Vec<f32> {
    let mut result = data.to_vec();
    let pixels = data.len() / channels;
    
    for i in 0..pixels {
        let idx = i * channels;
        let (r, g, b) = rrt_odt_srgb(data[idx], data[idx + 1], data[idx + 2]);
        result[idx] = r;
        result[idx + 1] = g;
        result[idx + 2] = b;
        // Alpha (if present) passes through unchanged
    }
    
    result
}

/// Apply inverse ODT (sRGB to ACEScg linear) to an image buffer.
pub fn apply_inverse_odt_srgb(data: &[f32], channels: usize) -> Vec<f32> {
    let mut result = data.to_vec();
    let pixels = data.len() / channels;
    
    for i in 0..pixels {
        let idx = i * channels;
        // Decode sRGB gamma
        let r = vfx_transfer::srgb::eotf(data[idx]);
        let g = vfx_transfer::srgb::eotf(data[idx + 1]);
        let b = vfx_transfer::srgb::eotf(data[idx + 2]);
        
        // sRGB linear -> ACEScg
        let (r, g, b) = srgb_to_acescg(r, g, b);
        
        result[idx] = r;
        result[idx + 1] = g;
        result[idx + 2] = b;
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrt_black_is_black() {
        let (r, g, b) = rrt_default(0.0, 0.0, 0.0);
        assert!(r.abs() < 0.01);
        assert!(g.abs() < 0.01);
        assert!(b.abs() < 0.01);
    }

    #[test]
    fn test_rrt_white_approaches_one() {
        let params = RrtParams::default();
        // High values should approach 1.0
        let y = rrt_tonemap(10.0, &params);
        assert!(y > 0.95);
        assert!(y <= 1.0);
    }

    #[test]
    fn test_rrt_monotonic() {
        let params = RrtParams::default();
        let mut prev = 0.0;
        for i in 0..100 {
            let x = i as f32 / 10.0;
            let y = rrt_tonemap(x, &params);
            assert!(y >= prev, "RRT should be monotonic");
            prev = y;
        }
    }

    #[test]
    fn test_acescg_srgb_roundtrip() {
        let (r, g, b) = (0.5, 0.3, 0.2);
        let (ar, ag, ab) = srgb_to_acescg(r, g, b);
        let (rr, rg, rb) = acescg_to_srgb(ar, ag, ab);
        
        assert!((r - rr).abs() < 0.001);
        assert!((g - rg).abs() < 0.001);
        assert!((b - rb).abs() < 0.001);
    }

    #[test]
    fn test_rrt_odt_output_range() {
        // Typical scene-referred values
        for val in [0.0, 0.18, 0.5, 1.0, 2.0, 5.0] {
            let (r, g, b) = rrt_odt_srgb(val, val, val);
            assert!(r >= 0.0 && r <= 1.0, "r={} out of range for input {}", r, val);
            assert!(g >= 0.0 && g <= 1.0, "g={} out of range for input {}", g, val);
            assert!(b >= 0.0 && b <= 1.0, "b={} out of range for input {}", b, val);
        }
    }
}
