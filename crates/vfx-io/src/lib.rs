//! # vfx-io
//!
//! Image I/O for VFX pipelines.
//!
//! This crate provides reading and writing of common image formats used in
//! visual effects and film production:
//!
//! - **EXR** - OpenEXR for HDR/linear workflow
//! - **HDR** - Radiance RGBE format
//! - **PNG** - Lossless with alpha support
//! - **JPEG** - Lossy compression for previews
//! - **TIFF** - Print/archival with LZW compression
//! - **DPX** - Film scanning/output (10-bit log)
//!
//! # Architecture
//!
//! The crate uses a trait-based design for extensibility:
//!
//! - [`FormatReader`] - Trait for format readers with options
//! - [`FormatWriter`] - Trait for format writers with options
//! - [`read`] / [`write`] - High-level functions with format auto-detection
//!
//! Each format provides:
//! - Reader struct (e.g., `DpxReader`) implementing `FormatReader`
//! - Writer struct (e.g., `DpxWriter`) implementing `FormatWriter`
//! - Convenience functions (`dpx::read`, `dpx::write`)
//!
//! # Quick Start
//!
//! ```ignore
//! use vfx_io::{read, write};
//!
//! // Read any supported format (auto-detected)
//! let image = read("input.exr")?;
//!
//! // Write to a different format
//! write("output.png", &image)?;
//! ```
//!
//! # Format-Specific Usage
//!
//! ```ignore
//! use vfx_io::dpx::{DpxReader, DpxWriter, DpxWriterOptions, BitDepth};
//!
//! // Read with default options
//! let reader = DpxReader::default();
//! let image = reader.read("scan.0001.dpx")?;
//!
//! // Write with specific bit depth
//! let writer = DpxWriter::with_options(DpxWriterOptions {
//!     bit_depth: BitDepth::Bit10,
//!     ..Default::default()
//! });
//! writer.write("output.0001.dpx", &image)?;
//! ```
//!
//! # Metadata
//!
//! All formats extract metadata into [`Attrs`], a typed attribute container:
//!
//! ```ignore
//! use vfx_io::read;
//!
//! let image = read("photo.jpg")?;
//!
//! // Access metadata
//! if let Some(make) = image.metadata.attrs.get_str("Make") {
//!     println!("Camera: {}", make);
//! }
//! if let Some(iso) = image.metadata.attrs.get_u32("ISO") {
//!     println!("ISO: {}", iso);
//! }
//! ```
//!
//! # Supported Formats
//!
//! | Format | Read | Write | Bit Depths | Features |
//! |--------|------|-------|------------|----------|
//! | EXR | Yes | Yes | 16f, 32f | Layers, compression, metadata |
//! | HDR | Yes | Yes | 32f | RGBE, header metadata |
//! | PNG | Yes | Yes | 8, 16 | Alpha, gamma |
//! | JPEG | Yes | Yes | 8 | Quality setting |
//! | TIFF | Yes | Yes | 8, 16, 32f | LZW, Deflate compression |
//! | DPX | Yes | Yes | 8, 10, 12, 16 | Film metadata, log encoding |
//!
//! # Feature Flags
//!
//! - `exr` - OpenEXR support (default)
//! - `png` - PNG support (default)
//! - `jpeg` - JPEG support (default)
//! - `tiff` - TIFF support (default)
//! - `dpx` - DPX support (default)
//! - `hdr` - Radiance HDR support (default)

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

mod error;
mod traits;
mod detect;

pub mod attrs;
pub mod registry;

#[cfg(feature = "exr")]
pub mod exr;

#[cfg(feature = "png")]
pub mod png;

#[cfg(feature = "jpeg")]
pub mod jpeg;

#[cfg(feature = "tiff")]
pub mod tiff;

#[cfg(feature = "dpx")]
pub mod dpx;

#[cfg(feature = "hdr")]
pub mod hdr;

pub mod sequence;

// Re-exports
pub use error::{IoError, IoResult};
pub use traits::{FormatReader, FormatWriter, ReadSeek, WriteSeek};
pub use detect::Format;
pub use attrs::{Attrs, AttrValue};
pub use registry::{FormatRegistry, FormatInfo, FormatReaderDyn, FormatWriterDyn};

use std::path::Path;

/// Reads an image from a file, auto-detecting the format.
///
/// The format is detected by file extension and magic bytes.
///
/// # Example
///
/// ```ignore
/// use vfx_io::read;
///
/// let image = read("input.exr")?;
/// println!("Size: {}x{}", image.width, image.height);
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be opened
/// - The format is not supported
/// - The file is corrupted
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    let path = path.as_ref();
    let format = Format::detect(path)?;
    
    match format {
        #[cfg(feature = "exr")]
        Format::Exr => exr::read(path),
        
        #[cfg(feature = "png")]
        Format::Png => png::read(path),
        
        #[cfg(feature = "jpeg")]
        Format::Jpeg => jpeg::read(path),
        
        #[cfg(feature = "tiff")]
        Format::Tiff => tiff::read(path),
        
        #[cfg(feature = "dpx")]
        Format::Dpx => dpx::read(path),

        #[cfg(feature = "hdr")]
        Format::Hdr => hdr::read(path),
        
        Format::Unknown => Err(IoError::UnsupportedFormat(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_string()
        )),
    }
}

/// Writes an image to a file, detecting format from extension.
///
/// # Example
///
/// ```ignore
/// use vfx_io::{read, write};
///
/// let image = read("input.exr")?;
/// write("output.png", &image)?;
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be created
/// - The format is not supported for writing
/// - The image data is incompatible with the format
pub fn write<P: AsRef<Path>>(path: P, image: &ImageData) -> IoResult<()> {
    let path = path.as_ref();
    let format = Format::from_extension(path);
    
    match format {
        #[cfg(feature = "exr")]
        Format::Exr => exr::write(path, image),
        
        #[cfg(feature = "png")]
        Format::Png => png::write(path, image),
        
        #[cfg(feature = "jpeg")]
        Format::Jpeg => jpeg::write(path, image),
        
        #[cfg(feature = "tiff")]
        Format::Tiff => tiff::write(path, image),
        
        #[cfg(feature = "dpx")]
        Format::Dpx => dpx::write(path, image),

        #[cfg(feature = "hdr")]
        Format::Hdr => hdr::write(path, image),
        
        Format::Unknown => Err(IoError::UnsupportedFormat(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_string()
        )),
    }
}

/// Image data container for I/O operations.
///
/// This is a format-agnostic container that holds pixel data
/// along with metadata. It can represent various bit depths
/// and channel configurations.
///
/// # Fields
///
/// - `width`, `height` - Image dimensions in pixels
/// - `channels` - Number of channels (3 for RGB, 4 for RGBA)
/// - `format` - Pixel data format (U8, U16, F16, F32, U32)
/// - `data` - Raw pixel data
/// - `metadata` - Format-specific metadata
///
/// # Example
///
/// ```ignore
/// use vfx_io::{ImageData, PixelFormat};
///
/// // Create a 1920x1080 RGB float image
/// let image = ImageData::new(1920, 1080, 3, PixelFormat::F32);
///
/// // Create from existing data
/// let data = vec![0.5f32; 1920 * 1080 * 3];
/// let image = ImageData::from_f32(1920, 1080, 3, data);
/// ```
#[derive(Debug, Clone)]
pub struct ImageData {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Number of channels (3 for RGB, 4 for RGBA).
    pub channels: u32,
    /// Pixel data format.
    pub format: PixelFormat,
    /// Raw pixel data.
    pub data: PixelData,
    /// Image metadata.
    pub metadata: Metadata,
}

/// The sample type stored for a channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelSampleType {
    /// 16-bit float (stored as f32 values in memory).
    F16,
    /// 32-bit float.
    F32,
    /// 32-bit unsigned integer.
    U32,
}

/// Channel sample storage.
#[derive(Debug, Clone)]
pub enum ChannelSamples {
    /// Float sample storage (used for F16 and F32).
    F32(Vec<f32>),
    /// Integer sample storage.
    U32(Vec<u32>),
}

impl ChannelSamples {
    /// Number of samples in this channel.
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            Self::F32(data) => data.len(),
            Self::U32(data) => data.len(),
        }
    }
}

/// A single image channel.
#[derive(Debug, Clone)]
pub struct ImageChannel {
    /// Channel name (e.g., "R", "G", "B", "A", "Z", "ID").
    pub name: String,
    /// The intended sample type for serialization.
    pub sample_type: ChannelSampleType,
    /// Channel samples.
    pub samples: ChannelSamples,
    /// Channel subsampling (x, y).
    pub sampling: (usize, usize),
    /// Whether to quantize linearly (for lossy compression hints).
    pub quantize_linearly: bool,
}

/// A single named image layer with arbitrary channels.
#[derive(Debug, Clone)]
pub struct ImageLayer {
    /// Layer name (e.g., "beauty", "spec", "depth").
    pub name: String,
    /// Layer width in pixels.
    pub width: u32,
    /// Layer height in pixels.
    pub height: u32,
    /// Ordered list of channels in this layer.
    pub channels: Vec<ImageChannel>,
}

/// A multi-layer image container.
#[derive(Debug, Clone, Default)]
pub struct LayeredImage {
    /// Ordered list of layers.
    pub layers: Vec<ImageLayer>,
    /// Image-level metadata.
    pub metadata: Metadata,
}

/// Pixel data format.
///
/// Describes the numeric type and bit depth of pixel values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// 8-bit unsigned integer per channel (0-255).
    U8,
    /// 16-bit unsigned integer per channel (0-65535).
    U16,
    /// 16-bit float per channel (half precision).
    F16,
    /// 32-bit float per channel (full precision).
    F32,
    /// 32-bit unsigned integer per channel.
    U32,
}

/// Raw pixel data storage.
///
/// The variant matches the [`PixelFormat`].
#[derive(Debug, Clone)]
pub enum PixelData {
    /// 8-bit unsigned data.
    U8(Vec<u8>),
    /// 16-bit unsigned data.
    U16(Vec<u16>),
    /// 32-bit float data (also used for F16 after conversion).
    F32(Vec<f32>),
    /// 32-bit unsigned data.
    U32(Vec<u32>),
}

/// Image metadata container.
///
/// Stores both common metadata fields and format-specific attributes.
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    /// Color space name (e.g., "sRGB", "ACEScg", "linear", "log").
    pub colorspace: Option<String>,
    /// Gamma value if applicable.
    pub gamma: Option<f32>,
    /// DPI/PPI for print.
    pub dpi: Option<f32>,
    /// Typed attributes (format-specific).
    pub attrs: Attrs,
}

impl ImageData {
    /// Creates a new ImageData with the given dimensions and format.
    ///
    /// Pixel data is initialized to zero.
    pub fn new(width: u32, height: u32, channels: u32, format: PixelFormat) -> Self {
        let size = (width * height * channels) as usize;
        let data = match format {
            PixelFormat::U8 => PixelData::U8(vec![0u8; size]),
            PixelFormat::U16 => PixelData::U16(vec![0u16; size]),
            PixelFormat::F16 | PixelFormat::F32 => PixelData::F32(vec![0.0f32; size]),
            PixelFormat::U32 => PixelData::U32(vec![0u32; size]),
        };
        
        Self {
            width,
            height,
            channels,
            format,
            data,
            metadata: Metadata::default(),
        }
    }
    
    /// Creates ImageData from f32 pixel data.
    pub fn from_f32(width: u32, height: u32, channels: u32, data: Vec<f32>) -> Self {
        Self {
            width,
            height,
            channels,
            format: PixelFormat::F32,
            data: PixelData::F32(data),
            metadata: Metadata::default(),
        }
    }
    
    /// Creates ImageData from u8 pixel data.
    pub fn from_u8(width: u32, height: u32, channels: u32, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            channels,
            format: PixelFormat::U8,
            data: PixelData::U8(data),
            metadata: Metadata::default(),
        }
    }

    /// Creates ImageData from u32 pixel data.
    pub fn from_u32(width: u32, height: u32, channels: u32, data: Vec<u32>) -> Self {
        Self {
            width,
            height,
            channels,
            format: PixelFormat::U32,
            data: PixelData::U32(data),
            metadata: Metadata::default(),
        }
    }
    
    /// Returns the total number of pixels.
    #[inline]
    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }
    
    /// Returns the total number of samples (pixels * channels).
    #[inline]
    pub fn sample_count(&self) -> usize {
        (self.width * self.height * self.channels) as usize
    }
    
    /// Converts pixel data to f32 (for processing).
    ///
    /// Values are normalized to 0.0-1.0 range for U8/U16; U32 is cast without normalization.
    pub fn to_f32(&self) -> Vec<f32> {
        match &self.data {
            PixelData::U8(data) => data.iter().map(|&v| v as f32 / 255.0).collect(),
            PixelData::U16(data) => data.iter().map(|&v| v as f32 / 65535.0).collect(),
            PixelData::F32(data) => data.clone(),
            PixelData::U32(data) => data.iter().map(|&v| v as f32).collect(),
        }
    }
    
    /// Converts pixel data to u8 (for display/saving).
    ///
    /// Float values are clamped to 0.0-1.0 and scaled to 0-255.
    pub fn to_u8(&self) -> Vec<u8> {
        match &self.data {
            PixelData::U8(data) => data.clone(),
            PixelData::U16(data) => data.iter().map(|&v| (v >> 8) as u8).collect(),
            PixelData::F32(data) => data.iter().map(|&v| (v.clamp(0.0, 1.0) * 255.0) as u8).collect(),
            PixelData::U32(data) => data.iter().map(|&v| v.min(u8::MAX as u32) as u8).collect(),
        }
    }

    /// Converts pixel data to u16 (for 16-bit output).
    ///
    /// Float values are clamped to 0.0-1.0 and scaled to 0-65535.
    pub fn to_u16(&self) -> Vec<u16> {
        match &self.data {
            PixelData::U8(data) => data.iter().map(|&v| (v as u16) << 8 | v as u16).collect(),
            PixelData::U16(data) => data.clone(),
            PixelData::F32(data) => data.iter().map(|&v| (v.clamp(0.0, 1.0) * 65535.0) as u16).collect(),
            PixelData::U32(data) => data.iter().map(|&v| v.min(u16::MAX as u32) as u16).collect(),
        }
    }

    /// Converts this image into a single named layer with planar channels.
    pub fn to_layer(&self, name: impl Into<String>) -> ImageLayer {
        let name = name.into();
        let channel_names = default_channel_names(self.channels as usize);
        let pixel_count = self.pixel_count();
        let channels = self.channels as usize;

        let mut out_channels = Vec::with_capacity(channels);
        for (ch_index, ch_name) in channel_names.into_iter().enumerate() {
            let quantize_linearly = matches!(ch_name.as_str(), "A") || ch_name.starts_with("C");
            match &self.data {
                PixelData::U32(data) => {
                    let mut samples = Vec::with_capacity(pixel_count);
                    for i in 0..pixel_count {
                        let idx = i * channels + ch_index;
                        samples.push(*data.get(idx).unwrap_or(&0u32));
                    }
                    out_channels.push(ImageChannel {
                        name: ch_name,
                        sample_type: ChannelSampleType::U32,
                        samples: ChannelSamples::U32(samples),
                        sampling: (1, 1),
                        quantize_linearly,
                    });
                }
                _ => {
                    let interleaved = self.to_f32();
                    let mut samples = Vec::with_capacity(pixel_count);
                    for i in 0..pixel_count {
                        let idx = i * channels + ch_index;
                        samples.push(interleaved.get(idx).copied().unwrap_or(0.0));
                    }
                    out_channels.push(ImageChannel {
                        name: ch_name,
                        sample_type: ChannelSampleType::F32,
                        samples: ChannelSamples::F32(samples),
                        sampling: (1, 1),
                        quantize_linearly,
                    });
                }
            }
        }

        ImageLayer {
            name,
            width: self.width,
            height: self.height,
            channels: out_channels,
        }
    }

    /// Converts this image into a layered container with a single layer.
    pub fn to_layered(&self, name: impl Into<String>) -> LayeredImage {
        LayeredImage {
            layers: vec![self.to_layer(name)],
            metadata: self.metadata.clone(),
        }
    }
}

impl PixelFormat {
    /// Returns bytes per channel for this format.
    #[inline]
    pub fn bytes_per_channel(&self) -> usize {
        match self {
            Self::U8 => 1,
            Self::U16 | Self::F16 => 2,
            Self::F32 | Self::U32 => 4,
        }
    }
    
    /// Returns true if this is a floating-point format.
    #[inline]
    pub fn is_float(&self) -> bool {
        matches!(self, Self::F16 | Self::F32)
    }
}

impl ImageLayer {
    /// Attempts to convert this layer into a packed ImageData.
    ///
    /// If all channels are U32, the output is U32; otherwise U32 channels are cast to f32.
    pub fn to_image_data(&self) -> IoResult<ImageData> {
        self.to_image_data_with_order(&[])
    }

    /// Attempts to convert this layer into ImageData using an explicit channel order.
    ///
    /// If `order` is empty, a preferred default order is used.
    /// If all channels are U32, the output is U32; otherwise U32 channels are cast to f32.
    pub fn to_image_data_with_order(&self, order: &[&str]) -> IoResult<ImageData> {
        if self.channels.is_empty() {
            return Err(IoError::DecodeError("Layer has no channels".into()));
        }

        let pixel_count = (self.width as usize) * (self.height as usize);
        for channel in &self.channels {
            if channel.sampling != (1, 1) {
                return Err(IoError::DecodeError(
                    "Cannot convert subsampled channels to ImageData".into(),
                ));
            }
            match (&channel.sample_type, &channel.samples) {
                (ChannelSampleType::F16 | ChannelSampleType::F32, ChannelSamples::F32(values)) => {
                    if values.len() != pixel_count {
                        return Err(IoError::DecodeError(format!(
                            "Channel {} has {} samples, expected {}",
                            channel.name,
                            values.len(),
                            pixel_count
                        )));
                    }
                }
                (ChannelSampleType::U32, ChannelSamples::U32(values)) => {
                    if values.len() != pixel_count {
                        return Err(IoError::DecodeError(format!(
                            "Channel {} has {} samples, expected {}",
                            channel.name,
                            values.len(),
                            pixel_count
                        )));
                    }
                }
                _ => {
                    return Err(IoError::DecodeError(
                        "Unsupported channel sample storage".into(),
                    ));
                }
            }
        }

        let order = if order.is_empty() {
            preferred_channel_order(&self.channels)
        } else {
            let mut indices = Vec::with_capacity(order.len());
            for &name in order {
                let idx = self
                    .channels
                    .iter()
                    .position(|ch| ch.name == name)
                    .ok_or_else(|| {
                        IoError::DecodeError(format!(
                            "Channel {} not found in layer",
                            name
                        ))
                    })?;
                indices.push(idx);
            }
            indices
        };
        let all_u32 = order.iter().all(|&idx| matches!(self.channels[idx].sample_type, ChannelSampleType::U32));
        if all_u32 {
            let mut interleaved = Vec::with_capacity(pixel_count * order.len());
            for i in 0..pixel_count {
                for &idx in &order {
                    let channel = &self.channels[idx];
                    let ChannelSamples::U32(values) = &channel.samples else {
                        return Err(IoError::DecodeError(
                            "Unsupported channel sample storage".into(),
                        ));
                    };
                    interleaved.push(values[i]);
                }
            }

            return Ok(ImageData {
                width: self.width,
                height: self.height,
                channels: order.len() as u32,
                format: PixelFormat::U32,
                data: PixelData::U32(interleaved),
                metadata: Metadata::default(),
            });
        }

        let mut interleaved = Vec::with_capacity(pixel_count * order.len());

        for i in 0..pixel_count {
            for &idx in &order {
                let channel = &self.channels[idx];
                match &channel.samples {
                    ChannelSamples::F32(values) => {
                        interleaved.push(values[i]);
                    }
                    ChannelSamples::U32(values) => {
                        interleaved.push(values[i] as f32);
                    }
                }
            }
        }

        Ok(ImageData {
            width: self.width,
            height: self.height,
            channels: order.len() as u32,
            format: PixelFormat::F32,
            data: PixelData::F32(interleaved),
            metadata: Metadata::default(),
        })
    }
}

impl LayeredImage {
    /// Attempts to convert a single-layer image into ImageData.
    pub fn to_image_data(&self) -> IoResult<ImageData> {
        match self.layers.as_slice() {
            [layer] => layer.to_image_data(),
            [] => Err(IoError::DecodeError("No layers available".into())),
            _ => Err(IoError::DecodeError(
                "Multiple layers cannot be converted to ImageData".into(),
            )),
        }
    }

    /// Attempts to convert a single-layer image into ImageData using an explicit channel order.
    pub fn to_image_data_with_order(&self, order: &[&str]) -> IoResult<ImageData> {
        match self.layers.as_slice() {
            [layer] => layer.to_image_data_with_order(order),
            [] => Err(IoError::DecodeError("No layers available".into())),
            _ => Err(IoError::DecodeError(
                "Multiple layers cannot be converted to ImageData".into(),
            )),
        }
    }
}

fn default_channel_names(count: usize) -> Vec<String> {
    match count {
        1 => vec!["Y".to_string()],
        2 => vec!["Y".to_string(), "A".to_string()],
        3 => vec!["R".to_string(), "G".to_string(), "B".to_string()],
        4 => vec![
            "R".to_string(),
            "G".to_string(),
            "B".to_string(),
            "A".to_string(),
        ],
        _ => (0..count).map(|i| format!("C{}", i)).collect(),
    }
}

fn preferred_channel_order(channels: &[ImageChannel]) -> Vec<usize> {
    let mut indices = Vec::with_capacity(channels.len());
    let find = |name: &str| channels.iter().position(|ch| ch.name == name);

    if channels.len() == 1 {
        if let Some(y) = find("Y") {
            return vec![y];
        }
    }
    if channels.len() == 2 {
        if let (Some(y), Some(a)) = (find("Y"), find("A")) {
            return vec![y, a];
        }
    }
    if channels.len() >= 3 {
        if let (Some(r), Some(g), Some(b)) = (find("R"), find("G"), find("B")) {
            indices.push(r);
            indices.push(g);
            indices.push(b);
            if channels.len() == 4 {
                if let Some(a) = find("A") {
                    indices.push(a);
                    return indices;
                }
            }
            if channels.len() == 3 {
                return indices;
            }
            indices.clear();
        }
    }

    (0..channels.len()).collect()
}

// === Backwards compatibility ===
// Keep old metadata module re-exports working

/// Old AttrValue re-export for backwards compatibility.
#[doc(hidden)]
#[deprecated(since = "0.2.0", note = "Use vfx_io::attrs::AttrValue instead")]
pub mod metadata {
    //! Legacy metadata module (deprecated).
    //!
    //! Use `vfx_io::attrs` instead.
    pub use crate::attrs::{Attrs, AttrValue};
}
