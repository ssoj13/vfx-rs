//! Double-buffered streaming pipeline for parallel I/O.
//!
//! This module provides a streaming executor that overlaps I/O with processing
//! using double-buffering. While one region is being processed, the next is
//! prefetched, maximizing throughput.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │              Double-Buffered Pipeline                        │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                             │
//! │  Thread 1 (I/O):     [Read A] [Read B] [Read C] ...        │
//! │  Thread 2 (Process):        [Process A] [Process B] ...    │
//! │                                                             │
//! │  Timeline:  ─────────────────────────────────────────▶      │
//! │                                                             │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::streaming::{open_streaming, StreamingPipeline};
//!
//! let source = open_streaming("input.tif")?;
//! let output = create_streaming_output("output.tif", w, h, format)?;
//!
//! let pipeline = StreamingPipeline::new(source, output, 512, 512);
//!
//! // Process with color transform
//! pipeline.run(|region| {
//!     // Apply color transform to region.data
//!     process_colors(&mut region.data);
//! })?;
//! ```

use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

use super::{BoxedSource, BoxedOutput, Region};
use crate::IoResult;

/// Tile specification for iteration.
#[derive(Debug, Clone, Copy)]
pub struct TileSpec {
    /// X offset of tile.
    pub x: u32,
    /// Y offset of tile.
    pub y: u32,
    /// Tile width.
    pub w: u32,
    /// Tile height.
    pub h: u32,
}

/// Generates tile specifications for an image.
///
/// # Arguments
///
/// * `img_w`, `img_h` - Image dimensions
/// * `tile_w`, `tile_h` - Tile size
///
/// # Returns
///
/// Iterator over all tiles covering the image.
pub fn tile_iterator(
    img_w: u32,
    img_h: u32,
    tile_w: u32,
    tile_h: u32,
) -> impl Iterator<Item = TileSpec> {
    let num_tiles_x = (img_w + tile_w - 1) / tile_w;
    let num_tiles_y = (img_h + tile_h - 1) / tile_h;

    (0..num_tiles_y).flat_map(move |ty| {
        (0..num_tiles_x).map(move |tx| {
            let x = tx * tile_w;
            let y = ty * tile_h;
            let w = (img_w - x).min(tile_w);
            let h = (img_h - y).min(tile_h);
            TileSpec { x, y, w, h }
        })
    })
}

/// Double-buffered streaming pipeline.
///
/// Overlaps I/O with processing for maximum throughput.
/// Uses two buffers: while one is being processed, the other is filled.
pub struct StreamingPipeline {
    source: BoxedSource,
    output: BoxedOutput,
    tile_w: u32,
    tile_h: u32,
}

impl StreamingPipeline {
    /// Creates a new streaming pipeline.
    ///
    /// # Arguments
    ///
    /// * `source` - Streaming source
    /// * `output` - Streaming output
    /// * `tile_w`, `tile_h` - Processing tile size
    pub fn new(
        source: BoxedSource,
        output: BoxedOutput,
        tile_w: u32,
        tile_h: u32,
    ) -> Self {
        Self {
            source,
            output,
            tile_w,
            tile_h,
        }
    }

    /// Runs the pipeline with a processing function.
    ///
    /// The function receives each region for processing. Regions are
    /// automatically read from source and written to output.
    ///
    /// # Arguments
    ///
    /// * `process` - Function to apply to each region
    ///
    /// # Returns
    ///
    /// Ok(()) on success, error if I/O or processing fails.
    pub fn run<F>(mut self, mut process: F) -> IoResult<()>
    where
        F: FnMut(&mut Region),
    {
        let (img_w, img_h) = self.source.dimensions();
        let tiles: Vec<_> = tile_iterator(img_w, img_h, self.tile_w, self.tile_h).collect();

        for tile in tiles {
            // Read region
            let mut region = self.source.read_region(tile.x, tile.y, tile.w, tile.h)?;

            // Process
            process(&mut region);

            // Write
            self.output.write_region(&region)?;
        }

        // Finalize output
        self.output.finalize()
    }

    /// Runs the pipeline with double-buffering (parallel I/O).
    ///
    /// Uses separate thread for I/O to overlap with processing.
    /// This provides significant speedup when I/O and processing
    /// times are similar.
    ///
    /// # Arguments
    ///
    /// * `process` - Function to apply to each region
    ///
    /// # Returns
    ///
    /// Ok(()) on success, error if I/O or processing fails.
    pub fn run_double_buffered<F>(self, mut process: F) -> IoResult<()>
    where
        F: FnMut(&mut Region) + Send + 'static,
    {
        let (img_w, img_h) = self.source.dimensions();
        let tiles: Vec<_> = tile_iterator(img_w, img_h, self.tile_w, self.tile_h).collect();

        // Channel for prefetching regions
        let (read_tx, read_rx): (Sender<Region>, Receiver<Region>) = channel();

        // I/O thread handles reading
        let mut source = self.source;
        let tiles_for_read = tiles.clone();
        let io_handle = thread::spawn(move || -> IoResult<BoxedSource> {
            for tile in tiles_for_read {
                let region = source.read_region(tile.x, tile.y, tile.w, tile.h)?;
                if read_tx.send(region).is_err() {
                    break; // Processing thread died
                }
            }
            Ok(source)
        });

        // Process regions as they arrive
        let mut output = self.output;
        for _ in 0..tiles.len() {
            // Receive from read channel
            let mut region = match read_rx.recv() {
                Ok(r) => r,
                Err(_) => break, // I/O thread error
            };

            // Process
            process(&mut region);

            // Write directly (single-threaded write is usually fine)
            output.write_region(&region)?;
        }

        // Wait for I/O thread
        let _source = io_handle.join().map_err(|_| {
            crate::IoError::UnsupportedOperation("I/O thread panicked".into())
        })??;

        // Finalize output
        output.finalize()
    }
}

/// Progress callback for streaming operations.
pub trait ProgressCallback: Send {
    /// Called after each tile is processed.
    ///
    /// # Arguments
    ///
    /// * `completed` - Number of tiles completed
    /// * `total` - Total number of tiles
    fn on_progress(&mut self, completed: usize, total: usize);
}

impl<F: FnMut(usize, usize) + Send> ProgressCallback for F {
    fn on_progress(&mut self, completed: usize, total: usize) {
        self(completed, total);
    }
}

/// Runs streaming pipeline with progress reporting.
///
/// # Arguments
///
/// * `source` - Streaming source
/// * `output` - Streaming output
/// * `tile_w`, `tile_h` - Tile size
/// * `process` - Processing function
/// * `progress` - Progress callback
pub fn run_with_progress<F, P>(
    mut source: BoxedSource,
    mut output: BoxedOutput,
    tile_w: u32,
    tile_h: u32,
    mut process: F,
    mut progress: P,
) -> IoResult<()>
where
    F: FnMut(&mut Region),
    P: ProgressCallback,
{
    let (img_w, img_h) = source.dimensions();
    let tiles: Vec<_> = tile_iterator(img_w, img_h, tile_w, tile_h).collect();
    let total = tiles.len();

    for (i, tile) in tiles.iter().enumerate() {
        let mut region = source.read_region(tile.x, tile.y, tile.w, tile.h)?;
        process(&mut region);
        output.write_region(&region)?;
        progress.on_progress(i + 1, total);
    }

    output.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_iterator() {
        // 100x50 image with 30x30 tiles
        let tiles: Vec<_> = tile_iterator(100, 50, 30, 30).collect();

        // Should have 4x2 = 8 tiles
        assert_eq!(tiles.len(), 8);

        // First tile
        assert_eq!(tiles[0].x, 0);
        assert_eq!(tiles[0].y, 0);
        assert_eq!(tiles[0].w, 30);
        assert_eq!(tiles[0].h, 30);

        // Last tile (edge)
        let last = &tiles[7];
        assert_eq!(last.x, 90);
        assert_eq!(last.y, 30);
        assert_eq!(last.w, 10); // Edge: 100 - 90 = 10
        assert_eq!(last.h, 20); // Edge: 50 - 30 = 20
    }

    #[test]
    fn test_tile_iterator_exact() {
        // 64x64 with 32x32 tiles - exact fit
        let tiles: Vec<_> = tile_iterator(64, 64, 32, 32).collect();
        assert_eq!(tiles.len(), 4);

        for tile in &tiles {
            assert_eq!(tile.w, 32);
            assert_eq!(tile.h, 32);
        }
    }
}
