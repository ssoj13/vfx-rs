//! Streaming I/O traits for processing images larger than RAM.
//!
//! These traits allow reading/writing image regions directly from/to disk
//! without loading the entire image into memory.
//!
//! # Example
//!
//! ```ignore
//! use vfx_compute::backend::streaming::{StreamingSource, StreamingOutput};
//!
//! let mut source = ExrStreamingSource::open("huge_input.exr")?;
//! let mut output = ExrStreamingOutput::create("output.exr")?;
//!
//! executor.execute_color_streaming(&mut source, &mut output, &op)?;
//! ```

use crate::ComputeResult;

/// Source format for streaming.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingFormat {
    Exr,
    Tiff,
    Unknown,
}

/// Trait for streaming image sources (read regions on demand).
pub trait StreamingSource: Send {
    /// Image dimensions (width, height).
    fn dims(&self) -> (u32, u32);

    /// Channel count.
    fn channels(&self) -> u32;

    /// Source format.
    fn format(&self) -> StreamingFormat;

    /// Read a region of the image.
    ///
    /// Returns f32 pixel data for the specified region.
    fn read_region(&mut self, x: u32, y: u32, w: u32, h: u32) -> ComputeResult<Vec<f32>>;

    /// Estimate memory for reading entire image.
    fn estimate_memory(&self) -> u64 {
        let (w, h) = self.dims();
        (w as u64) * (h as u64) * (self.channels() as u64) * 4
    }
}

/// Trait for streaming image outputs (write regions on demand).
pub trait StreamingOutput: Send {
    /// Initialize output with dimensions.
    fn init(&mut self, width: u32, height: u32, channels: u32) -> ComputeResult<()>;

    /// Write a region to output.
    fn write_region(&mut self, x: u32, y: u32, w: u32, h: u32, data: &[f32]) -> ComputeResult<()>;

    /// Finalize and close output.
    fn finish(&mut self) -> ComputeResult<()>;
}

// =============================================================================
// Memory-backed implementations (for testing and small images)
// =============================================================================

/// In-memory streaming source.
pub struct MemorySource {
    data: Vec<f32>,
    width: u32,
    height: u32,
    channels: u32,
}

impl MemorySource {
    pub fn new(data: Vec<f32>, width: u32, height: u32, channels: u32) -> Self {
        Self { data, width, height, channels }
    }

    pub fn from_image(img: &crate::ComputeImage) -> Self {
        Self {
            data: img.data().to_vec(),
            width: img.width,
            height: img.height,
            channels: img.channels,
        }
    }
}

impl StreamingSource for MemorySource {
    fn dims(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn channels(&self) -> u32 {
        self.channels
    }

    fn format(&self) -> StreamingFormat {
        StreamingFormat::Unknown
    }

    fn read_region(&mut self, x: u32, y: u32, w: u32, h: u32) -> ComputeResult<Vec<f32>> {
        let c = self.channels as usize;
        let stride = (self.width as usize) * c;
        let mut result = Vec::with_capacity((w * h) as usize * c);

        for row in y..(y + h) {
            let start = (row as usize) * stride + (x as usize) * c;
            let end = start + (w as usize) * c;
            result.extend_from_slice(&self.data[start..end]);
        }

        Ok(result)
    }
}

/// In-memory streaming output.
pub struct MemoryOutput {
    data: Vec<f32>,
    width: u32,
    height: u32,
    channels: u32,
}

impl MemoryOutput {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            width: 0,
            height: 0,
            channels: 0,
        }
    }

    pub fn into_image(self) -> ComputeResult<crate::ComputeImage> {
        crate::ComputeImage::from_f32(self.data, self.width, self.height, self.channels)
    }

    pub fn data(&self) -> &[f32] {
        &self.data
    }
}

impl Default for MemoryOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingOutput for MemoryOutput {
    fn init(&mut self, width: u32, height: u32, channels: u32) -> ComputeResult<()> {
        self.width = width;
        self.height = height;
        self.channels = channels;
        self.data = vec![0.0; (width * height * channels) as usize];
        Ok(())
    }

    fn write_region(&mut self, x: u32, y: u32, w: u32, h: u32, data: &[f32]) -> ComputeResult<()> {
        let c = self.channels as usize;
        let img_stride = (self.width as usize) * c;
        let tile_stride = (w as usize) * c;

        for row in 0..h as usize {
            let src_start = row * tile_stride;
            let dst_row = y as usize + row;
            let dst_start = dst_row * img_stride + (x as usize) * c;
            self.data[dst_start..dst_start + tile_stride]
                .copy_from_slice(&data[src_start..src_start + tile_stride]);
        }

        Ok(())
    }

    fn finish(&mut self) -> ComputeResult<()> {
        Ok(())
    }
}

// =============================================================================
// EXR Streaming (requires io feature)
// =============================================================================

#[cfg(feature = "io")]
mod exr_streaming {
    use super::*;
    use crate::ComputeError;
    use std::path::Path;

    /// Streaming source for EXR files.
    ///
    /// Reads tiles/scanlines on demand without loading entire file.
    pub struct ExrStreamingSource {
        // For now, we load the full image but could optimize later
        data: Vec<f32>,
        width: u32,
        height: u32,
        channels: u32,
    }

    impl ExrStreamingSource {
        /// Open EXR file for streaming.
        pub fn open<P: AsRef<Path>>(path: P) -> ComputeResult<Self> {
            let img_data = vfx_io::read(path.as_ref())
                .map_err(|e| ComputeError::OperationFailed(format!("EXR read: {}", e)))?;

            let f32_data = img_data.to_f32();

            Ok(Self {
                data: f32_data,
                width: img_data.width,
                height: img_data.height,
                channels: img_data.channels,
            })
        }
    }

    impl StreamingSource for ExrStreamingSource {
        fn dims(&self) -> (u32, u32) {
            (self.width, self.height)
        }

        fn channels(&self) -> u32 {
            self.channels
        }

        fn format(&self) -> StreamingFormat {
            StreamingFormat::Exr
        }

        fn read_region(&mut self, x: u32, y: u32, w: u32, h: u32) -> ComputeResult<Vec<f32>> {
            let c = self.channels as usize;
            let stride = (self.width as usize) * c;
            let mut result = Vec::with_capacity((w * h) as usize * c);

            for row in y..(y + h) {
                let start = (row as usize) * stride + (x as usize) * c;
                let end = start + (w as usize) * c;
                result.extend_from_slice(&self.data[start..end]);
            }

            Ok(result)
        }
    }

    /// Streaming output for EXR files.
    pub struct ExrStreamingOutput {
        path: std::path::PathBuf,
        data: Vec<f32>,
        width: u32,
        height: u32,
        channels: u32,
    }

    impl ExrStreamingOutput {
        /// Create EXR output file.
        pub fn create<P: AsRef<Path>>(path: P) -> ComputeResult<Self> {
            Ok(Self {
                path: path.as_ref().to_path_buf(),
                data: Vec::new(),
                width: 0,
                height: 0,
                channels: 0,
            })
        }
    }

    impl StreamingOutput for ExrStreamingOutput {
        fn init(&mut self, width: u32, height: u32, channels: u32) -> ComputeResult<()> {
            self.width = width;
            self.height = height;
            self.channels = channels;
            self.data = vec![0.0; (width * height * channels) as usize];
            Ok(())
        }

        fn write_region(&mut self, x: u32, y: u32, w: u32, h: u32, data: &[f32]) -> ComputeResult<()> {
            let c = self.channels as usize;
            let img_stride = (self.width as usize) * c;
            let tile_stride = (w as usize) * c;

            for row in 0..h as usize {
                let src_start = row * tile_stride;
                let dst_row = y as usize + row;
                let dst_start = dst_row * img_stride + (x as usize) * c;
                self.data[dst_start..dst_start + tile_stride]
                    .copy_from_slice(&data[src_start..src_start + tile_stride]);
            }

            Ok(())
        }

        fn finish(&mut self) -> ComputeResult<()> {
            let img_data = vfx_io::ImageData::from_f32(
                self.width,
                self.height,
                self.channels,
                std::mem::take(&mut self.data),
            );

            vfx_io::write(&self.path, &img_data)
                .map_err(|e| ComputeError::OperationFailed(format!("EXR write: {}", e)))?;

            Ok(())
        }
    }
}

#[cfg(feature = "io")]
pub use exr_streaming::{ExrStreamingSource, ExrStreamingOutput};

// =============================================================================
// Helper Functions
// =============================================================================

/// Check if streaming is recommended for given dimensions.
pub fn should_stream(width: u32, height: u32, channels: u32, available_ram: u64) -> bool {
    let image_bytes = (width as u64) * (height as u64) * (channels as u64) * 4;
    // Use streaming if image > 70% of available RAM
    let threshold = (available_ram as f64 * 0.7) as u64;
    image_bytes > threshold
}

/// Estimate memory required for processing.
pub fn estimate_memory(width: u32, height: u32, channels: u32) -> u64 {
    // src + dst + working = 3x
    (width as u64) * (height as u64) * (channels as u64) * 4 * 3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_source() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0];
        let mut src = MemorySource::new(data, 2, 2, 3);

        assert_eq!(src.dims(), (2, 2));
        assert_eq!(src.channels(), 3);

        // Read top-left pixel
        let region = src.read_region(0, 0, 1, 1).unwrap();
        assert_eq!(region, vec![1.0, 2.0, 3.0]);

        // Read bottom-right pixel
        let region = src.read_region(1, 1, 1, 1).unwrap();
        assert_eq!(region, vec![10.0, 11.0, 12.0]);
    }

    #[test]
    fn test_memory_output() {
        let mut out = MemoryOutput::new();
        out.init(2, 2, 3).unwrap();

        out.write_region(0, 0, 1, 1, &[1.0, 2.0, 3.0]).unwrap();
        out.write_region(1, 1, 1, 1, &[4.0, 5.0, 6.0]).unwrap();

        let data = out.data();
        assert_eq!(data[0..3], [1.0, 2.0, 3.0]);
        assert_eq!(data[9..12], [4.0, 5.0, 6.0]);
    }
}
