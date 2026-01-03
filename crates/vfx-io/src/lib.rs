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
//! - [`ImageReader`] - Trait for format readers
//! - [`ImageWriter`] - Trait for format writers
//! - [`read`] / [`write`] - High-level functions with format auto-detection
//!
//! # Quick Start
//!
//! ```rust,ignore
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
//! ```rust,ignore
//! use vfx_io::exr::{ExrReader, ExrWriter};
//!
//! // Read with specific options
//! let reader = ExrReader::new()
//!     .with_layer("beauty");
//! let image = reader.read("render.exr")?;
//!
//! // Write with compression
//! let writer = ExrWriter::new()
//!     .with_compression(Compression::Zip);
//! writer.write("output.exr", &image)?;
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
//! # Dependencies
//!
//! - [`vfx-core`] - Core image types
//! - [`exr`] - OpenEXR support
//! - [`png`] - PNG support
//! - [`jpeg-decoder`] - JPEG decoding
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
pub mod metadata;

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

pub use error::{IoError, IoResult};
pub use traits::{ImageReader, ImageWriter};
pub use detect::Format;
pub use metadata::{Attrs, AttrValue};

use std::path::Path;

/// Reads an image from a file, auto-detecting the format.
///
/// The format is detected by file extension and magic bytes.
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::read;
///
/// let image = read("input.exr")?;
/// println!("Size: {}x{}", image.width(), image.height());
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
/// ```rust,ignore
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
    /// Optional metadata.
    pub metadata: Metadata,
}

/// Pixel data format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// 8-bit unsigned integer per channel.
    U8,
    /// 16-bit unsigned integer per channel.
    U16,
    /// 16-bit float per channel.
    F16,
    /// 32-bit float per channel.
    F32,
}

/// Raw pixel data storage.
#[derive(Debug, Clone)]
pub enum PixelData {
    /// 8-bit unsigned data.
    U8(Vec<u8>),
    /// 16-bit unsigned data.
    U16(Vec<u16>),
    /// 32-bit float data (also used for f16 after conversion).
    F32(Vec<f32>),
}

/// Image metadata.
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    /// Color space name (e.g., "sRGB", "ACEScg").
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
    pub fn new(width: u32, height: u32, channels: u32, format: PixelFormat) -> Self {
        let size = (width * height * channels) as usize;
        let data = match format {
            PixelFormat::U8 => PixelData::U8(vec![0u8; size]),
            PixelFormat::U16 => PixelData::U16(vec![0u16; size]),
            PixelFormat::F16 | PixelFormat::F32 => PixelData::F32(vec![0.0f32; size]),
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
    
    /// Returns the total number of pixels.
    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }
    
    /// Returns the total number of samples (pixels * channels).
    pub fn sample_count(&self) -> usize {
        (self.width * self.height * self.channels) as usize
    }
    
    /// Converts pixel data to f32 (for processing).
    pub fn to_f32(&self) -> Vec<f32> {
        match &self.data {
            PixelData::U8(data) => data.iter().map(|&v| v as f32 / 255.0).collect(),
            PixelData::U16(data) => data.iter().map(|&v| v as f32 / 65535.0).collect(),
            PixelData::F32(data) => data.clone(),
        }
    }
    
    /// Converts pixel data to u8 (for display/saving).
    pub fn to_u8(&self) -> Vec<u8> {
        match &self.data {
            PixelData::U8(data) => data.clone(),
            PixelData::U16(data) => data.iter().map(|&v| (v >> 8) as u8).collect(),
            PixelData::F32(data) => data.iter().map(|&v| (v.clamp(0.0, 1.0) * 255.0) as u8).collect(),
        }
    }
}

impl PixelFormat {
    /// Returns bytes per channel for this format.
    pub fn bytes_per_channel(&self) -> usize {
        match self {
            Self::U8 => 1,
            Self::U16 | Self::F16 => 2,
            Self::F32 => 4,
        }
    }
    
    /// Returns true if this is a floating-point format.
    pub fn is_float(&self) -> bool {
        matches!(self, Self::F16 | Self::F32)
    }
}
