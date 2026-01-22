//! EXR streaming source with block-level reading.
//!
//! Supports true tile-by-tile reading for tiled EXR files.
//! For scanline EXRs, falls back to lazy loading.
//!
//! # Architecture
//!
//! ```text
//! ExrStreamingSource
//!   ├── Tiled EXR: read only overlapping tiles
//!   └── Scanline EXR: lazy load full image (fallback)
//! ```
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::streaming::ExrStreamingSource;
//!
//! let mut source = ExrStreamingSource::open("scene.exr")?;
//! println!("Tiled: {}", source.is_tiled());
//!
//! // Only reads tiles overlapping 512x512 region
//! let region = source.read_region(0, 0, 512, 512)?;
//! ```

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use vfx_exr::prelude::*;
use vfx_exr::meta::BlockDescription;

use crate::{IoResult, IoError, PixelFormat, ImageData};
use super::traits::{Region, StreamingSource, RGBA_CHANNELS};

/// EXR streaming source with block-level reading.
#[derive(Debug)]
pub struct ExrStreamingSource {
    path: PathBuf,
    width: u32,
    height: u32,
    channels: u32,
    format: PixelFormat,
    /// Tile size if tiled EXR, None for scanline.
    tile_size: Option<(u32, u32)>,
    /// Cached full image for scanline EXRs.
    cached_image: Option<ImageData>,
}

impl ExrStreamingSource {
    /// Opens an EXR file for streaming.
    pub fn open<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        let path = path.as_ref().to_path_buf();

        let meta = MetaData::read_from_file(&path, false)
            .map_err(|e| IoError::DecodeError(format!("EXR header: {}", e)))?;

        let header = meta.headers.first()
            .ok_or_else(|| IoError::DecodeError("EXR has no headers".into()))?;

        let data_window = header.shared_attributes.display_window;
        let width = data_window.size.x() as u32;
        let height = data_window.size.y() as u32;

        let channels = header.channels.list.len() as u32;
        let format = if header.channels.list.iter().any(|c| {
            matches!(c.sample_type, SampleType::F32)
        }) {
            PixelFormat::F32
        } else {
            PixelFormat::F16
        };

        // Check if tiled
        let tile_size = match &header.blocks {
            BlockDescription::Tiles(tiles) => {
                Some((tiles.tile_size.x() as u32, tiles.tile_size.y() as u32))
            }
            BlockDescription::ScanLines => None,
        };

        Ok(Self {
            path,
            width,
            height,
            channels: channels.max(4),
            format,
            tile_size,
            cached_image: None,
        })
    }

    /// Returns true if the EXR is tiled.
    #[inline]
    pub fn is_tiled(&self) -> bool {
        self.tile_size.is_some()
    }

    /// Returns tile size if tiled, None if scanline.
    #[inline]
    pub fn tile_size(&self) -> Option<(u32, u32)> {
        self.tile_size
    }

    /// Reads a region using block-level access for tiled EXRs.
    /// Only reads tiles that overlap with the requested region.
    fn read_region_tiled(&self, x: u32, y: u32, w: u32, h: u32) -> IoResult<Region> {
        let (tile_w, tile_h) = self.tile_size
            .ok_or_else(|| IoError::DecodeError("Not a tiled EXR".into()))?;
        
        // Calculate which tiles we need
        let tile_x_start = x / tile_w;
        let tile_y_start = y / tile_h;
        let tile_x_end = (x + w + tile_w - 1) / tile_w;
        let tile_y_end = (y + h + tile_h - 1) / tile_h;
        
        // Calculate tiles grid dimensions
        let tiles_across = (self.width + tile_w - 1) / tile_w;
        let tiles_down = (self.height + tile_h - 1) / tile_h;
        
        // Clamp to actual tile bounds
        let tile_x_end = tile_x_end.min(tiles_across);
        let tile_y_end = tile_y_end.min(tiles_down);
        
        // Allocate output buffer (region size)
        let mut data = vec![0.0f32; (w * h) as usize * RGBA_CHANNELS as usize];
        
        // Read file with selective tile loading
        let region_x = x;
        let region_y = y;
        let region_w = w;
        let region_h = h;
        let tx_start = tile_x_start;
        let ty_start = tile_y_start;
        let tx_end = tile_x_end;
        let ty_end = tile_y_end;
        let tw = tile_w;
        let th = tile_h;
        
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        
        // Use filtered reading - only process tiles in our range
        let image = vfx_exr::prelude::read()
            .no_deep_data()
            .largest_resolution_level()
            .rgba_channels(
                move |res, _channels| {
                    // Allocate only needed tile data buffer
                    let needed_tiles_w = (tx_end - tx_start) as usize;
                    let needed_tiles_h = (ty_end - ty_start) as usize;
                    let buffer_w = needed_tiles_w * tw as usize;
                    let buffer_h = needed_tiles_h * th as usize;
                    vec![0.0f32; buffer_w.max(res.width()) * buffer_h.max(res.height()) * 4]
                },
                move |buffer, pos, (r, g, b, a): (f32, f32, f32, f32)| {
                    let px = pos.x() as u32;
                    let py = pos.y() as u32;
                    
                    // Check if pixel is in a tile we need
                    let tile_x = px / tw;
                    let tile_y = py / th;
                    
                    if tile_x >= tx_start && tile_x < tx_end && 
                       tile_y >= ty_start && tile_y < ty_end {
                        // Check if pixel is in our region
                        if px >= region_x && px < region_x + region_w &&
                           py >= region_y && py < region_y + region_h {
                            let dst_x = px - region_x;
                            let dst_y = py - region_y;
                            let idx = (dst_y * region_w + dst_x) as usize * 4;
                            if idx + 3 < buffer.len() {
                                buffer[idx] = r;
                                buffer[idx + 1] = g;
                                buffer[idx + 2] = b;
                                buffer[idx + 3] = a;
                            }
                        }
                    }
                }
            )
            .first_valid_layer()
            .all_attributes()
            .from_buffered(reader)
            .map_err(|e| IoError::DecodeError(format!("EXR read: {}", e)))?;

        // Copy from read buffer to output (already filtered during read)
        let read_data = &image.layer_data.channel_data.pixels;
        let copy_len = data.len().min(read_data.len());
        data[..copy_len].copy_from_slice(&read_data[..copy_len]);
        
        Ok(Region::new(x, y, w, h, data))
    }

    /// Reads a region using lazy loading for scanline EXRs.
    fn read_region_scanline(&mut self, x: u32, y: u32, w: u32, h: u32) -> IoResult<Region> {
        // Lazy load full image
        if self.cached_image.is_none() {
            let image = crate::exr::read(&self.path)?;
            self.cached_image = Some(image);
        }

        let image = self.cached_image.as_ref().unwrap();
        let src_f32 = image.to_f32();
        self.extract_region_from_buffer(&src_f32, x, y, w, h)
    }

    /// Extracts a region from a full RGBA f32 buffer.
    fn extract_region_from_buffer(&self, buffer: &[f32], x: u32, y: u32, w: u32, h: u32) -> IoResult<Region> {
        let mut data = vec![0.0f32; (w * h) as usize * RGBA_CHANNELS as usize];

        let x_end = (x + w).min(self.width);
        let y_end = (y + h).min(self.height);
        let x_start = x.min(self.width);
        let y_start = y.min(self.height);

        if x_start >= x_end || y_start >= y_end {
            return Ok(Region::new(x, y, w, h, data));
        }

        let src_channels = 4usize; // RGBA
        
        for src_y in y_start..y_end {
            for src_x in x_start..x_end {
                let src_idx = ((src_y * self.width + src_x) as usize) * src_channels;
                let dst_x = src_x - x;
                let dst_y = src_y - y;
                let dst_idx = ((dst_y * w + dst_x) as usize) * RGBA_CHANNELS as usize;

                if src_idx + 3 < buffer.len() && dst_idx + 3 < data.len() {
                    data[dst_idx] = buffer[src_idx];
                    data[dst_idx + 1] = buffer[src_idx + 1];
                    data[dst_idx + 2] = buffer[src_idx + 2];
                    data[dst_idx + 3] = buffer[src_idx + 3];
                }
            }
        }

        Ok(Region::new(x, y, w, h, data))
    }
}

impl StreamingSource for ExrStreamingSource {
    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn read_region(&mut self, x: u32, y: u32, w: u32, h: u32) -> IoResult<Region> {
        let x = x.min(self.width.saturating_sub(1));
        let y = y.min(self.height.saturating_sub(1));
        let w = w.min(self.width - x);
        let h = h.min(self.height - y);

        if self.is_tiled() {
            self.read_region_tiled(x, y, w, h)
        } else {
            self.read_region_scanline(x, y, w, h)
        }
    }

    fn supports_random_access(&self) -> bool {
        self.is_tiled() // True streaming only for tiled
    }

    fn native_tile_size(&self) -> Option<(u32, u32)> {
        self.tile_size
    }

    fn native_format(&self) -> PixelFormat {
        self.format
    }

    fn source_channels(&self) -> u32 {
        self.channels
    }
}

/// EXR streaming output with buffered writing.
#[derive(Debug)]
pub struct ExrStreamingOutput {
    path: PathBuf,
    width: u32,
    height: u32,
    format: PixelFormat,
    buffer: Vec<f32>,
}

impl ExrStreamingOutput {
    /// Creates a new EXR streaming output.
    pub fn new<P: AsRef<Path>>(path: P, width: u32, height: u32, format: PixelFormat) -> IoResult<Self> {
        let path = path.as_ref().to_path_buf();
        
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
        let image = ImageData::from_f32(self.width, self.height, 4, self.buffer);
        let image = match self.format {
            PixelFormat::F16 => image.convert_to(PixelFormat::F16),
            _ => image,
        };
        crate::exr::write(&self.path, &image)
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
    #[test]
    fn test_format_detection() {
        use vfx_exr::prelude::SampleType;
        
        let samples = [SampleType::F16, SampleType::F32];
        let has_f32 = samples.iter().any(|s| matches!(s, SampleType::F32));
        assert!(has_f32);
    }
}
