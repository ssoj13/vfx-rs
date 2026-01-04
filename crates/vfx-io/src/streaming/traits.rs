//! Core streaming I/O traits.
//!
//! This module defines the fundamental abstractions for streaming image I/O,
//! enabling processing of images larger than available RAM.
//!
//! # Architecture
//!
//! ```text
//! +-------------------+       +--------------------+
//! | StreamingSource   |       | StreamingOutput    |
//! +-------------------+       +--------------------+
//!         ^                           ^
//!         |                           |
//! +-------+-------+           +-------+-------+
//! | TiffSource    |           | TiffOutput    |
//! | ExrSource     |           | ExrOutput     |
//! | MemorySource  |           | MemoryOutput  |
//! +---------------+           +---------------+
//! ```
//!
//! # Key Concepts
//!
//! - **Random Access**: Read/write arbitrary regions without loading entire image
//! - **Tile Alignment**: Efficient access when aligned to native tile boundaries
//! - **Memory Efficiency**: Only the requested region is held in memory
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::streaming::{StreamingSource, open_streaming};
//!
//! // Open for streaming (auto-detects format)
//! let mut source = open_streaming("huge_image.tif")?;
//!
//! // Read only what we need
//! let region = source.read_region(1000, 1000, 512, 512)?;
//! ```
//!
//! # Ported from stool-rs
//!
//! Based on `stool-rs/warper/src/backend/streaming_io.rs` with adaptations
//! for vfx-rs multi-format PixelFormat support.

use crate::{ImageData, IoResult, PixelFormat};

// === Constants ===

/// Number of channels in RGBA output format.
pub const RGBA_CHANNELS: u32 = 4;
/// Default cache size for region caching (number of regions).
pub const DEFAULT_CACHE_SIZE: usize = 16;

/// Region read from a streaming source.
///
/// Contains pixel data for a rectangular region of the image.
/// Data is always in RGBA F32 format for processing uniformity,
/// with original format info preserved for write-back optimization.
#[derive(Debug, Clone)]
pub struct Region {
    /// X offset of region origin in source image.
    pub x: u32,
    /// Y offset of region origin in source image.
    pub y: u32,
    /// Region width in pixels.
    pub width: u32,
    /// Region height in pixels.
    pub height: u32,
    /// Pixel data in RGBA F32 interleaved format.
    /// Layout: [R0, G0, B0, A0, R1, G1, B1, A1, ...]
    pub data: Vec<f32>,
}

impl Region {
    /// Creates a new region with the given bounds and data.
    #[inline]
    pub fn new(x: u32, y: u32, width: u32, height: u32, data: Vec<f32>) -> Self {
        debug_assert_eq!(
            data.len(),
            (width * height * RGBA_CHANNELS) as usize,
            "Region data size mismatch"
        );
        Self { x, y, width, height, data }
    }

    /// Creates a region filled with zeros (transparent black).
    pub fn zeroed(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            data: vec![0.0; (width * height * RGBA_CHANNELS) as usize],
        }
    }

    /// Returns the number of pixels in this region.
    #[inline]
    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }

    /// Returns pixel value at local coordinates (relative to region origin).
    ///
    /// # Panics
    ///
    /// Panics if coordinates are out of bounds.
    #[inline]
    pub fn pixel(&self, local_x: u32, local_y: u32) -> [f32; 4] {
        let idx = ((local_y * self.width + local_x) * RGBA_CHANNELS) as usize;
        [
            self.data[idx],
            self.data[idx + 1],
            self.data[idx + 2],
            self.data[idx + 3],
        ]
    }

    /// Sets pixel value at local coordinates.
    #[inline]
    pub fn set_pixel(&mut self, local_x: u32, local_y: u32, rgba: [f32; 4]) {
        let idx = ((local_y * self.width + local_x) * RGBA_CHANNELS) as usize;
        self.data[idx..idx + RGBA_CHANNELS as usize].copy_from_slice(&rgba);
    }

    /// Converts region to ImageData for compatibility with existing APIs.
    pub fn to_image_data(&self) -> ImageData {
        ImageData::from_f32(self.width, self.height, RGBA_CHANNELS, self.data.clone())
    }

    /// Applies a transform function to the pixel data.
    ///
    /// The transform receives a mutable reference to the entire data buffer.
    /// This is the primary way to integrate color processing with streaming.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use vfx_color::{ColorProcessor, Pipeline};
    ///
    /// let mut region = source.read_region(0, 0, 512, 512)?;
    ///
    /// // Apply color transform
    /// let mut proc = ColorProcessor::new();
    /// let pipeline = Pipeline::srgb_to_linear();
    /// region.transform(|data| {
    ///     proc.apply_buffer(&pipeline, data, 512, 512);
    /// });
    /// ```
    #[inline]
    pub fn transform<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut [f32]),
    {
        f(&mut self.data);
    }

    /// Applies a per-pixel transform function.
    ///
    /// Each pixel is passed as `[R, G, B, A]` and should be modified in place.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Invert colors
    /// region.transform_pixels(|rgba| {
    ///     rgba[0] = 1.0 - rgba[0];
    ///     rgba[1] = 1.0 - rgba[1];
    ///     rgba[2] = 1.0 - rgba[2];
    /// });
    /// ```
    #[inline]
    pub fn transform_pixels<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut [f32; 4]),
    {
        for chunk in self.data.chunks_exact_mut(RGBA_CHANNELS as usize) {
            let pixel: &mut [f32; 4] = chunk.try_into().unwrap();
            f(pixel);
        }
    }
}

/// Streaming image source trait.
///
/// Enables random-access reading of image regions without loading
/// the entire image into memory. Essential for processing images
/// larger than available RAM.
///
/// # Implementation Notes
///
/// - `read_region` may be called from multiple threads (hence `Send`)
/// - Implementations should cache file handles / decoders internally
/// - Return alpha=1.0 for RGB images (no alpha channel)
///
/// # Format Support
///
/// | Format | Random Access | Native Tiles |
/// |--------|---------------|--------------|
/// | TIFF   | Yes (tiled)   | Yes          |
/// | EXR    | Yes (scanline)| Partial      |
/// | PNG    | No (fallback) | No           |
/// | JPEG   | No (fallback) | No           |
pub trait StreamingSource: Send {
    /// Returns image dimensions (width, height).
    fn dimensions(&self) -> (u32, u32);

    /// Reads a rectangular region from the image.
    ///
    /// # Arguments
    ///
    /// * `x`, `y` - Top-left corner of the region in image coordinates
    /// * `w`, `h` - Width and height of the region
    ///
    /// # Returns
    ///
    /// Region containing RGBA F32 pixel data for the requested area.
    /// If region extends beyond image bounds, out-of-bounds pixels
    /// are filled with zeros (transparent black).
    ///
    /// # Errors
    ///
    /// Returns error if the underlying file read fails.
    fn read_region(&mut self, x: u32, y: u32, w: u32, h: u32) -> IoResult<Region>;

    /// Returns true if format supports efficient random access.
    ///
    /// - `true`: Format can read arbitrary regions efficiently (TIFF, EXR)
    /// - `false`: Entire image must be loaded first (PNG, JPEG)
    ///
    /// When `false`, the streaming infrastructure falls back to
    /// MemorySource which loads the entire image upfront.
    fn supports_random_access(&self) -> bool;

    /// Returns native tile size if format is tiled.
    ///
    /// For tiled formats (TIFF), reading aligned to tile boundaries
    /// is significantly faster. Returns `None` for scanline formats.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some((tw, th)) = source.native_tile_size() {
    ///     // Align reads to tile boundaries for efficiency
    ///     let aligned_x = (x / tw) * tw;
    ///     let aligned_y = (y / th) * th;
    /// }
    /// ```
    fn native_tile_size(&self) -> Option<(u32, u32)>;

    /// Returns the native pixel format of the source.
    ///
    /// Used for optimal memory estimation and avoiding unnecessary
    /// format conversions during write-back.
    fn native_format(&self) -> PixelFormat;

    /// Returns number of channels in source (3=RGB, 4=RGBA).
    fn channels(&self) -> u32;

    /// Hint for optimal region cache size.
    ///
    /// Returns recommended number of regions to cache based on
    /// access patterns and memory constraints. Default is 16.
    fn recommended_cache_size(&self) -> usize {
        DEFAULT_CACHE_SIZE
    }
}

/// Streaming image output trait.
///
/// Enables writing image regions to disk without holding the
/// entire output in memory. Essential for generating large images.
///
/// # Implementation Notes
///
/// - Regions may be written in any order (not necessarily top-to-bottom)
/// - Implementations must handle overlapping writes (later wins)
/// - `finalize()` must be called to flush and close the file
pub trait StreamingOutput: Send {
    /// Returns output dimensions (width, height).
    fn dimensions(&self) -> (u32, u32);

    /// Writes a region to the output.
    ///
    /// # Arguments
    ///
    /// * `region` - Region containing pixel data and position
    ///
    /// # Errors
    ///
    /// Returns error if the underlying file write fails.
    fn write_region(&mut self, region: &Region) -> IoResult<()>;

    /// Finalizes the output, flushing all buffered data.
    ///
    /// Must be called to ensure all data is written and the file
    /// is properly closed. After calling this, the output is consumed.
    ///
    /// # Errors
    ///
    /// Returns error if final flush or file close fails.
    fn finalize(self: Box<Self>) -> IoResult<()>;

    /// Returns true if format supports random-access writing.
    ///
    /// - `true`: Regions can be written in any order (TIFF, EXR)
    /// - `false`: Must buffer until finalize (PNG, JPEG)
    fn supports_random_write(&self) -> bool;

    /// Returns native tile size for optimal write alignment.
    fn native_tile_size(&self) -> Option<(u32, u32)>;
}

/// Box type for streaming source (for dynamic dispatch).
pub type BoxedSource = Box<dyn StreamingSource>;

/// Box type for streaming output (for dynamic dispatch).
pub type BoxedOutput = Box<dyn StreamingOutput>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_creation() {
        let data = vec![0.5f32; 16 * 16 * 4];
        let region = Region::new(100, 200, 16, 16, data);
        
        assert_eq!(region.x, 100);
        assert_eq!(region.y, 200);
        assert_eq!(region.width, 16);
        assert_eq!(region.height, 16);
        assert_eq!(region.pixel_count(), 256);
    }

    #[test]
    fn test_region_pixel_access() {
        let mut region = Region::zeroed(0, 0, 4, 4);
        
        region.set_pixel(1, 2, [1.0, 0.5, 0.25, 1.0]);
        let px = region.pixel(1, 2);
        
        assert_eq!(px, [1.0, 0.5, 0.25, 1.0]);
    }

    #[test]
    fn test_region_transform() {
        let data = vec![0.5f32; 2 * 2 * 4];
        let mut region = Region::new(0, 0, 2, 2, data);
        
        // Double all values
        region.transform(|data| {
            for v in data.iter_mut() {
                *v *= 2.0;
            }
        });
        
        let px = region.pixel(0, 0);
        assert_eq!(px, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_region_transform_pixels() {
        let mut region = Region::zeroed(0, 0, 2, 2);
        region.set_pixel(0, 0, [0.5, 0.3, 0.2, 1.0]);
        
        // Invert RGB, keep alpha
        region.transform_pixels(|rgba| {
            rgba[0] = 1.0 - rgba[0];
            rgba[1] = 1.0 - rgba[1];
            rgba[2] = 1.0 - rgba[2];
        });
        
        let px = region.pixel(0, 0);
        assert!((px[0] - 0.5).abs() < 0.001);
        assert!((px[1] - 0.7).abs() < 0.001);
        assert!((px[2] - 0.8).abs() < 0.001);
    }
}
