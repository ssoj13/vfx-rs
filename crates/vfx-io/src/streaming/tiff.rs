//! TIFF streaming source with true random access.
//!
//! Uses the `tiff` crate's `read_chunk()` API to read only the strips
//! or tiles needed for a requested region, without loading the entire image.
//!
//! # Architecture
//!
//! TIFF files can be organized as:
//! - **Tiled**: Image divided into rectangular tiles (optimal for streaming)
//! - **Stripped**: Image divided into horizontal strips (row-based access)
//!
//! This implementation handles both, using the unified "chunk" abstraction
//! from the tiff crate.
//!
//! # Performance
//!
//! For a 32K tiled TIFF with 512x512 tiles:
//! - Reading a 1024x1024 region requires only 4 tiles (~8 MB)
//! - vs. full decode requiring ~16 GB
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::streaming::TiffStreamingSource;
//!
//! let mut source = TiffStreamingSource::open("large_scan.tif")?;
//! println!("Tile size: {:?}", source.native_tile_size());
//!
//! // Read only what we need
//! let region = source.read_region(1000, 1000, 512, 512)?;
//! ```

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use crate::{IoResult, IoError, PixelFormat};
use super::traits::{Region, StreamingSource, RGBA_CHANNELS};

// === Constants ===

/// Maximum value for U8 normalization.
const U8_MAX_F32: f32 = u8::MAX as f32;
/// Maximum value for U16 normalization.
const U16_MAX_F32: f32 = u16::MAX as f32;
/// Maximum value for U32 normalization.
const U32_MAX_F32: f32 = u32::MAX as f32;
/// Default alpha for pixels without alpha channel.
const ALPHA_OPAQUE: f32 = 1.0;

/// TIFF streaming source with chunk-based random access.
///
/// Reads TIFF tiles/strips on demand without loading the entire image.
/// Optimal for processing large images that exceed available RAM.
#[derive(Debug)]
pub struct TiffStreamingSource {
    /// Path to the TIFF file (for error messages).
    path: PathBuf,
    /// Image width in pixels.
    width: u32,
    /// Image height in pixels.
    height: u32,
    /// Chunk dimensions (tile size for tiled, strip height for stripped).
    chunk_dims: (u32, u32),
    /// True if image is tiled (vs stripped).
    is_tiled: bool,
    /// Bits per sample (kept for future format-aware processing).
    #[allow(dead_code)]
    bits_per_sample: u8,
    /// Samples per pixel (channels).
    samples_per_pixel: u16,
    /// Native pixel format.
    format: PixelFormat,
    /// TIFF decoder with cached file handle.
    decoder: tiff::decoder::Decoder<BufReader<File>>,
}

impl TiffStreamingSource {
    /// Opens a TIFF file for streaming access.
    ///
    /// Reads only the file header to determine dimensions and tiling.
    /// Actual pixel data is loaded on demand via `read_region()`.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the TIFF file
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be opened or is not a valid TIFF.
    pub fn open<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;

        let mut decoder = tiff::decoder::Decoder::new(BufReader::new(file))
            .map_err(|e| IoError::DecodeError(format!("TIFF header: {}", e)))?;

        let (width, height) = decoder.dimensions()
            .map_err(|e| IoError::DecodeError(format!("TIFF dimensions: {}", e)))?;

        let chunk_dims = decoder.chunk_dimensions();
        
        // If chunk width < image width, it's tiled; otherwise stripped
        let is_tiled = chunk_dims.0 < width;

        let color_type = decoder.colortype()
            .map_err(|e| IoError::DecodeError(format!("TIFF colortype: {}", e)))?;

        let (bits_per_sample, samples_per_pixel, format) = match color_type {
            tiff::ColorType::Gray(8) => (8, 1, PixelFormat::U8),
            tiff::ColorType::Gray(16) => (16, 1, PixelFormat::U16),
            tiff::ColorType::Gray(32) => (32, 1, PixelFormat::F32),
            tiff::ColorType::RGB(8) => (8, 3, PixelFormat::U8),
            tiff::ColorType::RGB(16) => (16, 3, PixelFormat::U16),
            tiff::ColorType::RGB(32) => (32, 3, PixelFormat::F32),
            tiff::ColorType::RGBA(8) => (8, 4, PixelFormat::U8),
            tiff::ColorType::RGBA(16) => (16, 4, PixelFormat::U16),
            tiff::ColorType::RGBA(32) => (32, 4, PixelFormat::F32),
            tiff::ColorType::GrayA(8) => (8, 2, PixelFormat::U8),
            tiff::ColorType::GrayA(16) => (16, 2, PixelFormat::U16),
            _ => (8, 4, PixelFormat::U8), // Fallback
        };

        Ok(Self {
            path,
            width,
            height,
            chunk_dims,
            is_tiled,
            bits_per_sample,
            samples_per_pixel,
            format,
            decoder,
        })
    }

    /// Returns the path to the TIFF file.
    #[inline]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns true if the TIFF is tiled (vs stripped).
    #[inline]
    pub fn is_tiled(&self) -> bool {
        self.is_tiled
    }

    /// Returns the chunk dimensions (tile or strip size).
    #[inline]
    pub fn chunk_dims(&self) -> (u32, u32) {
        self.chunk_dims
    }

    /// Calculates which chunks overlap with a region.
    ///
    /// Returns list of (chunk_x, chunk_y, pixel_x, pixel_y) tuples.
    fn chunks_for_region(&self, x: u32, y: u32, w: u32, h: u32) -> Vec<(u32, u32, u32, u32)> {
        let (chunk_w, chunk_h) = self.chunk_dims;

        let start_cx = x / chunk_w;
        let start_cy = y / chunk_h;
        let end_cx = (x + w - 1) / chunk_w;
        let end_cy = (y + h - 1) / chunk_h;

        let mut chunks = Vec::with_capacity(
            ((end_cx - start_cx + 1) * (end_cy - start_cy + 1)) as usize
        );

        for cy in start_cy..=end_cy {
            for cx in start_cx..=end_cx {
                chunks.push((cx, cy, cx * chunk_w, cy * chunk_h));
            }
        }

        chunks
    }

    /// Reads a single chunk and converts to RGBA F32.
    fn read_chunk(&mut self, chunk_x: u32, chunk_y: u32) -> IoResult<(Vec<f32>, u32, u32)> {
        let chunks_per_row = (self.width + self.chunk_dims.0 - 1) / self.chunk_dims.0;
        let chunk_index = chunk_y * chunks_per_row + chunk_x;

        let result = self.decoder.read_chunk(chunk_index)
            .map_err(|e| IoError::DecodeError(format!("TIFF chunk {}: {}", chunk_index, e)))?;

        let (actual_w, actual_h) = self.decoder.chunk_data_dimensions(chunk_index);
        let pixels = self.decode_chunk(&result, actual_w, actual_h)?;

        Ok((pixels, actual_w, actual_h))
    }

    /// Decodes raw chunk data to RGBA F32.
    fn decode_chunk(
        &self,
        data: &tiff::decoder::DecodingResult,
        width: u32,
        height: u32,
    ) -> IoResult<Vec<f32>> {
        use tiff::decoder::DecodingResult;

        let pixel_count = (width * height) as usize;
        let mut pixels = vec![0.0f32; pixel_count * RGBA_CHANNELS as usize];

        match data {
            DecodingResult::U8(data) => {
                self.convert_u8(&data, &mut pixels);
            }
            DecodingResult::U16(data) => {
                self.convert_u16(&data, &mut pixels);
            }
            DecodingResult::U32(data) => {
                self.convert_u32(&data, &mut pixels);
            }
            DecodingResult::F32(data) => {
                self.convert_f32(&data, &mut pixels);
            }
            DecodingResult::F64(data) => {
                self.convert_f64(&data, &mut pixels);
            }
            _ => {
                return Err(IoError::UnsupportedFormat("Unsupported TIFF sample format".into()));
            }
        }

        Ok(pixels)
    }

    /// Converts U8 samples to RGBA F32.
    fn convert_u8(&self, data: &[u8], pixels: &mut [f32]) {
        let spp = self.samples_per_pixel as usize;
        let pixel_count = pixels.len() / RGBA_CHANNELS as usize;

        for i in 0..pixel_count {
            let src = i * spp;
            let dst = i * RGBA_CHANNELS as usize;

            if src + spp <= data.len() {
                let r = data[src] as f32 / U8_MAX_F32;
                let g = if spp > 1 { data[src + 1] as f32 / U8_MAX_F32 } else { r };
                let b = if spp > 2 { data[src + 2] as f32 / U8_MAX_F32 } else { r };
                let a = if spp > 3 { data[src + 3] as f32 / U8_MAX_F32 } else { ALPHA_OPAQUE };

                pixels[dst] = r;
                pixels[dst + 1] = g;
                pixels[dst + 2] = b;
                pixels[dst + 3] = a;
            }
        }
    }

    /// Converts U16 samples to RGBA F32.
    fn convert_u16(&self, data: &[u16], pixels: &mut [f32]) {
        let spp = self.samples_per_pixel as usize;
        let pixel_count = pixels.len() / RGBA_CHANNELS as usize;

        for i in 0..pixel_count {
            let src = i * spp;
            let dst = i * RGBA_CHANNELS as usize;

            if src + spp <= data.len() {
                let r = data[src] as f32 / U16_MAX_F32;
                let g = if spp > 1 { data[src + 1] as f32 / U16_MAX_F32 } else { r };
                let b = if spp > 2 { data[src + 2] as f32 / U16_MAX_F32 } else { r };
                let a = if spp > 3 { data[src + 3] as f32 / U16_MAX_F32 } else { ALPHA_OPAQUE };

                pixels[dst] = r;
                pixels[dst + 1] = g;
                pixels[dst + 2] = b;
                pixels[dst + 3] = a;
            }
        }
    }

    /// Converts U32 samples to RGBA F32.
    fn convert_u32(&self, data: &[u32], pixels: &mut [f32]) {
        let spp = self.samples_per_pixel as usize;
        let pixel_count = pixels.len() / RGBA_CHANNELS as usize;

        for i in 0..pixel_count {
            let src = i * spp;
            let dst = i * RGBA_CHANNELS as usize;

            if src + spp <= data.len() {
                let r = data[src] as f32 / U32_MAX_F32;
                let g = if spp > 1 { data[src + 1] as f32 / U32_MAX_F32 } else { r };
                let b = if spp > 2 { data[src + 2] as f32 / U32_MAX_F32 } else { r };
                let a = if spp > 3 { data[src + 3] as f32 / U32_MAX_F32 } else { ALPHA_OPAQUE };

                pixels[dst] = r;
                pixels[dst + 1] = g;
                pixels[dst + 2] = b;
                pixels[dst + 3] = a;
            }
        }
    }

    /// Converts F32 samples to RGBA F32.
    fn convert_f32(&self, data: &[f32], pixels: &mut [f32]) {
        let spp = self.samples_per_pixel as usize;
        let pixel_count = pixels.len() / RGBA_CHANNELS as usize;

        for i in 0..pixel_count {
            let src = i * spp;
            let dst = i * RGBA_CHANNELS as usize;

            if src + spp <= data.len() {
                let r = data[src];
                let g = if spp > 1 { data[src + 1] } else { r };
                let b = if spp > 2 { data[src + 2] } else { r };
                let a = if spp > 3 { data[src + 3] } else { ALPHA_OPAQUE };

                pixels[dst] = r;
                pixels[dst + 1] = g;
                pixels[dst + 2] = b;
                pixels[dst + 3] = a;
            }
        }
    }

    /// Converts F64 samples to RGBA F32.
    fn convert_f64(&self, data: &[f64], pixels: &mut [f32]) {
        let spp = self.samples_per_pixel as usize;
        let pixel_count = pixels.len() / RGBA_CHANNELS as usize;

        for i in 0..pixel_count {
            let src = i * spp;
            let dst = i * RGBA_CHANNELS as usize;

            if src + spp <= data.len() {
                let r = data[src] as f32;
                let g = if spp > 1 { data[src + 1] as f32 } else { r };
                let b = if spp > 2 { data[src + 2] as f32 } else { r };
                let a = if spp > 3 { data[src + 3] as f32 } else { ALPHA_OPAQUE };

                pixels[dst] = r;
                pixels[dst + 1] = g;
                pixels[dst + 2] = b;
                pixels[dst + 3] = a;
            }
        }
    }
}

impl StreamingSource for TiffStreamingSource {
    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn read_region(&mut self, x: u32, y: u32, w: u32, h: u32) -> IoResult<Region> {
        // Clamp region to image bounds
        let x = x.min(self.width.saturating_sub(1));
        let y = y.min(self.height.saturating_sub(1));
        let w = w.min(self.width - x);
        let h = h.min(self.height - y);

        // Find overlapping chunks
        let chunks = self.chunks_for_region(x, y, w, h);

        // Create output buffer
        let mut result = vec![0.0f32; (w * h) as usize * RGBA_CHANNELS as usize];

        // Read and blit each chunk
        for &(chunk_x, chunk_y, chunk_px, chunk_py) in &chunks {
            let (chunk_data, chunk_w, chunk_h) = self.read_chunk(chunk_x, chunk_y)?;

            // Calculate intersection
            let src_x = x.saturating_sub(chunk_px);
            let src_y = y.saturating_sub(chunk_py);
            let dst_x = chunk_px.saturating_sub(x);
            let dst_y = chunk_py.saturating_sub(y);

            let copy_w = (chunk_w - src_x).min(w - dst_x);
            let copy_h = (chunk_h - src_y).min(h - dst_y);

            // Copy pixels row by row
            for row in 0..copy_h {
                for col in 0..copy_w {
                    let src_idx = ((src_y + row) * chunk_w + (src_x + col)) as usize * RGBA_CHANNELS as usize;
                    let dst_idx = ((dst_y + row) * w + (dst_x + col)) as usize * RGBA_CHANNELS as usize;

                    if src_idx + RGBA_CHANNELS as usize <= chunk_data.len() 
                       && dst_idx + RGBA_CHANNELS as usize <= result.len() 
                    {
                        result[dst_idx..dst_idx + RGBA_CHANNELS as usize]
                            .copy_from_slice(&chunk_data[src_idx..src_idx + RGBA_CHANNELS as usize]);
                    }
                }
            }
        }

        Ok(Region::new(x, y, w, h, result))
    }

    fn supports_random_access(&self) -> bool {
        true
    }

    fn native_tile_size(&self) -> Option<(u32, u32)> {
        if self.is_tiled {
            Some(self.chunk_dims)
        } else {
            None
        }
    }

    fn native_format(&self) -> PixelFormat {
        self.format
    }

    fn channels(&self) -> u32 {
        self.samples_per_pixel as u32
    }
}

// =============================================================================
// TIFF Streaming Output
// =============================================================================

/// TIFF streaming output with buffered writing.
///
/// Currently buffers all tiles in memory and writes the complete image
/// when `finalize()` is called. True tiled TIFF writing is planned for future.
///
/// # Limitations
///
/// The `tiff` crate's encoder doesn't easily support incremental tile writing,
/// so we buffer the entire image. For very large outputs (>RAM), consider:
/// - Writing in strips/passes
/// - Using a memory-mapped file
/// - External tools like ImageMagick for final assembly
///
/// # Example
///
/// ```ignore
/// use vfx_io::streaming::{TiffStreamingOutput, StreamingOutput};
///
/// let mut output = TiffStreamingOutput::create("output.tif", PixelFormat::F32)?;
/// output.init(4096, 4096)?;
///
/// // Write tiles as they're processed
/// for tile in tiles {
///     output.write_region(&tile)?;
/// }
///
/// // Finalize writes the TIFF to disk
/// Box::new(output).finalize()?;
/// ```
#[derive(Debug)]
pub struct TiffStreamingOutput {
    /// Output file path.
    path: PathBuf,
    /// Output width.
    width: u32,
    /// Output height.
    height: u32,
    /// Target pixel format for output.
    format: PixelFormat,
    /// Accumulated pixel data (RGBA F32).
    buffer: Option<Vec<f32>>,
}

impl TiffStreamingOutput {
    /// Creates a new TIFF streaming output with known dimensions.
    ///
    /// Use this when you know the output size upfront.
    ///
    /// # Arguments
    ///
    /// * `path` - Output file path
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `format` - Target pixel format (F32 recommended for quality)
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
            buffer: Some(vec![0.0f32; (width * height) as usize * RGBA_CHANNELS as usize]),
        })
    }

    /// Creates a new TIFF streaming output with deferred dimensions.
    ///
    /// Call `init()` before writing to set dimensions.
    ///
    /// # Arguments
    ///
    /// * `path` - Output file path
    /// * `format` - Target pixel format (F32 recommended for quality)
    pub fn create<P: AsRef<Path>>(path: P, format: PixelFormat) -> IoResult<Self> {
        let path = path.as_ref().to_path_buf();
        
        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        
        Ok(Self {
            path,
            width: 0,
            height: 0,
            format,
            buffer: None,
        })
    }

    /// Initializes the output with dimensions.
    ///
    /// Must be called before `write_region()`.
    pub fn init(&mut self, width: u32, height: u32) -> IoResult<()> {
        self.width = width;
        self.height = height;
        self.buffer = Some(vec![0.0f32; (width * height) as usize * RGBA_CHANNELS as usize]);
        Ok(())
    }

    /// Returns the output path.
    #[inline]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl super::traits::StreamingOutput for TiffStreamingOutput {
    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn write_region(&mut self, region: &Region) -> IoResult<()> {
        let buffer = self.buffer.as_mut()
            .ok_or_else(|| IoError::InvalidFile("Output not initialized".into()))?;

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

                buffer[dst_idx..dst_idx + RGBA_CHANNELS as usize]
                    .copy_from_slice(&region.data[src_idx..src_idx + RGBA_CHANNELS as usize]);
            }
        }

        Ok(())
    }

    fn finalize(self: Box<Self>) -> IoResult<()> {
        use tiff::encoder::TiffEncoder;

        let buffer = self.buffer
            .ok_or_else(|| IoError::InvalidFile("Output not initialized".into()))?;

        let file = std::fs::File::create(&self.path)?;
        let mut encoder = TiffEncoder::new(file)
            .map_err(|e| IoError::EncodeError(format!("TIFF encoder: {}", e)))?;

        // Write based on target format
        match self.format {
            PixelFormat::F32 | PixelFormat::F16 => {
                // Write as RGBA F32
                encoder.write_image::<tiff::encoder::colortype::RGBA32Float>(
                    self.width,
                    self.height,
                    &buffer,
                ).map_err(|e| IoError::EncodeError(format!("TIFF write: {}", e)))?;
            }
            PixelFormat::U16 => {
                // Convert to U16
                let u16_data: Vec<u16> = buffer.iter()
                    .map(|&v| (v.clamp(0.0, 1.0) * U16_MAX_F32) as u16)
                    .collect();
                encoder.write_image::<tiff::encoder::colortype::RGBA16>(
                    self.width,
                    self.height,
                    &u16_data,
                ).map_err(|e| IoError::EncodeError(format!("TIFF write: {}", e)))?;
            }
            PixelFormat::U8 | PixelFormat::U32 => {
                // Convert to U8
                let u8_data: Vec<u8> = buffer.iter()
                    .map(|&v| (v.clamp(0.0, 1.0) * U8_MAX_F32) as u8)
                    .collect();
                encoder.write_image::<tiff::encoder::colortype::RGBA8>(
                    self.width,
                    self.height,
                    &u8_data,
                ).map_err(|e| IoError::EncodeError(format!("TIFF write: {}", e)))?;
            }
        }

        Ok(())
    }

    fn supports_random_write(&self) -> bool {
        true // Buffer supports any write order
    }

    fn native_tile_size(&self) -> Option<(u32, u32)> {
        None // Buffered output has no native tiles
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    // Note: These tests require actual TIFF files.
    // Integration tests would be in tests/ directory with fixtures.

    #[test]
    fn test_chunks_for_region() {
        // Mock a 1024x1024 image with 256x256 tiles
        // Can't construct TiffStreamingSource without file, so test logic directly
        
        let chunk_w = 256u32;
        let chunk_h = 256u32;
        
        // Region at (100, 100) with size (300, 300) should need 4 chunks
        let x = 100;
        let y = 100;
        let w = 300;
        let h = 300;
        
        let start_cx = x / chunk_w; // 0
        let start_cy = y / chunk_h; // 0
        let end_cx = (x + w - 1) / chunk_w; // 1
        let end_cy = (y + h - 1) / chunk_h; // 1
        
        let chunk_count = (end_cx - start_cx + 1) * (end_cy - start_cy + 1);
        assert_eq!(chunk_count, 4); // 2x2 chunks
    }
}
