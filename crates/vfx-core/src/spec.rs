//! Image specification and metadata.
//!
//! This module provides [`ImageSpec`] - a comprehensive description of image
//! properties including dimensions, pixel format, channel layout, and metadata.
//! It is designed to be compatible with OpenImageIO's `ImageSpec`.
//!
//! # Overview
//!
//! [`ImageSpec`] contains everything needed to interpret raw pixel data:
//!
//! - **Dimensions**: width, height, depth (for 3D textures)
//! - **Origin**: x, y, z coordinates of the data window origin
//! - **Full/Display window**: full_x, full_y, full_z, full_width, full_height, full_depth
//! - **Tiling**: tile_width, tile_height, tile_depth for tiled images
//! - **Channels**: nchannels, channel_names, channelformats
//! - **Special channels**: alpha_channel, z_channel indices
//! - **Deep images**: deep flag for deep/volumetric data
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
//! spec.full_width = 2048; // 2K DCI with letterbox
//!
//! // Add metadata
//! spec.set_attr("Author", "VFX Artist");
//! spec.set_attr("Software", "vfx-rs");
//!
//! // Compute memory requirements
//! let scanline_bytes = spec.scanline_bytes(false);
//! let image_bytes = spec.image_bytes(false);
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
//! │   (full_x, full_y) origin   │
//! │   ┌───────────────────┐     │
//! │   │    Data Window    │     │
//! │   │   (x, y) origin   │     │
//! │   └───────────────────┘     │
//! │                             │
//! └─────────────────────────────┘
//! ```
//!
//! # Dependencies
//!
//! - [`crate::rect::Rect`] - For window definitions
//! - [`crate::format::TypeDesc`] - For type descriptors
//!
//! # Used By
//!
//! - [`crate::image::Image`] - Stores spec alongside pixel data
//! - `vfx-io` - Reads/writes spec from image files

use crate::format::{DataFormat, TypeDesc};
use crate::Rect;
use crate::Roi3D;
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
/// It is designed to be compatible with OpenImageIO's `ImageSpec`.
///
/// # Design
///
/// Modeled after OpenImageIO's `ImageSpec`:
/// - Uses Rust enums instead of type codes
/// - Strongly typed channel format via [`DataFormat`]
/// - HashMap for flexible metadata
/// - Full support for origin coordinates, tiling, and deep images
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
    // === Dimensions (data window) ===
    /// Image width in pixels (data window)
    pub width: u32,
    /// Image height in pixels (data window)
    pub height: u32,
    /// Image depth (1 for 2D images, >1 for 3D textures)
    pub depth: u32,

    // === Data window origin ===
    /// X origin of the data window (pixel data)
    pub x: i32,
    /// Y origin of the data window (pixel data)
    pub y: i32,
    /// Z origin of the data window (for 3D images)
    pub z: i32,

    // === Full/Display window ===
    /// Full/display width
    pub full_width: u32,
    /// Full/display height
    pub full_height: u32,
    /// Full/display depth
    pub full_depth: u32,
    /// Full/display X origin
    pub full_x: i32,
    /// Full/display Y origin
    pub full_y: i32,
    /// Full/display Z origin
    pub full_z: i32,

    // === Tiling ===
    /// Tile width (0 = not tiled, scanline-based)
    pub tile_width: u32,
    /// Tile height (0 = not tiled)
    pub tile_height: u32,
    /// Tile depth (for 3D tiled images)
    pub tile_depth: u32,

    // === Channels ===
    /// Number of channels per pixel
    pub nchannels: u8,
    /// Default data type for channels
    pub format: DataFormat,
    /// Per-channel data formats (if different from default)
    pub channelformats: Vec<DataFormat>,
    /// Channel names (e.g., ["R", "G", "B", "A"])
    pub channel_names: Vec<String>,
    /// Index of alpha channel (-1 if none)
    pub alpha_channel: i32,
    /// Index of depth/Z channel (-1 if none)
    pub z_channel: i32,

    // === Deep image ===
    /// Whether this is a deep (multi-sample per pixel) image
    pub deep: bool,

    // === Legacy fields for backward compatibility ===
    /// Data window - region containing actual pixel data
    #[deprecated(since = "0.3.0", note = "Use x, y, width, height instead")]
    pub data_window: Rect,
    /// Display window - final output region
    #[deprecated(since = "0.3.0", note = "Use full_x, full_y, full_width, full_height instead")]
    pub display_window: Rect,
    /// Alias for nchannels (backward compatibility)
    #[deprecated(since = "0.3.0", note = "Use nchannels instead")]
    pub channels: u8,

    // === Metadata ===
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
    /// * `nchannels` - Number of channels per pixel
    /// * `format` - Channel data type
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_core::{ImageSpec, DataFormat};
    ///
    /// let spec = ImageSpec::new(1920, 1080, 4, DataFormat::F16);
    /// assert_eq!(spec.width, 1920);
    /// assert_eq!(spec.nchannels, 4);
    /// ```
    #[allow(deprecated)]
    pub fn new(width: u32, height: u32, nchannels: u8, format: DataFormat) -> Self {
        let window = Rect::from_size(width, height);
        Self {
            // Dimensions
            width,
            height,
            depth: 1,
            // Origin
            x: 0,
            y: 0,
            z: 0,
            // Full/Display window
            full_width: width,
            full_height: height,
            full_depth: 1,
            full_x: 0,
            full_y: 0,
            full_z: 0,
            // Tiling
            tile_width: 0,
            tile_height: 0,
            tile_depth: 0,
            // Channels
            nchannels,
            format,
            channelformats: Vec::new(),
            channel_names: Vec::new(),
            alpha_channel: -1,
            z_channel: -1,
            // Deep
            deep: false,
            // Legacy
            data_window: window,
            display_window: window,
            channels: nchannels,
            // Metadata
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
        spec.alpha_channel = 3;
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
        spec.alpha_channel = 1;
        spec
    }

    /// Creates a spec from a [`Roi3D`].
    ///
    /// The ROI defines the image dimensions and channel count.
    /// Format defaults to F32.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_core::{ImageSpec, Roi3D};
    ///
    /// let roi = Roi3D::new_2d_with_channels(0, 1920, 0, 1080, 0, 4);
    /// let spec = ImageSpec::from_roi(&roi);
    /// assert_eq!(spec.width, 1920);
    /// assert_eq!(spec.height, 1080);
    /// assert_eq!(spec.nchannels, 4);
    /// ```
    pub fn from_roi(roi: &Roi3D) -> Self {
        let width = (roi.xend - roi.xbegin).max(0) as u32;
        let height = (roi.yend - roi.ybegin).max(0) as u32;
        let depth = (roi.zend - roi.zbegin).max(0) as u32;
        let nchannels = (roi.chend - roi.chbegin).max(0).min(255) as u8;

        let mut spec = Self::new(width, height, nchannels, DataFormat::F32);
        spec.depth = depth;
        spec.x = roi.xbegin;
        spec.y = roi.ybegin;
        spec.z = roi.zbegin;
        spec.default_channel_names();
        spec
    }

    /// Creates a spec from a [`Roi3D`] with a specified number of channels.
    ///
    /// The ROI defines spatial dimensions, but channel count is overridden.
    pub fn from_roi_nchannels(roi: &Roi3D, nchannels: u32) -> Self {
        let width = (roi.xend - roi.xbegin).max(0) as u32;
        let height = (roi.yend - roi.ybegin).max(0) as u32;
        let depth = (roi.zend - roi.zbegin).max(0) as u32;

        let mut spec = Self::new(width, height, nchannels.min(255) as u8, DataFormat::F32);
        spec.depth = depth;
        spec.x = roi.xbegin;
        spec.y = roi.ybegin;
        spec.z = roi.zbegin;
        spec.default_channel_names();
        spec
    }

    // ==========================================================================
    // OIIO-Compatible Methods
    // ==========================================================================

    /// Sets default channel names based on channel count.
    ///
    /// - 1 channel: ["Y"]
    /// - 2 channels: ["Y", "A"]
    /// - 3 channels: ["R", "G", "B"]
    /// - 4 channels: ["R", "G", "B", "A"]
    /// - 5+ channels: ["R", "G", "B", "A", "Z", ...]
    pub fn default_channel_names(&mut self) {
        self.channel_names = match self.nchannels {
            1 => vec!["Y".into()],
            2 => vec!["Y".into(), "A".into()],
            3 => vec!["R".into(), "G".into(), "B".into()],
            4 => vec!["R".into(), "G".into(), "B".into(), "A".into()],
            n => {
                let mut names = vec!["R".into(), "G".into(), "B".into(), "A".into()];
                for i in 4..n {
                    names.push(format!("channel{}", i));
                }
                names
            }
        };

        // Update alpha_channel based on names
        self.alpha_channel = self
            .channel_names
            .iter()
            .position(|n| n.eq_ignore_ascii_case("a") || n.eq_ignore_ascii_case("alpha"))
            .map(|i| i as i32)
            .unwrap_or(-1);

        // Update z_channel based on names
        self.z_channel = self
            .channel_names
            .iter()
            .position(|n| n.eq_ignore_ascii_case("z") || n.eq_ignore_ascii_case("depth"))
            .map(|i| i as i32)
            .unwrap_or(-1);
    }

    /// Returns the number of bytes for a single channel in the given format.
    ///
    /// If `native` is true, uses per-channel formats; otherwise uses default format.
    #[inline]
    pub fn channel_bytes(&self, chan: usize, native: bool) -> usize {
        if native && chan < self.channelformats.len() {
            self.channelformats[chan].bytes_per_channel()
        } else {
            self.format.bytes_per_channel()
        }
    }

    /// Returns the number of bytes for a single scanline (row).
    ///
    /// If `native` is true, uses per-channel formats.
    #[inline]
    pub fn scanline_bytes(&self, native: bool) -> usize {
        self.width as usize * self.pixel_bytes(native)
    }

    /// Returns the number of bytes for a single pixel.
    ///
    /// If `native` is true and per-channel formats exist, sums their sizes.
    #[inline]
    pub fn pixel_bytes(&self, native: bool) -> usize {
        if native && !self.channelformats.is_empty() {
            self.channelformats
                .iter()
                .map(|f| f.bytes_per_channel())
                .sum()
        } else {
            self.nchannels as usize * self.format.bytes_per_channel()
        }
    }

    /// Returns the number of pixels per tile.
    ///
    /// Returns 0 if the image is not tiled.
    #[inline]
    pub fn tile_pixels(&self) -> u64 {
        if self.tile_width == 0 || self.tile_height == 0 {
            0
        } else {
            let d = self.tile_depth.max(1) as u64;
            (self.tile_width as u64) * (self.tile_height as u64) * d
        }
    }

    /// Returns the number of bytes per tile.
    ///
    /// Returns 0 if the image is not tiled.
    #[inline]
    pub fn tile_bytes(&self, native: bool) -> usize {
        self.tile_pixels() as usize * self.pixel_bytes(native)
    }

    /// Returns the total number of bytes for the entire image.
    #[inline]
    pub fn image_bytes(&self, native: bool) -> u64 {
        self.image_pixels() * self.pixel_bytes(native) as u64
    }

    /// Returns the total number of pixels in the image.
    #[inline]
    pub fn image_pixels(&self) -> u64 {
        (self.width as u64) * (self.height as u64) * (self.depth as u64)
    }

    /// Returns true if all size calculations are safe (won't overflow).
    #[inline]
    pub fn size_t_safe(&self) -> bool {
        // Check if image_bytes fits in usize
        let bytes = self.image_bytes(false);
        bytes <= usize::MAX as u64
    }

    /// Computes automatic strides for pixel data.
    ///
    /// Returns (x_stride, y_stride, z_stride) in bytes.
    pub fn auto_stride(&self, native: bool) -> (usize, usize, usize) {
        let pixel_size = self.pixel_bytes(native);
        let x_stride = pixel_size;
        let y_stride = self.width as usize * pixel_size;
        let z_stride = y_stride * self.height as usize;
        (x_stride, y_stride, z_stride)
    }

    /// Checks if a tile range is valid for this image.
    ///
    /// Returns true if the tile coordinates are within the tiled region.
    pub fn valid_tile_range(&self, x: i32, y: i32, z: i32) -> bool {
        if self.tile_width == 0 || self.tile_height == 0 {
            return false;
        }

        let tw = self.tile_width as i32;
        let th = self.tile_height as i32;
        let td = self.tile_depth.max(1) as i32;

        x >= self.x
            && x < self.x + self.width as i32
            && (x - self.x) % tw == 0
            && y >= self.y
            && y < self.y + self.height as i32
            && (y - self.y) % th == 0
            && z >= self.z
            && z < self.z + self.depth as i32
            && (z - self.z) % td == 0
    }

    /// Copies dimensions from another ImageSpec.
    ///
    /// Copies width, height, depth, x, y, z, full_* dimensions,
    /// tile dimensions, and nchannels (but not format or metadata).
    pub fn copy_dimensions(&mut self, other: &ImageSpec) {
        self.width = other.width;
        self.height = other.height;
        self.depth = other.depth;
        self.x = other.x;
        self.y = other.y;
        self.z = other.z;
        self.full_width = other.full_width;
        self.full_height = other.full_height;
        self.full_depth = other.full_depth;
        self.full_x = other.full_x;
        self.full_y = other.full_y;
        self.full_z = other.full_z;
        self.tile_width = other.tile_width;
        self.tile_height = other.tile_height;
        self.tile_depth = other.tile_depth;
    }

    /// Sets the format for all channels.
    pub fn set_format(&mut self, format: DataFormat) {
        self.format = format;
        self.channelformats.clear();
    }

    /// Sets the colorspace metadata attribute.
    pub fn set_colorspace(&mut self, colorspace: &str) {
        self.set_attr("oiio:ColorSpace", colorspace);
    }

    /// Gets the colorspace metadata attribute.
    pub fn get_colorspace(&self) -> Option<&str> {
        self.get_string("oiio:ColorSpace")
    }

    /// Returns the TypeDesc for the primary format.
    pub fn format_typedesc(&self) -> TypeDesc {
        TypeDesc::from(self.format)
    }

    /// Returns the TypeDesc for a specific channel.
    pub fn channel_typedesc(&self, chan: usize) -> TypeDesc {
        if chan < self.channelformats.len() {
            TypeDesc::from(self.channelformats[chan])
        } else {
            TypeDesc::from(self.format)
        }
    }

    /// Returns true if the spec represents an undefined/invalid image.
    #[inline]
    pub fn undefined(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Returns the number of bytes per pixel.
    ///
    /// This is `nchannels * bytes_per_channel`.
    #[inline]
    pub fn bytes_per_pixel(&self) -> usize {
        self.nchannels as usize * self.format.bytes_per_channel()
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
    /// Uses the `alpha_channel` field if set, otherwise checks channel names.
    pub fn has_alpha(&self) -> bool {
        if self.alpha_channel >= 0 {
            return true;
        }
        self.channel_names.iter().any(|name| {
            let lower = name.to_lowercase();
            lower == "a" || lower == "alpha"
        })
    }

    /// Returns the index of the alpha channel, if present.
    ///
    /// Uses the `alpha_channel` field if set, otherwise searches channel names.
    pub fn get_alpha_channel(&self) -> Option<usize> {
        if self.alpha_channel >= 0 {
            return Some(self.alpha_channel as usize);
        }
        self.channel_names.iter().position(|name| {
            let lower = name.to_lowercase();
            lower == "a" || lower == "alpha"
        })
    }

    /// Returns the index of the depth/Z channel, if present.
    pub fn get_z_channel(&self) -> Option<usize> {
        if self.z_channel >= 0 {
            return Some(self.z_channel as usize);
        }
        self.channel_names.iter().position(|name| {
            let lower = name.to_lowercase();
            lower == "z" || lower == "depth"
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

    /// Gets an attribute as an integer with a default value.
    pub fn get_int_attribute(&self, key: &str, default: i64) -> i64 {
        self.get_int(key).unwrap_or(default)
    }

    /// Gets an attribute as a float with a default value.
    pub fn get_float_attribute(&self, key: &str, default: f64) -> f64 {
        self.get_float(key).unwrap_or(default)
    }

    /// Gets an attribute as a string with a default value.
    pub fn get_string_attribute(&self, key: &str, default: &str) -> String {
        self.get_string(key).map(|s| s.to_string()).unwrap_or_else(|| default.to_string())
    }

    /// Removes an attribute by key.
    ///
    /// Returns `true` if the attribute existed and was removed.
    pub fn erase_attribute(&mut self, key: &str) -> bool {
        self.attributes.remove(key).is_some()
    }

    /// Returns the type of an attribute.
    pub fn getattributetype(&self, key: &str) -> Option<TypeDesc> {
        self.get_attr(key).map(|v| match v {
            AttrValue::Int(_) => TypeDesc::INT64,
            AttrValue::Float(_) => TypeDesc::DOUBLE,
            AttrValue::String(_) => TypeDesc::STRING,
            AttrValue::IntArray(arr) => TypeDesc::INT64.array(arr.len() as i32),
            AttrValue::FloatArray(arr) => TypeDesc::DOUBLE.array(arr.len() as i32),
            AttrValue::Matrix3(_) => TypeDesc::matrix33(),
            AttrValue::Matrix4(_) => TypeDesc::matrix44(),
        })
    }

    /// Returns `true` if data and display windows differ.
    ///
    /// Indicates the image has overscan or is a crop.
    #[inline]
    pub fn has_overscan(&self) -> bool {
        self.x != self.full_x
            || self.y != self.full_y
            || self.z != self.full_z
            || self.width != self.full_width
            || self.height != self.full_height
            || self.depth != self.full_depth
    }

    /// Returns true if the image is tiled.
    #[inline]
    pub fn is_tiled(&self) -> bool {
        self.tile_width > 0 && self.tile_height > 0
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
        spec.channelformats.clear();
        spec
    }

    /// Creates a copy with different dimensions.
    #[allow(deprecated)]
    pub fn with_size(&self, width: u32, height: u32) -> Self {
        let mut spec = self.clone();
        spec.width = width;
        spec.height = height;
        spec.full_width = width;
        spec.full_height = height;
        spec.data_window = Rect::from_size(width, height);
        spec.display_window = Rect::from_size(width, height);
        spec
    }

    /// Creates a ROI from the image dimensions.
    pub fn roi(&self) -> crate::Roi3D {
        crate::Roi3D::new(
            self.x,
            self.x + self.width as i32,
            self.y,
            self.y + self.height as i32,
            self.z,
            self.z + self.depth as i32,
            0,
            self.nchannels as i32,
        )
    }

    /// Creates a ROI from the full/display dimensions.
    pub fn roi_full(&self) -> crate::Roi3D {
        crate::Roi3D::new(
            self.full_x,
            self.full_x + self.full_width as i32,
            self.full_y,
            self.full_y + self.full_height as i32,
            self.full_z,
            self.full_z + self.full_depth as i32,
            0,
            self.nchannels as i32,
        )
    }

    /// Sets dimensions from a ROI.
    pub fn set_roi(&mut self, roi: &crate::Roi3D) {
        self.x = roi.xbegin;
        self.y = roi.ybegin;
        self.z = roi.zbegin;
        self.width = roi.width() as u32;
        self.height = roi.height() as u32;
        self.depth = roi.depth() as u32;
    }

    /// Sets full/display dimensions from a ROI.
    pub fn set_roi_full(&mut self, roi: &crate::Roi3D) {
        self.full_x = roi.xbegin;
        self.full_y = roi.ybegin;
        self.full_z = roi.zbegin;
        self.full_width = roi.width() as u32;
        self.full_height = roi.height() as u32;
        self.full_depth = roi.depth() as u32;
    }
}

impl Default for ImageSpec {
    #[allow(deprecated)]
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            depth: 1,
            x: 0,
            y: 0,
            z: 0,
            full_width: 0,
            full_height: 0,
            full_depth: 1,
            full_x: 0,
            full_y: 0,
            full_z: 0,
            tile_width: 0,
            tile_height: 0,
            tile_depth: 0,
            nchannels: 0,
            format: DataFormat::default(),
            channelformats: Vec::new(),
            channel_names: Vec::new(),
            alpha_channel: -1,
            z_channel: -1,
            deep: false,
            data_window: Rect::default(),
            display_window: Rect::default(),
            channels: 0,
            attributes: HashMap::new(),
        }
    }
}

impl std::fmt::Display for ImageSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.depth > 1 {
            write!(
                f,
                "{}x{}x{} {} {}ch",
                self.width, self.height, self.depth, self.format, self.nchannels
            )
        } else {
            write!(
                f,
                "{}x{} {} {}ch",
                self.width, self.height, self.format, self.nchannels
            )
        }
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
        assert_eq!(spec.nchannels, 4);
        assert_eq!(spec.format, DataFormat::F16);
        // New OIIO-compatible fields
        assert_eq!(spec.x, 0);
        assert_eq!(spec.y, 0);
        assert_eq!(spec.full_width, 1920);
        assert_eq!(spec.full_height, 1080);
    }

    #[test]
    fn test_spec_rgb_rgba() {
        let rgb = ImageSpec::rgb(100, 100);
        assert_eq!(rgb.nchannels, 3);
        assert!(!rgb.has_alpha());

        let rgba = ImageSpec::rgba(100, 100);
        assert_eq!(rgba.nchannels, 4);
        assert!(rgba.has_alpha());
        assert_eq!(rgba.get_alpha_channel(), Some(3));
        assert_eq!(rgba.alpha_channel, 3);
    }

    #[test]
    fn test_spec_bytes() {
        let spec = ImageSpec::rgba(100, 100);
        assert_eq!(spec.bytes_per_pixel(), 8); // 4 * 2
        assert_eq!(spec.bytes_per_row(), 800);
        assert_eq!(spec.data_size(), 80000);
        // New methods
        assert_eq!(spec.pixel_bytes(false), 8);
        assert_eq!(spec.scanline_bytes(false), 800);
        assert_eq!(spec.image_bytes(false), 80000);
        assert_eq!(spec.image_pixels(), 10000);
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

        // Test new attribute methods
        assert_eq!(spec.get_int_attribute("FrameRate", 0), 24);
        assert_eq!(spec.get_int_attribute("NonExistent", 42), 42);

        // Test erase_attribute
        assert!(spec.erase_attribute("Author"));
        assert!(!spec.erase_attribute("Author")); // Already removed
        assert!(spec.get_string("Author").is_none());
    }

    #[test]
    fn test_spec_overscan() {
        let mut spec = ImageSpec::rgba(1920, 1080);
        assert!(!spec.has_overscan());

        // Set overscan using new fields
        spec.full_width = 2048;
        spec.full_height = 1156;
        assert!(spec.has_overscan());
    }

    #[test]
    fn test_spec_tiling() {
        let mut spec = ImageSpec::rgba(1920, 1080);
        assert!(!spec.is_tiled());
        assert_eq!(spec.tile_pixels(), 0);

        spec.tile_width = 64;
        spec.tile_height = 64;
        spec.tile_depth = 1;
        assert!(spec.is_tiled());
        assert_eq!(spec.tile_pixels(), 64 * 64);
        assert_eq!(spec.tile_bytes(false), 64 * 64 * 8);
    }

    #[test]
    fn test_spec_auto_stride() {
        let spec = ImageSpec::rgba(100, 100);
        let (x, y, z) = spec.auto_stride(false);
        assert_eq!(x, 8);    // pixel size
        assert_eq!(y, 800);  // scanline
        assert_eq!(z, 80000); // full image
    }

    #[test]
    fn test_spec_roi() {
        let spec = ImageSpec::rgba(1920, 1080);
        let roi = spec.roi();
        assert_eq!(roi.width(), 1920);
        assert_eq!(roi.height(), 1080);
        assert_eq!(roi.nchannels(), 4);
    }

    #[test]
    fn test_spec_copy_dimensions() {
        let src = ImageSpec::rgba(1920, 1080);
        let mut dst = ImageSpec::default();
        dst.copy_dimensions(&src);
        assert_eq!(dst.width, 1920);
        assert_eq!(dst.height, 1080);
        assert_eq!(dst.nchannels, 0); // Not copied
    }

    #[test]
    fn test_spec_colorspace() {
        let mut spec = ImageSpec::rgba(100, 100);
        spec.set_colorspace("ACEScg");
        assert_eq!(spec.get_colorspace(), Some("ACEScg"));
    }

    #[test]
    fn test_default_channel_names() {
        let mut spec = ImageSpec::new(100, 100, 4, DataFormat::F16);
        spec.default_channel_names();
        assert_eq!(spec.channel_names, vec!["R", "G", "B", "A"]);
        assert_eq!(spec.alpha_channel, 3);
    }
}
