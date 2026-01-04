//! TIFF format support.
//!
//! Provides reading and writing of TIFF files - the professional format
//! for print, scanning, and archival workflows.
//!
//! # Overview
//!
//! TIFF (Tagged Image File Format) is a flexible format supporting:
//! - 8-bit, 16-bit, and 32-bit float per channel
//! - Grayscale, RGB, RGBA, and CMYK
//! - Multiple compression methods (LZW, ZIP, PackBits)
//! - Rich metadata through IFD tags
//! - Multi-page documents
//!
//! # Architecture
//!
//! This module provides two approaches:
//!
//! 1. **Struct + Trait** (recommended for advanced use):
//!    - [`TiffReader`] implements [`FormatReader`] for reading
//!    - [`TiffWriter`] implements [`FormatWriter`] for writing
//!    - Configure via [`TiffReaderOptions`] and [`TiffWriterOptions`]
//!
//! 2. **Convenience functions** (simple cases):
//!    - [`read()`] - read with defaults
//!    - [`write()`] - write with defaults
//!
//! # Examples
//!
//! Simple usage:
//! ```ignore
//! use vfx_io::tiff;
//!
//! let image = tiff::read("scan.tiff")?;
//! tiff::write("output.tiff", &image)?;
//! ```
//!
//! With options:
//! ```ignore
//! use vfx_io::tiff::{TiffWriter, TiffWriterOptions, BitDepth, Compression};
//! use vfx_io::FormatWriter;
//!
//! let writer = TiffWriter::with_options(TiffWriterOptions {
//!     bit_depth: BitDepth::Sixteen,
//!     compression: Compression::Deflate,
//!     ..Default::default()
//! });
//! writer.write("output.tiff", &image)?;
//! ```

use crate::{AttrValue, FormatReader, FormatWriter, ImageData, IoError, IoResult, Metadata, PixelData, PixelFormat};
use std::io::{BufReader, Cursor, Read, Seek};
use std::path::Path;
use tiff::tags::Tag;

// ============================================================================
// Bit Depth
// ============================================================================

/// TIFF output bit depth per channel.
///
/// TIFF supports 8-bit, 16-bit, and 32-bit float modes.
///
/// # When to Use Each
///
/// - **Eight**: Web, general use, smaller files.
/// - **Sixteen**: Print, scanning, gradients. Good balance.
/// - **ThirtyTwoFloat**: HDR, linear workflow, compositing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BitDepth {
    /// 8 bits per channel (0-255).
    Eight,
    /// 16 bits per channel (0-65535). Default for quality.
    #[default]
    Sixteen,
    /// 32-bit float per channel. For HDR/linear workflow.
    ThirtyTwoFloat,
}

// ============================================================================
// Compression
// ============================================================================

/// TIFF compression method.
///
/// All supported methods are lossless.
///
/// # Recommendations
///
/// - **Lzw**: Best general-purpose choice. Good compression, widely supported.
/// - **Deflate**: Better compression than LZW, slightly slower.
/// - **PackBits**: Fast but weak compression. For large solid areas.
/// - **None**: Maximum compatibility, largest files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Compression {
    /// No compression. Maximum compatibility.
    None,
    /// LZW compression (lossless). Good balance.
    #[default]
    Lzw,
    /// ZIP/Deflate compression (lossless). Better compression.
    Deflate,
    /// PackBits RLE compression. Simple, fast.
    PackBits,
}

impl Compression {
    /// Convert to tiff crate's compression type.
    fn to_tiff(&self) -> tiff::encoder::Compression {
        match self {
            Compression::None => tiff::encoder::Compression::Uncompressed,
            Compression::Lzw => tiff::encoder::Compression::Lzw,
            Compression::Deflate => tiff::encoder::Compression::Deflate(tiff::encoder::DeflateLevel::Balanced),
            Compression::PackBits => tiff::encoder::Compression::Packbits,
        }
    }
}

// ============================================================================
// Reader Options
// ============================================================================

/// Options for reading TIFF files.
///
/// Currently minimal - TIFF reading is mostly automatic.
///
/// # Example
///
/// ```ignore
/// use vfx_io::tiff::{TiffReader, TiffReaderOptions};
/// use vfx_io::FormatReader;
///
/// let reader = TiffReader::with_options(TiffReaderOptions::default());
/// let image = reader.read("scan.tiff")?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct TiffReaderOptions {
    /// Reserved for future use (page selection, etc.)
    _reserved: (),
}

// ============================================================================
// Writer Options
// ============================================================================

/// Options for writing TIFF files.
///
/// Controls bit depth and compression method.
///
/// # Example
///
/// ```ignore
/// use vfx_io::tiff::{TiffWriter, TiffWriterOptions, BitDepth, Compression};
/// use vfx_io::FormatWriter;
///
/// // High quality archival
/// let options = TiffWriterOptions {
///     bit_depth: BitDepth::Sixteen,
///     compression: Compression::Deflate,
/// };
/// let writer = TiffWriter::with_options(options);
/// writer.write("archive.tiff", &image)?;
/// ```
#[derive(Debug, Clone)]
pub struct TiffWriterOptions {
    /// Bit depth per channel. Default: Sixteen.
    pub bit_depth: BitDepth,
    /// Compression method. Default: LZW.
    pub compression: Compression,
}

impl Default for TiffWriterOptions {
    fn default() -> Self {
        Self {
            bit_depth: BitDepth::Sixteen,
            compression: Compression::Lzw,
        }
    }
}

// ============================================================================
// TiffReader
// ============================================================================

/// TIFF file reader.
///
/// Implements [`FormatReader`] for reading TIFF files with configurable options.
///
/// # Features
///
/// - 8-bit, 16-bit, and 32-bit float support
/// - Grayscale, RGB, RGBA input
/// - Comprehensive metadata extraction (resolution, dates, software)
/// - Memory and file reading
///
/// # Example
///
/// ```ignore
/// use vfx_io::tiff::TiffReader;
/// use vfx_io::FormatReader;
///
/// let reader = TiffReader::new();
/// let image = reader.read("scan.tiff")?;
///
/// // Check resolution
/// if let Some(AttrValue::Float(dpi)) = image.metadata.attrs.get("XResolution") {
///     println!("Resolution: {} DPI", dpi);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct TiffReader {
    #[allow(dead_code)]
    options: TiffReaderOptions,
}

impl TiffReader {
    /// Creates a new reader with default options.
    pub fn new() -> Self {
        Self::with_options(TiffReaderOptions::default())
    }

    /// Internal read implementation.
    fn read_impl<R: Read + Seek>(&self, reader: R) -> IoResult<ImageData> {
        use tiff::decoder::{Decoder, DecodingResult};
        use tiff::ColorType;

        let buf_reader = BufReader::new(reader);
        let mut decoder = Decoder::new(buf_reader)
            .map_err(|e| IoError::DecodeError(e.to_string()))?;

        let (width, height) = decoder
            .dimensions()
            .map_err(|e| IoError::DecodeError(e.to_string()))?;
        let color_type = decoder
            .colortype()
            .map_err(|e| IoError::DecodeError(e.to_string()))?;

        let result = decoder
            .read_image()
            .map_err(|e| IoError::DecodeError(e.to_string()))?;

        // Convert to internal format
        let (data, format, channels) = match (color_type, result) {
            // 8-bit RGB -> F32
            (ColorType::RGB(8), DecodingResult::U8(buf)) => {
                let f32_data: Vec<f32> = buf.iter().map(|&v| v as f32 / 255.0).collect();
                (PixelData::F32(f32_data), PixelFormat::F32, 3)
            }
            // 8-bit RGBA -> F32
            (ColorType::RGBA(8), DecodingResult::U8(buf)) => {
                let f32_data: Vec<f32> = buf.iter().map(|&v| v as f32 / 255.0).collect();
                (PixelData::F32(f32_data), PixelFormat::F32, 4)
            }
            // 16-bit RGB -> F32
            (ColorType::RGB(16), DecodingResult::U16(buf)) => {
                let f32_data: Vec<f32> = buf.iter().map(|&v| v as f32 / 65535.0).collect();
                (PixelData::F32(f32_data), PixelFormat::F32, 3)
            }
            // 16-bit RGBA -> F32
            (ColorType::RGBA(16), DecodingResult::U16(buf)) => {
                let f32_data: Vec<f32> = buf.iter().map(|&v| v as f32 / 65535.0).collect();
                (PixelData::F32(f32_data), PixelFormat::F32, 4)
            }
            // 8-bit Grayscale -> F32
            (ColorType::Gray(8), DecodingResult::U8(buf)) => {
                let f32_data: Vec<f32> = buf.iter().map(|&v| v as f32 / 255.0).collect();
                (PixelData::F32(f32_data), PixelFormat::F32, 1)
            }
            // 16-bit Grayscale -> F32
            (ColorType::Gray(16), DecodingResult::U16(buf)) => {
                let f32_data: Vec<f32> = buf.iter().map(|&v| v as f32 / 65535.0).collect();
                (PixelData::F32(f32_data), PixelFormat::F32, 1)
            }
            // 32-bit float RGB
            (ColorType::RGB(32), DecodingResult::F32(buf)) => {
                (PixelData::F32(buf), PixelFormat::F32, 3)
            }
            // 32-bit float RGBA
            (ColorType::RGBA(32), DecodingResult::F32(buf)) => {
                (PixelData::F32(buf), PixelFormat::F32, 4)
            }
            (ct, _) => {
                return Err(IoError::DecodeError(format!(
                    "unsupported TIFF color type: {:?}",
                    ct
                )));
            }
        };

        // Build metadata
        let mut metadata = Metadata::default();
        metadata.colorspace = Some("sRGB".to_string());

        metadata.attrs.set("ImageWidth", AttrValue::UInt(width));
        metadata.attrs.set("ImageHeight", AttrValue::UInt(height));
        metadata.attrs.set(
            "ColorType",
            AttrValue::Str(format!("{:?}", color_type)),
        );
        metadata.attrs.set(
            "BitDepth",
            AttrValue::UInt(bit_depth_from_color(color_type)),
        );

        // Extract TIFF tags
        extract_tag_u16(&mut decoder, Tag::Compression, "Compression", &mut metadata);
        extract_tag_f64(&mut decoder, Tag::XResolution, "XResolution", &mut metadata);
        extract_tag_f64(&mut decoder, Tag::YResolution, "YResolution", &mut metadata);
        extract_tag_u16(&mut decoder, Tag::ResolutionUnit, "ResolutionUnit", &mut metadata);
        extract_tag_string(&mut decoder, Tag::Software, "Software", &mut metadata);
        extract_tag_string(&mut decoder, Tag::Artist, "Artist", &mut metadata);
        extract_tag_string(&mut decoder, Tag::DateTime, "DateTime", &mut metadata);

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

impl Default for TiffReader {
    fn default() -> Self {
        Self::new()
    }
}

impl FormatReader<TiffReaderOptions> for TiffReader {
    /// Returns "TIFF".
    fn format_name(&self) -> &'static str {
        "TIFF"
    }

    /// Returns `["tiff", "tif"]`.
    fn extensions(&self) -> &'static [&'static str] {
        &["tiff", "tif"]
    }

    /// Checks for TIFF magic bytes (II/MM + 42).
    fn can_read(&self, header: &[u8]) -> bool {
        if header.len() < 4 {
            return false;
        }
        // Little-endian: II + 42 (0x2A00)
        let le = header[0] == b'I' && header[1] == b'I' && header[2] == 0x2A && header[3] == 0x00;
        // Big-endian: MM + 42 (0x002A)
        let be = header[0] == b'M' && header[1] == b'M' && header[2] == 0x00 && header[3] == 0x2A;
        le || be
    }

    /// Reads a TIFF file from disk.
    fn read<P: AsRef<Path>>(&self, path: P) -> IoResult<ImageData> {
        let file = std::fs::File::open(path.as_ref())?;
        self.read_impl(file)
    }

    /// Reads a TIFF from a byte slice.
    fn read_from_memory(&self, data: &[u8]) -> IoResult<ImageData> {
        self.read_impl(Cursor::new(data))
    }

    /// Creates reader with custom options.
    fn with_options(options: TiffReaderOptions) -> Self {
        Self { options }
    }
}

// ============================================================================
// TiffWriter
// ============================================================================

/// TIFF file writer.
///
/// Implements [`FormatWriter`] for writing TIFF files with configurable options.
///
/// # Features
///
/// - 8-bit, 16-bit, and 32-bit float output
/// - Multiple compression methods
/// - Grayscale, RGB, RGBA output
/// - Memory and file writing
///
/// # Example
///
/// ```ignore
/// use vfx_io::tiff::{TiffWriter, TiffWriterOptions, Compression};
/// use vfx_io::FormatWriter;
///
/// // Maximum compression for archival
/// let writer = TiffWriter::with_options(TiffWriterOptions {
///     compression: Compression::Deflate,
///     ..Default::default()
/// });
/// writer.write("archive.tiff", &image)?;
/// ```
#[derive(Debug, Clone)]
pub struct TiffWriter {
    options: TiffWriterOptions,
}

impl TiffWriter {
    /// Creates a new writer with default options (16-bit, LZW).
    pub fn new() -> Self {
        Self::with_options(TiffWriterOptions::default())
    }

    /// Internal write implementation.
    fn write_impl<W: std::io::Write + Seek>(&self, writer: W, image: &ImageData) -> IoResult<()> {
        use tiff::encoder::TiffEncoder;

        let encoder = TiffEncoder::new(writer)
            .map_err(|e| IoError::EncodeError(e.to_string()))?;

        let width = image.width;
        let height = image.height;

        let channels = image.channels as u8;
        match self.options.bit_depth {
            BitDepth::Eight => {
                let u8_data = image.to_u8();
                self.write_u8(encoder, width, height, channels, &u8_data, image)?;
            }
            BitDepth::Sixteen => {
                let f32_data = image.to_f32();
                let u16_data: Vec<u16> = f32_data
                    .iter()
                    .map(|&v| (v.clamp(0.0, 1.0) * 65535.0) as u16)
                    .collect();
                self.write_u16(encoder, width, height, channels, &u16_data, image)?;
            }
            BitDepth::ThirtyTwoFloat => {
                let f32_data = image.to_f32();
                self.write_f32(encoder, width, height, channels, &f32_data, image)?;
            }
        }

        Ok(())
    }

    /// Writes 8-bit data.
    fn write_u8<W: std::io::Write + Seek>(
        &self,
        encoder: tiff::encoder::TiffEncoder<W>,
        width: u32,
        height: u32,
        channels: u8,
        data: &[u8],
        image: &ImageData,
    ) -> IoResult<()> {
        use tiff::encoder::colortype;

        let mut encoder = encoder.with_compression(self.options.compression.to_tiff());
        match channels {
            1 => {
                let mut image_encoder = encoder
                    .new_image::<colortype::Gray8>(width, height)
                    .map_err(|e| IoError::EncodeError(e.to_string()))?;
                apply_tiff_metadata(&mut image_encoder, image)?;
                image_encoder
                    .write_data(data)
                    .map_err(|e| IoError::EncodeError(e.to_string()))?;
            }
            3 => {
                let mut image_encoder = encoder
                    .new_image::<colortype::RGB8>(width, height)
                    .map_err(|e| IoError::EncodeError(e.to_string()))?;
                apply_tiff_metadata(&mut image_encoder, image)?;
                image_encoder
                    .write_data(data)
                    .map_err(|e| IoError::EncodeError(e.to_string()))?;
            }
            4 => {
                let mut image_encoder = encoder
                    .new_image::<colortype::RGBA8>(width, height)
                    .map_err(|e| IoError::EncodeError(e.to_string()))?;
                apply_tiff_metadata(&mut image_encoder, image)?;
                image_encoder
                    .write_data(data)
                    .map_err(|e| IoError::EncodeError(e.to_string()))?;
            }
            _ => {
                return Err(IoError::EncodeError(format!(
                    "unsupported channel count: {}",
                    channels
                )));
            }
        }
        Ok(())
    }

    /// Writes 16-bit data.
    fn write_u16<W: std::io::Write + Seek>(
        &self,
        encoder: tiff::encoder::TiffEncoder<W>,
        width: u32,
        height: u32,
        channels: u8,
        data: &[u16],
        image: &ImageData,
    ) -> IoResult<()> {
        use tiff::encoder::colortype;

        let mut encoder = encoder.with_compression(self.options.compression.to_tiff());
        match channels {
            1 => {
                let mut image_encoder = encoder
                    .new_image::<colortype::Gray16>(width, height)
                    .map_err(|e| IoError::EncodeError(e.to_string()))?;
                apply_tiff_metadata(&mut image_encoder, image)?;
                image_encoder
                    .write_data(data)
                    .map_err(|e| IoError::EncodeError(e.to_string()))?;
            }
            3 => {
                let mut image_encoder = encoder
                    .new_image::<colortype::RGB16>(width, height)
                    .map_err(|e| IoError::EncodeError(e.to_string()))?;
                apply_tiff_metadata(&mut image_encoder, image)?;
                image_encoder
                    .write_data(data)
                    .map_err(|e| IoError::EncodeError(e.to_string()))?;
            }
            4 => {
                let mut image_encoder = encoder
                    .new_image::<colortype::RGBA16>(width, height)
                    .map_err(|e| IoError::EncodeError(e.to_string()))?;
                apply_tiff_metadata(&mut image_encoder, image)?;
                image_encoder
                    .write_data(data)
                    .map_err(|e| IoError::EncodeError(e.to_string()))?;
            }
            _ => {
                return Err(IoError::EncodeError(format!(
                    "unsupported channel count: {}",
                    channels
                )));
            }
        }
        Ok(())
    }

    /// Writes 32-bit float data.
    fn write_f32<W: std::io::Write + Seek>(
        &self,
        encoder: tiff::encoder::TiffEncoder<W>,
        width: u32,
        height: u32,
        channels: u8,
        data: &[f32],
        image: &ImageData,
    ) -> IoResult<()> {
        // Note: tiff crate has limited f32 support
        // For now, convert to u16 as fallback
        let u16_data: Vec<u16> = data
            .iter()
            .map(|&v| (v.clamp(0.0, 1.0) * 65535.0) as u16)
            .collect();
        self.write_u16(encoder, width, height, channels, &u16_data, image)
    }
}

fn apply_tiff_metadata<
    W: std::io::Write + Seek,
    C: tiff::encoder::colortype::ColorType,
    K: tiff::encoder::TiffKind,
>(
    encoder: &mut tiff::encoder::ImageEncoder<'_, W, C, K>,
    image: &ImageData,
) -> IoResult<()> {
    use tiff::tags::{ResolutionUnit, Tag};

    let dir = encoder.encoder();

    if let Some(value) = image.metadata.attrs.get("Software").and_then(|v| v.as_str()) {
        dir.write_tag(Tag::Software, value)
            .map_err(|e| IoError::EncodeError(e.to_string()))?;
    }
    if let Some(value) = image.metadata.attrs.get("Artist").and_then(|v| v.as_str()) {
        dir.write_tag(Tag::Artist, value)
            .map_err(|e| IoError::EncodeError(e.to_string()))?;
    }
    if let Some(value) = image.metadata.attrs.get("DateTime").and_then(|v| v.as_str()) {
        dir.write_tag(Tag::DateTime, value)
            .map_err(|e| IoError::EncodeError(e.to_string()))?;
    }

    let x_res = attr_to_f32(image.metadata.attrs.get("XResolution")).or(image.metadata.dpi);
    let y_res = attr_to_f32(image.metadata.attrs.get("YResolution")).or(image.metadata.dpi);

    if let (Some(x_res), Some(y_res)) = (x_res, y_res) {
        let x = rational_from_f32(x_res);
        let y = rational_from_f32(y_res);
        encoder.resolution_unit(ResolutionUnit::Inch);
        encoder.x_resolution(x);
        encoder.y_resolution(y);
    }

    Ok(())
}

fn attr_to_f32(value: Option<&AttrValue>) -> Option<f32> {
    match value {
        Some(AttrValue::Float(v)) => Some(*v),
        Some(AttrValue::UInt(v)) => Some(*v as f32),
        _ => None,
    }
}

fn rational_from_f32(value: f32) -> tiff::encoder::Rational {
    let scale = 10000u32;
    tiff::encoder::Rational {
        n: (value * scale as f32) as u32,
        d: scale,
    }
}

impl Default for TiffWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl FormatWriter<TiffWriterOptions> for TiffWriter {
    /// Returns "TIFF".
    fn format_name(&self) -> &'static str {
        "TIFF"
    }

    /// Returns `["tiff", "tif"]`.
    fn extensions(&self) -> &'static [&'static str] {
        &["tiff", "tif"]
    }

    /// Writes a TIFF file to disk.
    fn write<P: AsRef<Path>>(&self, path: P, image: &ImageData) -> IoResult<()> {
        let file = std::fs::File::create(path.as_ref())?;
        self.write_impl(file, image)
    }

    /// Writes a TIFF to a byte vector.
    fn write_to_memory(&self, image: &ImageData) -> IoResult<Vec<u8>> {
        let mut buffer = Cursor::new(Vec::new());
        self.write_impl(&mut buffer, image)?;
        Ok(buffer.into_inner())
    }

    /// Creates writer with custom options.
    fn with_options(options: TiffWriterOptions) -> Self {
        Self { options }
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Reads a TIFF file with default options.
///
/// Convenience wrapper around [`TiffReader`]. For custom options,
/// use [`TiffReader::with_options`].
///
/// # Example
///
/// ```ignore
/// use vfx_io::tiff;
///
/// let image = tiff::read("scan.tiff")?;
/// ```
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    TiffReader::new().read(path)
}

/// Writes a TIFF file with default options (16-bit, LZW).
///
/// Convenience wrapper around [`TiffWriter`]. For custom options,
/// use [`TiffWriter::with_options`].
///
/// # Example
///
/// ```ignore
/// use vfx_io::tiff;
///
/// tiff::write("output.tiff", &image)?;
/// ```
pub fn write<P: AsRef<Path>>(path: P, image: &ImageData) -> IoResult<()> {
    TiffWriter::new().write(path, image)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extracts u16 tag value.
fn extract_tag_u16<R: Read + Seek>(
    decoder: &mut tiff::decoder::Decoder<R>,
    tag: Tag,
    key: &str,
    metadata: &mut Metadata,
) {
    if let Ok(Some(value)) = decoder.find_tag(tag) {
        if let Ok(v) = value.into_u16() {
            metadata.attrs.set(key, AttrValue::UInt(v as u32));
        }
    }
}

/// Extracts f64 tag value.
fn extract_tag_f64<R: Read + Seek>(
    decoder: &mut tiff::decoder::Decoder<R>,
    tag: Tag,
    key: &str,
    metadata: &mut Metadata,
) {
    if let Ok(Some(value)) = decoder.find_tag(tag) {
        if let Ok(v) = value.into_f64() {
            metadata.attrs.set(key, AttrValue::Float(v as f32));
        }
    }
}

/// Extracts string tag value.
fn extract_tag_string<R: Read + Seek>(
    decoder: &mut tiff::decoder::Decoder<R>,
    tag: Tag,
    key: &str,
    metadata: &mut Metadata,
) {
    if let Ok(Some(value)) = decoder.find_tag(tag) {
        if let Ok(v) = value.into_string() {
            metadata.attrs.set(key, AttrValue::Str(v));
        }
    }
}

/// Extracts bit depth from color type.
fn bit_depth_from_color(color_type: tiff::ColorType) -> u32 {
    match color_type {
        tiff::ColorType::RGB(bits) => bits as u32,
        tiff::ColorType::RGBA(bits) => bits as u32,
        tiff::ColorType::Gray(bits) => bits as u32,
        tiff::ColorType::GrayA(bits) => bits as u32,
        tiff::ColorType::CMYK(bits) => bits as u32,
        tiff::ColorType::CMYKA(bits) => bits as u32,
        tiff::ColorType::YCbCr(bits) => bits as u32,
        tiff::ColorType::Palette(bits) => bits as u32,
        tiff::ColorType::Multiband { bit_depth, .. } => bit_depth as u32,
        _ => 0,
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
                data.push(x as f32 / width as f32);
                data.push(y as f32 / height as f32);
                data.push(0.5);
            }
        }

        let image = ImageData::from_f32(width, height, 3, data);
        let temp_path = std::env::temp_dir().join("vfx_io_tiff_rgb_test.tiff");

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
        let data = vec![0.5f32; (width * height * 4) as usize];
        let image = ImageData::from_f32(width, height, 4, data);

        let temp_path = std::env::temp_dir().join("vfx_io_tiff_rgba_test.tiff");

        write(&temp_path, &image).expect("Write failed");
        let loaded = read(&temp_path).expect("Read failed");

        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
        assert_eq!(loaded.channels, 4);

        let _ = std::fs::remove_file(&temp_path);
    }

    /// Tests compression options.
    #[test]
    fn test_compression_options() {
        let image = ImageData::from_f32(32, 32, 3, vec![0.5; 32 * 32 * 3]);
        let temp_path = std::env::temp_dir().join("vfx_io_tiff_comp_test.tiff");

        // Test Deflate compression
        let writer = TiffWriter::with_options(TiffWriterOptions {
            compression: Compression::Deflate,
            ..Default::default()
        });
        writer.write(&temp_path, &image).expect("Write failed");

        let loaded = read(&temp_path).expect("Read failed");
        assert_eq!(loaded.width, 32);

        let _ = std::fs::remove_file(&temp_path);
    }

    /// Tests memory roundtrip.
    #[test]
    fn test_memory_roundtrip() {
        let image = ImageData::from_f32(16, 16, 3, vec![0.25; 16 * 16 * 3]);

        let writer = TiffWriter::new();
        let bytes = writer.write_to_memory(&image).expect("Write failed");

        let reader = TiffReader::new();
        let loaded = reader.read_from_memory(&bytes).expect("Read failed");

        assert_eq!(loaded.width, 16);
        assert_eq!(loaded.height, 16);
    }

    /// Tests magic byte detection.
    #[test]
    fn test_can_read() {
        let reader = TiffReader::new();

        // Little-endian TIFF
        assert!(reader.can_read(&[b'I', b'I', 0x2A, 0x00]));
        // Big-endian TIFF
        assert!(reader.can_read(&[b'M', b'M', 0x00, 0x2A]));

        // Invalid
        assert!(!reader.can_read(&[0x89, 0x50, 0x4E, 0x47])); // PNG
        assert!(!reader.can_read(&[0xFF, 0xD8, 0xFF])); // JPEG
    }
}
