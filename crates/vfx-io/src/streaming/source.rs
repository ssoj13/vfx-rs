//! Memory-backed streaming source.
//!
//! Provides a [`StreamingSource`] implementation for images that are
//! fully loaded into memory. Used as fallback for formats that don't
//! support true random access (PNG, JPEG) or for small images where
//! streaming overhead isn't worth it.
//!
//! # Memory Optimization
//!
//! Unlike naive approaches, MemorySource keeps the image in its
//! **native format** (U8, U16, F16, F32) rather than converting
//! to F32 upfront. Conversion happens on-demand in `read_region()`.
//!
//! This saves significant memory for 8-bit images:
//! - 4K RGBA U8: 32 MB (vs 128 MB as F32)
//! - 8K RGBA U8: 128 MB (vs 512 MB as F32)
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::streaming::MemorySource;
//! use vfx_io::read;
//!
//! let image = read("small_image.png")?;
//! let source = MemorySource::new(image);
//!
//! // Now can use as StreamingSource
//! let region = source.read_region(0, 0, 256, 256)?;
//! ```

use std::path::{Path, PathBuf};

use crate::{ImageData, IoResult, PixelData, PixelFormat};
use super::traits::{Region, StreamingSource};

// === Constants ===

/// Maximum value for 8-bit unsigned normalization.
const U8_MAX_F32: f32 = u8::MAX as f32;
/// Maximum value for 16-bit unsigned normalization.
const U16_MAX_F32: f32 = u16::MAX as f32;
/// Default alpha value for opaque pixels.
const ALPHA_OPAQUE: f32 = 1.0;
/// Default alpha value for transparent pixels.
const ALPHA_TRANSPARENT: f32 = 0.0;
/// Number of channels in RGBA output.
const RGBA_CHANNELS: usize = 4;
/// Minimum channels for alpha presence.
const MIN_ALPHA_CHANNELS: usize = 4;

/// Memory-backed streaming source.
///
/// Wraps an [`ImageData`] to provide the [`StreamingSource`] interface.
/// Keeps image in native format for memory efficiency.
///
/// # Thread Safety
///
/// This type is `Send` but not `Sync`. For concurrent access,
/// each thread should have its own clone or use synchronization.
#[derive(Debug, Clone)]
pub struct MemorySource {
    /// The underlying image data (kept in native format).
    image: ImageData,
}

impl MemorySource {
    /// Creates a new memory source from image data.
    ///
    /// The image is stored as-is without format conversion.
    /// Conversion to RGBA F32 happens during `read_region()`.
    #[inline]
    pub fn new(image: ImageData) -> Self {
        Self { image }
    }

    /// Returns reference to the underlying image.
    #[inline]
    pub fn image(&self) -> &ImageData {
        &self.image
    }

    /// Consumes the source and returns the underlying image.
    #[inline]
    pub fn into_inner(self) -> ImageData {
        self.image
    }

    /// Extracts a region as RGBA F32 from the native format image.
    ///
    /// Handles boundary clamping and channel expansion (RGB -> RGBA).
    fn extract_region(&self, x: u32, y: u32, w: u32, h: u32) -> Region {
        let (img_w, img_h) = (self.image.width, self.image.height);
        let channels = self.image.channels as usize;
        
        // Output buffer (RGBA F32)
        let mut data = vec![ALPHA_TRANSPARENT; (w * h) as usize * RGBA_CHANNELS];

        // Clamp region to image bounds
        let x_end = (x + w).min(img_w);
        let y_end = (y + h).min(img_h);
        let x_start = x.min(img_w);
        let y_start = y.min(img_h);

        // Early return if completely out of bounds
        if x_start >= x_end || y_start >= y_end {
            return Region::new(x, y, w, h, data);
        }

        // Copy pixels with format conversion
        for src_y in y_start..y_end {
            for src_x in x_start..x_end {
                let src_idx = ((src_y * img_w + src_x) as usize) * channels;
                let dst_x = src_x - x;
                let dst_y = src_y - y;
                let dst_idx = ((dst_y * w + dst_x) as usize) * RGBA_CHANNELS;

                // Read source pixel and convert to F32
                let rgba = self.read_pixel_f32(src_idx, channels);
                data[dst_idx..dst_idx + RGBA_CHANNELS].copy_from_slice(&rgba);
            }
        }

        Region::new(x, y, w, h, data)
    }

    /// Reads a single pixel at the given index and converts to RGBA F32.
    #[inline]
    fn read_pixel_f32(&self, idx: usize, channels: usize) -> [f32; 4] {
        match &self.image.data {
            PixelData::U8(data) => {
                let r = data.get(idx).copied().unwrap_or(0) as f32 / U8_MAX_F32;
                let g = data.get(idx + 1.min(channels - 1)).copied().unwrap_or(0) as f32 / U8_MAX_F32;
                let b = data.get(idx + 2.min(channels - 1)).copied().unwrap_or(0) as f32 / U8_MAX_F32;
                let a = if channels >= MIN_ALPHA_CHANNELS {
                    data.get(idx + 3).copied().unwrap_or(u8::MAX) as f32 / U8_MAX_F32
                } else {
                    ALPHA_OPAQUE
                };
                [r, g, b, a]
            }
            PixelData::U16(data) => {
                let r = data.get(idx).copied().unwrap_or(0) as f32 / U16_MAX_F32;
                let g = data.get(idx + 1.min(channels - 1)).copied().unwrap_or(0) as f32 / U16_MAX_F32;
                let b = data.get(idx + 2.min(channels - 1)).copied().unwrap_or(0) as f32 / U16_MAX_F32;
                let a = if channels >= MIN_ALPHA_CHANNELS {
                    data.get(idx + 3).copied().unwrap_or(u16::MAX) as f32 / U16_MAX_F32
                } else {
                    ALPHA_OPAQUE
                };
                [r, g, b, a]
            }
            PixelData::F32(data) => {
                let r = data.get(idx).copied().unwrap_or(ALPHA_TRANSPARENT);
                let g = data.get(idx + 1.min(channels - 1)).copied().unwrap_or(ALPHA_TRANSPARENT);
                let b = data.get(idx + 2.min(channels - 1)).copied().unwrap_or(ALPHA_TRANSPARENT);
                let a = if channels >= MIN_ALPHA_CHANNELS {
                    data.get(idx + 3).copied().unwrap_or(ALPHA_OPAQUE)
                } else {
                    ALPHA_OPAQUE
                };
                [r, g, b, a]
            }
            PixelData::U32(data) => {
                // U32 is typically used for IDs, not colors - just cast
                let r = data.get(idx).copied().unwrap_or(0) as f32;
                let g = data.get(idx + 1.min(channels - 1)).copied().unwrap_or(0) as f32;
                let b = data.get(idx + 2.min(channels - 1)).copied().unwrap_or(0) as f32;
                let a = if channels >= MIN_ALPHA_CHANNELS {
                    data.get(idx + 3).copied().unwrap_or(1) as f32
                } else {
                    ALPHA_OPAQUE
                };
                [r, g, b, a]
            }
        }
    }
}

impl StreamingSource for MemorySource {
    fn dimensions(&self) -> (u32, u32) {
        (self.image.width, self.image.height)
    }

    fn read_region(&mut self, x: u32, y: u32, w: u32, h: u32) -> IoResult<Region> {
        Ok(self.extract_region(x, y, w, h))
    }

    fn supports_random_access(&self) -> bool {
        // Memory source has "random access" in the sense that any region
        // can be read, but it's not streaming-friendly since the whole
        // image is in memory anyway.
        false
    }

    fn native_tile_size(&self) -> Option<(u32, u32)> {
        // Memory source has no native tiling
        None
    }

    fn native_format(&self) -> PixelFormat {
        self.image.format
    }

    fn channels(&self) -> u32 {
        self.image.channels
    }
}

/// Memory-backed streaming output.
///
/// Accumulates written regions into an in-memory buffer.
/// Used for formats that don't support incremental writing.
#[derive(Debug)]
pub struct MemoryOutput {
    /// Output dimensions.
    width: u32,
    height: u32,
    /// Accumulated pixel data (RGBA F32).
    data: Vec<f32>,
    /// Target pixel format for finalization.
    target_format: PixelFormat,
}

impl MemoryOutput {
    /// Creates a new memory output with given dimensions.
    ///
    /// Buffer is initialized to transparent black.
    pub fn new(width: u32, height: u32, format: PixelFormat) -> Self {
        Self {
            width,
            height,
            data: vec![ALPHA_TRANSPARENT; (width * height) as usize * RGBA_CHANNELS],
            target_format: format,
        }
    }

    /// Returns reference to accumulated data.
    pub fn data(&self) -> &[f32] {
        &self.data
    }

    /// Converts to ImageData with the target format.
    pub fn to_image_data(&self) -> ImageData {
        let img = ImageData::from_f32(self.width, self.height, 4, self.data.clone());
        img.convert_to(self.target_format)
    }
}

impl super::traits::StreamingOutput for MemoryOutput {
    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn write_region(&mut self, region: &Region) -> IoResult<()> {
        let img_w = self.width;
        
        for local_y in 0..region.height {
            for local_x in 0..region.width {
                let img_x = region.x + local_x;
                let img_y = region.y + local_y;
                
                // Skip out-of-bounds pixels
                if img_x >= self.width || img_y >= self.height {
                    continue;
                }
                
                let src_idx = ((local_y * region.width + local_x) as usize) * RGBA_CHANNELS;
                let dst_idx = ((img_y * img_w + img_x) as usize) * RGBA_CHANNELS;
                
                self.data[dst_idx..dst_idx + RGBA_CHANNELS]
                    .copy_from_slice(&region.data[src_idx..src_idx + RGBA_CHANNELS]);
            }
        }
        
        Ok(())
    }

    fn finalize(self: Box<Self>) -> IoResult<()> {
        // Memory output is already complete - nothing to finalize
        Ok(())
    }

    fn supports_random_write(&self) -> bool {
        true // Memory buffer supports any write order
    }

    fn native_tile_size(&self) -> Option<(u32, u32)> {
        None
    }
}

/// File-backed streaming output.
///
/// Wraps `MemoryOutput` and writes to a file on `finalize()`.
/// Used for formats without native streaming support.
#[derive(Debug)]
pub struct FileOutput {
    /// Output file path.
    path: PathBuf,
    /// Memory buffer.
    inner: MemoryOutput,
}

impl FileOutput {
    /// Creates a new file output with given dimensions.
    pub fn new<P: AsRef<Path>>(path: P, width: u32, height: u32, format: PixelFormat) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            inner: MemoryOutput::new(width, height, format),
        }
    }
}

impl super::traits::StreamingOutput for FileOutput {
    fn dimensions(&self) -> (u32, u32) {
        self.inner.dimensions()
    }

    fn write_region(&mut self, region: &Region) -> IoResult<()> {
        self.inner.write_region(region)
    }

    fn finalize(self: Box<Self>) -> IoResult<()> {
        let image = self.inner.to_image_data();
        crate::write(&self.path, &image)
    }

    fn supports_random_write(&self) -> bool {
        true
    }

    fn native_tile_size(&self) -> Option<(u32, u32)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_source_basic() {
        // Create a 4x4 red image
        let mut data = vec![0.0f32; 4 * 4 * 4];
        for i in 0..16 {
            data[i * 4] = 1.0;     // R
            data[i * 4 + 3] = 1.0; // A
        }
        let image = ImageData::from_f32(4, 4, 4, data);
        let mut source = MemorySource::new(image);

        assert_eq!(source.dimensions(), (4, 4));
        assert!(!source.supports_random_access());
        assert_eq!(source.native_format(), PixelFormat::F32);

        let region = source.read_region(0, 0, 2, 2).unwrap();
        assert_eq!(region.width, 2);
        assert_eq!(region.height, 2);
        
        // Check first pixel is red
        let px = region.pixel(0, 0);
        assert_eq!(px, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_memory_source_out_of_bounds() {
        let image = ImageData::from_f32(4, 4, 4, vec![0.5f32; 64]);
        let mut source = MemorySource::new(image);

        // Request region partially outside image
        let region = source.read_region(2, 2, 4, 4).unwrap();
        
        // Should get 4x4 region, but only 2x2 has valid data
        assert_eq!(region.width, 4);
        assert_eq!(region.height, 4);
        
        // Out-of-bounds pixel should be zero
        let px = region.pixel(3, 3);
        assert_eq!(px, [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_memory_output() {
        use super::super::traits::StreamingOutput;
        
        let mut output = Box::new(MemoryOutput::new(4, 4, PixelFormat::F32));
        
        // Write a 2x2 red region at (1, 1)
        let mut region_data = vec![0.0f32; 2 * 2 * 4];
        for i in 0..4 {
            region_data[i * 4] = 1.0;     // R
            region_data[i * 4 + 3] = 1.0; // A
        }
        let region = Region::new(1, 1, 2, 2, region_data);
        
        output.write_region(&region).unwrap();
        
        // Check pixel at (1, 1)
        let idx = ((1 * 4 + 1) * 4) as usize;
        assert_eq!(output.data()[idx], 1.0); // R
        
        output.finalize().unwrap();
    }
}
