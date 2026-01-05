//! # vfx-primaries
//!
//! Color primaries, white points, and RGB-XYZ matrix generation.
//!
//! This crate provides the mathematical foundation for color space conversions
//! by defining the chromaticity coordinates of RGB primaries and generating
//! the matrices to convert between RGB and CIE XYZ.
//!
//! # Integration with vfx-core
//!
//! This crate bridges [`vfx_core::ColorSpaceId`] to runtime math:
//!
//! ```rust
//! use vfx_core::ColorSpaceId;
//! use vfx_primaries::{Primaries, conversion_matrix};
//!
//! // From compile-time type via ID
//! let p = Primaries::from_id(ColorSpaceId::AcesCg);
//!
//! // Direct conversion matrix
//! let m = conversion_matrix(ColorSpaceId::Srgb, ColorSpaceId::AcesCg);
//! ```
//!
//! # What are Color Primaries?
//!
//! Color primaries define the gamut (range of colors) a color space can represent.
//! Each primary is specified as CIE xy chromaticity coordinates.
//!
//! # Included Color Spaces
//!
//! | Color Space | Gamut Size | Primary Use |
//! |-------------|------------|-------------|
//! | sRGB / Rec.709 | Small | Web, HDTV |
//! | DCI-P3 | Medium | Cinema, Apple displays |
//! | Rec.2020 | Large | UHDTV, HDR |
//! | ACES AP0 | Very Large | Archival, interchange |
//! | ACES AP1 | Large | Working space (ACEScg) |
//!
//! # Usage
//!
//! ```rust
//! use vfx_primaries::{SRGB, rgb_to_xyz_matrix};
//! use vfx_math::Vec3;
//!
//! // Get the RGB to XYZ matrix for sRGB
//! let matrix = rgb_to_xyz_matrix(&SRGB);
//!
//! // Convert sRGB to XYZ
//! let rgb = Vec3::new(1.0, 0.0, 0.0);
//! let xyz = matrix * rgb;
//! ```
//!
//! # Dependencies
//!
//! - [`vfx-core`] - Core types
//! - [`vfx-math`] - Matrix operations
//! - [`glam`] - SIMD math
//!
//! # Used By
//!
//! - `vfx-color` - Full color space conversions

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

use vfx_core::ColorSpaceId;
use vfx_math::{Mat3, Vec3};

/// RGB color space primaries definition.
///
/// Defines a color space by its three primary colors (R, G, B) and white point,
/// all specified as CIE xy chromaticity coordinates.
///
/// # Example
///
/// ```rust
/// use vfx_primaries::Primaries;
///
/// let my_space = Primaries {
///     r: (0.64, 0.33),
///     g: (0.30, 0.60),
///     b: (0.15, 0.06),
///     w: (0.3127, 0.3290),
///     name: "Custom",
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Primaries {
    /// Red primary (x, y) chromaticity
    pub r: (f32, f32),
    /// Green primary (x, y) chromaticity
    pub g: (f32, f32),
    /// Blue primary (x, y) chromaticity
    pub b: (f32, f32),
    /// White point (x, y) chromaticity
    pub w: (f32, f32),
    /// Color space name
    pub name: &'static str,
}

impl Primaries {
    /// White point as XYZ (Y=1).
    #[inline]
    pub fn white_xyz(&self) -> Vec3 {
        xy_to_xyz(self.w.0, self.w.1)
    }

    /// Create Primaries from a ColorSpaceId.
    ///
    /// Bridges compile-time color space markers to runtime primaries.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_primaries::Primaries;
    /// use vfx_core::ColorSpaceId;
    ///
    /// let p = Primaries::from_id(ColorSpaceId::AcesCg);
    /// assert_eq!(p.name, "ACEScg");
    /// ```
    pub const fn from_id(id: ColorSpaceId) -> Self {
        let prims = id.primaries();
        let w = id.white_point();
        Self {
            r: prims[0],
            g: prims[1],
            b: prims[2],
            w,
            name: id.name(),
        }
    }
}

impl From<ColorSpaceId> for Primaries {
    fn from(id: ColorSpaceId) -> Self {
        Self::from_id(id)
    }
}

// ============================================================================
// Standard White Points
// ============================================================================

/// D65 white point chromaticity (daylight, ~6500K).
pub const D65_XY: (f32, f32) = (0.31270, 0.32900);

/// D50 white point chromaticity (~5000K).
pub const D50_XY: (f32, f32) = (0.34567, 0.35850);

/// D60 white point chromaticity (~6000K, used by ACES).
pub const D60_XY: (f32, f32) = (0.32168, 0.33767);

/// DCI white point chromaticity (theatrical projection).
pub const DCI_XY: (f32, f32) = (0.31400, 0.35100);

// ============================================================================
// Standard Color Space Primaries
// ============================================================================

/// sRGB / Rec.709 primaries (D65 white point).
///
/// The most common color space for web and consumer displays.
pub const SRGB: Primaries = Primaries {
    r: (0.6400, 0.3300),
    g: (0.3000, 0.6000),
    b: (0.1500, 0.0600),
    w: D65_XY,
    name: "sRGB",
};

/// Rec.709 primaries (identical to sRGB).
pub const REC709: Primaries = SRGB;

/// Rec.2020 primaries (D65 white point).
///
/// Ultra HD TV color space with a much wider gamut than Rec.709.
pub const REC2020: Primaries = Primaries {
    r: (0.7080, 0.2920),
    g: (0.1700, 0.7970),
    b: (0.1310, 0.0460),
    w: D65_XY,
    name: "Rec.2020",
};

/// DCI-P3 primaries (DCI white point).
///
/// Digital Cinema Initiative color space.
pub const DCI_P3: Primaries = Primaries {
    r: (0.6800, 0.3200),
    g: (0.2650, 0.6900),
    b: (0.1500, 0.0600),
    w: DCI_XY,
    name: "DCI-P3",
};

/// Display P3 primaries (D65 white point).
///
/// Apple's wide gamut display standard, based on DCI-P3 primaries
/// but with a D65 white point.
pub const DISPLAY_P3: Primaries = Primaries {
    r: (0.6800, 0.3200),
    g: (0.2650, 0.6900),
    b: (0.1500, 0.0600),
    w: D65_XY,
    name: "Display P3",
};

/// ACES AP0 primaries (D60 white point).
///
/// Academy Color Encoding System primaries for ACES 2065-1.
/// Encompasses the entire human visual gamut and more.
pub const ACES_AP0: Primaries = Primaries {
    r: (0.7347, 0.2653),
    g: (0.0000, 1.0000),
    b: (0.0001, -0.0770),
    w: D60_XY,
    name: "ACES AP0",
};

/// ACES AP1 primaries (D60 white point).
///
/// Working color space for ACEScg, ACEScct, ACEScc.
/// More practical gamut than AP0 while still being very wide.
pub const ACES_AP1: Primaries = Primaries {
    r: (0.7130, 0.2930),
    g: (0.1650, 0.8300),
    b: (0.1280, 0.0440),
    w: D60_XY,
    name: "ACES AP1",
};

/// Adobe RGB (1998) primaries (D65 white point).
pub const ADOBE_RGB: Primaries = Primaries {
    r: (0.6400, 0.3300),
    g: (0.2100, 0.7100),
    b: (0.1500, 0.0600),
    w: D65_XY,
    name: "Adobe RGB",
};

/// ProPhoto RGB primaries (D50 white point).
pub const PROPHOTO_RGB: Primaries = Primaries {
    r: (0.7347, 0.2653),
    g: (0.1596, 0.8404),
    b: (0.0366, 0.0001),
    w: D50_XY,
    name: "ProPhoto RGB",
};

/// ARRI Wide Gamut 3 primaries.
pub const ARRI_WIDE_GAMUT_3: Primaries = Primaries {
    r: (0.6840, 0.3130),
    g: (0.2210, 0.8480),
    b: (0.0861, -0.1020),
    w: D65_XY,
    name: "ARRI Wide Gamut 3",
};

/// Sony S-Gamut3 primaries.
pub const S_GAMUT3: Primaries = Primaries {
    r: (0.7300, 0.2800),
    g: (0.1400, 0.8550),
    b: (0.1000, -0.0500),
    w: D65_XY,
    name: "S-Gamut3",
};

/// Panasonic V-Gamut primaries.
pub const V_GAMUT: Primaries = Primaries {
    r: (0.7300, 0.2800),
    g: (0.1650, 0.8400),
    b: (0.1000, -0.0300),
    w: D65_XY,
    name: "V-Gamut",
};

// ============================================================================
// Matrix Generation
// ============================================================================

/// Converts xy chromaticity to XYZ (with Y=1).
fn xy_to_xyz(x: f32, y: f32) -> Vec3 {
    if y.abs() < 1e-10 {
        Vec3::ZERO
    } else {
        Vec3::new(x / y, 1.0, (1.0 - x - y) / y)
    }
}

/// Computes the RGB to XYZ matrix for a set of primaries.
///
/// This function implements the standard method for deriving the 3x3 matrix
/// that converts RGB values to CIE XYZ, given the chromaticity coordinates
/// of the primaries and white point.
///
/// # Algorithm
///
/// 1. Convert xy chromaticities to XYZ (with Y=1)
/// 2. Compute scaling factors so white point maps correctly
/// 3. Multiply primaries by scaling factors
///
/// # Example
///
/// ```rust
/// use vfx_primaries::{SRGB, rgb_to_xyz_matrix};
/// use vfx_math::Vec3;
///
/// let m = rgb_to_xyz_matrix(&SRGB);
///
/// // White (1,1,1) should map to the white point XYZ
/// let white = m * Vec3::ONE;
/// // Y should be 1.0 (normalized)
/// assert!((white.y - 1.0).abs() < 0.001);
/// ```
pub fn rgb_to_xyz_matrix(primaries: &Primaries) -> Mat3 {
    // Convert primaries from xy to XYZ
    let r_xyz = xy_to_xyz(primaries.r.0, primaries.r.1);
    let g_xyz = xy_to_xyz(primaries.g.0, primaries.g.1);
    let b_xyz = xy_to_xyz(primaries.b.0, primaries.b.1);
    let w_xyz = xy_to_xyz(primaries.w.0, primaries.w.1);

    // Build matrix from primaries as columns
    let m = Mat3::from_col_vecs(r_xyz, g_xyz, b_xyz);

    // Solve for scaling factors: M * S = W
    // S = M^-1 * W
    let m_inv = m.inverse().unwrap_or(Mat3::IDENTITY);
    let s = m_inv * w_xyz;

    // Scale each column by the corresponding factor
    Mat3::from_col_vecs(r_xyz * s.x, g_xyz * s.y, b_xyz * s.z)
}

/// Computes the XYZ to RGB matrix for a set of primaries.
///
/// This is the inverse of [`rgb_to_xyz_matrix`].
///
/// # Example
///
/// ```rust
/// use vfx_primaries::{SRGB, xyz_to_rgb_matrix, rgb_to_xyz_matrix};
/// use vfx_math::Vec3;
///
/// let to_xyz = rgb_to_xyz_matrix(&SRGB);
/// let to_rgb = xyz_to_rgb_matrix(&SRGB);
///
/// // Roundtrip should give identity
/// let result = to_rgb * to_xyz;
/// // Check diagonal is 1, off-diagonal is 0
/// ```
pub fn xyz_to_rgb_matrix(primaries: &Primaries) -> Mat3 {
    rgb_to_xyz_matrix(primaries).inverse().unwrap_or(Mat3::IDENTITY)
}

/// Computes a matrix to convert from one RGB color space to another.
///
/// The conversion goes through XYZ: `RGB_src -> XYZ -> RGB_dst`
///
/// # Note
///
/// This does NOT include chromatic adaptation. If the source and destination
/// have different white points, you should apply chromatic adaptation
/// (see `vfx_math::adapt_matrix`).
///
/// # Example
///
/// ```rust
/// use vfx_primaries::{SRGB, REC2020, rgb_to_rgb_matrix};
///
/// let srgb_to_rec2020 = rgb_to_rgb_matrix(&SRGB, &REC2020);
/// ```
pub fn rgb_to_rgb_matrix(src: &Primaries, dst: &Primaries) -> Mat3 {
    let src_to_xyz = rgb_to_xyz_matrix(src);
    let xyz_to_dst = xyz_to_rgb_matrix(dst);
    xyz_to_dst * src_to_xyz
}

/// Computes conversion matrix between two color spaces by ID.
///
/// Convenience function that bridges ColorSpaceId to matrix generation.
///
/// # Example
///
/// ```rust
/// use vfx_primaries::conversion_matrix;
/// use vfx_core::ColorSpaceId;
///
/// let m = conversion_matrix(ColorSpaceId::Srgb, ColorSpaceId::AcesCg);
/// ```
pub fn conversion_matrix(from: ColorSpaceId, to: ColorSpaceId) -> Mat3 {
    let src = Primaries::from_id(from);
    let dst = Primaries::from_id(to);
    rgb_to_rgb_matrix(&src, &dst)
}

// ============================================================================
// Pre-computed Common Matrices
// ============================================================================

/// sRGB to XYZ (D65) matrix.
pub const SRGB_TO_XYZ: Mat3 = Mat3::from_rows([
    [0.4124564, 0.3575761, 0.1804375],
    [0.2126729, 0.7151522, 0.0721750],
    [0.0193339, 0.1191920, 0.9503041],
]);

/// XYZ (D65) to sRGB matrix.
pub const XYZ_TO_SRGB: Mat3 = Mat3::from_rows([
    [3.2404542, -1.5371385, -0.4985314],
    [-0.9692660, 1.8760108, 0.0415560],
    [0.0556434, -0.2040259, 1.0572252],
]);

/// ACES AP0 to XYZ (D60) matrix.
pub const ACES_AP0_TO_XYZ: Mat3 = Mat3::from_rows([
    [0.9525523959, 0.0000000000, 0.0000936786],
    [0.3439664498, 0.7281660966, -0.0721325464],
    [0.0000000000, 0.0000000000, 1.0088251844],
]);

/// XYZ (D60) to ACES AP0 matrix.
pub const XYZ_TO_ACES_AP0: Mat3 = Mat3::from_rows([
    [1.0498110175, 0.0000000000, -0.0000974845],
    [-0.4959030231, 1.3733130458, 0.0982400361],
    [0.0000000000, 0.0000000000, 0.9912520182],
]);

/// ACES AP1 to XYZ (D60) matrix.
pub const ACES_AP1_TO_XYZ: Mat3 = Mat3::from_rows([
    [0.6624541811, 0.1340042065, 0.1561876870],
    [0.2722287168, 0.6740817658, 0.0536895174],
    [-0.0055746495, 0.0040607335, 1.0103391003],
]);

/// XYZ (D60) to ACES AP1 matrix.
pub const XYZ_TO_ACES_AP1: Mat3 = Mat3::from_rows([
    [1.6410233797, -0.3248032942, -0.2364246952],
    [-0.6636628587, 1.6153315917, 0.0167563477],
    [0.0117218943, -0.0082844420, 0.9883948585],
]);

#[cfg(test)]
mod tests {
    use super::*;
    use vfx_core::ColorSpaceId;

    #[test]
    fn test_primaries_from_id() {
        let p = Primaries::from_id(ColorSpaceId::AcesCg);
        assert_eq!(p.name, "ACEScg");
        assert!((p.r.0 - 0.713).abs() < 0.001);
        
        // Test From trait
        let p2: Primaries = ColorSpaceId::Srgb.into();
        assert_eq!(p2.name, "sRGB");
    }

    #[test]
    fn test_conversion_matrix_by_id() {
        let m = conversion_matrix(ColorSpaceId::Srgb, ColorSpaceId::AcesCg);
        // Check matrix is not identity (spaces differ)
        assert!((m.m[0][0] - 1.0).abs() > 0.01);
    }

    #[test]
    fn test_srgb_matrix() {
        let m = rgb_to_xyz_matrix(&SRGB);
        
        // Check against known values
        assert!((m.m[0][0] - 0.4124564).abs() < 0.001);
        assert!((m.m[1][0] - 0.2126729).abs() < 0.001);
    }

    #[test]
    fn test_white_point() {
        let m = rgb_to_xyz_matrix(&SRGB);
        let white = m * Vec3::ONE;
        
        // Y should be 1.0
        assert!((white.y - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_roundtrip() {
        let to_xyz = rgb_to_xyz_matrix(&SRGB);
        let to_rgb = xyz_to_rgb_matrix(&SRGB);
        
        let rgb = Vec3::new(0.5, 0.3, 0.8);
        let xyz = to_xyz * rgb;
        let back = to_rgb * xyz;
        
        assert!((rgb.x - back.x).abs() < 0.001);
        assert!((rgb.y - back.y).abs() < 0.001);
        assert!((rgb.z - back.z).abs() < 0.001);
    }

    #[test]
    fn test_rgb_to_rgb() {
        let m = rgb_to_rgb_matrix(&SRGB, &SRGB);
        
        // Should be identity
        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((m.m[i][j] - expected).abs() < 0.001);
            }
        }
    }

    #[test]
    fn test_primaries_have_correct_white() {
        // All primaries should have valid white points
        let spaces = [SRGB, REC2020, DCI_P3, DISPLAY_P3, ACES_AP0, ACES_AP1];
        
        for space in spaces {
            let m = rgb_to_xyz_matrix(&space);
            let white = m * Vec3::ONE;
            assert!(white.y > 0.9 && white.y < 1.1, "{} white Y = {}", space.name, white.y);
        }
    }
}
