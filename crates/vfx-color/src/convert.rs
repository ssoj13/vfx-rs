//! High-level color conversion traits.
//!
//! Provides convenient traits for converting RGB values between color spaces.
//!
//! # Traits
//!
//! - [`Convert`] - General conversion trait for any type
//! - [`RgbConvert`] - Specialized for RGB arrays
//!
//! # Example
//!
//! ```rust
//! use vfx_color::convert::RgbConvert;
//! use vfx_color::transfer::srgb;
//! use vfx_color::primaries::SRGB;
//!
//! let srgb_pixel = [0.5_f32, 0.3, 0.2];
//!
//! // Convert to linear
//! let linear = srgb_pixel.linearize(srgb::eotf);
//!
//! // Convert to XYZ
//! let xyz = linear.to_xyz(&SRGB);
//!
//! // Convert to different color space
//! use vfx_color::primaries::REC2020;
//! let rec2020 = xyz.from_xyz(&REC2020);
//! ```

use vfx_primaries::{Primaries, rgb_to_xyz_matrix, xyz_to_rgb_matrix};
use vfx_math::{Mat3, Vec3, adapt_matrix, BRADFORD};

/// General conversion trait.
///
/// Implement this for types that can be converted to/from RGB.
pub trait Convert {
    /// The output type after conversion.
    type Output;

    /// Converts to the target type.
    fn convert(self) -> Self::Output;
}

/// RGB-specific conversion operations.
///
/// Provides chainable methods for common color operations.
///
/// # Example
///
/// ```rust
/// use vfx_color::RgbConvert;
/// use vfx_color::transfer::{srgb, pq};
/// use vfx_color::primaries::{SRGB, REC2020};
///
/// let result = [0.5_f32, 0.3, 0.2]
///     .linearize(srgb::eotf)          // sRGB -> linear
///     .to_xyz(&SRGB)                   // linear RGB -> XYZ
///     .from_xyz(&REC2020)              // XYZ -> Rec.2020 linear
///     .encode(pq::oetf);               // linear -> PQ
/// ```
pub trait RgbConvert: Sized {
    /// Applies a transfer function to linearize (decode) RGB values.
    ///
    /// This is typically an EOTF (electro-optical transfer function).
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_color::RgbConvert;
    /// use vfx_color::transfer::srgb;
    ///
    /// let display = [0.5_f32, 0.3, 0.2];
    /// let linear = display.linearize(srgb::eotf);
    /// ```
    fn linearize(self, f: fn(f32) -> f32) -> Self;

    /// Applies a transfer function to encode RGB values.
    ///
    /// This is typically an OETF (opto-electronic transfer function).
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_color::RgbConvert;
    /// use vfx_color::transfer::pq;
    ///
    /// let linear = [0.18_f32, 0.18, 0.18];
    /// let encoded = linear.encode(pq::oetf);
    /// ```
    fn encode(self, f: fn(f32) -> f32) -> Self;

    /// Converts RGB to XYZ using the given primaries.
    ///
    /// Input must be linear RGB (no transfer function applied).
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_color::RgbConvert;
    /// use vfx_color::primaries::SRGB;
    ///
    /// let rgb = [0.5_f32, 0.3, 0.2];
    /// let xyz = rgb.to_xyz(&SRGB);
    /// ```
    fn to_xyz(self, primaries: &Primaries) -> Self;

    /// Converts XYZ to RGB using the given primaries.
    ///
    /// Output is linear RGB (no transfer function applied).
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_color::RgbConvert;
    /// use vfx_color::primaries::REC2020;
    ///
    /// let xyz = [0.2_f32, 0.2, 0.2];
    /// let rgb = xyz.from_xyz(&REC2020);
    /// ```
    fn from_xyz(self, primaries: &Primaries) -> Self;

    /// Applies a 3x3 matrix transformation.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_color::RgbConvert;
    /// use vfx_math::Mat3;
    ///
    /// let rgb = [0.5_f32, 0.3, 0.2];
    /// let scaled = rgb.transform(&Mat3::scale(2.0));
    /// ```
    fn transform(self, matrix: &Mat3) -> Self;

    /// Applies chromatic adaptation between white points.
    ///
    /// Uses the specified adaptation method (Bradford, CAT02, etc.).
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_color::RgbConvert;
    /// use vfx_math::{BRADFORD, D65, D50};
    ///
    /// let xyz_d65 = [0.95047_f32, 1.0, 1.08883];
    /// let xyz_d50 = xyz_d65.adapt(BRADFORD, D65, D50);
    /// ```
    fn adapt(self, method: Mat3, from_white: Vec3, to_white: Vec3) -> Self;

    /// Scales RGB values by per-channel factors.
    fn scale(self, factors: [f32; 3]) -> Self;

    /// Adds per-channel offsets to RGB values.
    fn offset(self, offsets: [f32; 3]) -> Self;

    /// Clamps RGB values to a range.
    fn clamp(self, min: f32, max: f32) -> Self;

    /// Clamps RGB values to [0, 1].
    fn clamp_01(self) -> Self {
        self.clamp(0.0, 1.0)
    }
}

impl RgbConvert for [f32; 3] {
    fn linearize(self, f: fn(f32) -> f32) -> Self {
        [f(self[0]), f(self[1]), f(self[2])]
    }

    fn encode(self, f: fn(f32) -> f32) -> Self {
        [f(self[0]), f(self[1]), f(self[2])]
    }

    fn to_xyz(self, primaries: &Primaries) -> Self {
        let m = rgb_to_xyz_matrix(primaries);
        let v = Vec3::from_array(self);
        m.transform(v).to_array()
    }

    fn from_xyz(self, primaries: &Primaries) -> Self {
        let m = xyz_to_rgb_matrix(primaries);
        let v = Vec3::from_array(self);
        m.transform(v).to_array()
    }

    fn transform(self, matrix: &Mat3) -> Self {
        let v = Vec3::from_array(self);
        matrix.transform(v).to_array()
    }

    fn adapt(self, method: Mat3, from_white: Vec3, to_white: Vec3) -> Self {
        let m = adapt_matrix(method, from_white, to_white);
        let v = Vec3::from_array(self);
        m.transform(v).to_array()
    }

    fn scale(self, factors: [f32; 3]) -> Self {
        [
            self[0] * factors[0],
            self[1] * factors[1],
            self[2] * factors[2],
        ]
    }

    fn offset(self, offsets: [f32; 3]) -> Self {
        [
            self[0] + offsets[0],
            self[1] + offsets[1],
            self[2] + offsets[2],
        ]
    }

    fn clamp(self, min: f32, max: f32) -> Self {
        [
            self[0].clamp(min, max),
            self[1].clamp(min, max),
            self[2].clamp(min, max),
        ]
    }
}

/// Converts between color spaces with full pipeline.
///
/// This is the high-level API for complete color space conversions
/// including transfer functions, primaries, and chromatic adaptation.
///
/// # Example
///
/// ```rust
/// use vfx_color::convert::convert_rgb;
/// use vfx_color::transfer::{srgb, pq};
/// use vfx_color::primaries::{SRGB, REC2020};
///
/// let srgb_pixel = [0.5, 0.3, 0.2];
///
/// // sRGB -> Rec.2020 PQ
/// let rec2020_pq = convert_rgb(
///     srgb_pixel,
///     Some(srgb::eotf),           // decode sRGB
///     &SRGB,                       // source primaries
///     &REC2020,                    // target primaries
///     Some(pq::oetf),              // encode PQ
/// );
/// ```
pub fn convert_rgb(
    rgb: [f32; 3],
    decode: Option<fn(f32) -> f32>,
    from_primaries: &Primaries,
    to_primaries: &Primaries,
    encode: Option<fn(f32) -> f32>,
) -> [f32; 3] {
    // Decode (linearize)
    let mut result = match decode {
        Some(f) => rgb.linearize(f),
        None => rgb,
    };

    // Convert primaries via XYZ
    if from_primaries != to_primaries {
        result = result.to_xyz(from_primaries);
        
        // Chromatic adaptation if white points differ
        if from_primaries.w != to_primaries.w {
            let src_w = from_primaries.white_xyz();
            let dst_w = to_primaries.white_xyz();
            result = result.adapt(BRADFORD, src_w, dst_w);
        }
        
        result = result.from_xyz(to_primaries);
    }

    // Encode
    match encode {
        Some(f) => result.encode(f),
        None => result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vfx_transfer::srgb;
    use vfx_primaries::{SRGB, REC2020};

    #[test]
    fn test_linearize_encode() {
        let display = [0.5_f32, 0.3, 0.2];
        let linear = display.linearize(srgb::eotf);
        let back = linear.encode(srgb::oetf);

        assert!((back[0] - display[0]).abs() < 0.001);
        assert!((back[1] - display[1]).abs() < 0.001);
        assert!((back[2] - display[2]).abs() < 0.001);
    }

    #[test]
    fn test_to_from_xyz() {
        let rgb = [0.5_f32, 0.3, 0.2];
        let xyz = rgb.to_xyz(&SRGB);
        let back = xyz.from_xyz(&SRGB);

        assert!((back[0] - rgb[0]).abs() < 0.001);
        assert!((back[1] - rgb[1]).abs() < 0.001);
        assert!((back[2] - rgb[2]).abs() < 0.001);
    }

    #[test]
    fn test_scale_offset() {
        let rgb = [0.5_f32, 0.3, 0.2];
        let result = rgb.scale([2.0, 2.0, 2.0]).offset([0.1, 0.1, 0.1]);

        assert!((result[0] - 1.1).abs() < 0.001);
        assert!((result[1] - 0.7).abs() < 0.001);
        assert!((result[2] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_clamp() {
        let rgb = [1.5_f32, -0.2, 0.5];
        let clamped = rgb.clamp_01();

        assert_eq!(clamped[0], 1.0);
        assert_eq!(clamped[1], 0.0);
        assert_eq!(clamped[2], 0.5);
    }

    #[test]
    fn test_convert_rgb_identity() {
        let rgb = [0.5_f32, 0.3, 0.2];
        let result = convert_rgb(
            rgb,
            None,
            &SRGB,
            &SRGB,
            None,
        );

        assert!((result[0] - rgb[0]).abs() < 0.001);
        assert!((result[1] - rgb[1]).abs() < 0.001);
        assert!((result[2] - rgb[2]).abs() < 0.001);
    }

    #[test]
    fn test_chained_operations() {
        let result = [0.5_f32, 0.3, 0.2]
            .linearize(srgb::eotf)
            .to_xyz(&SRGB)
            .from_xyz(&REC2020)
            .encode(srgb::oetf)
            .clamp_01();

        // Just verify it runs without panicking
        assert!(result[0] >= 0.0 && result[0] <= 1.0);
        assert!(result[1] >= 0.0 && result[1] <= 1.0);
        assert!(result[2] >= 0.0 && result[2] <= 1.0);
    }

    #[test]
    fn test_chromatic_adaptation_d65_to_d60() {
        use vfx_primaries::ACES_AP1;
        
        // White in sRGB (D65) should stay white in ACEScg (D60)
        let white = convert_rgb([1.0, 1.0, 1.0], None, &SRGB, &ACES_AP1, None);
        
        // After chromatic adaptation, white maps to white
        assert!((white[0] - 1.0).abs() < 0.02, "R={}", white[0]);
        assert!((white[1] - 1.0).abs() < 0.02, "G={}", white[1]);
        assert!((white[2] - 1.0).abs() < 0.02, "B={}", white[2]);
    }

    #[test]
    fn test_chromatic_adaptation_roundtrip() {
        use vfx_primaries::ACES_AP1;
        
        let original = [0.5_f32, 0.3, 0.2];
        
        // sRGB -> ACEScg -> sRGB
        let acescg = convert_rgb(original, None, &SRGB, &ACES_AP1, None);
        let back = convert_rgb(acescg, None, &ACES_AP1, &SRGB, None);
        
        assert!((back[0] - original[0]).abs() < 0.001, "R: {} vs {}", back[0], original[0]);
        assert!((back[1] - original[1]).abs() < 0.001, "G: {} vs {}", back[1], original[1]);
        assert!((back[2] - original[2]).abs() < 0.001, "B: {} vs {}", back[2], original[2]);
    }
}
