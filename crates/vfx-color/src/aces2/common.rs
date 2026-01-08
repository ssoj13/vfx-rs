//! Common constants and types for ACES2 Output Transform.

use std::f32::consts::PI;

// ============================================================================
// Basic Types
// ============================================================================

/// 2D float array
pub type F2 = [f32; 2];

/// 3D float array (RGB, JMh, XYZ, etc.)
pub type F3 = [f32; 3];

/// 3x3 matrix in row-major order
pub type M33 = [f32; 9];

// ============================================================================
// CAM Constants
// ============================================================================

/// Reference luminance (cd/m²)
pub const REFERENCE_LUMINANCE: f32 = 100.0;

/// Adapting luminance
pub const L_A: f32 = 100.0;

/// Background relative luminance
pub const Y_B: f32 = 20.0;

/// Dim surround parameters [c, Nc, F]
pub const SURROUND: F3 = [0.9, 0.59, 0.9];

/// J scale factor
pub const J_SCALE: f32 = 100.0;

/// CAM nonlinearity reference
pub const CAM_NL_Y_REF: f32 = 100.0;

/// CAM nonlinearity offset
pub const CAM_NL_OFFSET: f32 = 0.2713 * CAM_NL_Y_REF;

/// CAM nonlinearity scale
pub const CAM_NL_SCALE: f32 = 4.0 * CAM_NL_Y_REF;

// ============================================================================
// Hue Constants
// ============================================================================

/// Hue limit (360 degrees)
pub const HUE_LIMIT: f32 = 360.0;

/// Number of cusp corners (R, Y, G, C, B, M)
pub const CUSP_CORNER_COUNT: usize = 6;

/// Total corners including wrap
pub const TOTAL_CORNER_COUNT: usize = CUSP_CORNER_COUNT + 2;

/// Max sorted corners for gamut tables
pub const MAX_SORTED_CORNERS: usize = 2 * CUSP_CORNER_COUNT;

// ============================================================================
// Chroma Compression Constants
// ============================================================================

/// Chroma compression strength
pub const CHROMA_COMPRESS: f32 = 2.4;

/// Chroma compression factor
pub const CHROMA_COMPRESS_FACT: f32 = 3.3;

/// Chroma expansion strength
pub const CHROMA_EXPAND: f32 = 1.3;

/// Chroma expansion factor
pub const CHROMA_EXPAND_FACT: f32 = 0.69;

/// Chroma expansion threshold
pub const CHROMA_EXPAND_THR: f32 = 0.5;

// ============================================================================
// Gamut Compression Constants
// ============================================================================

/// Cusp smoothing factor
pub const SMOOTH_CUSPS: f32 = 0.12;

/// M smoothing factor
pub const SMOOTH_M: f32 = 0.27;

/// Cusp mid blend factor
pub const CUSP_MID_BLEND: f32 = 1.3;

/// Focus gain blend
pub const FOCUS_GAIN_BLEND: f32 = 0.3;

/// Focus adjust gain inverse
pub const FOCUS_ADJUST_GAIN_INV: f32 = 1.0 / 0.55;

/// Focus distance
pub const FOCUS_DISTANCE: f32 = 1.35;

/// Focus distance scaling
pub const FOCUS_DISTANCE_SCALING: f32 = 1.75;

/// Compression threshold
pub const COMPRESSION_THRESHOLD: f32 = 0.75;

// ============================================================================
// Table Generation Constants
// ============================================================================

/// Gamma search minimum
pub const GAMMA_MINIMUM: f32 = 0.0;

/// Gamma search maximum
pub const GAMMA_MAXIMUM: f32 = 5.0;

/// Gamma search step
pub const GAMMA_SEARCH_STEP: f32 = 0.4;

/// Gamma accuracy
pub const GAMMA_ACCURACY: f32 = 1e-5;

/// Reach cusp search tolerance
pub const REACH_CUSP_TOLERANCE: f32 = 1e-3;

/// Display cusp search tolerance
pub const DISPLAY_CUSP_TOLERANCE: f32 = 1e-7;

// ============================================================================
// CAM16 Primaries (for internal color model)
// ============================================================================

/// CAM16 red chromaticity x
pub const CAM16_RED_X: f32 = 0.8336;
/// CAM16 red chromaticity y
pub const CAM16_RED_Y: f32 = 0.1735;

/// CAM16 green chromaticity x
pub const CAM16_GREEN_X: f32 = 2.3854;
/// CAM16 green chromaticity y
pub const CAM16_GREEN_Y: f32 = -1.4659;

/// CAM16 blue chromaticity x
pub const CAM16_BLUE_X: f32 = 0.087;
/// CAM16 blue chromaticity y
pub const CAM16_BLUE_Y: f32 = -0.125;

/// CAM16 white point x (E illuminant)
pub const CAM16_WHITE_X: f32 = 1.0 / 3.0;
/// CAM16 white point y (E illuminant)
pub const CAM16_WHITE_Y: f32 = 1.0 / 3.0;

// ============================================================================
// Utility Functions
// ============================================================================

/// Wrap hue to [0, HUE_LIMIT) range
#[inline]
pub fn wrap_hue(hue: f32) -> f32 {
    let mut h = hue % HUE_LIMIT;
    if h < 0.0 {
        h += HUE_LIMIT;
    }
    h
}

/// Convert degrees to radians
#[inline]
pub fn to_radians(deg: f32) -> f32 {
    PI * deg / 180.0
}

/// Convert radians to degrees (wrapped)
#[inline]
pub fn from_radians(rad: f32) -> f32 {
    wrap_hue(180.0 * rad / PI)
}

/// Linear interpolation
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

/// Linear interpolation for F3
#[inline]
pub fn lerp_f3(a: &F3, b: &F3, t: f32) -> F3 {
    [
        lerp(a[0], b[0], t),
        lerp(a[1], b[1], t),
        lerp(a[2], b[2], t),
    ]
}

/// Midpoint of two values
#[inline]
pub fn midpoint(a: f32, b: f32) -> f32 {
    (a + b) * 0.5
}

/// Create F3 from single value
#[inline]
pub fn f3_from_f(v: f32) -> F3 {
    [v, v, v]
}

/// Add scalar to F3
#[inline]
pub fn add_f_f3(v: f32, f: &F3) -> F3 {
    [v + f[0], v + f[1], v + f[2]]
}

/// Multiply scalar with F3
#[inline]
pub fn mult_f_f3(v: f32, f: &F3) -> F3 {
    [v * f[0], v * f[1], v * f[2]]
}

/// Multiply F3 by M33 (row-major)
#[inline]
pub fn mult_f3_m33(f: &F3, m: &M33) -> F3 {
    [
        f[0] * m[0] + f[1] * m[1] + f[2] * m[2],
        f[0] * m[3] + f[1] * m[4] + f[2] * m[5],
        f[0] * m[6] + f[1] * m[7] + f[2] * m[8],
    ]
}

/// Multiply two M33 matrices
#[inline]
pub fn mult_m33_m33(a: &M33, b: &M33) -> M33 {
    [
        a[0] * b[0] + a[1] * b[3] + a[2] * b[6],
        a[0] * b[1] + a[1] * b[4] + a[2] * b[7],
        a[0] * b[2] + a[1] * b[5] + a[2] * b[8],
        a[3] * b[0] + a[4] * b[3] + a[5] * b[6],
        a[3] * b[1] + a[4] * b[4] + a[5] * b[7],
        a[3] * b[2] + a[4] * b[5] + a[5] * b[8],
        a[6] * b[0] + a[7] * b[3] + a[8] * b[6],
        a[6] * b[1] + a[7] * b[4] + a[8] * b[7],
        a[6] * b[2] + a[7] * b[5] + a[8] * b[8],
    ]
}

/// Scale M33 diagonal
#[inline]
pub fn scale_m33(m: &M33, s: &F3) -> M33 {
    [
        m[0] * s[0], m[1], m[2],
        m[3], m[4] * s[1], m[5],
        m[6], m[7], m[8] * s[2],
    ]
}

/// Identity matrix
pub const IDENTITY_M33: M33 = [
    1.0, 0.0, 0.0,
    0.0, 1.0, 0.0,
    0.0, 0.0, 1.0,
];

/// Invert 3x3 matrix
pub fn invert_m33(m: &M33) -> M33 {
    let det = m[0] * (m[4] * m[8] - m[5] * m[7])
            - m[1] * (m[3] * m[8] - m[5] * m[6])
            + m[2] * (m[3] * m[7] - m[4] * m[6]);
    
    if det.abs() < 1e-10 {
        return IDENTITY_M33;
    }
    
    let inv_det = 1.0 / det;
    
    [
        (m[4] * m[8] - m[5] * m[7]) * inv_det,
        (m[2] * m[7] - m[1] * m[8]) * inv_det,
        (m[1] * m[5] - m[2] * m[4]) * inv_det,
        (m[5] * m[6] - m[3] * m[8]) * inv_det,
        (m[0] * m[8] - m[2] * m[6]) * inv_det,
        (m[2] * m[3] - m[0] * m[5]) * inv_det,
        (m[3] * m[7] - m[4] * m[6]) * inv_det,
        (m[1] * m[6] - m[0] * m[7]) * inv_det,
        (m[0] * m[4] - m[1] * m[3]) * inv_det,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_hue() {
        assert!((wrap_hue(0.0) - 0.0).abs() < 1e-6);
        assert!((wrap_hue(180.0) - 180.0).abs() < 1e-6);
        assert!((wrap_hue(360.0) - 0.0).abs() < 1e-6);
        assert!((wrap_hue(370.0) - 10.0).abs() < 1e-6);
        assert!((wrap_hue(-10.0) - 350.0).abs() < 1e-6);
    }

    #[test]
    fn test_lerp() {
        assert!((lerp(0.0, 1.0, 0.5) - 0.5).abs() < 1e-6);
        assert!((lerp(0.0, 1.0, 0.0) - 0.0).abs() < 1e-6);
        assert!((lerp(0.0, 1.0, 1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_matrix_identity_invert() {
        let inv = invert_m33(&IDENTITY_M33);
        for i in 0..9 {
            assert!((inv[i] - IDENTITY_M33[i]).abs() < 1e-6);
        }
    }

    #[test]
    fn test_mult_f3_m33() {
        let f = [1.0, 2.0, 3.0];
        let result = mult_f3_m33(&f, &IDENTITY_M33);
        assert!((result[0] - 1.0).abs() < 1e-6);
        assert!((result[1] - 2.0).abs() < 1e-6);
        assert!((result[2] - 3.0).abs() < 1e-6);
    }
}
