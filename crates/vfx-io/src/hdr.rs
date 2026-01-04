//! Radiance HDR (RGBE) format support.
//!
//! Provides reading and writing of Radiance HDR files - the classic format
//! for high dynamic range environment maps and lighting.
//!
//! # Overview
//!
//! Radiance HDR uses RGBE encoding (RGB + shared exponent) to store HDR data
//! in a compact format. Features include:
//! - High dynamic range (HDR) pixel storage
//! - RLE compression for efficient storage
//! - Metadata for exposure, gamma, primaries
//! - Linear or XYZ color space
//!
//! # Architecture
//!
//! This module provides two approaches:
//!
//! 1. **Struct + Trait** (recommended for advanced use):
//!    - [`HdrReader`] implements [`FormatReader`] for reading
//!    - [`HdrWriter`] implements [`FormatWriter`] for writing
//!    - Configure via [`HdrReaderOptions`] and [`HdrWriterOptions`]
//!
//! 2. **Convenience functions** (simple cases):
//!    - [`read()`] - read with defaults
//!    - [`write()`] - write with defaults
//!
//! # RGBE Encoding
//!
//! RGBE stores each pixel as 4 bytes: R, G, B mantissas and shared exponent.
//! This allows 32 orders of magnitude dynamic range with 8-bit precision.
//!
//! # Examples
//!
//! Simple usage:
//! ```ignore
//! use vfx_io::hdr;
//!
//! let image = hdr::read("environment.hdr")?;
//! hdr::write("output.hdr", &image)?;
//! ```
//!
//! With options:
//! ```ignore
//! use vfx_io::hdr::{HdrWriter, HdrWriterOptions};
//! use vfx_io::FormatWriter;
//!
//! let writer = HdrWriter::with_options(HdrWriterOptions {
//!     use_rle: true,
//!     format_id: "RADIANCE".into(),
//!     ..Default::default()
//! });
//! writer.write("output.hdr", &image)?;
//! ```

use crate::{AttrValue, FormatReader, FormatWriter, ImageData, IoError, IoResult, Metadata, PixelData, PixelFormat};
use std::io::{BufRead, BufReader, BufWriter, Cursor, Read, Write};
use std::path::Path;

/// HDR file magic bytes.
const HDR_MAGIC: &str = "#?";

// ============================================================================
// Reader Options
// ============================================================================

/// Options for reading HDR files.
///
/// Currently minimal - HDR reading is mostly automatic.
///
/// # Example
///
/// ```ignore
/// use vfx_io::hdr::{HdrReader, HdrReaderOptions};
/// use vfx_io::FormatReader;
///
/// let reader = HdrReader::with_options(HdrReaderOptions::default());
/// let image = reader.read("environment.hdr")?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct HdrReaderOptions {
    /// Reserved for future use.
    _reserved: (),
}

// ============================================================================
// Writer Options
// ============================================================================

/// Options for writing HDR files.
///
/// Controls RLE compression and metadata.
///
/// # Example
///
/// ```ignore
/// use vfx_io::hdr::{HdrWriter, HdrWriterOptions};
/// use vfx_io::FormatWriter;
///
/// let options = HdrWriterOptions {
///     use_rle: true,
///     format_id: "RADIANCE".into(),
///     ..Default::default()
/// };
/// let writer = HdrWriter::with_options(options);
/// writer.write("output.hdr", &image)?;
/// ```
#[derive(Debug, Clone)]
pub struct HdrWriterOptions {
    /// Use RLE compression. Default: true.
    /// RLE is more efficient for typical HDR content.
    pub use_rle: bool,
    /// Format identifier (appears after #?). Default: "RADIANCE".
    pub format_id: String,
    /// Format string (appears as FORMAT=). Default: "32-bit_rle_rgbe".
    pub format: String,
}

impl Default for HdrWriterOptions {
    fn default() -> Self {
        Self {
            use_rle: true,
            format_id: "RADIANCE".into(),
            format: "32-bit_rle_rgbe".into(),
        }
    }
}

// ============================================================================
// HdrReader
// ============================================================================

/// Radiance HDR file reader.
///
/// Implements [`FormatReader`] for reading HDR files with configurable options.
///
/// # Features
///
/// - RGBE decoding with RLE support
/// - Comprehensive metadata extraction (exposure, gamma, primaries)
/// - Linear or XYZ color space detection
/// - Memory and file reading
///
/// # Example
///
/// ```ignore
/// use vfx_io::hdr::HdrReader;
/// use vfx_io::FormatReader;
///
/// let reader = HdrReader::new();
/// let image = reader.read("environment.hdr")?;
///
/// // Check exposure
/// if let Some(AttrValue::Float(exp)) = image.metadata.attrs.get("Exposure") {
///     println!("Exposure: {}", exp);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct HdrReader {
    #[allow(dead_code)]
    options: HdrReaderOptions,
}

impl HdrReader {
    /// Creates a new reader with default options.
    pub fn new() -> Self {
        Self::with_options(HdrReaderOptions::default())
    }

    /// Internal read implementation.
    fn read_impl<R: BufRead>(&self, reader: &mut R) -> IoResult<ImageData> {
        let (mut metadata, width, height, format) = self.read_header(reader)?;
        let data = self.read_pixels(reader, width as usize, height as usize)?;

        // Set colorspace based on format
        if format.to_lowercase().contains("xyze") {
            metadata.colorspace = Some("xyz".to_string());
        } else {
            metadata.colorspace = Some("linear".to_string());
        }

        Ok(ImageData {
            width,
            height,
            channels: 3,
            format: PixelFormat::F32,
            data: PixelData::F32(data),
            metadata,
        })
    }

    /// Reads HDR header, extracting metadata and dimensions.
    fn read_header<R: BufRead>(&self, reader: &mut R) -> IoResult<(Metadata, u32, u32, String)> {
        let mut metadata = Metadata::default();
        let mut line = String::new();

        // Read magic line
        reader.read_line(&mut line)?;
        let magic_line = trim_line(&line);
        if !magic_line.starts_with(HDR_MAGIC) {
            return Err(IoError::InvalidFile("HDR magic not found".into()));
        }

        // Extract format identifier
        let format_id = magic_line.trim_start_matches(HDR_MAGIC);
        if !format_id.is_empty() {
            metadata.attrs.set("FormatIdentifier", AttrValue::Str(format_id.into()));
        }

        let mut width = None;
        let mut height = None;
        let mut format = "32-bit_rle_rgbe".to_string();

        // Parse header fields
        loop {
            line.clear();
            let bytes = reader.read_line(&mut line)?;
            if bytes == 0 {
                break;
            }
            let line = trim_line(&line);

            if line.is_empty() {
                continue;
            }

            // Resolution line
            if line.starts_with('+') || line.starts_with('-') {
                if let Some((w, h)) = parse_resolution(line) {
                    width = Some(w);
                    height = Some(h);
                    metadata.attrs.set("ImageWidth", AttrValue::UInt(w));
                    metadata.attrs.set("ImageHeight", AttrValue::UInt(h));
                    break;
                } else {
                    return Err(IoError::InvalidFile("Invalid HDR resolution".into()));
                }
            }

            // Skip comments
            if line.starts_with('#') {
                continue;
            }

            // Parse key=value
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                match key.to_uppercase().as_str() {
                    "FORMAT" => {
                        format = value.to_string();
                        metadata.attrs.set("Format", AttrValue::Str(format.clone()));
                    }
                    "EXPOSURE" => {
                        if let Ok(v) = value.parse::<f32>() {
                            metadata.attrs.set("Exposure", AttrValue::Float(v));
                        } else {
                            metadata.attrs.set("Exposure", AttrValue::Str(value.into()));
                        }
                    }
                    "GAMMA" => {
                        if let Ok(v) = value.parse::<f32>() {
                            metadata.gamma = Some(v);
                            metadata.attrs.set("Gamma", AttrValue::Float(v));
                        } else {
                            metadata.attrs.set("Gamma", AttrValue::Str(value.into()));
                        }
                    }
                    "PIXASPECT" => {
                        if let Ok(v) = value.parse::<f32>() {
                            metadata.attrs.set("PixelAspectRatio", AttrValue::Float(v));
                        } else {
                            metadata.attrs.set("PixelAspectRatio", AttrValue::Str(value.into()));
                        }
                    }
                    "SOFTWARE" => {
                        metadata.attrs.set("Software", AttrValue::Str(value.into()));
                    }
                    "PRIMARIES" => {
                        metadata.attrs.set("Primaries", AttrValue::Str(value.into()));
                    }
                    "COLORCORR" => {
                        metadata.attrs.set("ColorCorrection", AttrValue::Str(value.into()));
                    }
                    "VIEW" => {
                        metadata.attrs.set("View", AttrValue::Str(value.into()));
                    }
                    _ => {
                        metadata.attrs.set(format!("HDR:{}", key), AttrValue::Str(value.into()));
                    }
                }
            }
        }

        let width = width.ok_or_else(|| IoError::InvalidFile("Missing HDR width".into()))?;
        let height = height.ok_or_else(|| IoError::InvalidFile("Missing HDR height".into()))?;

        Ok((metadata, width, height, format))
    }

    /// Reads pixel data, handling RLE if present.
    fn read_pixels<R: Read>(&self, reader: &mut R, width: usize, height: usize) -> IoResult<Vec<f32>> {
        let mut first = [0u8; 4];
        reader.read_exact(&mut first)?;

        // Check for RLE encoding
        let use_rle = width >= 8
            && width <= 0x7fff
            && first[0] == 2
            && first[1] == 2
            && ((first[2] as usize) << 8 | first[3] as usize) == width;

        let mut rgbe = vec![0u8; width * height * 4];

        if use_rle {
            let mut scanline = vec![0u8; width * 4];
            self.decode_rle_scanline(reader, width, &mut scanline, first)?;
            rgbe[0..width * 4].copy_from_slice(&scanline);

            for y in 1..height {
                let mut header = [0u8; 4];
                reader.read_exact(&mut header)?;
                self.decode_rle_scanline(reader, width, &mut scanline, header)?;
                let offset = y * width * 4;
                rgbe[offset..offset + width * 4].copy_from_slice(&scanline);
            }
        } else {
            // Raw RGBE data
            rgbe[0..4].copy_from_slice(&first);
            reader.read_exact(&mut rgbe[4..])?;
        }

        // Convert RGBE to f32 RGB
        let mut data = Vec::with_capacity(width * height * 3);
        for chunk in rgbe.chunks_exact(4) {
            let (r, g, b) = rgbe_to_f32(chunk[0], chunk[1], chunk[2], chunk[3]);
            data.push(r);
            data.push(g);
            data.push(b);
        }

        Ok(data)
    }

    /// Decodes an RLE-compressed scanline.
    fn decode_rle_scanline<R: Read>(
        &self,
        reader: &mut R,
        width: usize,
        out: &mut [u8],
        header: [u8; 4],
    ) -> IoResult<()> {
        if header[0] != 2 || header[1] != 2 {
            return Err(IoError::InvalidFile("HDR RLE header invalid".into()));
        }
        let encoded_width = ((header[2] as usize) << 8) | (header[3] as usize);
        if encoded_width != width {
            return Err(IoError::InvalidFile("HDR RLE width mismatch".into()));
        }

        let mut channel = vec![0u8; width];
        for c in 0..4 {
            let mut idx = 0usize;
            while idx < width {
                let mut count = [0u8; 1];
                reader.read_exact(&mut count)?;
                let count = count[0] as usize;
                if count > 128 {
                    // Run-length encoded
                    let run = count - 128;
                    let mut value = [0u8; 1];
                    reader.read_exact(&mut value)?;
                    for _ in 0..run {
                        channel[idx] = value[0];
                        idx += 1;
                    }
                } else {
                    // Literal run
                    let run = count;
                    reader.read_exact(&mut channel[idx..idx + run])?;
                    idx += run;
                }
            }

            // Interleave into output
            for x in 0..width {
                out[x * 4 + c] = channel[x];
            }
        }

        Ok(())
    }
}

impl Default for HdrReader {
    fn default() -> Self {
        Self::new()
    }
}

impl FormatReader<HdrReaderOptions> for HdrReader {
    /// Returns "Radiance HDR".
    fn format_name(&self) -> &'static str {
        "Radiance HDR"
    }

    /// Returns `["hdr", "pic"]`.
    fn extensions(&self) -> &'static [&'static str] {
        &["hdr", "pic"]
    }

    /// Checks for HDR magic bytes (#?).
    fn can_read(&self, header: &[u8]) -> bool {
        header.len() >= 2 && header[0] == b'#' && header[1] == b'?'
    }

    /// Reads an HDR file from disk.
    fn read<P: AsRef<Path>>(&self, path: P) -> IoResult<ImageData> {
        let file = std::fs::File::open(path.as_ref())?;
        let mut reader = BufReader::new(file);
        self.read_impl(&mut reader)
    }

    /// Reads an HDR from a byte slice.
    fn read_from_memory(&self, data: &[u8]) -> IoResult<ImageData> {
        let mut reader = BufReader::new(Cursor::new(data));
        self.read_impl(&mut reader)
    }

    /// Creates reader with custom options.
    fn with_options(options: HdrReaderOptions) -> Self {
        Self { options }
    }
}

// ============================================================================
// HdrWriter
// ============================================================================

/// Radiance HDR file writer.
///
/// Implements [`FormatWriter`] for writing HDR files with configurable options.
///
/// # Features
///
/// - RGBE encoding with optional RLE compression
/// - Metadata preservation (exposure, gamma, primaries)
/// - Memory and file writing
///
/// # Example
///
/// ```ignore
/// use vfx_io::hdr::{HdrWriter, HdrWriterOptions};
/// use vfx_io::FormatWriter;
///
/// let writer = HdrWriter::with_options(HdrWriterOptions {
///     use_rle: true,
///     ..Default::default()
/// });
/// writer.write("environment.hdr", &image)?;
/// ```
#[derive(Debug, Clone)]
pub struct HdrWriter {
    options: HdrWriterOptions,
}

impl HdrWriter {
    /// Creates a new writer with default options.
    pub fn new() -> Self {
        Self::with_options(HdrWriterOptions::default())
    }

    /// Internal write implementation.
    fn write_impl<W: Write>(&self, writer: W, image: &ImageData) -> IoResult<()> {
        let mut buf_writer = BufWriter::new(writer);

        // Get format info from metadata or use options
        let format_id = image
            .metadata
            .attrs
            .get("FormatIdentifier")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.options.format_id);

        let format = image
            .metadata
            .attrs
            .get("Format")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.options.format);

        // Write header
        writeln!(buf_writer, "{}{}", HDR_MAGIC, format_id)?;
        write_header_field(&mut buf_writer, "FORMAT", format)?;

        // Write optional metadata
        if let Some(v) = image.metadata.attrs.get("Software").and_then(|v| v.as_str()) {
            write_header_field(&mut buf_writer, "SOFTWARE", v)?;
        }
        if let Some(v) = image.metadata.attrs.get("Exposure").and_then(|v| v.as_f32()) {
            write_header_field(&mut buf_writer, "EXPOSURE", &format!("{}", v))?;
        }
        if let Some(v) = image.metadata.attrs.get("Gamma").and_then(|v| v.as_f32()) {
            write_header_field(&mut buf_writer, "GAMMA", &format!("{}", v))?;
        } else if let Some(gamma) = image.metadata.gamma {
            write_header_field(&mut buf_writer, "GAMMA", &format!("{}", gamma))?;
        }
        if let Some(v) = image.metadata.attrs.get("PixelAspectRatio").and_then(|v| v.as_f32()) {
            write_header_field(&mut buf_writer, "PIXASPECT", &format!("{}", v))?;
        }
        if let Some(v) = image.metadata.attrs.get("Primaries").and_then(|v| v.as_str()) {
            write_header_field(&mut buf_writer, "PRIMARIES", v)?;
        }
        if let Some(v) = image.metadata.attrs.get("ColorCorrection").and_then(|v| v.as_str()) {
            write_header_field(&mut buf_writer, "COLORCORR", v)?;
        }
        if let Some(v) = image.metadata.attrs.get("View").and_then(|v| v.as_str()) {
            write_header_field(&mut buf_writer, "VIEW", v)?;
        }

        // Write custom HDR: prefixed attributes
        for (key, value) in image.metadata.attrs.iter() {
            if let Some(hdr_key) = key.strip_prefix("HDR:") {
                if let AttrValue::Str(v) = value {
                    write_header_field(&mut buf_writer, hdr_key, v)?;
                }
            }
        }

        // End header, write resolution
        writeln!(buf_writer)?;
        writeln!(buf_writer, "-Y {} +X {}", image.height, image.width)?;

        // Write pixel data
        self.write_pixels(&mut buf_writer, image)?;

        Ok(())
    }

    /// Writes pixel data with optional RLE compression.
    fn write_pixels<W: Write>(&self, writer: &mut W, image: &ImageData) -> IoResult<()> {
        let width = image.width as usize;
        let height = image.height as usize;
        let channels = image.channels as usize;
        let f32_data = image.to_f32();

        let use_rle = self.options.use_rle && width >= 8 && width <= 0x7fff;

        let mut scanline = vec![0u8; width * 4];
        for y in 0..height {
            // Convert scanline to RGBE
            for x in 0..width {
                let base = (y * width + x) * channels;
                let r = *f32_data.get(base).unwrap_or(&0.0);
                let g = *f32_data.get(base + 1).unwrap_or(&0.0);
                let b = *f32_data.get(base + 2).unwrap_or(&0.0);
                let rgbe = f32_to_rgbe(r, g, b);
                let offset = x * 4;
                scanline[offset..offset + 4].copy_from_slice(&rgbe);
            }

            if use_rle {
                // Write RLE header
                let header = [2u8, 2u8, (width >> 8) as u8, (width & 0xFF) as u8];
                writer.write_all(&header)?;
                self.encode_rle_scanline(writer, width, &scanline)?;
            } else {
                writer.write_all(&scanline)?;
            }
        }

        Ok(())
    }

    /// Encodes a scanline with RLE compression.
    fn encode_rle_scanline<W: Write>(&self, writer: &mut W, width: usize, scanline: &[u8]) -> IoResult<()> {
        let mut channel = vec![0u8; width];
        for c in 0..4 {
            // Extract channel
            for x in 0..width {
                channel[x] = scanline[x * 4 + c];
            }
            // Encode and write
            let encoded = encode_rle_channel(&channel);
            writer.write_all(&encoded)?;
        }
        Ok(())
    }
}

impl Default for HdrWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl FormatWriter<HdrWriterOptions> for HdrWriter {
    /// Returns "Radiance HDR".
    fn format_name(&self) -> &'static str {
        "Radiance HDR"
    }

    /// Returns `["hdr", "pic"]`.
    fn extensions(&self) -> &'static [&'static str] {
        &["hdr", "pic"]
    }

    /// Writes an HDR file to disk.
    fn write<P: AsRef<Path>>(&self, path: P, image: &ImageData) -> IoResult<()> {
        let file = std::fs::File::create(path.as_ref())?;
        self.write_impl(file, image)
    }

    /// Writes an HDR to a byte vector.
    fn write_to_memory(&self, image: &ImageData) -> IoResult<Vec<u8>> {
        let mut buffer = Vec::new();
        self.write_impl(&mut buffer, image)?;
        Ok(buffer)
    }

    /// Creates writer with custom options.
    fn with_options(options: HdrWriterOptions) -> Self {
        Self { options }
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Reads an HDR file with default options.
///
/// Convenience wrapper around [`HdrReader`]. For custom options,
/// use [`HdrReader::with_options`].
///
/// # Example
///
/// ```ignore
/// use vfx_io::hdr;
///
/// let image = hdr::read("environment.hdr")?;
/// ```
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    HdrReader::new().read(path)
}

/// Writes an HDR file with default options (RLE compression).
///
/// Convenience wrapper around [`HdrWriter`]. For custom options,
/// use [`HdrWriter::with_options`].
///
/// # Example
///
/// ```ignore
/// use vfx_io::hdr;
///
/// hdr::write("output.hdr", &image)?;
/// ```
pub fn write<P: AsRef<Path>>(path: P, image: &ImageData) -> IoResult<()> {
    HdrWriter::new().write(path, image)
}

// ============================================================================
// RGBE Encoding/Decoding
// ============================================================================

/// Converts f32 RGB to RGBE (4 bytes).
fn f32_to_rgbe(r: f32, g: f32, b: f32) -> [u8; 4] {
    let r = r.max(0.0);
    let g = g.max(0.0);
    let b = b.max(0.0);
    let max = r.max(g).max(b);

    if max < 1.0e-32 {
        return [0, 0, 0, 0];
    }

    let (m, e) = frexp(max);
    let scale = m * 256.0 / max;

    [
        (r * scale).clamp(0.0, 255.0) as u8,
        (g * scale).clamp(0.0, 255.0) as u8,
        (b * scale).clamp(0.0, 255.0) as u8,
        (e + 128) as u8,
    ]
}

/// Converts RGBE (4 bytes) to f32 RGB.
fn rgbe_to_f32(r: u8, g: u8, b: u8, e: u8) -> (f32, f32, f32) {
    if e == 0 {
        return (0.0, 0.0, 0.0);
    }
    let exp = (e as i32) - 136;
    let f = 2.0_f32.powi(exp);
    (r as f32 * f, g as f32 * f, b as f32 * f)
}

/// Extracts mantissa and exponent (like C's frexp).
fn frexp(x: f32) -> (f32, i32) {
    if x == 0.0 {
        return (0.0, 0);
    }
    let e = x.abs().log2().floor() as i32 + 1;
    let m = x / 2.0_f32.powi(e);
    (m, e)
}

// ============================================================================
// RLE Encoding
// ============================================================================

/// Encodes a channel with RLE compression.
fn encode_rle_channel(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 2);
    let mut i = 0usize;

    while i < data.len() {
        // Check for run
        let mut run = 1usize;
        while i + run < data.len() && run < 127 && data[i] == data[i + run] {
            run += 1;
        }

        if run >= 4 {
            // Encode run
            out.push((128 + run) as u8);
            out.push(data[i]);
            i += run;
            continue;
        }

        // Encode literal sequence
        let start = i;
        let mut literal = 0usize;
        while i < data.len() {
            run = 1;
            while i + run < data.len() && run < 127 && data[i] == data[i + run] {
                run += 1;
            }
            if run >= 4 {
                break;
            }
            i += 1;
            literal += 1;
            if literal == 128 {
                break;
            }
        }
        out.push(literal as u8);
        out.extend_from_slice(&data[start..start + literal]);
    }

    out
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parses resolution line (-Y h +X w or similar).
fn parse_resolution(line: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() != 4 {
        return None;
    }

    let mut width = 0u32;
    let mut height = 0u32;

    for i in (0..4).step_by(2) {
        let axis = parts[i];
        let value: u32 = parts.get(i + 1)?.parse().ok()?;

        if axis.ends_with('X') {
            width = value;
        } else if axis.ends_with('Y') {
            height = value;
        }
    }

    if width > 0 && height > 0 {
        Some((width, height))
    } else {
        None
    }
}

/// Writes a header field.
fn write_header_field<W: Write>(writer: &mut W, key: &str, value: &str) -> IoResult<()> {
    writeln!(writer, "{}={}", key, value)?;
    Ok(())
}

/// Trims line endings.
fn trim_line(line: &str) -> &str {
    line.trim_end_matches(&['\r', '\n'][..])
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    /// Tests resolution parsing.
    #[test]
    fn test_parse_resolution() {
        assert_eq!(parse_resolution("-Y 2 +X 3"), Some((3, 2)));
        assert_eq!(parse_resolution("+X 4 -Y 5"), Some((4, 5)));
    }

    /// Tests basic roundtrip.
    #[test]
    fn test_roundtrip() {
        let width = 4;
        let height = 2;
        let data: Vec<f32> = (0..(width * height * 3))
            .map(|i| (i as f32) / 10.0)
            .collect();

        let image = ImageData::from_f32(width, height, 3, data.clone());
        let temp_path = std::env::temp_dir().join("vfx_io_hdr_test.hdr");

        write(&temp_path, &image).expect("Write failed");
        let loaded = read(&temp_path).expect("Read failed");

        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
        assert_eq!(loaded.channels, 3);

        let loaded_data = match loaded.data {
            PixelData::F32(v) => v,
            _ => panic!("Unexpected format"),
        };

        // RGBE has limited precision
        assert_relative_eq!(loaded_data[0], data[0], epsilon = 1e-2);

        let _ = std::fs::remove_file(&temp_path);
    }

    /// Tests memory roundtrip.
    #[test]
    fn test_memory_roundtrip() {
        let image = ImageData::from_f32(8, 8, 3, vec![0.5; 8 * 8 * 3]);

        let writer = HdrWriter::new();
        let bytes = writer.write_to_memory(&image).expect("Write failed");

        let reader = HdrReader::new();
        let loaded = reader.read_from_memory(&bytes).expect("Read failed");

        assert_eq!(loaded.width, 8);
        assert_eq!(loaded.height, 8);
    }

    /// Tests magic byte detection.
    #[test]
    fn test_can_read() {
        let reader = HdrReader::new();

        // Valid HDR magic
        assert!(reader.can_read(b"#?RADIANCE"));
        assert!(reader.can_read(b"#?RGBE"));

        // Invalid
        assert!(!reader.can_read(&[0x89, 0x50, 0x4E, 0x47])); // PNG
        assert!(!reader.can_read(&[0xFF, 0xD8, 0xFF])); // JPEG
    }

    /// Tests RGBE encoding/decoding.
    #[test]
    fn test_rgbe_roundtrip() {
        let test_values = [(1.0, 0.5, 0.25), (0.001, 0.002, 0.003), (100.0, 50.0, 25.0)];

        for (r, g, b) in test_values {
            let rgbe = f32_to_rgbe(r, g, b);
            let (r2, g2, b2) = rgbe_to_f32(rgbe[0], rgbe[1], rgbe[2], rgbe[3]);

            // RGBE has ~1% precision
            assert_relative_eq!(r, r2, epsilon = r * 0.02);
            assert_relative_eq!(g, g2, epsilon = g * 0.02);
            assert_relative_eq!(b, b2, epsilon = b * 0.02);
        }
    }
}
