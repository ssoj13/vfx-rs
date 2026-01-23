//! Format detection and memory estimation for streaming decisions.
//!
//! This module provides utilities to determine whether streaming I/O
//! should be used based on file size, format, and available memory.
//!
//! # Key Functions
//!
//! - [`native_bpp`] - Get bytes per pixel from file header without full decode
//! - [`estimate_memory`] - Estimate memory needed to load an image
//! - [`should_use_streaming`] - Decision helper for streaming vs memory
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::streaming::format::{estimate_memory, should_use_streaming};
//!
//! let estimate = estimate_memory("huge_scan.tif")?;
//! println!("Native: {} MB, F32: {} MB",
//!     estimate.native_bytes / 1024 / 1024,
//!     estimate.f32_bytes / 1024 / 1024);
//!
//! if should_use_streaming("huge_scan.tif", 8 * 1024 * 1024 * 1024)? {
//!     println!("Use streaming pipeline");
//! }
//! ```
//!
//! # Ported from stool-rs
//!
//! Based on `stool-rs/warper/src/format.rs` with adaptations for
//! vfx-rs format detection.

use std::path::Path;
use crate::{IoResult, IoError, PixelFormat};

// === Constants ===

/// Bytes per pixel for RGBA F32 (processing format).
pub const RGBA_F32_BPP: usize = 16;

/// Bytes per pixel for RGB F32.
pub const RGB_F32_BPP: usize = 12;

/// Bytes per pixel for RGBA U8.
pub const RGBA_U8_BPP: usize = 4;

/// Bytes per pixel for RGB U8.
pub const RGB_U8_BPP: usize = 3;

/// Bytes per pixel for RGBA U16 / F16.
pub const RGBA_U16_BPP: usize = 8;

/// Safety margin for memory estimation (1.2x = 20% overhead).
pub const MEMORY_SAFETY_MARGIN: f64 = 1.2;

/// Threshold ratio for streaming decision (if image > 80% of available, stream).
pub const STREAMING_THRESHOLD_RATIO: f64 = 0.8;

/// Memory estimate for an image file.
///
/// Provides two estimates:
/// - `native_bytes`: Memory if kept in native format (U8, U16, F16, F32)
/// - `f32_bytes`: Memory if converted to RGBA F32 for processing
///
/// Native format is more memory-efficient for storage, while F32 is
/// required for most processing operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryEstimate {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Number of channels.
    pub channels: u32,
    /// Native pixel format.
    pub format: PixelFormat,
    /// Estimated bytes in native format.
    pub native_bytes: u64,
    /// Estimated bytes if converted to RGBA F32.
    pub f32_bytes: u64,
}

impl MemoryEstimate {
    /// Creates a new memory estimate from dimensions and format.
    pub fn new(width: u32, height: u32, channels: u32, format: PixelFormat) -> Self {
        let pixels = width as u64 * height as u64;
        let native_bpp = format.bytes_per_channel() as u64 * channels as u64;
        let f32_bpp = RGBA_F32_BPP as u64;

        Self {
            width,
            height,
            channels,
            format,
            native_bytes: pixels * native_bpp,
            f32_bytes: pixels * f32_bpp,
        }
    }

    /// Returns the more memory-efficient option.
    #[inline]
    pub fn min_bytes(&self) -> u64 {
        self.native_bytes.min(self.f32_bytes)
    }

    /// Returns the larger memory requirement.
    #[inline]
    pub fn max_bytes(&self) -> u64 {
        self.native_bytes.max(self.f32_bytes)
    }

    /// Returns true if native format is more memory-efficient than F32.
    #[inline]
    pub fn native_is_smaller(&self) -> bool {
        self.native_bytes < self.f32_bytes
    }

    /// Returns native bytes as megabytes (for display).
    #[inline]
    pub fn native_mb(&self) -> f64 {
        self.native_bytes as f64 / (1024.0 * 1024.0)
    }

    /// Returns F32 bytes as megabytes (for display).
    #[inline]
    pub fn f32_mb(&self) -> f64 {
        self.f32_bytes as f64 / (1024.0 * 1024.0)
    }
}

/// Returns bytes per pixel for a given format and channel count.
///
/// # Arguments
///
/// * `format` - Pixel format (U8, U16, F16, F32, U32)
/// * `channels` - Number of channels (typically 3 or 4)
///
/// # Example
///
/// ```ignore
/// use vfx_io::streaming::format::bytes_per_pixel;
/// use vfx_io::PixelFormat;
///
/// assert_eq!(bytes_per_pixel(PixelFormat::U8, 4), 4);   // RGBA U8
/// assert_eq!(bytes_per_pixel(PixelFormat::F32, 4), 16); // RGBA F32
/// ```
#[inline]
pub fn bytes_per_pixel(format: PixelFormat, channels: u32) -> usize {
    format.bytes_per_channel() * channels as usize
}

/// Estimates memory requirements from image dimensions.
///
/// This is useful when you have metadata (from header parsing)
/// but haven't loaded the pixel data yet.
///
/// # Arguments
///
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
/// * `channels` - Number of channels
/// * `format` - Native pixel format
#[inline]
pub fn estimate_from_dims(
    width: u32,
    height: u32,
    channels: u32,
    format: PixelFormat,
) -> MemoryEstimate {
    MemoryEstimate::new(width, height, channels, format)
}

/// Checks if streaming should be used based on image size and available memory.
///
/// Uses a conservative threshold: if the image would use more than
/// [`STREAMING_THRESHOLD_RATIO`] (80%) of available memory, streaming
/// is recommended.
///
/// # Arguments
///
/// * `estimate` - Memory estimate from [`estimate_memory`] or [`estimate_from_dims`]
/// * `available_bytes` - Available RAM in bytes
///
/// # Returns
///
/// `true` if streaming is recommended, `false` if memory loading is fine.
///
/// # Example
///
/// ```ignore
/// use vfx_io::streaming::format::{estimate_from_dims, should_stream};
/// use vfx_io::PixelFormat;
///
/// let estimate = estimate_from_dims(8192, 8192, 4, PixelFormat::F32);
/// let available = 8 * 1024 * 1024 * 1024; // 8 GB
///
/// if should_stream(&estimate, available) {
///     println!("Use streaming for this large image");
/// }
/// ```
pub fn should_stream(estimate: &MemoryEstimate, available_bytes: u64) -> bool {
    let required = (estimate.f32_bytes as f64 * MEMORY_SAFETY_MARGIN) as u64;
    let threshold = (available_bytes as f64 * STREAMING_THRESHOLD_RATIO) as u64;
    required > threshold
}

/// Strategy recommendation based on memory analysis.
///
/// Used by the processing pipeline to select optimal execution strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessingStrategy {
    /// Load entire image into memory.
    /// Best for small images where overhead of streaming isn't worth it.
    InMemory,

    /// Use streaming I/O with region caching.
    /// For images that fit in memory but benefit from lazy loading.
    Streaming,

    /// Use tiled streaming with minimal cache.
    /// For very large images that exceed available memory.
    TiledStreaming,
}

impl ProcessingStrategy {
    /// Recommends a strategy based on image size and available resources.
    ///
    /// # Thresholds
    ///
    /// - `< 25%` of memory: InMemory (full load is fine)
    /// - `25-80%` of memory: Streaming (cache helps)
    /// - `> 80%` of memory: TiledStreaming (must use tiles)
    ///
    /// # Arguments
    ///
    /// * `estimate` - Memory estimate for the image
    /// * `available_bytes` - Available RAM in bytes
    pub fn recommend(estimate: &MemoryEstimate, available_bytes: u64) -> Self {
        let ratio = estimate.f32_bytes as f64 / available_bytes as f64;

        if ratio < 0.25 {
            Self::InMemory
        } else if ratio < STREAMING_THRESHOLD_RATIO {
            Self::Streaming
        } else {
            Self::TiledStreaming
        }
    }
}

/// Reads image header to estimate memory without loading pixels.
///
/// This is the primary entry point for memory estimation from files.
/// It reads only the file header to determine dimensions and format.
///
/// # Arguments
///
/// * `path` - Path to the image file
///
/// # Returns
///
/// Memory estimate on success, or error if header cannot be parsed.
///
/// # Supported Formats
///
/// - TIFF: Full support (reads IFD for dimensions and sample format)
/// - EXR: Full support (reads header for data window and channel types)
/// - PNG: Reads IHDR chunk
/// - JPEG: Reads SOF marker
/// - DPX: Reads file header
///
/// # Example
///
/// ```ignore
/// use vfx_io::streaming::format::estimate_memory;
///
/// let estimate = estimate_memory("scan.0001.exr")?;
/// println!("This {} x {} image needs {} MB as F32",
///     estimate.width, estimate.height, estimate.f32_mb());
/// ```
pub fn estimate_memory<P: AsRef<Path>>(path: P) -> IoResult<MemoryEstimate> {
    use std::io::Read;
    
    let path = path.as_ref();
    
    // Read only first 4KB for format detection and header parsing
    // This is sufficient for all supported formats (PNG/JPEG/DPX headers fit in <1KB)
    const HEADER_SIZE: usize = 4096;
    
    let mut file = std::fs::File::open(path)
        .map_err(|e| IoError::Io(e))?;
    
    let mut header = vec![0u8; HEADER_SIZE];
    let bytes_read = file.read(&mut header)
        .map_err(|e| IoError::Io(e))?;
    
    header.truncate(bytes_read);
    
    if header.len() < 8 {
        return Err(IoError::InvalidFile("File too small for header detection".into()));
    }

    // Detect format and parse header
    estimate_from_header(path, &header)
}

/// Parses header bytes to extract dimensions and format.
///
/// Internal function that handles format-specific header parsing.
fn estimate_from_header(path: &Path, header: &[u8]) -> IoResult<MemoryEstimate> {
    // TIFF: starts with II or MM
    if header.starts_with(b"II") || header.starts_with(b"MM") {
        return estimate_tiff(path);
    }

    // EXR: magic number 0x762f3101
    if header.len() >= 4 && header[0..4] == [0x76, 0x2f, 0x31, 0x01] {
        return estimate_exr(path);
    }

    // PNG: magic number
    if header.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return estimate_png(header);
    }

    // JPEG: starts with FFD8
    if header.starts_with(&[0xFF, 0xD8]) {
        return estimate_jpeg(header);
    }

    // DPX: magic SDPX or XPDS
    if header.starts_with(b"SDPX") || header.starts_with(b"XPDS") {
        return estimate_dpx(header);
    }

    Err(IoError::UnsupportedFormat(
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown")
            .to_string()
    ))
}

/// Estimates memory for TIFF files.
fn estimate_tiff(path: &Path) -> IoResult<MemoryEstimate> {
    // Use tiff crate to read just the header
    let file = std::fs::File::open(path)?;
    let mut decoder = tiff::decoder::Decoder::new(std::io::BufReader::new(file))
        .map_err(|e| IoError::DecodeError(format!("TIFF header: {}", e)))?;

    let (width, height) = decoder.dimensions()
        .map_err(|e| IoError::DecodeError(format!("TIFF dimensions: {}", e)))?;

    let color_type = decoder.colortype()
        .map_err(|e| IoError::DecodeError(format!("TIFF colortype: {}", e)))?;

    let (channels, format) = match color_type {
        tiff::ColorType::Gray(8) => (1, PixelFormat::U8),
        tiff::ColorType::Gray(16) => (1, PixelFormat::U16),
        tiff::ColorType::Gray(32) => (1, PixelFormat::F32),
        tiff::ColorType::RGB(8) => (3, PixelFormat::U8),
        tiff::ColorType::RGB(16) => (3, PixelFormat::U16),
        tiff::ColorType::RGB(32) => (3, PixelFormat::F32),
        tiff::ColorType::RGBA(8) => (4, PixelFormat::U8),
        tiff::ColorType::RGBA(16) => (4, PixelFormat::U16),
        tiff::ColorType::RGBA(32) => (4, PixelFormat::F32),
        _ => (4, PixelFormat::F32), // Fallback assumption
    };

    Ok(MemoryEstimate::new(width, height, channels, format))
}

/// Estimates memory for EXR files.
fn estimate_exr(path: &Path) -> IoResult<MemoryEstimate> {
    use vfx_exr::prelude::*;

    let meta = MetaData::read_from_file(path, false)
        .map_err(|e| IoError::DecodeError(format!("EXR header: {}", e)))?;

    // Use first header for dimensions
    let header = meta.headers.first()
        .ok_or_else(|| IoError::DecodeError("EXR has no headers".into()))?;

    let data_window = header.shared_attributes.display_window;
    let width = (data_window.size.x()) as u32;
    let height = (data_window.size.y()) as u32;

    // Count channels and detect sample type
    let channels = header.channels.list.len() as u32;
    let format = if header.channels.list.iter().any(|c| {
        matches!(c.sample_type, SampleType::F32)
    }) {
        PixelFormat::F32
    } else {
        PixelFormat::F16
    };

    Ok(MemoryEstimate::new(width, height, channels.max(4), format))
}

/// Estimates memory for PNG files from header bytes.
fn estimate_png(header: &[u8]) -> IoResult<MemoryEstimate> {
    // PNG IHDR is at offset 8 (after signature), length 4, type 4, then data
    // IHDR format: width (4), height (4), bit depth (1), color type (1), ...
    const IHDR_OFFSET: usize = 16; // 8 (sig) + 4 (len) + 4 (type)

    if header.len() < IHDR_OFFSET + 10 {
        return Err(IoError::InvalidFile("PNG header too short".into()));
    }

    let width = u32::from_be_bytes([
        header[IHDR_OFFSET],
        header[IHDR_OFFSET + 1],
        header[IHDR_OFFSET + 2],
        header[IHDR_OFFSET + 3],
    ]);
    let height = u32::from_be_bytes([
        header[IHDR_OFFSET + 4],
        header[IHDR_OFFSET + 5],
        header[IHDR_OFFSET + 6],
        header[IHDR_OFFSET + 7],
    ]);
    let bit_depth = header[IHDR_OFFSET + 8];
    let color_type = header[IHDR_OFFSET + 9];

    let channels = match color_type {
        0 => 1, // Grayscale
        2 => 3, // RGB
        3 => 1, // Indexed (palette)
        4 => 2, // Grayscale + Alpha
        6 => 4, // RGBA
        _ => 4, // Fallback
    };

    let format = if bit_depth > 8 {
        PixelFormat::U16
    } else {
        PixelFormat::U8
    };

    Ok(MemoryEstimate::new(width, height, channels, format))
}

/// Estimates memory for JPEG files from header bytes.
fn estimate_jpeg(header: &[u8]) -> IoResult<MemoryEstimate> {
    // Find SOF0 or SOF2 marker for dimensions
    // Format: FF C0/C2, length (2), precision (1), height (2), width (2), channels (1)
    let mut i = 2; // Skip FFD8

    while i + 4 < header.len() {
        if header[i] != 0xFF {
            i += 1;
            continue;
        }

        let marker = header[i + 1];
        let length = u16::from_be_bytes([header[i + 2], header[i + 3]]) as usize;

        // SOF0 (baseline) or SOF2 (progressive)
        if marker == 0xC0 || marker == 0xC2 {
            if i + 9 < header.len() {
                let height = u16::from_be_bytes([header[i + 5], header[i + 6]]) as u32;
                let width = u16::from_be_bytes([header[i + 7], header[i + 8]]) as u32;
                let channels = header[i + 9] as u32;

                return Ok(MemoryEstimate::new(width, height, channels, PixelFormat::U8));
            }
        }

        i += 2 + length;
    }

    Err(IoError::InvalidFile("JPEG: SOF marker not found".into()))
}

/// Estimates memory for DPX files from header bytes.
fn estimate_dpx(header: &[u8]) -> IoResult<MemoryEstimate> {
    // DPX header: magic (4), offset (4), version (8), file_size (4), ...
    // Image header at offset 768: orientation (2), elements (2), width (4), height (4)
    const IMAGE_HEADER_OFFSET: usize = 768;
    const PIXELS_PER_LINE_OFFSET: usize = IMAGE_HEADER_OFFSET + 4;
    const LINES_PER_IMAGE_OFFSET: usize = IMAGE_HEADER_OFFSET + 8;

    if header.len() < LINES_PER_IMAGE_OFFSET + 4 {
        return Err(IoError::InvalidFile("DPX header too short".into()));
    }

    let is_big_endian = header.starts_with(b"SDPX");

    let width = if is_big_endian {
        u32::from_be_bytes([
            header[PIXELS_PER_LINE_OFFSET],
            header[PIXELS_PER_LINE_OFFSET + 1],
            header[PIXELS_PER_LINE_OFFSET + 2],
            header[PIXELS_PER_LINE_OFFSET + 3],
        ])
    } else {
        u32::from_le_bytes([
            header[PIXELS_PER_LINE_OFFSET],
            header[PIXELS_PER_LINE_OFFSET + 1],
            header[PIXELS_PER_LINE_OFFSET + 2],
            header[PIXELS_PER_LINE_OFFSET + 3],
        ])
    };

    let height = if is_big_endian {
        u32::from_be_bytes([
            header[LINES_PER_IMAGE_OFFSET],
            header[LINES_PER_IMAGE_OFFSET + 1],
            header[LINES_PER_IMAGE_OFFSET + 2],
            header[LINES_PER_IMAGE_OFFSET + 3],
        ])
    } else {
        u32::from_le_bytes([
            header[LINES_PER_IMAGE_OFFSET],
            header[LINES_PER_IMAGE_OFFSET + 1],
            header[LINES_PER_IMAGE_OFFSET + 2],
            header[LINES_PER_IMAGE_OFFSET + 3],
        ])
    };

    // DPX is typically 10-bit RGB, stored as U16
    Ok(MemoryEstimate::new(width, height, 3, PixelFormat::U16))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_estimate() {
        // 4K RGBA U8
        let est = MemoryEstimate::new(3840, 2160, 4, PixelFormat::U8);
        assert_eq!(est.native_bytes, 3840 * 2160 * 4);
        assert_eq!(est.f32_bytes, 3840 * 2160 * 16);
        assert!(est.native_is_smaller());
    }

    #[test]
    fn test_memory_estimate_f32() {
        // 4K RGBA F32 - native and f32 should be equal
        let est = MemoryEstimate::new(3840, 2160, 4, PixelFormat::F32);
        assert_eq!(est.native_bytes, 3840 * 2160 * 16);
        assert_eq!(est.f32_bytes, 3840 * 2160 * 16);
    }

    #[test]
    fn test_should_stream() {
        let small = MemoryEstimate::new(1920, 1080, 4, PixelFormat::U8);
        // 32K x 32K RGBA F32 = 16 GB - should exceed 8 GB threshold
        let large = MemoryEstimate::new(32768, 32768, 4, PixelFormat::F32);

        let available = 8 * 1024 * 1024 * 1024; // 8 GB

        assert!(!should_stream(&small, available));
        assert!(should_stream(&large, available));
    }

    #[test]
    fn test_processing_strategy() {
        let available = 8 * 1024 * 1024 * 1024u64; // 8 GB

        // Small image: ~132 MB (< 25% of 8 GB = 2 GB)
        let small = MemoryEstimate::new(1920, 1080, 4, PixelFormat::F32);
        assert_eq!(ProcessingStrategy::recommend(&small, available), ProcessingStrategy::InMemory);

        // Medium image: 16K x 16K = 4 GB (~50% of 8 GB, between 25% and 80%)
        let medium = MemoryEstimate::new(16384, 16384, 4, PixelFormat::F32);
        assert_eq!(ProcessingStrategy::recommend(&medium, available), ProcessingStrategy::Streaming);

        // Large image: 32K x 32K = 16 GB (> 80% of 8 GB)
        let large = MemoryEstimate::new(32768, 32768, 4, PixelFormat::F32);
        assert_eq!(ProcessingStrategy::recommend(&large, available), ProcessingStrategy::TiledStreaming);
    }

    #[test]
    fn test_bytes_per_pixel() {
        assert_eq!(bytes_per_pixel(PixelFormat::U8, 4), 4);
        assert_eq!(bytes_per_pixel(PixelFormat::U16, 4), 8);
        assert_eq!(bytes_per_pixel(PixelFormat::F16, 4), 8);
        assert_eq!(bytes_per_pixel(PixelFormat::F32, 4), 16);
        assert_eq!(bytes_per_pixel(PixelFormat::U32, 4), 16);
    }
}
