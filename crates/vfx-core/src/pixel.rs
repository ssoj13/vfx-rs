//! Pixel types and data formats for image processing.
//!
//! This module provides generic pixel types that track both their color space
//! and underlying data type at compile time.
//!
//! # Types
//!
//! - [`Rgb`] - RGB pixel (3 channels)
//! - [`Rgba`] - RGBA pixel with alpha (4 channels)
//! - [`PixelFormat`] - Trait for pixel data types (u8, u16, f16, f32)
//!
//! # Design
//!
//! Pixels are parameterized by:
//! 1. **Color Space** (`C: ColorSpace`) - compile-time color space tracking
//! 2. **Data Type** (`T: PixelFormat`) - the underlying numeric type
//!
//! ```
//! use vfx_core::prelude::*;
//!
//! // 8-bit sRGB pixel
//! let srgb_pixel: Rgba<Srgb, u8> = Rgba::new(255, 128, 64, 255);
//!
//! // 32-bit float ACEScg pixel
//! let aces_pixel: Rgba<AcesCg, f32> = Rgba::new(0.18, 0.18, 0.18, 1.0);
//! ```
//!
//! # Memory Layout
//!
//! All pixel types use `#[repr(C)]` for predictable memory layout,
//! enabling safe zero-copy operations with GPU buffers and FFI.
//!
//! # Dependencies
//!
//! - `half` crate for `f16` support
//!
//! # Used By
//!
//! - `vfx-core::Image` - image buffers store pixels
//! - `vfx-ops` - pixel-level operations
//! - `vfx-io` - format readers/writers

use crate::colorspace::ColorSpace;
use half::f16;
use std::fmt;
use std::marker::PhantomData;
use std::ops::{Add, Mul, Sub};

// ============================================================================
// Rec.709 Luminance Constants
// ============================================================================

/// Rec.709 luminance coefficient for red channel.
///
/// Used in the standard luminance formula: `Y = 0.2126*R + 0.7152*G + 0.0722*B`
pub const REC709_LUMA_R: f32 = 0.2126;

/// Rec.709 luminance coefficient for green channel.
pub const REC709_LUMA_G: f32 = 0.7152;

/// Rec.709 luminance coefficient for blue channel.
pub const REC709_LUMA_B: f32 = 0.0722;

/// Rec.709 luminance coefficients as an array [R, G, B].
///
/// # Example
/// ```
/// use vfx_core::pixel::REC709_LUMA;
/// let rgb = [0.5, 0.3, 0.2];
/// let luma = rgb[0] * REC709_LUMA[0] + rgb[1] * REC709_LUMA[1] + rgb[2] * REC709_LUMA[2];
/// ```
pub const REC709_LUMA: [f32; 3] = [REC709_LUMA_R, REC709_LUMA_G, REC709_LUMA_B];

/// Calculate Rec.709 luminance from RGB values.
///
/// This is the standard luminance calculation for sRGB/Rec.709 primaries:
/// `Y = 0.2126*R + 0.7152*G + 0.0722*B`
///
/// # Arguments
/// * `rgb` - RGB values as [R, G, B] array
///
/// # Returns
/// The luminance value
///
/// # Example
/// ```
/// use vfx_core::pixel::luminance_rec709;
/// let luma = luminance_rec709([0.5, 0.3, 0.2]);
/// // 0.5 * 0.2126 + 0.3 * 0.7152 + 0.2 * 0.0722 = 0.3353
/// assert!((luma - 0.3353).abs() < 0.0001);
/// ```
#[inline]
pub fn luminance_rec709(rgb: [f32; 3]) -> f32 {
    rgb[0] * REC709_LUMA_R + rgb[1] * REC709_LUMA_G + rgb[2] * REC709_LUMA_B
}

/// Trait for pixel data types.
///
/// Implemented for standard numeric types used in image processing:
/// - `u8` - 8-bit unsigned (0-255)
/// - `u16` - 16-bit unsigned (0-65535)
/// - `f16` - 16-bit float (half precision)
/// - `f32` - 32-bit float (single precision)
///
/// # Required Methods
///
/// - [`to_f32`](PixelFormat::to_f32) - Convert to normalized f32 [0.0, 1.0] for integers
/// - [`from_f32`](PixelFormat::from_f32) - Convert from normalized f32
///
/// # Constants
///
/// - [`BITS`](PixelFormat::BITS) - Bit depth of the type
/// - [`IS_FLOAT`](PixelFormat::IS_FLOAT) - Whether this is a floating-point type
/// - [`MAX_VALUE`](PixelFormat::MAX_VALUE) - Maximum representable value
///
/// # Example
///
/// ```
/// use vfx_core::PixelFormat;
///
/// // Convert 8-bit to float
/// let byte_val: u8 = 128;
/// let float_val = byte_val.to_f32();
/// assert!((float_val - 0.502).abs() < 0.01);
///
/// // Convert float to 16-bit
/// let back: u16 = PixelFormat::from_f32(0.5);
/// assert_eq!(back, 32768);
/// ```
pub trait PixelFormat: Copy + Clone + Default + Send + Sync + PartialOrd + 'static {
    /// Number of bits per channel.
    const BITS: u32;

    /// Whether this is a floating-point format.
    ///
    /// - `true` for f16, f32
    /// - `false` for u8, u16
    const IS_FLOAT: bool;

    /// Maximum representable value.
    ///
    /// - 255 for u8
    /// - 65535 for u16
    /// - f32::MAX for floats
    const MAX_VALUE: f32;

    /// Minimum representable value (for floats, can be negative).
    const MIN_VALUE: f32;

    /// Convert to f32.
    ///
    /// For integers, normalizes to [0.0, 1.0] range.
    /// For floats, returns the value directly.
    fn to_f32(self) -> f32;

    /// Convert from f32.
    ///
    /// For integers, expects [0.0, 1.0] range and clamps.
    /// For floats, returns the value directly (may clamp for f16).
    fn from_f32(v: f32) -> Self;

    /// Linear interpolation between two values.
    #[inline]
    fn lerp(a: Self, b: Self, t: f32) -> Self {
        Self::from_f32(a.to_f32() * (1.0 - t) + b.to_f32() * t)
    }

    /// Clamp value to valid range.
    fn clamp(self, min: Self, max: Self) -> Self;

    /// Zero value.
    fn zero() -> Self;

    /// One value (1.0 for floats, max for integers).
    fn one() -> Self;
}

impl PixelFormat for u8 {
    const BITS: u32 = 8;
    const IS_FLOAT: bool = false;
    const MAX_VALUE: f32 = 255.0;
    const MIN_VALUE: f32 = 0.0;

    #[inline]
    fn to_f32(self) -> f32 {
        self as f32 / 255.0
    }

    #[inline]
    fn from_f32(v: f32) -> Self {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }

    #[inline]
    fn clamp(self, min: Self, max: Self) -> Self {
        Ord::clamp(self, min, max)
    }

    #[inline]
    fn zero() -> Self {
        0
    }

    #[inline]
    fn one() -> Self {
        255
    }
}

impl PixelFormat for u16 {
    const BITS: u32 = 16;
    const IS_FLOAT: bool = false;
    const MAX_VALUE: f32 = 65535.0;
    const MIN_VALUE: f32 = 0.0;

    #[inline]
    fn to_f32(self) -> f32 {
        self as f32 / 65535.0
    }

    #[inline]
    fn from_f32(v: f32) -> Self {
        (v.clamp(0.0, 1.0) * 65535.0).round() as u16
    }

    #[inline]
    fn clamp(self, min: Self, max: Self) -> Self {
        Ord::clamp(self, min, max)
    }

    #[inline]
    fn zero() -> Self {
        0
    }

    #[inline]
    fn one() -> Self {
        65535
    }
}

impl PixelFormat for f16 {
    const BITS: u32 = 16;
    const IS_FLOAT: bool = true;
    const MAX_VALUE: f32 = 65504.0; // f16 max
    const MIN_VALUE: f32 = -65504.0;

    #[inline]
    fn to_f32(self) -> f32 {
        self.to_f32()
    }

    #[inline]
    fn from_f32(v: f32) -> Self {
        f16::from_f32(v)
    }

    #[inline]
    fn clamp(self, min: Self, max: Self) -> Self {
        if self < min {
            min
        } else if self > max {
            max
        } else {
            self
        }
    }

    #[inline]
    fn zero() -> Self {
        f16::ZERO
    }

    #[inline]
    fn one() -> Self {
        f16::ONE
    }
}

impl PixelFormat for f32 {
    const BITS: u32 = 32;
    const IS_FLOAT: bool = true;
    const MAX_VALUE: f32 = f32::MAX;
    const MIN_VALUE: f32 = f32::MIN;

    #[inline]
    fn to_f32(self) -> f32 {
        self
    }

    #[inline]
    fn from_f32(v: f32) -> Self {
        v
    }

    #[inline]
    fn clamp(self, min: Self, max: Self) -> Self {
        f32::clamp(self, min, max)
    }

    #[inline]
    fn zero() -> Self {
        0.0
    }

    #[inline]
    fn one() -> Self {
        1.0
    }
}

/// RGB pixel with color space tracking.
///
/// A 3-channel pixel type for red, green, and blue values.
///
/// # Type Parameters
///
/// - `C: ColorSpace` - The color space this pixel belongs to
/// - `T: PixelFormat` - The underlying data type (u8, u16, f16, f32)
///
/// # Memory Layout
///
/// Uses `#[repr(C)]` for predictable layout: `[R, G, B]`
///
/// # Example
///
/// ```
/// use vfx_core::prelude::*;
///
/// // Create an sRGB pixel
/// let pixel: Rgb<Srgb, u8> = Rgb::new(255, 128, 64);
/// assert_eq!(pixel.r, 255);
///
/// // Convert to float
/// let float_pixel: Rgb<Srgb, f32> = pixel.convert_format();
/// assert!((float_pixel.r - 1.0).abs() < 0.01);
/// ```
#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq)]
pub struct Rgb<C: ColorSpace, T: PixelFormat> {
    /// Red channel value.
    pub r: T,
    /// Green channel value.
    pub g: T,
    /// Blue channel value.
    pub b: T,
    /// PhantomData to track color space at compile time.
    _colorspace: PhantomData<C>,
}

impl<C: ColorSpace, T: PixelFormat> Rgb<C, T> {
    /// Create a new RGB pixel.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel value
    /// * `g` - Green channel value
    /// * `b` - Blue channel value
    #[inline]
    pub const fn new(r: T, g: T, b: T) -> Self {
        Self {
            r,
            g,
            b,
            _colorspace: PhantomData,
        }
    }

    /// Create a grayscale pixel with equal RGB values.
    #[inline]
    pub const fn gray(v: T) -> Self {
        Self::new(v, v, v)
    }

    /// Create a black pixel (all zeros).
    #[inline]
    pub fn black() -> Self {
        Self::new(T::zero(), T::zero(), T::zero())
    }

    /// Create a white pixel (all max values).
    #[inline]
    pub fn white() -> Self {
        Self::new(T::one(), T::one(), T::one())
    }

    /// Get RGB values as an array.
    #[inline]
    pub fn to_array(self) -> [T; 3] {
        [self.r, self.g, self.b]
    }

    /// Create from an array.
    #[inline]
    pub fn from_array(arr: [T; 3]) -> Self {
        Self::new(arr[0], arr[1], arr[2])
    }

    /// Convert to f32 array (normalized for integers).
    #[inline]
    pub fn to_f32_array(self) -> [f32; 3] {
        [self.r.to_f32(), self.g.to_f32(), self.b.to_f32()]
    }

    /// Create from f32 array.
    #[inline]
    pub fn from_f32_array(arr: [f32; 3]) -> Self {
        Self::new(
            T::from_f32(arr[0]),
            T::from_f32(arr[1]),
            T::from_f32(arr[2]),
        )
    }

    /// Convert to a different pixel format (same color space).
    ///
    /// # Example
    ///
    /// ```
    /// use vfx_core::prelude::*;
    ///
    /// let byte_pixel: Rgb<Srgb, u8> = Rgb::new(255, 128, 0);
    /// let float_pixel: Rgb<Srgb, f32> = byte_pixel.convert_format();
    /// ```
    #[inline]
    pub fn convert_format<U: PixelFormat>(self) -> Rgb<C, U> {
        Rgb::new(
            U::from_f32(self.r.to_f32()),
            U::from_f32(self.g.to_f32()),
            U::from_f32(self.b.to_f32()),
        )
    }

    /// Apply a function to each channel.
    #[inline]
    pub fn map<F: Fn(T) -> T>(self, f: F) -> Self {
        Self::new(f(self.r), f(self.g), f(self.b))
    }

    /// Apply a function with f32 intermediates.
    #[inline]
    pub fn map_f32<F: Fn(f32) -> f32>(self, f: F) -> Self {
        Self::new(
            T::from_f32(f(self.r.to_f32())),
            T::from_f32(f(self.g.to_f32())),
            T::from_f32(f(self.b.to_f32())),
        )
    }

    /// Calculate luminance using Rec.709 coefficients.
    ///
    /// Note: This is only accurate for Rec.709/sRGB primaries.
    /// For other color spaces, use proper luminance matrices.
    #[inline]
    pub fn luminance_rec709(self) -> f32 {
        luminance_rec709(self.to_f32_array())
    }
}

impl<C: ColorSpace, T: PixelFormat + fmt::Debug> fmt::Debug for Rgb<C, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&format!("Rgb<{}>", C::NAME))
            .field("r", &self.r)
            .field("g", &self.g)
            .field("b", &self.b)
            .finish()
    }
}

impl<C: ColorSpace, T: PixelFormat + fmt::Display> fmt::Display for Rgb<C, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RGB({}, {}, {})", self.r, self.g, self.b)
    }
}

/// RGBA pixel with color space tracking.
///
/// A 4-channel pixel type for red, green, blue, and alpha values.
///
/// # Alpha Handling
///
/// By default, alpha is **NOT** premultiplied. Use explicit functions
/// to premultiply/unpremultiply when needed.
///
/// # Type Parameters
///
/// - `C: ColorSpace` - The color space this pixel belongs to
/// - `T: PixelFormat` - The underlying data type (u8, u16, f16, f32)
///
/// # Memory Layout
///
/// Uses `#[repr(C)]` for predictable layout: `[R, G, B, A]`
///
/// # Example
///
/// ```
/// use vfx_core::prelude::*;
///
/// // Opaque red pixel in ACEScg
/// let pixel: Rgba<AcesCg, f32> = Rgba::new(1.0, 0.0, 0.0, 1.0);
///
/// // Semi-transparent
/// let semi: Rgba<AcesCg, f32> = Rgba::with_alpha(pixel.rgb(), 0.5);
/// ```
#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq)]
pub struct Rgba<C: ColorSpace, T: PixelFormat> {
    /// Red channel value.
    pub r: T,
    /// Green channel value.
    pub g: T,
    /// Blue channel value.
    pub b: T,
    /// Alpha channel value.
    pub a: T,
    /// PhantomData to track color space at compile time.
    _colorspace: PhantomData<C>,
}

impl<C: ColorSpace, T: PixelFormat> Rgba<C, T> {
    /// Create a new RGBA pixel.
    #[inline]
    pub const fn new(r: T, g: T, b: T, a: T) -> Self {
        Self {
            r,
            g,
            b,
            a,
            _colorspace: PhantomData,
        }
    }

    /// Create from RGB with specified alpha.
    #[inline]
    pub fn with_alpha(rgb: Rgb<C, T>, a: T) -> Self {
        Self::new(rgb.r, rgb.g, rgb.b, a)
    }

    /// Create an opaque pixel (alpha = 1.0).
    #[inline]
    pub fn opaque(r: T, g: T, b: T) -> Self {
        Self::new(r, g, b, T::one())
    }

    /// Create from RGB with opaque alpha.
    #[inline]
    pub fn from_rgb(rgb: Rgb<C, T>) -> Self {
        Self::with_alpha(rgb, T::one())
    }

    /// Create a grayscale pixel with equal RGB values.
    #[inline]
    pub fn gray(v: T, a: T) -> Self {
        Self::new(v, v, v, a)
    }

    /// Create a black pixel (all zeros, opaque).
    #[inline]
    pub fn black() -> Self {
        Self::new(T::zero(), T::zero(), T::zero(), T::one())
    }

    /// Create a white pixel (all max values, opaque).
    #[inline]
    pub fn white() -> Self {
        Self::new(T::one(), T::one(), T::one(), T::one())
    }

    /// Create a transparent pixel (all zeros including alpha).
    #[inline]
    pub fn transparent() -> Self {
        Self::new(T::zero(), T::zero(), T::zero(), T::zero())
    }

    /// Get RGB component (discarding alpha).
    #[inline]
    pub fn rgb(self) -> Rgb<C, T> {
        Rgb::new(self.r, self.g, self.b)
    }

    /// Get RGBA values as an array.
    #[inline]
    pub fn to_array(self) -> [T; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Create from an array.
    #[inline]
    pub fn from_array(arr: [T; 4]) -> Self {
        Self::new(arr[0], arr[1], arr[2], arr[3])
    }

    /// Convert to f32 array.
    #[inline]
    pub fn to_f32_array(self) -> [f32; 4] {
        [
            self.r.to_f32(),
            self.g.to_f32(),
            self.b.to_f32(),
            self.a.to_f32(),
        ]
    }

    /// Create from f32 array.
    #[inline]
    pub fn from_f32_array(arr: [f32; 4]) -> Self {
        Self::new(
            T::from_f32(arr[0]),
            T::from_f32(arr[1]),
            T::from_f32(arr[2]),
            T::from_f32(arr[3]),
        )
    }

    /// Convert to a different pixel format (same color space).
    #[inline]
    pub fn convert_format<U: PixelFormat>(self) -> Rgba<C, U> {
        Rgba::new(
            U::from_f32(self.r.to_f32()),
            U::from_f32(self.g.to_f32()),
            U::from_f32(self.b.to_f32()),
            U::from_f32(self.a.to_f32()),
        )
    }

    /// Apply a function to RGB channels (preserving alpha).
    #[inline]
    pub fn map_rgb<F: Fn(T) -> T>(self, f: F) -> Self {
        Self::new(f(self.r), f(self.g), f(self.b), self.a)
    }

    /// Apply a function to all channels including alpha.
    #[inline]
    pub fn map<F: Fn(T) -> T>(self, f: F) -> Self {
        Self::new(f(self.r), f(self.g), f(self.b), f(self.a))
    }

    /// Apply a function with f32 intermediates (RGB only).
    #[inline]
    pub fn map_rgb_f32<F: Fn(f32) -> f32>(self, f: F) -> Self {
        Self::new(
            T::from_f32(f(self.r.to_f32())),
            T::from_f32(f(self.g.to_f32())),
            T::from_f32(f(self.b.to_f32())),
            self.a,
        )
    }

    /// Premultiply RGB by alpha.
    ///
    /// Converts straight alpha to premultiplied alpha:
    /// `(R, G, B, A) -> (R*A, G*A, B*A, A)`
    #[inline]
    pub fn premultiply(self) -> Self {
        let a = self.a.to_f32();
        Self::new(
            T::from_f32(self.r.to_f32() * a),
            T::from_f32(self.g.to_f32() * a),
            T::from_f32(self.b.to_f32() * a),
            self.a,
        )
    }

    /// Unpremultiply RGB by alpha.
    ///
    /// Converts premultiplied alpha to straight alpha:
    /// `(R*A, G*A, B*A, A) -> (R, G, B, A)`
    ///
    /// Returns transparent black if alpha is zero.
    #[inline]
    pub fn unpremultiply(self) -> Self {
        let a = self.a.to_f32();
        if a < 1e-6 {
            Self::transparent()
        } else {
            let inv_a = 1.0 / a;
            Self::new(
                T::from_f32(self.r.to_f32() * inv_a),
                T::from_f32(self.g.to_f32() * inv_a),
                T::from_f32(self.b.to_f32() * inv_a),
                self.a,
            )
        }
    }

    /// Check if pixel is fully opaque.
    #[inline]
    pub fn is_opaque(self) -> bool {
        self.a.to_f32() >= 1.0 - 1e-6
    }

    /// Check if pixel is fully transparent.
    #[inline]
    pub fn is_transparent(self) -> bool {
        self.a.to_f32() < 1e-6
    }
}

impl<C: ColorSpace, T: PixelFormat + fmt::Debug> fmt::Debug for Rgba<C, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&format!("Rgba<{}>", C::NAME))
            .field("r", &self.r)
            .field("g", &self.g)
            .field("b", &self.b)
            .field("a", &self.a)
            .finish()
    }
}

impl<C: ColorSpace, T: PixelFormat + fmt::Display> fmt::Display for Rgba<C, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RGBA({}, {}, {}, {})", self.r, self.g, self.b, self.a)
    }
}

// ============================================================================
// Arithmetic Operations
// ============================================================================

impl<C: ColorSpace, T: PixelFormat> Add for Rgb<C, T> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(
            T::from_f32(self.r.to_f32() + rhs.r.to_f32()),
            T::from_f32(self.g.to_f32() + rhs.g.to_f32()),
            T::from_f32(self.b.to_f32() + rhs.b.to_f32()),
        )
    }
}

impl<C: ColorSpace, T: PixelFormat> Sub for Rgb<C, T> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(
            T::from_f32(self.r.to_f32() - rhs.r.to_f32()),
            T::from_f32(self.g.to_f32() - rhs.g.to_f32()),
            T::from_f32(self.b.to_f32() - rhs.b.to_f32()),
        )
    }
}

impl<C: ColorSpace, T: PixelFormat> Mul<f32> for Rgb<C, T> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(
            T::from_f32(self.r.to_f32() * rhs),
            T::from_f32(self.g.to_f32() * rhs),
            T::from_f32(self.b.to_f32() * rhs),
        )
    }
}

impl<C: ColorSpace, T: PixelFormat> Add for Rgba<C, T> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(
            T::from_f32(self.r.to_f32() + rhs.r.to_f32()),
            T::from_f32(self.g.to_f32() + rhs.g.to_f32()),
            T::from_f32(self.b.to_f32() + rhs.b.to_f32()),
            T::from_f32(self.a.to_f32() + rhs.a.to_f32()),
        )
    }
}

impl<C: ColorSpace, T: PixelFormat> Mul<f32> for Rgba<C, T> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(
            T::from_f32(self.r.to_f32() * rhs),
            T::from_f32(self.g.to_f32() * rhs),
            T::from_f32(self.b.to_f32() * rhs),
            T::from_f32(self.a.to_f32() * rhs),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colorspace::Srgb;

    #[test]
    fn test_pixel_format_u8() {
        assert_eq!(u8::BITS, 8);
        assert!(!u8::IS_FLOAT);
        assert!((128u8.to_f32() - 0.502).abs() < 0.01);
        assert_eq!(u8::from_f32(0.5), 128);
    }

    #[test]
    fn test_pixel_format_f32() {
        assert_eq!(f32::BITS, 32);
        assert!(f32::IS_FLOAT);
        assert_eq!(0.5f32.to_f32(), 0.5);
        assert_eq!(f32::from_f32(0.5), 0.5);
    }

    #[test]
    fn test_rgb_creation() {
        let pixel: Rgb<Srgb, u8> = Rgb::new(255, 128, 64);
        assert_eq!(pixel.r, 255);
        assert_eq!(pixel.g, 128);
        assert_eq!(pixel.b, 64);
    }

    #[test]
    fn test_rgb_format_conversion() {
        let byte_pixel: Rgb<Srgb, u8> = Rgb::new(255, 128, 0);
        let float_pixel: Rgb<Srgb, f32> = byte_pixel.convert_format();
        assert!((float_pixel.r - 1.0).abs() < 0.01);
        assert!((float_pixel.g - 0.502).abs() < 0.01);
        assert!((float_pixel.b - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_rgba_premultiply() {
        let pixel: Rgba<Srgb, f32> = Rgba::new(1.0, 0.5, 0.25, 0.5);
        let premul = pixel.premultiply();
        assert!((premul.r - 0.5).abs() < 0.001);
        assert!((premul.g - 0.25).abs() < 0.001);
        assert!((premul.b - 0.125).abs() < 0.001);
        assert!((premul.a - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_rgba_unpremultiply() {
        let premul: Rgba<Srgb, f32> = Rgba::new(0.5, 0.25, 0.125, 0.5);
        let straight = premul.unpremultiply();
        assert!((straight.r - 1.0).abs() < 0.001);
        assert!((straight.g - 0.5).abs() < 0.001);
        assert!((straight.b - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_pixel_arithmetic() {
        let a: Rgb<Srgb, f32> = Rgb::new(0.5, 0.3, 0.2);
        let b: Rgb<Srgb, f32> = Rgb::new(0.1, 0.2, 0.3);
        let sum = a + b;
        assert!((sum.r - 0.6).abs() < 0.001);
        assert!((sum.g - 0.5).abs() < 0.001);
        assert!((sum.b - 0.5).abs() < 0.001);
    }
}
