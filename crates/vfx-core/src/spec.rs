//! Image specification and metadata.
//!
//! This module provides [`ImageSpec`] - a comprehensive description of image
//! properties including dimensions, pixel format, channel layout, and metadata.
//!
//! # Overview
//!
//! [`ImageSpec`] is inspired by OpenImageIO's `ImageSpec` but simplified for
//! Rust's type system. It serves as the "header" for image data, containing
//! everything needed to interpret raw pixel data.
//!
//! # Key Properties
//!
//! - **Dimensions**: width, height, depth (for 3D textures)
//! - **Channels**: number of channels, channel names
//! - **Data window**: active pixel region
//! - **Display window**: full frame dimensions (for overscan)
//! - **Metadata**: arbitrary key-value attributes
//!
//! # Usage
//!
//! ```rust
//! use vfx_core::{ImageSpec, Rect, DataFormat};
//!
//! // Simple RGB image
//! let spec = ImageSpec::rgb(1920, 1080);
//!
//! // RGBA with custom display window
//! let mut spec = ImageSpec::rgba(1920, 1080);
//! spec.display_window = Rect::new(0, 0, 2048, 1080); // 2K DCI with letterbox
//!
//! // Add metadata
//! spec.set_attr("Author", "VFX Artist");
//! spec.set_attr("Software", "vfx-rs");
//! ```
//!
//! # Display vs Data Window
//!
//! VFX workflows often use overscan - extra pixels beyond the final frame
//! for filtering and transforms. The display window is the final output area,
//! while the data window is where actual pixel data exists.
//!
//! ```text
//! ┌─────────────────────────────┐
//! │        Display Window       │
//! │   ┌───────────────────┐     │
//! │   │    Data Window    │     │
//! │   │   (actual pixels) │     │
//! │   └───────────────────┘     │
//! │                             │
//! └─────────────────────────────┘
//! ```
//!
//! # Dependencies
//!
//! - [`crate::rect::Rect`] - For window definitions
//!
//! # Used By
//!
//! - [`crate::image::Image`] - Stores spec alongside pixel data
//! - `vfx-io` - Reads/writes spec from image files

use crate::format::DataFormat;
use crate::Rect;
use std::collections::HashMap;

/// Alias for backward compatibility.
/// Prefer using [`DataFormat`] directly.
#[deprecated(since = "0.2.0", note = "Use DataFormat instead")]
pub type ChannelFormat = DataFormat;

/// Attribute value that can be stored in image metadata.
///
/// Supports common types found in image file metadata.
#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    /// Integer value
    Int(i64),
    /// Floating-point value
    Float(f64),
    /// String value
    String(String),
    /// Integer array
    IntArray(Vec<i64>),
    /// Float array
    FloatArray(Vec<f64>),
    /// 3x3 matrix (row-major)
    Matrix3([f32; 9]),
    /// 4x4 matrix (row-major)
    Matrix4([f32; 16]),
}

impl AttrValue {
    /// Returns this value as an integer, if applicable.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(v) => Some(*v),
            Self::Float(v) => Some(*v as i64),
            _ => None,
        }
    }

    /// Returns this value as a float, if applicable.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Int(v) => Some(*v as f64),
            Self::Float(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns this value as a string, if applicable.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
}

impl From<i32> for AttrValue {
    fn from(v: i32) -> Self {
        Self::Int(v as i64)
    }
}

impl From<i64> for AttrValue {
    fn from(v: i64) -> Self {
        Self::Int(v)
    }
}

impl From<f32> for AttrValue {
    fn from(v: f32) -> Self {
        Self::Float(v as f64)
    }
}

impl From<f64> for AttrValue {
    fn from(v: f64) -> Self {
        Self::Float(v)
    }
}

impl From<String> for AttrValue {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}

impl From<&str> for AttrValue {
    fn from(v: &str) -> Self {
        Self::String(v.to_string())
    }
}

/// Comprehensive image specification describing format and metadata.
///
/// This struct contains all information needed to interpret raw pixel data,
/// including dimensions, channel layout, data type, and arbitrary metadata.
///
/// # Design
///
/// Modeled after OpenImageIO's `ImageSpec` but simplified:
/// - Uses Rust enums instead of type codes
/// - Strongly typed channel format via [`DataFormat`]
/// - HashMap for flexible metadata
///
/// # Example
///
/// ```rust
/// use vfx_core::{ImageSpec, DataFormat, Rect};
///
/// let mut spec = ImageSpec::new(1920, 1080, 4, DataFormat::F16);
/// spec.channel_names = vec!["R".into(), "G".into(), "B".into(), "A".into()];
/// spec.set_attr("compression", "piz");
///
/// assert_eq!(spec.bytes_per_pixel(), 8); // 4 channels * 2 bytes
/// ```
#[derive(Debug, Clone)]
pub struct ImageSpec {
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Image depth (1 for 2D images, >1 for 3D textures)
    pub depth: u32,
    /// Number of channels per pixel
    pub channels: u8,
    /// Data type for each channel
    pub format: DataFormat,
    /// Optional channel names (e.g., ["R", "G", "B", "A"])
    pub channel_names: Vec<String>,
    /// Data window - region containing actual pixel data
    pub data_window: Rect,
    /// Display window - final output region
    pub display_window: Rect,
    /// Arbitrary metadata attributes
    pub attributes: HashMap<String, AttrValue>,
}

impl ImageSpec {
    /// Creates a new image specification with given dimensions and format.
    ///
    /// Both data and display windows are initialized to the full image.
    ///
    /// # Arguments
    ///
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `channels` - Number of channels per pixel
    /// * `format` - Channel data type
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_core::{ImageSpec, DataFormat};
    ///
    /// let spec = ImageSpec::new(1920, 1080, 4, DataFormat::F16);
    /// assert_eq!(spec.width, 1920);
    /// assert_eq!(spec.channels, 4);
    /// ```
    pub fn new(width: u32, height: u32, channels: u8, format: DataFormat) -> Self {
        let window = Rect::from_size(width, height);
        Self {
            width,
            height,
            depth: 1,
            channels,
            format,
            channel_names: Vec::new(),
            data_window: window,
            display_window: window,
            attributes: HashMap::new(),
        }
    }

    /// Creates a spec for an RGB image (3 channels).
    ///
    /// Uses F16 format by default (standard for VFX).
    #[inline]
    pub fn rgb(width: u32, height: u32) -> Self {
        let mut spec = Self::new(width, height, 3, DataFormat::F16);
        spec.channel_names = vec!["R".into(), "G".into(), "B".into()];
        spec
    }

    /// Creates a spec for an RGBA image (4 channels).
    ///
    /// Uses F16 format by default (standard for VFX).
    #[inline]
    pub fn rgba(width: u32, height: u32) -> Self {
        let mut spec = Self::new(width, height, 4, DataFormat::F16);
        spec.channel_names = vec!["R".into(), "G".into(), "B".into(), "A".into()];
        spec
    }

    /// Creates a spec for a grayscale image (1 channel).
    #[inline]
    pub fn gray(width: u32, height: u32) -> Self {
        let mut spec = Self::new(width, height, 1, DataFormat::F16);
        spec.channel_names = vec!["Y".into()];
        spec
    }

    /// Creates a spec for a grayscale + alpha image (2 channels).
    #[inline]
    pub fn gray_alpha(width: u32, height: u32) -> Self {
        let mut spec = Self::new(width, height, 2, DataFormat::F16);
        spec.channel_names = vec!["Y".into(), "A".into()];
        spec
    }

    /// Returns the number of bytes per pixel.
    ///
    /// This is `channels * bytes_per_channel`.
    #[inline]
    pub fn bytes_per_pixel(&self) -> usize {
        self.channels as usize * self.format.bytes_per_channel()
    }

    /// Returns the number of bytes per scanline (row).
    ///
    /// This is `width * bytes_per_pixel`.
    #[inline]
    pub fn bytes_per_row(&self) -> usize {
        self.width as usize * self.bytes_per_pixel()
    }

    /// Returns the total number of pixels in the image.
    #[inline]
    pub fn pixel_count(&self) -> u64 {
        self.width as u64 * self.height as u64 * self.depth as u64
    }

    /// Returns the total size of pixel data in bytes.
    ///
    /// This is the minimum buffer size needed to store all pixels.
    #[inline]
    pub fn data_size(&self) -> usize {
        self.pixel_count() as usize * self.bytes_per_pixel()
    }

    /// Returns `true` if the image has an alpha channel.
    ///
    /// Checks if channel names include "A" or "Alpha".
    pub fn has_alpha(&self) -> bool {
        self.channel_names.iter().any(|name| {
            let lower = name.to_lowercase();
            lower == "a" || lower == "alpha"
        })
    }

    /// Returns the index of the alpha channel, if present.
    pub fn alpha_channel(&self) -> Option<usize> {
        self.channel_names.iter().position(|name| {
            let lower = name.to_lowercase();
            lower == "a" || lower == "alpha"
        })
    }

    /// Sets an attribute value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_core::ImageSpec;
    ///
    /// let mut spec = ImageSpec::rgb(1920, 1080);
    /// spec.set_attr("Author", "VFX Artist");
    /// spec.set_attr("FrameRate", 24);
    /// spec.set_attr("ExposureTime", 0.041667);
    /// ```
    pub fn set_attr(&mut self, key: impl Into<String>, value: impl Into<AttrValue>) {
        self.attributes.insert(key.into(), value.into());
    }

    /// Gets an attribute value by key.
    pub fn get_attr(&self, key: &str) -> Option<&AttrValue> {
        self.attributes.get(key)
    }

    /// Gets an attribute as a string.
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.get_attr(key).and_then(|v| v.as_str())
    }

    /// Gets an attribute as an integer.
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.get_attr(key).and_then(|v| v.as_int())
    }

    /// Gets an attribute as a float.
    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.get_attr(key).and_then(|v| v.as_float())
    }

    /// Returns `true` if data and display windows differ.
    ///
    /// Indicates the image has overscan or is a crop.
    #[inline]
    pub fn has_overscan(&self) -> bool {
        self.data_window != self.display_window
    }

    /// Returns `true` if this is a 3D (volumetric) texture.
    #[inline]
    pub fn is_3d(&self) -> bool {
        self.depth > 1
    }

    /// Creates a copy with a different format.
    pub fn with_format(&self, format: DataFormat) -> Self {
        let mut spec = self.clone();
        spec.format = format;
        spec
    }

    /// Creates a copy with different dimensions.
    pub fn with_size(&self, width: u32, height: u32) -> Self {
        let mut spec = self.clone();
        spec.width = width;
        spec.height = height;
        spec.data_window = Rect::from_size(width, height);
        spec.display_window = Rect::from_size(width, height);
        spec
    }
}

impl Default for ImageSpec {
    fn default() -> Self {
        Self::rgba(0, 0)
    }
}

impl std::fmt::Display for ImageSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}x{}x{} {} {}ch",
            self.width, self.height, self.depth, self.format, self.channels
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_format() {
        assert_eq!(DataFormat::U8.bytes_per_channel(), 1);
        assert_eq!(DataFormat::U16.bytes_per_channel(), 2);
        assert_eq!(DataFormat::F16.bytes_per_channel(), 2);
        assert_eq!(DataFormat::F32.bytes_per_channel(), 4);

        assert!(!DataFormat::U8.is_float());
        assert!(DataFormat::F16.is_float());
    }

    #[test]
    fn test_spec_new() {
        let spec = ImageSpec::new(1920, 1080, 4, DataFormat::F16);
        assert_eq!(spec.width, 1920);
        assert_eq!(spec.height, 1080);
        assert_eq!(spec.channels, 4);
        assert_eq!(spec.format, DataFormat::F16);
    }

    #[test]
    fn test_spec_rgb_rgba() {
        let rgb = ImageSpec::rgb(100, 100);
        assert_eq!(rgb.channels, 3);
        assert!(!rgb.has_alpha());

        let rgba = ImageSpec::rgba(100, 100);
        assert_eq!(rgba.channels, 4);
        assert!(rgba.has_alpha());
        assert_eq!(rgba.alpha_channel(), Some(3));
    }

    #[test]
    fn test_spec_bytes() {
        let spec = ImageSpec::rgba(100, 100);
        assert_eq!(spec.bytes_per_pixel(), 8); // 4 * 2
        assert_eq!(spec.bytes_per_row(), 800);
        assert_eq!(spec.data_size(), 80000);
    }

    #[test]
    fn test_spec_attributes() {
        let mut spec = ImageSpec::rgb(100, 100);
        spec.set_attr("Author", "Test");
        spec.set_attr("FrameRate", 24);
        spec.set_attr("ExposureTime", 0.041667);

        assert_eq!(spec.get_string("Author"), Some("Test"));
        assert_eq!(spec.get_int("FrameRate"), Some(24));
        assert!((spec.get_float("ExposureTime").unwrap() - 0.041667).abs() < 0.0001);
    }

    #[test]
    fn test_spec_overscan() {
        let mut spec = ImageSpec::rgba(1920, 1080);
        assert!(!spec.has_overscan());

        spec.data_window = Rect::new(0, 0, 2048, 1156); // 5% overscan
        assert!(spec.has_overscan());
    }
}
