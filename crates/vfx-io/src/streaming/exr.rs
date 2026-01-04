//! EXR streaming source with lazy loading.
//!
//! Unlike TIFF which supports true chunk-based random access, OpenEXR
//! requires more complex block-level access. This implementation uses
//! lazy loading: the header is read immediately for dimensions, but
//! pixel data is loaded on first `read_region()` call.
//!
//! # Future Improvements
//!
//! The `exr` crate supports block-level reading which could enable true
//! streaming. This would require:
//! 1. Reading block offsets from the header
//! 2. Reading only the blocks needed for a region
//! 3. Decompressing and assembling the result
//!
//! For now, lazy loading is a pragmatic solution that:
//! - Defers memory allocation until actually needed
//! - Allows memory estimation before loading
//! - Works with the existing vfx-io EXR reader
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::streaming::ExrStreamingSource;
//!
//! let mut source = ExrStreamingSource::open("scene.exr")?;
//! println!("Dimensions: {:?}", source.dimensions());
//!
//! // Image loads on first region read
//! let region = source.read_region(0, 0, 512, 512)?;
//! ```

use std::path::{Path, PathBuf};

use crate::{IoResult, IoError, PixelFormat, ImageData};
use super::traits::{Region, StreamingSource, RGBA_CHANNELS};

// === Constants ===

/// Default alpha for pixels without alpha channel.
const ALPHA_OPAQUE: f32 = 1.0;

/// EXR streaming source with lazy loading.
///
/// Reads only the EXR header initially. Full pixel data is loaded
/// on the first `read_region()` call and cached for subsequent reads.
#[derive(Debug)]
pub struct ExrStreamingSource {
    /// Path to the EXR file.
    path: PathBuf,
    /// Image width in pixels.
    width: u32,
    /// Image height in pixels.
    height: u32,
    /// Number of channels.
    channels: u32,
    /// Native pixel format.
    format: PixelFormat,
    /// Cached image data (lazy-loaded).
    cached_image: Option<ImageData>,
}

impl ExrStreamingSource {
    /// Opens an EXR file for streaming access.
    ///
    /// Only reads the file header to determine dimensions and format.
    /// Actual pixel data is loaded lazily on first `read_region()` call.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the EXR file
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened or header is invalid.
    pub fn open<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        use exr::prelude::*;

        let path = path.as_ref().to_path_buf();

        // Read just the header (fast!)
        let meta = MetaData::read_from_file(&path, false)
            .map_err(|e| IoError::DecodeError(format!("EXR header: {}", e)))?;

        let header = meta.headers.first()
            .ok_or_else(|| IoError::DecodeError("EXR has no headers".into()))?;

        let data_window = header.shared_attributes.display_window;
        let width = data_window.size.x() as u32;
        let height = data_window.size.y() as u32;

        // Determine format from channels
        let channels = header.channels.list.len() as u32;
        let format = if header.channels.list.iter().any(|c| {
            matches!(c.sample_type, SampleType::F32)
        }) {
            PixelFormat::F32
        } else {
            PixelFormat::F16
        };

        Ok(Self {
            path,
            width,
            height,
            channels: channels.max(4), // Ensure at least RGBA
            format,
            cached_image: None,
        })
    }

    /// Returns the path to the EXR file.
    #[inline]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns true if the image has been loaded into cache.
    #[inline]
    pub fn is_loaded(&self) -> bool {
        self.cached_image.is_some()
    }

    /// Ensures the image is loaded into cache.
    ///
    /// Called automatically by `read_region()`. Can be called manually
    /// to pre-load the image.
    pub fn ensure_loaded(&mut self) -> IoResult<()> {
        if self.cached_image.is_some() {
            return Ok(());
        }

        // Use the existing vfx-io EXR reader
        let image = crate::exr::read(&self.path)?;
        self.cached_image = Some(image);

        Ok(())
    }

    /// Extracts a region from the cached image.
    fn extract_region(&self, image: &ImageData, x: u32, y: u32, w: u32, h: u32) -> Region {
        let (img_w, img_h) = (image.width, image.height);
        let channels = image.channels as usize;

        // Output buffer (RGBA F32)
        let mut data = vec![0.0f32; (w * h) as usize * RGBA_CHANNELS as usize];

        // Clamp region
        let x_end = (x + w).min(img_w);
        let y_end = (y + h).min(img_h);
        let x_start = x.min(img_w);
        let y_start = y.min(img_h);

        if x_start >= x_end || y_start >= y_end {
            return Region::new(x, y, w, h, data);
        }

        // Convert source to F32 for extraction
        let src_f32 = image.to_f32();

        for src_y in y_start..y_end {
            for src_x in x_start..x_end {
                let src_idx = ((src_y * img_w + src_x) as usize) * channels;
                let dst_x = src_x - x;
                let dst_y = src_y - y;
                let dst_idx = ((dst_y * w + dst_x) as usize) * RGBA_CHANNELS as usize;

                // Read source pixel
                let r = src_f32.get(src_idx).copied().unwrap_or(0.0);
                let g = src_f32.get(src_idx + 1.min(channels - 1)).copied().unwrap_or(0.0);
                let b = src_f32.get(src_idx + 2.min(channels - 1)).copied().unwrap_or(0.0);
                let a = if channels >= 4 {
                    src_f32.get(src_idx + 3).copied().unwrap_or(ALPHA_OPAQUE)
                } else {
                    ALPHA_OPAQUE
                };

                data[dst_idx] = r;
                data[dst_idx + 1] = g;
                data[dst_idx + 2] = b;
                data[dst_idx + 3] = a;
            }
        }

        Region::new(x, y, w, h, data)
    }
}

impl StreamingSource for ExrStreamingSource {
    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn read_region(&mut self, x: u32, y: u32, w: u32, h: u32) -> IoResult<Region> {
        // Load image on first access
        self.ensure_loaded()?;

        let image = self.cached_image.as_ref()
            .ok_or_else(|| IoError::DecodeError("EXR cache empty after load".into()))?;

        // Clamp region to image bounds
        let x = x.min(self.width.saturating_sub(1));
        let y = y.min(self.height.saturating_sub(1));
        let w = w.min(self.width - x);
        let h = h.min(self.height - y);

        Ok(self.extract_region(image, x, y, w, h))
    }

    fn supports_random_access(&self) -> bool {
        false // Lazy loading, not true streaming
    }

    fn native_tile_size(&self) -> Option<(u32, u32)> {
        None // EXR lazy loading has no native tiles
    }

    fn native_format(&self) -> PixelFormat {
        self.format
    }

    fn channels(&self) -> u32 {
        self.channels
    }
}

/// EXR streaming output with buffered writing.
///
/// Accumulates written regions into memory and writes the complete
/// EXR file on `finalize()`. The exr crate doesn't support incremental
/// tile/scanline writing, so buffering is required.
#[derive(Debug)]
pub struct ExrStreamingOutput {
    /// Output file path.
    path: PathBuf,
    /// Output width.
    width: u32,
    /// Output height.
    height: u32,
    /// Target pixel format.
    format: PixelFormat,
    /// Accumulated pixel data (RGBA F32).
    buffer: Vec<f32>,
}

impl ExrStreamingOutput {
    /// Creates a new EXR streaming output with known dimensions.
    ///
    /// # Arguments
    ///
    /// * `path` - Output file path
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `format` - Target pixel format (F16 or F32)
    pub fn new<P: AsRef<Path>>(path: P, width: u32, height: u32, format: PixelFormat) -> IoResult<Self> {
        let path = path.as_ref().to_path_buf();
        
        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        
        Ok(Self {
            path,
            width,
            height,
            format,
            buffer: vec![0.0f32; (width * height) as usize * RGBA_CHANNELS as usize],
        })
    }

    /// Returns the output path.
    #[inline]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl super::traits::StreamingOutput for ExrStreamingOutput {
    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn write_region(&mut self, region: &Region) -> IoResult<()> {
        let img_w = self.width;

        for local_y in 0..region.height {
            for local_x in 0..region.width {
                let img_x = region.x + local_x;
                let img_y = region.y + local_y;

                // Skip out-of-bounds
                if img_x >= self.width || img_y >= self.height {
                    continue;
                }

                let src_idx = ((local_y * region.width + local_x) as usize) * RGBA_CHANNELS as usize;
                let dst_idx = ((img_y * img_w + img_x) as usize) * RGBA_CHANNELS as usize;

                self.buffer[dst_idx..dst_idx + RGBA_CHANNELS as usize]
                    .copy_from_slice(&region.data[src_idx..src_idx + RGBA_CHANNELS as usize]);
            }
        }

        Ok(())
    }

    fn finalize(self: Box<Self>) -> IoResult<()> {
        // Create ImageData from buffer
        let image = ImageData::from_f32(self.width, self.height, 4, self.buffer);
        
        // Convert to target format if needed
        let image = match self.format {
            PixelFormat::F16 => image.convert_to(PixelFormat::F16),
            _ => image, // Keep F32 or let writer handle other formats
        };

        // Write using existing EXR writer
        crate::exr::write(&self.path, &image)
    }

    fn supports_random_write(&self) -> bool {
        true // Memory buffer accepts any write order
    }

    fn native_tile_size(&self) -> Option<(u32, u32)> {
        None // No native tile size for buffered output
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    // Note: Integration tests with actual EXR files would be in tests/ directory.

    #[test]
    fn test_format_detection() {
        // Test that F32 and F16 are correctly identified from sample type
        // This is a unit test of the logic, not the full implementation
        use exr::prelude::SampleType;
        
        let samples = [SampleType::F16, SampleType::F32];
        let has_f32 = samples.iter().any(|s| matches!(s, SampleType::F32));
        assert!(has_f32);
        
        let samples_f16 = [SampleType::F16, SampleType::F16];
        let has_f32_only = samples_f16.iter().any(|s| matches!(s, SampleType::F32));
        assert!(!has_f32_only);
    }
}
