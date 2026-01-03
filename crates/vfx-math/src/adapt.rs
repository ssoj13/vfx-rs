//! Chromatic Adaptation Transforms (CAT).
//!
//! This module provides matrices and functions for adapting colors
//! between different illuminants (white points).
//!
//! # What is Chromatic Adaptation?
//!
//! When viewing a white object under tungsten light (warm) vs daylight (cool),
//! our visual system adapts so that both appear "white". Chromatic adaptation
//! transforms simulate this effect mathematically.
//!
//! # Common Use Cases
//!
//! - Converting between D65 and D50 white points
//! - Adapting camera white balance
//! - Converting between color spaces with different illuminants
//!
//! # Supported Methods
//!
//! - [`BRADFORD`] - Best overall accuracy (default choice)
//! - [`VON_KRIES`] - Classic cone response model
//! - [`CAT02`] - From CIECAM02 color appearance model
//! - [`XYZ_SCALING`] - Simple but less accurate
//!
//! # Usage
//!
//! ```rust
//! use vfx_math::{adapt_matrix, BRADFORD, D65, D50, Mat3, Vec3};
//!
//! // Create adaptation matrix from D65 to D50
//! let d65_to_d50 = adapt_matrix(BRADFORD, D65, D50);
//!
//! // Adapt a color
//! let xyz_d65 = Vec3::new(0.95047, 1.0, 1.08883);
//! let xyz_d50 = d65_to_d50 * xyz_d65;
//! ```

use crate::{Mat3, Vec3};

// ============================================================================
// Standard Illuminants (XYZ white points)
// ============================================================================

/// CIE Standard Illuminant D65 (daylight, ~6500K).
///
/// The most common reference illuminant for:
/// - sRGB
/// - Rec.709 / Rec.2020
/// - AdobeRGB (1998)
pub const D65: Vec3 = Vec3::new(0.95047, 1.0, 1.08883);

/// CIE Standard Illuminant D50 (horizon light, ~5000K).
///
/// Standard reference for:
/// - ICC color profiles
/// - Printing industry
pub const D50: Vec3 = Vec3::new(0.96422, 1.0, 0.82521);

/// CIE Standard Illuminant D55 (~5500K).
pub const D55: Vec3 = Vec3::new(0.95682, 1.0, 0.92149);

/// CIE Standard Illuminant D60 (~6000K).
///
/// Used by ACES (Academy Color Encoding System).
pub const D60: Vec3 = Vec3::new(0.95265, 1.0, 1.00883);

/// CIE Standard Illuminant A (tungsten, ~2856K).
pub const A: Vec3 = Vec3::new(1.09850, 1.0, 0.35585);

/// CIE Standard Illuminant E (equal energy).
pub const E: Vec3 = Vec3::new(1.0, 1.0, 1.0);

/// ACES white point (slightly different from D60).
pub const ACES_WHITE: Vec3 = Vec3::new(0.95265, 1.0, 1.00883);

/// DCI-P3 theatrical white point.
pub const DCI_WHITE: Vec3 = Vec3::new(0.89459, 1.0, 0.95441);

// ============================================================================
// Chromatic Adaptation Matrices
// ============================================================================

/// Bradford chromatic adaptation matrix.
///
/// Transforms XYZ to a "sharpened" cone response space.
/// Generally considered the best overall method for most applications.
///
/// # Reference
///
/// Lam, K.M. (1985). Metamerism and Colour Constancy.
pub const BRADFORD: Mat3 = Mat3::from_rows([
    [0.8951, 0.2664, -0.1614],
    [-0.7502, 1.7135, 0.0367],
    [0.0389, -0.0685, 1.0296],
]);

/// Inverse Bradford matrix.
pub const BRADFORD_INV: Mat3 = Mat3::from_rows([
    [0.9869929, -0.1470543, 0.1599627],
    [0.4323053, 0.5183603, 0.0492912],
    [-0.0085287, 0.0400428, 0.9684867],
]);

/// Von Kries chromatic adaptation matrix.
///
/// Classic cone response model using Hunt-Pointer-Estevez transformation.
/// Simpler than Bradford but less accurate for large white point changes.
pub const VON_KRIES: Mat3 = Mat3::from_rows([
    [0.40024, 0.70760, -0.08081],
    [-0.22630, 1.16532, 0.04570],
    [0.00000, 0.00000, 0.91822],
]);

/// Inverse Von Kries matrix.
pub const VON_KRIES_INV: Mat3 = Mat3::from_rows([
    [1.8599364, -1.1293816, 0.2198974],
    [0.3611914, 0.6388125, -0.0000064],
    [0.0000000, 0.0000000, 1.0890636],
]);

/// CAT02 chromatic adaptation matrix.
///
/// From the CIECAM02 color appearance model.
/// Good balance between accuracy and computational efficiency.
pub const CAT02: Mat3 = Mat3::from_rows([
    [0.7328, 0.4296, -0.1624],
    [-0.7036, 1.6975, 0.0061],
    [0.0030, 0.0136, 0.9834],
]);

/// Inverse CAT02 matrix.
pub const CAT02_INV: Mat3 = Mat3::from_rows([
    [1.0961238, -0.2788690, 0.1827452],
    [0.4543690, 0.4735332, 0.0720978],
    [-0.0096276, -0.0056980, 1.0153256],
]);

/// XYZ Scaling (simple diagonal adaptation).
///
/// The simplest method - just scales XYZ components.
/// Fast but inaccurate for large white point differences.
pub const XYZ_SCALING: Mat3 = Mat3::IDENTITY;

/// Inverse XYZ Scaling matrix.
pub const XYZ_SCALING_INV: Mat3 = Mat3::IDENTITY;

// ============================================================================
// Adaptation Functions
// ============================================================================

/// Computes a chromatic adaptation matrix between two white points.
///
/// The resulting matrix transforms XYZ values from the source illuminant
/// to the destination illuminant.
///
/// # Arguments
///
/// * `method` - The CAT matrix to use ([`BRADFORD`], [`VON_KRIES`], etc.)
/// * `src_white` - Source white point in XYZ
/// * `dst_white` - Destination white point in XYZ
///
/// # Example
///
/// ```rust
/// use vfx_math::{adapt_matrix, BRADFORD, D65, D50, Vec3};
///
/// let d65_to_d50 = adapt_matrix(BRADFORD, D65, D50);
///
/// // Verify white point transforms correctly
/// let result = d65_to_d50 * D65;
/// assert!((result.x - D50.x).abs() < 0.001);
/// assert!((result.y - D50.y).abs() < 0.001);
/// assert!((result.z - D50.z).abs() < 0.001);
/// ```
pub fn adapt_matrix(method: Mat3, src_white: Vec3, dst_white: Vec3) -> Mat3 {
    // Get the inverse of the method matrix
    let method_inv = method.inverse().unwrap_or(Mat3::IDENTITY);

    // Transform white points to cone/adapted space
    let src_cone = method * src_white;
    let dst_cone = method * dst_white;

    // Create diagonal scaling matrix
    let scale = Mat3::diagonal(
        dst_cone.x / src_cone.x,
        dst_cone.y / src_cone.y,
        dst_cone.z / src_cone.z,
    );

    // Combine: M^-1 * S * M
    method_inv * scale * method
}

/// Computes the inverse adaptation matrix.
///
/// Equivalent to `adapt_matrix(method, dst_white, src_white)`.
#[inline]
pub fn adapt_matrix_inv(method: Mat3, src_white: Vec3, dst_white: Vec3) -> Mat3 {
    adapt_matrix(method, dst_white, src_white)
}

/// Pre-computed D65 to D50 Bradford adaptation matrix.
///
/// Commonly needed for ICC profile conversions.
pub const D65_TO_D50_BRADFORD: Mat3 = Mat3::from_rows([
    [1.0478112, 0.0228866, -0.0501270],
    [0.0295424, 0.9904844, -0.0170491],
    [-0.0092345, 0.0150436, 0.7521316],
]);

/// Pre-computed D50 to D65 Bradford adaptation matrix.
pub const D50_TO_D65_BRADFORD: Mat3 = Mat3::from_rows([
    [0.9555766, -0.0230393, 0.0631636],
    [-0.0282895, 1.0099416, 0.0210077],
    [0.0122982, -0.0204830, 1.3299098],
]);

/// Pre-computed D65 to D60 Bradford adaptation matrix (for ACES).
pub const D65_TO_D60_BRADFORD: Mat3 = Mat3::from_rows([
    [1.0130349, 0.0061053, -0.0149710],
    [0.0076982, 0.9981648, -0.0050321],
    [-0.0028413, 0.0046261, 0.9245276],
]);

/// Pre-computed D60 to D65 Bradford adaptation matrix.
pub const D60_TO_D65_BRADFORD: Mat3 = Mat3::from_rows([
    [0.9872240, -0.0061132, 0.0159533],
    [-0.0075983, 1.0018614, 0.0053300],
    [0.0030725, -0.0050959, 1.0816860],
]);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_d65_to_d50_white() {
        let result = D65_TO_D50_BRADFORD * D65;
        assert!((result.x - D50.x).abs() < 0.01);
        assert!((result.y - D50.y).abs() < 0.01);
        assert!((result.z - D50.z).abs() < 0.01);
    }

    #[test]
    fn test_d50_to_d65_white() {
        let result = D50_TO_D65_BRADFORD * D50;
        assert!((result.x - D65.x).abs() < 0.01);
        assert!((result.y - D65.y).abs() < 0.01);
        assert!((result.z - D65.z).abs() < 0.01);
    }

    #[test]
    fn test_adapt_matrix_roundtrip() {
        let d65_to_d50 = adapt_matrix(BRADFORD, D65, D50);
        let d50_to_d65 = adapt_matrix(BRADFORD, D50, D65);

        // Should be approximately inverse
        let roundtrip = d50_to_d65 * d65_to_d50;

        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!(
                    (roundtrip.m[i][j] - expected).abs() < 0.001,
                    "roundtrip[{}][{}] = {} (expected {})",
                    i,
                    j,
                    roundtrip.m[i][j],
                    expected
                );
            }
        }
    }

    #[test]
    fn test_adapt_identity() {
        // Adapting to same white point should be identity
        let same = adapt_matrix(BRADFORD, D65, D65);

        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((same.m[i][j] - expected).abs() < 0.001);
            }
        }
    }

    #[test]
    fn test_illuminants() {
        // All illuminants should have Y = 1.0
        assert_eq!(D65.y, 1.0);
        assert_eq!(D50.y, 1.0);
        assert_eq!(D60.y, 1.0);
        assert_eq!(A.y, 1.0);
        assert_eq!(E.y, 1.0);
    }
}
