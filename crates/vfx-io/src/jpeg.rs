//! JPEG format support.
//!
//! Provides reading and writing of JPEG files - the universal lossy format
//! for photographic images.
//!
//! # Overview
//!
//! JPEG (Joint Photographic Experts Group) is a lossy format optimized for
//! photographic content. It supports:
//! - 8-bit per channel only
//! - RGB, Grayscale, and CMYK color modes
//! - Variable quality/compression ratio
//! - EXIF, XMP, and ICC profile metadata
//!
//! # Architecture
//!
//! This module provides two approaches:
//!
//! 1. **Struct + Trait** (recommended for advanced use):
//!    - [`JpegReader`] implements [`FormatReader`] for reading
//!    - [`JpegWriter`] implements [`FormatWriter`] for writing
//!    - Configure via [`JpegReaderOptions`] and [`JpegWriterOptions`]
//!
//! 2. **Convenience functions** (simple cases):
//!    - [`read()`] - read with defaults
//!    - [`write()`] - write with defaults
//!
//! # Examples
//!
//! Simple usage:
//! ```rust,ignore
//! use vfx_io::jpeg;
//!
//! let image = jpeg::read("photo.jpg")?;
//! jpeg::write("output.jpg", &image)?;
//! ```
//!
//! With quality control:
//! ```rust,ignore
//! use vfx_io::jpeg::{JpegWriter, JpegWriterOptions};
//! use vfx_io::FormatWriter;
//!
//! let writer = JpegWriter::with_options(JpegWriterOptions {
//!     quality: 95,  // High quality
//!     ..Default::default()
//! });
//! writer.write("highq.jpg", &image)?;
//! ```
//!
//! # VFX Usage Notes
//!
//! JPEG is typically used in VFX for:
//! - Reference images and textures
//! - Preview generation
//! - Web delivery
//!
//! **Not recommended** for:
//! - Compositing (use EXR)
//! - Color grading (use DPX/EXR)
//! - Anything requiring lossless quality

use crate::{AttrValue, FormatReader, FormatWriter, ImageData, IoError, IoResult, Metadata, PixelData, PixelFormat};
use std::io::{BufReader, Cursor};
use std::path::Path;

// ============================================================================
// Color Type
// ============================================================================

/// JPEG output color mode.
///
/// JPEG supports RGB (color) and grayscale output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorType {
    /// Full color RGB output.
    #[default]
    Rgb,
    /// Grayscale output (smaller files for B&W images).
    Grayscale,
}

// ============================================================================
// Reader Options
// ============================================================================

/// Options for reading JPEG files.
///
/// Currently minimal - JPEG reading is mostly automatic.
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::jpeg::{JpegReader, JpegReaderOptions};
/// use vfx_io::FormatReader;
///
/// let reader = JpegReader::with_options(JpegReaderOptions::default());
/// let image = reader.read("photo.jpg")?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct JpegReaderOptions {
    /// Reserved for future use.
    _reserved: (),
}

// ============================================================================
// Writer Options
// ============================================================================

/// Options for writing JPEG files.
///
/// Controls quality and color output mode.
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::jpeg::{JpegWriter, JpegWriterOptions};
/// use vfx_io::FormatWriter;
///
/// // High quality for reference images
/// let options = JpegWriterOptions {
///     quality: 95,
///     ..Default::default()
/// };
/// let writer = JpegWriter::with_options(options);
/// writer.write("reference.jpg", &image)?;
/// ```
#[derive(Debug, Clone)]
pub struct JpegWriterOptions {
    /// Quality level 1-100. Higher = better quality, larger files.
    /// Default: 90 (good balance for most uses).
    pub quality: u8,
    /// Output color mode. Default: RGB.
    pub color_type: ColorType,
}

impl Default for JpegWriterOptions {
    fn default() -> Self {
        Self {
            quality: 90,
            color_type: ColorType::Rgb,
        }
    }
}

// ============================================================================
// JpegReader
// ============================================================================

/// JPEG file reader.
///
/// Implements [`FormatReader`] for reading JPEG files with configurable options.
///
/// # Features
///
/// - RGB, Grayscale, CMYK input support
/// - Automatic grayscale/CMYK to RGB conversion
/// - Comprehensive metadata extraction (JFIF, EXIF, ICC)
/// - Memory and file reading
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::jpeg::JpegReader;
/// use vfx_io::FormatReader;
///
/// let reader = JpegReader::new();
/// let image = reader.read("photo.jpg")?;
///
/// // Check DPI if available
/// if let Some(dpi) = image.metadata.dpi {
///     println!("DPI: {}", dpi);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct JpegReader {
    #[allow(dead_code)]
    options: JpegReaderOptions,
}

impl JpegReader {
    /// Creates a new reader with default options.
    pub fn new() -> Self {
        Self::with_options(JpegReaderOptions::default())
    }

    /// Internal read implementation.
    fn read_impl<R: std::io::Read>(&self, reader: R, raw_data: Option<&[u8]>) -> IoResult<ImageData> {
        let buf_reader = BufReader::new(reader);
        let mut decoder = jpeg_decoder::Decoder::new(buf_reader);
        let pixels = decoder
            .decode()
            .map_err(|e| IoError::DecodeError(e.to_string()))?;

        let info = decoder
            .info()
            .ok_or_else(|| IoError::DecodeError("missing JPEG info".into()))?;

        let width = info.width as u32;
        let height = info.height as u32;

        // Convert to RGB based on input format
        let (channels, data) = match info.pixel_format {
            jpeg_decoder::PixelFormat::RGB24 => (3, pixels),
            jpeg_decoder::PixelFormat::L8 => {
                // Grayscale to RGB
                let rgb: Vec<u8> = pixels.iter().flat_map(|&g| [g, g, g]).collect();
                (3, rgb)
            }
            jpeg_decoder::PixelFormat::CMYK32 => {
                // CMYK to RGB (approximate conversion)
                let rgb: Vec<u8> = pixels
                    .chunks(4)
                    .flat_map(|cmyk| {
                        let c = cmyk[0] as f32 / 255.0;
                        let m = cmyk[1] as f32 / 255.0;
                        let y = cmyk[2] as f32 / 255.0;
                        let k = cmyk[3] as f32 / 255.0;

                        let r = ((1.0 - c) * (1.0 - k) * 255.0) as u8;
                        let g = ((1.0 - m) * (1.0 - k) * 255.0) as u8;
                        let b = ((1.0 - y) * (1.0 - k) * 255.0) as u8;

                        [r, g, b]
                    })
                    .collect();
                (3, rgb)
            }
            jpeg_decoder::PixelFormat::L16 => {
                // 16-bit grayscale to 8-bit RGB (use high byte)
                let rgb: Vec<u8> = pixels
                    .chunks(2)
                    .flat_map(|l16| {
                        let g = l16[0]; // High byte
                        [g, g, g]
                    })
                    .collect();
                (3, rgb)
            }
        };

        // Build metadata
        let mut metadata = Metadata::default();
        metadata.colorspace = Some("sRGB".to_string());
        metadata.attrs.set("ImageWidth", AttrValue::UInt(width));
        metadata.attrs.set("ImageHeight", AttrValue::UInt(height));
        metadata.attrs.set(
            "PixelFormat",
            AttrValue::Str(format!("{:?}", info.pixel_format)),
        );
        metadata.attrs.set("BitDepth", AttrValue::UInt(8));

        // Parse additional metadata from raw bytes if available
        if let Some(raw) = raw_data {
            self.parse_metadata(raw, &mut metadata);
        }

        Ok(ImageData {
            width,
            height,
            channels,
            format: PixelFormat::U8,
            data: PixelData::U8(data),
            metadata,
        })
    }

    /// Parses JPEG segments for metadata.
    fn parse_metadata(&self, data: &[u8], metadata: &mut Metadata) {
        if data.len() < 2 || data[0] != 0xFF || data[1] != 0xD8 {
            return;
        }

        let mut icc_chunks: Vec<(u8, u8, Vec<u8>)> = Vec::new();
        let mut pos = 2usize;

        while pos + 1 < data.len() {
            if data[pos] != 0xFF {
                pos += 1;
                continue;
            }
            while pos < data.len() && data[pos] == 0xFF {
                pos += 1;
            }
            if pos >= data.len() {
                break;
            }

            let marker = data[pos];
            pos += 1;

            // End markers
            if marker == 0xD9 || marker == 0xDA {
                break;
            }

            // Standalone markers
            if (0xD0..=0xD7).contains(&marker) || marker == 0x01 {
                continue;
            }

            if pos + 2 > data.len() {
                break;
            }
            let seg_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
            pos += 2;
            if seg_len < 2 || pos + seg_len - 2 > data.len() {
                break;
            }
            let segment = &data[pos..pos + seg_len - 2];

            match marker {
                0xE0 => self.parse_jfif(segment, metadata),
                0xE1 => {
                    if segment.starts_with(b"Exif\0\0") && segment.len() > 6 {
                        metadata.attrs.set(
                            "ExifSize",
                            AttrValue::UInt((segment.len() - 6) as u32),
                        );
                    } else if segment.starts_with(b"http://ns.adobe.com/xap/1.0/\0") {
                        let len = segment.len().saturating_sub(29);
                        metadata.attrs.set("XMPSize", AttrValue::UInt(len as u32));
                    }
                }
                0xE2 => {
                    if segment.starts_with(b"ICC_PROFILE\0") && segment.len() > 14 {
                        let chunk_num = segment[12];
                        let total_chunks = segment[13];
                        icc_chunks.push((chunk_num, total_chunks, segment[14..].to_vec()));
                    }
                }
                0xC0 | 0xC1 | 0xC2 | 0xC3 | 0xC5 | 0xC6 | 0xC7 | 0xC9 | 0xCA | 0xCB
                | 0xCD | 0xCE | 0xCF => self.parse_sof(marker, segment, metadata),
                _ => {}
            }
            pos += seg_len - 2;
        }

        if !icc_chunks.is_empty() {
            self.parse_icc_profile(&mut icc_chunks, metadata);
        }
    }

    /// Parses JFIF APP0 segment.
    fn parse_jfif(&self, data: &[u8], metadata: &mut Metadata) {
        if data.starts_with(b"JFIF\0") && data.len() >= 14 {
            let version_major = data[5];
            let version_minor = data[6];
            metadata.attrs.set(
                "JFIFVersion",
                AttrValue::Str(format!("{}.{:02}", version_major, version_minor)),
            );

            let units = data[7];
            let x_density = u16::from_be_bytes([data[8], data[9]]);
            let y_density = u16::from_be_bytes([data[10], data[11]]);
            let unit_str = match units {
                0 => "aspect ratio",
                1 => "dpi",
                2 => "dpcm",
                _ => "unknown",
            };

            if x_density > 0 && y_density > 0 {
                metadata.attrs.set("XResolution", AttrValue::UInt(x_density as u32));
                metadata.attrs.set("YResolution", AttrValue::UInt(y_density as u32));
                metadata.attrs.set("ResolutionUnit", AttrValue::Str(unit_str.into()));
                if units == 1 && x_density == y_density {
                    metadata.dpi = Some(x_density as f32);
                }
            }
        }
    }

    /// Parses SOF (Start of Frame) segment.
    fn parse_sof(&self, marker: u8, data: &[u8], metadata: &mut Metadata) {
        if data.len() < 6 {
            return;
        }
        let precision = data[0];
        let components = data[5];

        metadata.attrs.set("BitsPerSample", AttrValue::UInt(precision as u32));
        metadata.attrs.set("ColorComponents", AttrValue::UInt(components as u32));

        let compression = match marker {
            0xC0 => "Baseline DCT",
            0xC1 => "Extended Sequential DCT",
            0xC2 => "Progressive DCT",
            0xC3 => "Lossless",
            0xC5 => "Differential Sequential DCT",
            0xC6 => "Differential Progressive DCT",
            0xC7 => "Differential Lossless",
            0xC9 => "Extended Sequential DCT (Arithmetic)",
            0xCA => "Progressive DCT (Arithmetic)",
            0xCB => "Lossless (Arithmetic)",
            0xCD => "Differential Sequential (Arithmetic)",
            0xCE => "Differential Progressive (Arithmetic)",
            0xCF => "Differential Lossless (Arithmetic)",
            _ => "Unknown",
        };
        metadata.attrs.set("Compression", AttrValue::Str(compression.into()));
    }

    /// Parses ICC profile from chunks.
    fn parse_icc_profile(&self, chunks: &mut [(u8, u8, Vec<u8>)], metadata: &mut Metadata) {
        chunks.sort_by_key(|(num, _, _)| *num);
        let mut profile_data = Vec::new();
        for (_, _, data) in chunks.iter() {
            profile_data.extend_from_slice(data);
        }

        if profile_data.len() < 20 {
            metadata.attrs.set(
                "ICCProfileSize",
                AttrValue::UInt(profile_data.len() as u32),
            );
            return;
        }

        let profile_size = u32::from_be_bytes([
            profile_data[0],
            profile_data[1],
            profile_data[2],
            profile_data[3],
        ]);
        metadata.attrs.set("ICCProfileSize", AttrValue::UInt(profile_size));

        if let Ok(space) = std::str::from_utf8(&profile_data[16..20]) {
            metadata.attrs.set(
                "ICCColorSpace",
                AttrValue::Str(space.trim().to_string()),
            );
        }
    }
}

impl Default for JpegReader {
    fn default() -> Self {
        Self::new()
    }
}

impl FormatReader<JpegReaderOptions> for JpegReader {
    /// Returns "JPEG".
    fn format_name(&self) -> &'static str {
        "JPEG"
    }

    /// Returns `["jpg", "jpeg"]`.
    fn extensions(&self) -> &'static [&'static str] {
        &["jpg", "jpeg"]
    }

    /// Checks for JPEG magic bytes (0xFF, 0xD8, 0xFF).
    fn can_read(&self, header: &[u8]) -> bool {
        header.len() >= 3 && header[0] == 0xFF && header[1] == 0xD8 && header[2] == 0xFF
    }

    /// Reads a JPEG file from disk.
    fn read<P: AsRef<Path>>(&self, path: P) -> IoResult<ImageData> {
        let data = std::fs::read(path.as_ref())?;
        self.read_impl(Cursor::new(&data), Some(&data))
    }

    /// Reads a JPEG from a byte slice.
    fn read_from_memory(&self, data: &[u8]) -> IoResult<ImageData> {
        self.read_impl(Cursor::new(data), Some(data))
    }

    /// Creates reader with custom options.
    fn with_options(options: JpegReaderOptions) -> Self {
        Self { options }
    }
}

// ============================================================================
// JpegWriter
// ============================================================================

/// JPEG file writer.
///
/// Implements [`FormatWriter`] for writing JPEG files with configurable options.
///
/// # Features
///
/// - Quality control (1-100)
/// - RGB and grayscale output
/// - Memory and file writing
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::jpeg::{JpegWriter, JpegWriterOptions};
/// use vfx_io::FormatWriter;
///
/// // Low quality for previews
/// let preview_writer = JpegWriter::with_options(JpegWriterOptions {
///     quality: 60,
///     ..Default::default()
/// });
/// preview_writer.write("preview.jpg", &image)?;
///
/// // High quality for final delivery
/// let hq_writer = JpegWriter::with_options(JpegWriterOptions {
///     quality: 98,
///     ..Default::default()
/// });
/// hq_writer.write("final.jpg", &image)?;
/// ```
#[derive(Debug, Clone)]
pub struct JpegWriter {
    options: JpegWriterOptions,
}

impl JpegWriter {
    /// Creates a new writer with default options (quality 90).
    pub fn new() -> Self {
        Self::with_options(JpegWriterOptions::default())
    }

    /// Internal write implementation.
    fn write_impl(&self, image: &ImageData) -> IoResult<Vec<u8>> {
        use jpeg_encoder::{ColorType as JpegColorType, Encoder};

        // Convert to u8
        let u8_data = image.to_u8();

        // Prepare pixel data based on color type
        let (color_type, pixel_data) = match self.options.color_type {
            ColorType::Rgb => {
                // Strip alpha if RGBA
                let rgb = if image.channels == 4 {
                    u8_data
                        .chunks(4)
                        .flat_map(|rgba| [rgba[0], rgba[1], rgba[2]])
                        .collect()
                } else if image.channels == 3 {
                    u8_data
                } else if image.channels == 1 {
                    // Expand grayscale to RGB
                    u8_data.iter().flat_map(|&g| [g, g, g]).collect()
                } else {
                    return Err(IoError::EncodeError(format!(
                        "unsupported channel count: {}",
                        image.channels
                    )));
                };
                (JpegColorType::Rgb, rgb)
            }
            ColorType::Grayscale => {
                // Convert to grayscale
                let gray = if image.channels >= 3 {
                    u8_data
                        .chunks(image.channels as usize)
                        .map(|px| {
                            // ITU-R BT.601 luma coefficients
                            let r = px[0] as f32;
                            let g = px[1] as f32;
                            let b = px[2] as f32;
                            (0.299 * r + 0.587 * g + 0.114 * b) as u8
                        })
                        .collect()
                } else {
                    u8_data
                };
                (JpegColorType::Luma, gray)
            }
        };

        // Encode to memory buffer
        let mut buffer = Vec::new();
        let encoder = Encoder::new(&mut buffer, self.options.quality);
        encoder
            .encode(&pixel_data, image.width as u16, image.height as u16, color_type)
            .map_err(|e: jpeg_encoder::EncodingError| IoError::EncodeError(e.to_string()))?;

        Ok(buffer)
    }
}

impl Default for JpegWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl FormatWriter<JpegWriterOptions> for JpegWriter {
    /// Returns "JPEG".
    fn format_name(&self) -> &'static str {
        "JPEG"
    }

    /// Returns `["jpg", "jpeg"]`.
    fn extensions(&self) -> &'static [&'static str] {
        &["jpg", "jpeg"]
    }

    /// Writes a JPEG file to disk.
    fn write<P: AsRef<Path>>(&self, path: P, image: &ImageData) -> IoResult<()> {
        let data = self.write_to_memory(image)?;
        std::fs::write(path.as_ref(), data)?;
        Ok(())
    }

    /// Writes a JPEG to a byte vector.
    fn write_to_memory(&self, image: &ImageData) -> IoResult<Vec<u8>> {
        self.write_impl(image)
    }

    /// Creates writer with custom options.
    fn with_options(options: JpegWriterOptions) -> Self {
        Self { options }
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Reads a JPEG file with default options.
///
/// Convenience wrapper around [`JpegReader`]. For custom options,
/// use [`JpegReader::with_options`].
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::jpeg;
///
/// let image = jpeg::read("photo.jpg")?;
/// ```
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    JpegReader::new().read(path)
}

/// Writes a JPEG file with default options (quality 90).
///
/// Convenience wrapper around [`JpegWriter`]. For custom options,
/// use [`JpegWriter::with_options`].
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::jpeg;
///
/// jpeg::write("output.jpg", &image)?;
/// ```
pub fn write<P: AsRef<Path>>(path: P, image: &ImageData) -> IoResult<()> {
    JpegWriter::new().write(path, image)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests basic roundtrip.
    #[test]
    fn test_roundtrip() {
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
        let temp_path = std::env::temp_dir().join("vfx_io_jpeg_test.jpg");

        write(&temp_path, &image).expect("Write failed");
        let loaded = read(&temp_path).expect("Read failed");

        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
        assert_eq!(loaded.channels, 3);

        let _ = std::fs::remove_file(&temp_path);
    }

    /// Tests quality options.
    #[test]
    fn test_quality_options() {
        let image = ImageData::from_u8(16, 16, 3, vec![128; 16 * 16 * 3]);
        let temp_path = std::env::temp_dir().join("vfx_io_jpeg_quality_test.jpg");

        // Low quality
        let writer = JpegWriter::with_options(JpegWriterOptions {
            quality: 50,
            ..Default::default()
        });
        writer.write(&temp_path, &image).expect("Write failed");
        let low_size = std::fs::metadata(&temp_path).unwrap().len();

        // High quality
        let writer = JpegWriter::with_options(JpegWriterOptions {
            quality: 99,
            ..Default::default()
        });
        writer.write(&temp_path, &image).expect("Write failed");
        let high_size = std::fs::metadata(&temp_path).unwrap().len();

        // High quality should be larger (usually)
        assert!(high_size >= low_size);

        let _ = std::fs::remove_file(&temp_path);
    }

    /// Tests memory roundtrip.
    #[test]
    fn test_memory_roundtrip() {
        let image = ImageData::from_u8(16, 16, 3, vec![100; 16 * 16 * 3]);

        let writer = JpegWriter::new();
        let bytes = writer.write_to_memory(&image).expect("Write failed");

        let reader = JpegReader::new();
        let loaded = reader.read_from_memory(&bytes).expect("Read failed");

        assert_eq!(loaded.width, 16);
        assert_eq!(loaded.height, 16);
    }

    /// Tests magic byte detection.
    #[test]
    fn test_can_read() {
        let reader = JpegReader::new();

        // Valid JPEG magic
        assert!(reader.can_read(&[0xFF, 0xD8, 0xFF, 0xE0]));
        assert!(reader.can_read(&[0xFF, 0xD8, 0xFF, 0xE1]));

        // Invalid
        assert!(!reader.can_read(&[0x89, 0x50, 0x4E, 0x47])); // PNG
        assert!(!reader.can_read(&[0x76, 0x2F, 0x31, 0x01])); // EXR
    }

    /// Tests grayscale output.
    #[test]
    fn test_grayscale_output() {
        let image = ImageData::from_u8(16, 16, 3, vec![128; 16 * 16 * 3]);
        let temp_path = std::env::temp_dir().join("vfx_io_jpeg_gray_test.jpg");

        let writer = JpegWriter::with_options(JpegWriterOptions {
            color_type: ColorType::Grayscale,
            ..Default::default()
        });
        writer.write(&temp_path, &image).expect("Write failed");

        // Should still be readable
        let loaded = read(&temp_path).expect("Read failed");
        assert_eq!(loaded.width, 16);

        let _ = std::fs::remove_file(&temp_path);
    }
}
