//! PNG format support.
//!
//! Provides reading and writing of PNG files - the universal lossless format
//! for web and general-purpose imaging.
//!
//! # Overview
//!
//! PNG (Portable Network Graphics) is a lossless format supporting:
//! - 8-bit and 16-bit per channel
//! - Grayscale, RGB, and RGBA
//! - Alpha channel transparency
//! - Gamma and color profile metadata
//! - Text chunks for metadata
//!
//! # Architecture
//!
//! This module provides two approaches:
//!
//! 1. **Struct + Trait** (recommended for advanced use):
//!    - [`PngReader`] implements [`FormatReader`] for reading
//!    - [`PngWriter`] implements [`FormatWriter`] for writing
//!    - Configure via [`PngReaderOptions`] and [`PngWriterOptions`]
//!
//! 2. **Convenience functions** (simple cases):
//!    - [`read()`] - read with defaults
//!    - [`write()`] - write with defaults
//!
//! # Examples
//!
//! Simple usage:
//! ```ignore
//! use vfx_io::png;
//!
//! let image = png::read("input.png")?;
//! png::write("output.png", &image)?;
//! ```
//!
//! With options:
//! ```ignore
//! use vfx_io::png::{PngWriter, PngWriterOptions, BitDepth, CompressionLevel};
//! use vfx_io::FormatWriter;
//!
//! let writer = PngWriter::with_options(PngWriterOptions {
//!     bit_depth: BitDepth::Sixteen,
//!     compression: CompressionLevel::Best,
//!     ..Default::default()
//! });
//! writer.write("output.png", &image)?;
//! ```

use crate::{AttrValue, FormatReader, FormatWriter, ImageData, IoError, IoResult, Metadata, PixelData, PixelFormat};
use std::io::{BufReader, BufWriter, Cursor};
use std::path::Path;

// ============================================================================
// Bit Depth
// ============================================================================

/// PNG bit depth per channel.
///
/// PNG supports 8-bit (standard) and 16-bit (high precision) modes.
///
/// # When to Use Each
///
/// - **Eight**: Web, general use, compatibility. Smaller files.
/// - **Sixteen**: Scientific imaging, gradients, banding-sensitive work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BitDepth {
    /// 8 bits per channel (0-255). Standard for most use cases.
    #[default]
    Eight,
    /// 16 bits per channel (0-65535). High precision for gradients.
    Sixteen,
}

impl BitDepth {
    /// Convert to png crate's bit depth.
    fn to_png(&self) -> png::BitDepth {
        match self {
            BitDepth::Eight => png::BitDepth::Eight,
            BitDepth::Sixteen => png::BitDepth::Sixteen,
        }
    }
}

// ============================================================================
// Compression Level
// ============================================================================

/// PNG compression level.
///
/// Higher compression = smaller files but slower encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompressionLevel {
    /// Fastest encoding, larger files.
    Fast,
    /// Default balance of speed and size.
    #[default]
    Default,
    /// Maximum compression, slowest encoding.
    Best,
}

impl CompressionLevel {
    /// Convert to png crate's compression type.
    fn to_png(&self) -> png::Compression {
        match self {
            CompressionLevel::Fast => png::Compression::Fast,
            CompressionLevel::Default => png::Compression::Balanced,
            CompressionLevel::Best => png::Compression::High,
        }
    }
}

// ============================================================================
// Reader Options
// ============================================================================

/// Options for reading PNG files.
///
/// Currently minimal - PNG reading is mostly automatic.
///
/// # Example
///
/// ```ignore
/// use vfx_io::png::{PngReader, PngReaderOptions};
/// use vfx_io::FormatReader;
///
/// let reader = PngReader::with_options(PngReaderOptions::default());
/// let image = reader.read("input.png")?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct PngReaderOptions {
    /// Reserved for future use.
    _reserved: (),
}

// ============================================================================
// Writer Options
// ============================================================================

/// Options for writing PNG files.
///
/// Controls bit depth, compression, and color space tagging.
///
/// # Example
///
/// ```ignore
/// use vfx_io::png::{PngWriter, PngWriterOptions, BitDepth, CompressionLevel};
/// use vfx_io::FormatWriter;
///
/// let options = PngWriterOptions {
///     bit_depth: BitDepth::Sixteen,
///     compression: CompressionLevel::Best,
///     ..Default::default()
/// };
/// let writer = PngWriter::with_options(options);
/// writer.write("output.png", &image)?;
/// ```
#[derive(Debug, Clone)]
pub struct PngWriterOptions {
    /// Bit depth per channel. Default: Eight.
    pub bit_depth: BitDepth,
    /// Compression level. Default: Default.
    pub compression: CompressionLevel,
    /// Tag as sRGB. Default: true.
    pub srgb: bool,
}

impl Default for PngWriterOptions {
    fn default() -> Self {
        Self {
            bit_depth: BitDepth::Eight,
            compression: CompressionLevel::Default,
            srgb: true,
        }
    }
}

// ============================================================================
// PngReader
// ============================================================================

/// PNG file reader.
///
/// Implements [`FormatReader`] for reading PNG files with configurable options.
///
/// # Features
///
/// - 8-bit and 16-bit support
/// - Grayscale auto-conversion to RGB
/// - Comprehensive metadata extraction (gamma, ICC, text chunks)
/// - Memory and file reading
///
/// # Example
///
/// ```ignore
/// use vfx_io::png::PngReader;
/// use vfx_io::FormatReader;
///
/// let reader = PngReader::new();
/// let image = reader.read("photo.png")?;
///
/// // Check gamma
/// if let Some(gamma) = image.metadata.gamma {
///     println!("Gamma: {}", gamma);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct PngReader {
    #[allow(dead_code)]
    options: PngReaderOptions,
}

impl PngReader {
    /// Creates a new reader with default options.
    pub fn new() -> Self {
        Self::with_options(PngReaderOptions::default())
    }

    /// Internal read implementation.
    fn read_impl<R: std::io::Read + std::io::Seek>(&self, reader: R) -> IoResult<ImageData> {
        let decoder = png::Decoder::new(BufReader::new(reader));
        let mut reader = decoder
            .read_info()
            .map_err(|e| IoError::DecodeError(e.to_string()))?;

        let buf_size = reader
            .output_buffer_size()
            .ok_or_else(|| IoError::DecodeError("cannot determine buffer size".into()))?;
        let mut buf = vec![0u8; buf_size];
        let output_info = reader
            .next_frame(&mut buf)
            .map_err(|e| IoError::DecodeError(e.to_string()))?;
        let info = reader.info();

        let width = output_info.width;
        let height = output_info.height;

        // Convert to channels/format/data based on color type and bit depth
        let (channels, format, data) = match (output_info.color_type, output_info.bit_depth) {
            (png::ColorType::Rgb, png::BitDepth::Eight) => {
                (3, PixelFormat::U8, PixelData::U8(buf[..output_info.buffer_size()].to_vec()))
            }
            (png::ColorType::Rgba, png::BitDepth::Eight) => {
                (4, PixelFormat::U8, PixelData::U8(buf[..output_info.buffer_size()].to_vec()))
            }
            (png::ColorType::Rgb, png::BitDepth::Sixteen) => {
                let u16_data = bytes_to_u16_be(&buf[..output_info.buffer_size()]);
                (3, PixelFormat::U16, PixelData::U16(u16_data))
            }
            (png::ColorType::Rgba, png::BitDepth::Sixteen) => {
                let u16_data = bytes_to_u16_be(&buf[..output_info.buffer_size()]);
                (4, PixelFormat::U16, PixelData::U16(u16_data))
            }
            (png::ColorType::Grayscale, png::BitDepth::Eight) => {
                // Convert grayscale to RGB
                let rgb: Vec<u8> = buf[..output_info.buffer_size()]
                    .iter()
                    .flat_map(|&g| [g, g, g])
                    .collect();
                (3, PixelFormat::U8, PixelData::U8(rgb))
            }
            (png::ColorType::GrayscaleAlpha, png::BitDepth::Eight) => {
                // Convert grayscale+alpha to RGBA
                let rgba: Vec<u8> = buf[..output_info.buffer_size()]
                    .chunks(2)
                    .flat_map(|ga| [ga[0], ga[0], ga[0], ga[1]])
                    .collect();
                (4, PixelFormat::U8, PixelData::U8(rgba))
            }
            (png::ColorType::Grayscale, png::BitDepth::Sixteen) => {
                // Convert 16-bit grayscale to RGB
                let u16_data = bytes_to_u16_be(&buf[..output_info.buffer_size()]);
                let rgb: Vec<u16> = u16_data.iter().flat_map(|&g| [g, g, g]).collect();
                (3, PixelFormat::U16, PixelData::U16(rgb))
            }
            (png::ColorType::GrayscaleAlpha, png::BitDepth::Sixteen) => {
                // Convert 16-bit grayscale+alpha to RGBA
                let u16_data = bytes_to_u16_be(&buf[..output_info.buffer_size()]);
                let rgba: Vec<u16> = u16_data
                    .chunks(2)
                    .flat_map(|ga| [ga[0], ga[0], ga[0], ga[1]])
                    .collect();
                (4, PixelFormat::U16, PixelData::U16(rgba))
            }
            (color_type, bit_depth) => {
                return Err(IoError::UnsupportedBitDepth(format!(
                    "{:?} {:?}",
                    color_type, bit_depth
                )));
            }
        };

        // Build metadata
        let mut metadata = Metadata::default();
        metadata.colorspace = Some("sRGB".to_string());

        // Basic image info
        metadata.attrs.set("ImageWidth", AttrValue::UInt(width));
        metadata.attrs.set("ImageHeight", AttrValue::UInt(height));
        metadata.attrs.set(
            "ColorType",
            AttrValue::Str(format!("{:?}", info.color_type)),
        );
        metadata.attrs.set(
            "BitDepth",
            AttrValue::UInt(bit_depth_to_u32(info.bit_depth)),
        );

        // Gamma
        if let Some(gamma) = info.gamma() {
            let gamma = gamma.into_value();
            metadata.gamma = Some(gamma);
            metadata.attrs.set("Gamma", AttrValue::Float(gamma));
        }

        // Resolution/DPI
        if let Some(dim) = info.pixel_dims {
            if dim.xppu > 0 && dim.yppu > 0 {
                match dim.unit {
                    png::Unit::Meter => {
                        let x_dpi = (dim.xppu as f64 * 0.0254) as f32;
                        let y_dpi = (dim.yppu as f64 * 0.0254) as f32;
                        metadata.attrs.set("XResolution", AttrValue::Float(x_dpi));
                        metadata.attrs.set("YResolution", AttrValue::Float(y_dpi));
                        metadata.attrs.set("ResolutionUnit", AttrValue::Str("dpi".into()));
                        if (x_dpi - y_dpi).abs() < f32::EPSILON {
                            metadata.dpi = Some(x_dpi);
                        }
                    }
                    png::Unit::Unspecified => {
                        metadata.attrs.set(
                            "PixelAspectRatio",
                            AttrValue::Str(format!("{}:{}", dim.xppu, dim.yppu)),
                        );
                    }
                }
            }
        }

        // sRGB rendering intent
        if let Some(intent) = info.srgb {
            metadata.attrs.set(
                "sRGBRendering",
                AttrValue::Str(format!("{:?}", intent)),
            );
        }

        // ICC profile
        if let Some(icc) = info.icc_profile.as_deref() {
            metadata.attrs.set("ICCProfileSize", AttrValue::UInt(icc.len() as u32));
            if icc.len() >= 20 {
                if let Ok(space) = std::str::from_utf8(&icc[16..20]) {
                    metadata.attrs.set(
                        "ICCColorSpace",
                        AttrValue::Str(space.trim().to_string()),
                    );
                }
            }
        }

        // EXIF data
        if let Some(exif) = info.exif_metadata.as_deref() {
            metadata.attrs.set("ExifSize", AttrValue::UInt(exif.len() as u32));
            if exif.len() <= 65536 {
                metadata.attrs.set("ExifData", AttrValue::Bytes(exif.to_vec()));
            }
        }

        // Text chunks (uncompressed)
        for text in &info.uncompressed_latin1_text {
            let key = format!("Text:{}", text.keyword);
            metadata.attrs.set(key, AttrValue::Str(text.text.clone()));
        }

        // Text chunks (compressed)
        for text in info.compressed_latin1_text.clone() {
            if let Ok(value) = text.get_text() {
                let key = format!("Text:{}", text.keyword);
                metadata.attrs.set(key, AttrValue::Str(value));
            }
        }

        // UTF-8 text chunks
        for text in info.utf8_text.clone() {
            if let Ok(value) = text.get_text() {
                let key = format!("Text:{}", text.keyword);
                metadata.attrs.set(key, AttrValue::Str(value));
            }
        }

        Ok(ImageData {
            width,
            height,
            channels,
            format,
            data,
            metadata,
        })
    }
}

impl Default for PngReader {
    fn default() -> Self {
        Self::new()
    }
}

impl FormatReader<PngReaderOptions> for PngReader {
    /// Returns "PNG".
    fn format_name(&self) -> &'static str {
        "PNG"
    }

    /// Returns `["png"]`.
    fn extensions(&self) -> &'static [&'static str] {
        &["png"]
    }

    /// Checks for PNG magic bytes (0x89, 'P', 'N', 'G').
    fn can_read(&self, header: &[u8]) -> bool {
        // PNG magic: 89 50 4E 47 0D 0A 1A 0A
        header.len() >= 8
            && header[0] == 0x89
            && header[1] == 0x50
            && header[2] == 0x4E
            && header[3] == 0x47
            && header[4] == 0x0D
            && header[5] == 0x0A
            && header[6] == 0x1A
            && header[7] == 0x0A
    }

    /// Reads a PNG file from disk.
    fn read<P: AsRef<Path>>(&self, path: P) -> IoResult<ImageData> {
        let file = std::fs::File::open(path.as_ref())?;
        self.read_impl(file)
    }

    /// Reads a PNG from a byte slice.
    fn read_from_memory(&self, data: &[u8]) -> IoResult<ImageData> {
        self.read_impl(Cursor::new(data))
    }

    /// Creates reader with custom options.
    fn with_options(options: PngReaderOptions) -> Self {
        Self { options }
    }
}

// ============================================================================
// PngWriter
// ============================================================================

/// PNG file writer.
///
/// Implements [`FormatWriter`] for writing PNG files with configurable options.
///
/// # Features
///
/// - 8-bit and 16-bit output
/// - Configurable compression level
/// - sRGB color space tagging
/// - Memory and file writing
///
/// # Example
///
/// ```ignore
/// use vfx_io::png::{PngWriter, PngWriterOptions, BitDepth};
/// use vfx_io::FormatWriter;
///
/// // Write 16-bit PNG
/// let writer = PngWriter::with_options(PngWriterOptions {
///     bit_depth: BitDepth::Sixteen,
///     ..Default::default()
/// });
/// writer.write("output_16bit.png", &image)?;
/// ```
#[derive(Debug, Clone)]
pub struct PngWriter {
    options: PngWriterOptions,
}

impl PngWriter {
    /// Creates a new writer with default options.
    pub fn new() -> Self {
        Self::with_options(PngWriterOptions::default())
    }

    /// Internal write implementation.
    fn write_impl<W: std::io::Write>(&self, writer: W, image: &ImageData) -> IoResult<()> {
        let buf_writer = BufWriter::new(writer);

        let color_type = match image.channels {
            1 => png::ColorType::Grayscale,
            2 => png::ColorType::GrayscaleAlpha,
            3 => png::ColorType::Rgb,
            4 => png::ColorType::Rgba,
            n => return Err(IoError::EncodeError(format!("unsupported channels: {}", n))),
        };

        let mut encoder = png::Encoder::new(buf_writer, image.width, image.height);
        encoder.set_color(color_type);
        encoder.set_depth(self.options.bit_depth.to_png());
        encoder.set_compression(self.options.compression.to_png());

        // Add sRGB chunk if requested or present in metadata
        let srgb_intent = image
            .metadata
            .attrs
            .get("sRGBRendering")
            .and_then(|v| v.as_str())
            .and_then(parse_srgb_intent)
            .or_else(|| {
                if self.options.srgb {
                    Some(png::SrgbRenderingIntent::Perceptual)
                } else {
                    None
                }
            })
            .or_else(|| {
                image
                    .metadata
                    .colorspace
                    .as_ref()
                    .and_then(|cs| cs.eq_ignore_ascii_case("srgb").then_some(png::SrgbRenderingIntent::Perceptual))
            });
        if let Some(intent) = srgb_intent {
            encoder.set_source_srgb(intent);
        }

        if let Some(gamma) = image
            .metadata
            .gamma
            .or_else(|| image.metadata.attrs.get("Gamma").and_then(|v| v.as_f32()))
        {
            encoder.set_source_gamma(png::ScaledFloat::new(gamma));
        }

        if let Some((x_dpi, y_dpi)) = dpi_from_metadata(&image.metadata) {
            let xppu = (x_dpi / 0.0254) as u32;
            let yppu = (y_dpi / 0.0254) as u32;
            if xppu > 0 && yppu > 0 {
                encoder.set_pixel_dims(Some(png::PixelDimensions {
                    xppu,
                    yppu,
                    unit: png::Unit::Meter,
                }));
            }
        }

        for (key, value) in image.metadata.attrs.iter() {
            if let Some(text_key) = key.strip_prefix("Text:") {
                if let AttrValue::Str(text) = value {
                    let _ = encoder.add_text_chunk(text_key.to_string(), text.clone());
                }
            }
        }

        let mut png_writer = encoder
            .write_header()
            .map_err(|e| IoError::EncodeError(e.to_string()))?;

        // Write pixel data based on bit depth
        match self.options.bit_depth {
            BitDepth::Eight => {
                let u8_data = image.to_u8();
                png_writer
                    .write_image_data(&u8_data)
                    .map_err(|e| IoError::EncodeError(e.to_string()))?;
            }
            BitDepth::Sixteen => {
                let u16_data = image.to_u16();
                let bytes = u16_to_bytes_be(&u16_data);
                png_writer
                    .write_image_data(&bytes)
                    .map_err(|e| IoError::EncodeError(e.to_string()))?;
            }
        }

        Ok(())
    }
}

fn dpi_from_metadata(metadata: &Metadata) -> Option<(f32, f32)> {
    let x = metadata
        .attrs
        .get("XResolution")
        .and_then(|v| v.as_f32())
        .or(metadata.dpi);
    let y = metadata
        .attrs
        .get("YResolution")
        .and_then(|v| v.as_f32())
        .or(metadata.dpi);
    match (x, y) {
        (Some(x), Some(y)) => Some((x, y)),
        _ => None,
    }
}

fn parse_srgb_intent(value: &str) -> Option<png::SrgbRenderingIntent> {
    match value.to_lowercase().as_str() {
        "perceptual" => Some(png::SrgbRenderingIntent::Perceptual),
        "relative colorimetric" | "relativecolorimetric" => {
            Some(png::SrgbRenderingIntent::RelativeColorimetric)
        }
        "saturation" => Some(png::SrgbRenderingIntent::Saturation),
        "absolute colorimetric" | "absolutecolorimetric" => {
            Some(png::SrgbRenderingIntent::AbsoluteColorimetric)
        }
        _ => None,
    }
}

impl Default for PngWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl FormatWriter<PngWriterOptions> for PngWriter {
    /// Returns "PNG".
    fn format_name(&self) -> &'static str {
        "PNG"
    }

    /// Returns `["png"]`.
    fn extensions(&self) -> &'static [&'static str] {
        &["png"]
    }

    /// Writes a PNG file to disk.
    fn write<P: AsRef<Path>>(&self, path: P, image: &ImageData) -> IoResult<()> {
        let file = std::fs::File::create(path.as_ref())?;
        self.write_impl(file, image)
    }

    /// Writes a PNG to a byte vector.
    fn write_to_memory(&self, image: &ImageData) -> IoResult<Vec<u8>> {
        let mut buffer = Vec::new();
        self.write_impl(Cursor::new(&mut buffer), image)?;
        Ok(buffer)
    }

    /// Creates writer with custom options.
    fn with_options(options: PngWriterOptions) -> Self {
        Self { options }
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Reads a PNG file with default options.
///
/// Convenience wrapper around [`PngReader`]. For custom options,
/// use [`PngReader::with_options`].
///
/// # Example
///
/// ```ignore
/// use vfx_io::png;
///
/// let image = png::read("photo.png")?;
/// ```
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    PngReader::new().read(path)
}

/// Writes a PNG file with default options (8-bit, default compression).
///
/// Convenience wrapper around [`PngWriter`]. For custom options,
/// use [`PngWriter::with_options`].
///
/// # Example
///
/// ```ignore
/// use vfx_io::png;
///
/// png::write("output.png", &image)?;
/// ```
pub fn write<P: AsRef<Path>>(path: P, image: &ImageData) -> IoResult<()> {
    PngWriter::new().write(path, image)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Converts big-endian bytes to u16 vector.
fn bytes_to_u16_be(bytes: &[u8]) -> Vec<u16> {
    bytes
        .chunks(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect()
}

/// Converts u16 vector to big-endian bytes.
fn u16_to_bytes_be(data: &[u16]) -> Vec<u8> {
    data.iter().flat_map(|v| v.to_be_bytes()).collect()
}

/// Converts png crate bit depth to u32.
fn bit_depth_to_u32(depth: png::BitDepth) -> u32 {
    match depth {
        png::BitDepth::One => 1,
        png::BitDepth::Two => 2,
        png::BitDepth::Four => 4,
        png::BitDepth::Eight => 8,
        png::BitDepth::Sixteen => 16,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests RGB roundtrip.
    #[test]
    fn test_roundtrip_rgb() {
        let width = 32;
        let height = 32;
        let mut data = Vec::with_capacity((width * height * 3) as usize);

        for y in 0..height {
            for x in 0..width {
                data.push((x * 8) as u8);
                data.push((y * 8) as u8);
                data.push(128);
            }
        }

        let image = ImageData::from_u8(width, height, 3, data);
        let temp_path = std::env::temp_dir().join("vfx_io_png_rgb_test.png");

        write(&temp_path, &image).expect("Write failed");
        let loaded = read(&temp_path).expect("Read failed");

        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
        assert_eq!(loaded.channels, 3);

        let _ = std::fs::remove_file(&temp_path);
    }

    /// Tests RGBA roundtrip.
    #[test]
    fn test_roundtrip_rgba() {
        let width = 16;
        let height = 16;
        let mut data = Vec::with_capacity((width * height * 4) as usize);

        for y in 0..height {
            for x in 0..width {
                data.push((x * 16) as u8);
                data.push((y * 16) as u8);
                data.push(64);
                data.push(255);
            }
        }

        let image = ImageData::from_u8(width, height, 4, data);
        let temp_path = std::env::temp_dir().join("vfx_io_png_rgba_test.png");

        write(&temp_path, &image).expect("Write failed");
        let loaded = read(&temp_path).expect("Read failed");

        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
        assert_eq!(loaded.channels, 4);

        let _ = std::fs::remove_file(&temp_path);
    }

    /// Tests 16-bit output.
    #[test]
    fn test_16bit_write() {
        let image = ImageData::from_u8(8, 8, 3, vec![128; 8 * 8 * 3]);
        let temp_path = std::env::temp_dir().join("vfx_io_png_16bit_test.png");

        let writer = PngWriter::with_options(PngWriterOptions {
            bit_depth: BitDepth::Sixteen,
            ..Default::default()
        });
        writer.write(&temp_path, &image).expect("Write failed");

        let loaded = read(&temp_path).expect("Read failed");
        assert_eq!(loaded.format, PixelFormat::U16);

        let _ = std::fs::remove_file(&temp_path);
    }

    /// Tests memory roundtrip.
    #[test]
    fn test_memory_roundtrip() {
        let image = ImageData::from_u8(16, 16, 4, vec![200; 16 * 16 * 4]);

        let writer = PngWriter::new();
        let bytes = writer.write_to_memory(&image).expect("Write failed");

        let reader = PngReader::new();
        let loaded = reader.read_from_memory(&bytes).expect("Read failed");

        assert_eq!(loaded.width, 16);
        assert_eq!(loaded.height, 16);
    }

    /// Tests magic byte detection.
    #[test]
    fn test_can_read() {
        let reader = PngReader::new();

        // Valid PNG magic
        assert!(reader.can_read(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]));

        // Invalid
        assert!(!reader.can_read(&[0x76, 0x2F, 0x31, 0x01])); // EXR
        assert!(!reader.can_read(&[0xFF, 0xD8, 0xFF])); // JPEG
    }
}
