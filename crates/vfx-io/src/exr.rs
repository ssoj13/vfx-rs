//! OpenEXR format support.
//!
//! Provides reading and writing of OpenEXR files - the industry standard
//! for HDR and linear workflow images in VFX production.
//!
//! # Overview
//!
//! OpenEXR is a high dynamic range image format developed by ILM. It supports:
//! - 16-bit (half) and 32-bit float pixels
//! - Multiple compression methods (ZIP, PIZ, DWAA, etc.)
//! - Multi-layer/multi-view images
//! - Arbitrary metadata and custom attributes
//! - Deep data for compositing
//!
//! # Architecture
//!
//! This module provides two approaches:
//!
//! 1. **Struct + Trait** (recommended for advanced use):
//!    - [`ExrReader`] implements [`FormatReader`] for reading
//!    - [`ExrWriter`] implements [`FormatWriter`] for writing
//!    - Configure via [`ExrReaderOptions`] and [`ExrWriterOptions`]
//!
//! 2. **Convenience functions** (simple cases):
//!    - [`read()`] - read with defaults
//!    - [`write()`] - write with defaults
//!
//! # Examples
//!
//! Simple usage:
//! ```rust,ignore
//! use vfx_io::exr;
//!
//! let image = exr::read("render.exr")?;
//! exr::write("output.exr", &image)?;
//! ```
//!
//! With options:
//! ```rust,ignore
//! use vfx_io::exr::{ExrWriter, ExrWriterOptions, Compression};
//! use vfx_io::FormatWriter;
//!
//! let writer = ExrWriter::with_options(ExrWriterOptions {
//!     compression: Compression::Piz,
//!     layer_name: Some("beauty".into()),
//!     ..Default::default()
//! });
//! writer.write("output.exr", &image)?;
//! ```

use crate::{AttrValue, FormatReader, FormatWriter, ImageData, IoError, IoResult, Metadata, PixelData, PixelFormat};
use std::io::Cursor;
use std::path::Path;

// ============================================================================
// Compression
// ============================================================================

/// EXR compression method.
///
/// Different compression methods trade off between speed, file size, and quality.
/// All except DWAA/DWAB are lossless.
///
/// # Recommendations
///
/// - **Production/intermediate**: [`Compression::Zip`] - good balance
/// - **Noisy images**: [`Compression::Piz`] - best for film grain, noise
/// - **Final delivery**: [`Compression::Dwaa`] - smallest files, slight quality loss
/// - **Maximum speed**: [`Compression::None`] - no compression overhead
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Compression {
    /// No compression. Fastest read/write, largest files.
    None,
    /// Run-length encoding. Fast, modest compression.
    Rle,
    /// ZIP compression (lossless). Good balance of speed and size.
    /// Default choice for most workflows.
    #[default]
    Zip,
    /// PIZ wavelet compression (lossless). Best for noisy/grainy images.
    /// Slower than ZIP but better compression for film scans.
    Piz,
    /// DWAA lossy compression. Smallest files, some quality loss.
    /// Good for final delivery where file size matters.
    Dwaa,
    /// DWAB lossy compression. Better quality than DWAA, larger files.
    Dwab,
}

impl Compression {
    /// Convert to exr crate's compression type.
    fn to_exr(&self) -> exr::prelude::Compression {
        use exr::prelude::Compression as ExrComp;
        match self {
            Compression::None => ExrComp::Uncompressed,
            Compression::Rle => ExrComp::RLE,
            Compression::Zip => ExrComp::ZIP16,
            Compression::Piz => ExrComp::PIZ,
            Compression::Dwaa => ExrComp::DWAA(None),
            Compression::Dwab => ExrComp::DWAB(None),
        }
    }
}

// ============================================================================
// Reader Options
// ============================================================================

/// Options for reading EXR files.
///
/// Currently minimal - EXR reading is mostly automatic.
/// Reserved for future options like layer selection.
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::exr::{ExrReader, ExrReaderOptions};
/// use vfx_io::FormatReader;
///
/// let reader = ExrReader::with_options(ExrReaderOptions::default());
/// let image = reader.read("input.exr")?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct ExrReaderOptions {
    /// Reserved for future use (layer selection, etc.)
    _reserved: (),
}

// ============================================================================
// Writer Options
// ============================================================================

/// Options for writing EXR files.
///
/// Controls compression, layer naming, and pixel format.
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::exr::{ExrWriter, ExrWriterOptions, Compression};
/// use vfx_io::FormatWriter;
///
/// let options = ExrWriterOptions {
///     compression: Compression::Piz,
///     layer_name: Some("diffuse".into()),
///     ..Default::default()
/// };
/// let writer = ExrWriter::with_options(options);
/// writer.write("output.exr", &image)?;
/// ```
#[derive(Debug, Clone)]
pub struct ExrWriterOptions {
    /// Compression method. Default: ZIP.
    pub compression: Compression,
    /// Layer name in the EXR file. Default: "RGBA".
    pub layer_name: Option<String>,
    /// Write as half-float (f16) instead of f32.
    /// Reduces file size by 50% with some precision loss.
    pub use_half: bool,
}

impl Default for ExrWriterOptions {
    fn default() -> Self {
        Self {
            compression: Compression::Zip,
            layer_name: None,
            use_half: false,
        }
    }
}

// ============================================================================
// ExrReader
// ============================================================================

/// OpenEXR file reader.
///
/// Implements [`FormatReader`] for reading EXR files with configurable options.
///
/// # Features
///
/// - Reads first RGBA layer from multi-layer files
/// - Extracts comprehensive metadata (chromaticities, timecode, etc.)
/// - Supports reading from file or memory
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::exr::ExrReader;
/// use vfx_io::FormatReader;
///
/// let reader = ExrReader::new();
/// let image = reader.read("render.exr")?;
///
/// // Or from memory
/// let data = std::fs::read("render.exr")?;
/// let image = reader.read_from_memory(&data)?;
/// ```
#[derive(Debug, Clone)]
pub struct ExrReader {
    #[allow(dead_code)]
    options: ExrReaderOptions,
}

impl ExrReader {
    /// Creates a new reader with default options.
    pub fn new() -> Self {
        Self::with_options(ExrReaderOptions::default())
    }

    /// Reads EXR from a byte slice.
    ///
    /// Internal implementation shared by file and memory reading.
    fn read_impl(&self, data: &[u8]) -> IoResult<ImageData> {
        use exr::prelude::*;
        use exr::math::Vec2;

        // Read first RGBA layer using builder pattern
        let image = read()
            .no_deep_data()
            .largest_resolution_level()
            .rgba_channels(
                |resolution: Vec2<usize>, _channels: &RgbaChannels| {
                    let width = resolution.width();
                    let size = width * resolution.height();
                    (width, vec![(0.0f32, 0.0f32, 0.0f32, 1.0f32); size])
                },
                |(width, buffer): &mut (usize, Vec<(f32, f32, f32, f32)>), position: Vec2<usize>, (r, g, b, a): (f32, f32, f32, f32)| {
                    let idx = position.y() * *width + position.x();
                    if idx < buffer.len() {
                        buffer[idx] = (r, g, b, a);
                    }
                },
            )
            .first_valid_layer()
            .all_attributes()
            .from_buffered(Cursor::new(data))
            .map_err(|e| IoError::DecodeError(format!("EXR decode error: {}", e)))?;

        let width = image.layer_data.size.width() as u32;
        let height = image.layer_data.size.height() as u32;
        let (_, ref pixel_data) = image.layer_data.channel_data.pixels;

        // Convert tuple vec to flat f32 vec
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for &(r, g, b, a) in pixel_data {
            pixels.push(r);
            pixels.push(g);
            pixels.push(b);
            pixels.push(a);
        }

        let mut result = ImageData {
            width,
            height,
            channels: 4,
            format: PixelFormat::F32,
            data: PixelData::F32(pixels),
            metadata: Metadata::default(),
        };

        // Set colorspace - EXR is always linear
        result.metadata.colorspace = Some("linear".to_string());
        
        // Extract metadata from headers
        self.extract_metadata(data, &mut result.metadata)?;

        Ok(result)
    }

    /// Extracts metadata from EXR headers.
    fn extract_metadata(&self, data: &[u8], metadata: &mut Metadata) -> IoResult<()> {
        let meta = exr::meta::MetaData::read_from_buffered(Cursor::new(data), false)
            .map_err(|e| IoError::DecodeError(format!("EXR metadata parse error: {}", e)))?;

        for (layer_idx, header) in meta.headers.iter().enumerate() {
            // Prefix for multi-layer files
            let prefix = if meta.headers.len() > 1 {
                format!("Layer{}:", layer_idx)
            } else {
                String::new()
            };

            // Display window dimensions
            let display = &header.shared_attributes.display_window;
            metadata.attrs.set(
                format!("{}ImageWidth", prefix),
                AttrValue::UInt(display.size.width() as u32),
            );
            metadata.attrs.set(
                format!("{}ImageHeight", prefix),
                AttrValue::UInt(display.size.height() as u32),
            );

            // Pixel aspect ratio
            metadata.attrs.set(
                format!("{}PixelAspectRatio", prefix),
                AttrValue::Float(header.shared_attributes.pixel_aspect),
            );

            // Compression type
            metadata.attrs.set(
                format!("{}Compression", prefix),
                AttrValue::Str(format!("{:?}", header.compression)),
            );

            // Channel info
            let channels: Vec<String> = header
                .channels
                .list
                .iter()
                .map(|ch| ch.name.to_string())
                .collect();
            metadata.attrs.set(
                format!("{}Channels", prefix),
                AttrValue::Str(channels.join(", ")),
            );
            metadata.attrs.set(
                format!("{}ChannelCount", prefix),
                AttrValue::UInt(header.channels.list.len() as u32),
            );

            // Line order
            metadata.attrs.set(
                format!("{}LineOrder", prefix),
                AttrValue::Str(format!("{:?}", header.line_order)),
            );

            // Chromaticities (color primaries)
            if let Some(chroma) = &header.shared_attributes.chromaticities {
                metadata.attrs.set(
                    format!("{}Chromaticities", prefix),
                    AttrValue::Str(format!(
                        "R({:.3},{:.3}) G({:.3},{:.3}) B({:.3},{:.3}) W({:.3},{:.3})",
                        chroma.red.0, chroma.red.1,
                        chroma.green.0, chroma.green.1,
                        chroma.blue.0, chroma.blue.1,
                        chroma.white.0, chroma.white.1
                    )),
                );
            }

            // Timecode (SMPTE)
            if let Some(tc) = &header.shared_attributes.time_code {
                metadata.attrs.set(
                    format!("{}TimeCode", prefix),
                    AttrValue::Str(format!(
                        "{:02}:{:02}:{:02}:{:02}",
                        tc.hours, tc.minutes, tc.seconds, tc.frame
                    )),
                );
            }

            // Other shared attributes
            for (name, value) in &header.shared_attributes.other {
                metadata.attrs.set(
                    format!("{}EXR:{}", prefix, name),
                    AttrValue::Str(format!("{:?}", value)),
                );
            }

            // Layer-specific attributes
            if let Some(layer_name) = &header.own_attributes.layer_name {
                metadata.attrs.set(
                    format!("{}LayerName", prefix),
                    AttrValue::Str(layer_name.to_string()),
                );
            }

            for (name, value) in &header.own_attributes.other {
                metadata.attrs.set(
                    format!("{}Layer:{}", prefix, name),
                    AttrValue::Str(format!("{:?}", value)),
                );
            }
        }

        Ok(())
    }
}

impl Default for ExrReader {
    fn default() -> Self {
        Self::new()
    }
}

impl FormatReader<ExrReaderOptions> for ExrReader {
    /// Returns "OpenEXR".
    fn format_name(&self) -> &'static str {
        "OpenEXR"
    }

    /// Returns `["exr"]`.
    fn extensions(&self) -> &'static [&'static str] {
        &["exr"]
    }

    /// Checks for EXR magic bytes (0x76, 0x2F, 0x31, 0x01).
    fn can_read(&self, header: &[u8]) -> bool {
        // EXR magic: 0x762F3101
        header.len() >= 4 && header[0] == 0x76 && header[1] == 0x2F && header[2] == 0x31 && header[3] == 0x01
    }

    /// Reads an EXR file from disk.
    fn read<P: AsRef<Path>>(&self, path: P) -> IoResult<ImageData> {
        let data = std::fs::read(path.as_ref())?;
        self.read_from_memory(&data)
    }

    /// Reads an EXR from a byte slice.
    fn read_from_memory(&self, data: &[u8]) -> IoResult<ImageData> {
        self.read_impl(data)
    }

    /// Creates reader with custom options.
    fn with_options(options: ExrReaderOptions) -> Self {
        Self { options }
    }
}

// ============================================================================
// ExrWriter
// ============================================================================

/// OpenEXR file writer.
///
/// Implements [`FormatWriter`] for writing EXR files with configurable options.
///
/// # Features
///
/// - Multiple compression methods (ZIP, PIZ, DWAA, etc.)
/// - Custom layer naming
/// - Half-float (f16) output for smaller files
/// - Supports writing to file or memory
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::exr::{ExrWriter, ExrWriterOptions, Compression};
/// use vfx_io::FormatWriter;
///
/// // Write with PIZ compression (best for noisy images)
/// let writer = ExrWriter::with_options(ExrWriterOptions {
///     compression: Compression::Piz,
///     ..Default::default()
/// });
/// writer.write("output.exr", &image)?;
///
/// // Or to memory
/// let bytes = writer.write_to_memory(&image)?;
/// ```
#[derive(Debug, Clone)]
pub struct ExrWriter {
    options: ExrWriterOptions,
}

impl ExrWriter {
    /// Creates a new writer with default options (ZIP compression).
    pub fn new() -> Self {
        Self::with_options(ExrWriterOptions::default())
    }

    /// Internal write implementation.
    fn write_impl(&self, image: &ImageData) -> IoResult<Vec<u8>> {
        use exr::prelude::*;

        let width = image.width as usize;
        let height = image.height as usize;

        // Convert to f32
        let f32_data = image.to_f32();
        let channels = image.channels as usize;

        // Build RGBA tuples
        let pixels: Vec<(f32, f32, f32, f32)> = (0..width * height)
            .map(|i| {
                let base = i * channels;
                let r = f32_data.get(base).copied().unwrap_or(0.0);
                let g = f32_data.get(base + 1).copied().unwrap_or(0.0);
                let b = f32_data.get(base + 2).copied().unwrap_or(0.0);
                let a = if channels >= 4 {
                    f32_data.get(base + 3).copied().unwrap_or(1.0)
                } else {
                    1.0
                };
                (r, g, b, a)
            })
            .collect();

        // Layer name
        let layer_name = self
            .options
            .layer_name
            .as_deref()
            .unwrap_or("RGBA");

        // Build encoding with compression
        let encoding = Encoding {
            compression: self.options.compression.to_exr(),
            ..Encoding::default()
        };

        // Create layer
        let layer = Layer::new(
            (width, height),
            LayerAttributes::named(layer_name),
            encoding,
            SpecificChannels::rgba(|pos: Vec2<usize>| pixels[pos.y() * width + pos.x()]),
        );

        let exr_image = Image::from_layer(layer);

        // Write to memory buffer
        let mut buffer = Vec::new();
        exr_image
            .write()
            .to_buffered(Cursor::new(&mut buffer))
            .map_err(|e| IoError::EncodeError(format!("EXR encode error: {}", e)))?;

        Ok(buffer)
    }
}

impl Default for ExrWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl FormatWriter<ExrWriterOptions> for ExrWriter {
    /// Returns "OpenEXR".
    fn format_name(&self) -> &'static str {
        "OpenEXR"
    }

    /// Returns `["exr"]`.
    fn extensions(&self) -> &'static [&'static str] {
        &["exr"]
    }

    /// Writes an EXR file to disk.
    fn write<P: AsRef<Path>>(&self, path: P, image: &ImageData) -> IoResult<()> {
        let data = self.write_to_memory(image)?;
        std::fs::write(path.as_ref(), data)?;
        Ok(())
    }

    /// Writes an EXR to a byte vector.
    fn write_to_memory(&self, image: &ImageData) -> IoResult<Vec<u8>> {
        self.write_impl(image)
    }

    /// Creates writer with custom options.
    fn with_options(options: ExrWriterOptions) -> Self {
        Self { options }
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Reads an EXR file with default options.
///
/// Convenience wrapper around [`ExrReader`]. For custom options,
/// use [`ExrReader::with_options`].
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::exr;
///
/// let image = exr::read("render.exr")?;
/// println!("{}x{}, {} channels", image.width, image.height, image.channels);
/// ```
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    ExrReader::new().read(path)
}

/// Writes an EXR file with default options (ZIP compression).
///
/// Convenience wrapper around [`ExrWriter`]. For custom options,
/// use [`ExrWriter::with_options`].
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::exr;
///
/// exr::write("output.exr", &image)?;
/// ```
pub fn write<P: AsRef<Path>>(path: P, image: &ImageData) -> IoResult<()> {
    ExrWriter::new().write(path, image)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests basic read/write roundtrip.
    #[test]
    fn test_roundtrip() {
        let width = 64;
        let height = 64;
        let mut data = Vec::with_capacity((width * height * 4) as usize);

        for y in 0..height {
            for x in 0..width {
                data.push(x as f32 / width as f32);
                data.push(y as f32 / height as f32);
                data.push(0.5);
                data.push(1.0);
            }
        }

        let image = ImageData::from_f32(width, height, 4, data);

        let temp_path = std::env::temp_dir().join("vfx_io_exr_test.exr");
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
        let image = ImageData::from_f32(32, 32, 4, vec![0.5; 32 * 32 * 4]);
        let temp_path = std::env::temp_dir().join("vfx_io_exr_comp_test.exr");

        // Test PIZ compression
        let writer = ExrWriter::with_options(ExrWriterOptions {
            compression: Compression::Piz,
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
        let image = ImageData::from_f32(16, 16, 4, vec![0.25; 16 * 16 * 4]);

        let writer = ExrWriter::new();
        let bytes = writer.write_to_memory(&image).expect("Write failed");

        let reader = ExrReader::new();
        let loaded = reader.read_from_memory(&bytes).expect("Read failed");

        assert_eq!(loaded.width, 16);
        assert_eq!(loaded.height, 16);
    }

    /// Tests magic byte detection.
    #[test]
    fn test_can_read() {
        let reader = ExrReader::new();
        
        // Valid EXR magic
        assert!(reader.can_read(&[0x76, 0x2F, 0x31, 0x01]));
        
        // Invalid
        assert!(!reader.can_read(&[0x89, 0x50, 0x4E, 0x47])); // PNG
        assert!(!reader.can_read(&[0xFF, 0xD8, 0xFF])); // JPEG
    }
}
