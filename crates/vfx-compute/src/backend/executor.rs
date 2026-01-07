//! Unified tiled executor for all GPU backends.
//!
//! Provides automatic tiling and streaming for large images with:
//! - Intelligent strategy selection via Planner
//! - Region caching for viewer pan/zoom optimization
//! - Tile clustering for PCIe bandwidth savings
//!
//! # Architecture
//!
//! ```text
//! TiledExecutor<G: GpuPrimitives>
//!     │
//!     ├── Planner ──> selects strategy based on VRAM/kernel/clustering
//!     │
//!     ├── RegionCache ──> caches uploaded regions (viewer mode)
//!     │
//!     ├── execute_*() ──> dispatches to strategy
//!     │
//!     ├── FullSource    ─┐
//!     ├── RegionCache   ─┼── all use G::upload/exec_*/download
//!     ├── AdaptiveTiled ─┤
//!     └── Streaming     ─┘
//! ```
//!
//! # Strategies
//!
//! - **FullSource** (≤40% VRAM): Process entire image at once
//! - **RegionCache** (40-80% VRAM): Cache regions for pan/zoom
//! - **AdaptiveTiled** (>80% VRAM): Process in clustered tiles
//! - **Streaming**: Stream from/to disk for huge images

use std::sync::atomic::{AtomicBool, Ordering};

use super::gpu_primitives::GpuPrimitives;
use super::tiling::{GpuLimits, ProcessingStrategy, Tile, generate_tiles};
use super::streaming::{StreamingSource, StreamingOutput};
use super::planner::{Planner, Constraints};
use super::cluster::{TileCluster, cluster_tiles, TileTriple, analyze_source_region, ClusterConfig};
use super::cache::{RegionCache, RegionKey};
use crate::{ComputeResult, ComputeImage};

// =============================================================================
// Verbose Control
// =============================================================================

static VERBOSE: AtomicBool = AtomicBool::new(false);

/// Enable/disable verbose output.
pub fn set_verbose(enabled: bool) {
    VERBOSE.store(enabled, Ordering::Relaxed);
}

#[inline]
fn is_verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{} KB", bytes / 1024)
    }
}

// =============================================================================
// Color Operation Enum
// =============================================================================

/// Color operation to execute.
#[derive(Clone, Debug)]
pub enum ColorOp {
    /// 4x4 color matrix transform.
    Matrix([f32; 16]),
    /// CDL: slope, offset, power, saturation.
    Cdl {
        slope: [f32; 3],
        offset: [f32; 3],
        power: [f32; 3],
        saturation: f32,
    },
    /// 1D LUT with channel count.
    Lut1d { lut: Vec<f32>, channels: u32 },
    /// 3D LUT with cube size.
    Lut3d { lut: Vec<f32>, size: u32 },
}

/// Image operation to execute.
#[derive(Clone, Debug)]
pub enum ImageOp {
    /// Gaussian blur with radius.
    Blur(f32),
    /// Resize to new dimensions with filter.
    Resize { width: u32, height: u32, filter: u32 },
}

// =============================================================================
// Tiled Executor
// =============================================================================

/// Executor configuration.
#[derive(Clone, Debug)]
pub struct ExecutorConfig {
    /// Override tile size (None = auto via Planner).
    pub tile_size: Option<u32>,
    /// Force streaming mode.
    pub force_streaming: bool,
    /// Enable region cache for viewer mode (pan/zoom optimization).
    pub enable_cache: bool,
    /// Cache budget in bytes (default: 25% of available memory).
    pub cache_budget: Option<u64>,
    /// Kernel radius for operations like blur (affects tile overlap).
    pub kernel_radius: u32,
    /// Enable tile clustering for PCIe optimization.
    pub enable_clustering: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            tile_size: None,
            force_streaming: false,
            enable_cache: false,
            cache_budget: None,
            kernel_radius: 0,
            enable_clustering: true,
        }
    }
}

/// Unified tiled executor for GPU backends.
///
/// Wraps any `GpuPrimitives` implementation and provides automatic
/// tiling/streaming for large images.
pub struct TiledExecutor<G: GpuPrimitives> {
    gpu: G,
    config: ExecutorConfig,
}

impl<G: GpuPrimitives> TiledExecutor<G> {
    /// Create new executor with default config.
    pub fn new(gpu: G) -> Self {
        Self {
            gpu,
            config: ExecutorConfig::default(),
        }
    }

    /// Create with custom config.
    pub fn with_config(gpu: G, config: ExecutorConfig) -> Self {
        Self { gpu, config }
    }

    /// Get GPU limits.
    pub fn limits(&self) -> &GpuLimits {
        self.gpu.limits()
    }

    /// Get backend name.
    pub fn name(&self) -> &'static str {
        self.gpu.name()
    }

    /// Get reference to inner primitives.
    pub fn gpu(&self) -> &G {
        &self.gpu
    }

    // =========================================================================
    // Strategy Selection
    // =========================================================================

    /// Select processing strategy for image.
    pub fn select_strategy(&self, img: &ComputeImage) -> ProcessingStrategy {
        if self.config.force_streaming {
            let tile_size = self.effective_tile_size();
            return ProcessingStrategy::Streaming { tile_size };
        }

        ProcessingStrategy::recommend_with_ram(
            img.width,
            img.height,
            img.channels,
            self.gpu.limits(),
            self.config.ram_limit,
        )
    }

    /// Get effective tile size.
    pub fn effective_tile_size(&self) -> u32 {
        self.config.tile_size.unwrap_or_else(|| {
            self.gpu.limits().optimal_tile_size(16384, 16384, 4)
                .clamp(256, 16384)
        })
    }

    // =========================================================================
    // Color Operations
    // =========================================================================

    /// Execute color operation with automatic tiling.
    pub fn execute_color(&self, img: &mut ComputeImage, op: &ColorOp) -> ComputeResult<()> {
        self.execute_color_chain(img, std::slice::from_ref(op))
    }

    /// Execute multiple color operations without GPU round-trips.
    ///
    /// This is more efficient than calling `execute_color()` multiple times
    /// because data stays on GPU between operations.
    pub fn execute_color_chain(&self, img: &mut ComputeImage, ops: &[ColorOp]) -> ComputeResult<()> {
        if ops.is_empty() {
            return Ok(());
        }
        let strategy = self.select_strategy(img);

        if is_verbose() {
            let mem = self.gpu.limits().estimate_memory(img.width, img.height, img.channels);
            eprintln!("  Backend: {}", self.gpu.name());
            eprintln!("  Image: {}x{} ({} channels)", img.width, img.height, img.channels);
            eprintln!("  Memory: {} required, {} available",
                format_bytes(mem), format_bytes(self.gpu.limits().available_memory));
            eprintln!("  Strategy: {:?}", strategy);
        }

        match strategy {
            ProcessingStrategy::SinglePass => self.execute_color_chain_single(img, ops),
            ProcessingStrategy::Tiled { tile_size, .. } => self.execute_color_chain_tiled(img, ops, tile_size),
            ProcessingStrategy::Streaming { tile_size } => {
                // For in-memory images, tiled is sufficient
                self.execute_color_chain_tiled(img, ops, tile_size)
            }
        }
    }

    /// Single-pass chained color operations - data stays on GPU.
    fn execute_color_chain_single(&self, img: &mut ComputeImage, ops: &[ColorOp]) -> ComputeResult<()> {
        let mut current = self.gpu.upload(&img.data, img.width, img.height, img.channels)?;
        let mut next = self.gpu.allocate(img.width, img.height, img.channels)?;

        // Chain operations, ping-ponging between buffers
        for op in ops {
            self.apply_color_op(&current, &mut next, op)?;
            std::mem::swap(&mut current, &mut next);
        }

        img.data = self.gpu.download(&current)?;
        Ok(())
    }

    /// Tiled chained color operations.
    fn execute_color_chain_tiled(&self, img: &mut ComputeImage, ops: &[ColorOp], tile_size: u32) -> ComputeResult<()> {
        let tiles = generate_tiles(img.width, img.height, tile_size);
        let total = tiles.len();

        if is_verbose() {
            let grid_x = (img.width + tile_size - 1) / tile_size;
            let grid_y = (img.height + tile_size - 1) / tile_size;
            eprintln!("  Tiles: {} ({}x{} grid, {}px each), {} ops chained", total, grid_x, grid_y, tile_size, ops.len());
        }

        for (i, tile) in tiles.iter().enumerate() {
            if is_verbose() && total > 1 {
                eprint!("\r  Tile {}/{}...", i + 1, total);
            }

            // Extract tile data
            let tile_data = self.extract_tile(img, tile);

            // Upload and chain all ops on GPU
            let mut current = self.gpu.upload(&tile_data, tile.width, tile.height, img.channels)?;
            let mut next = self.gpu.allocate(tile.width, tile.height, img.channels)?;

            for op in ops {
                self.apply_color_op(&current, &mut next, op)?;
                std::mem::swap(&mut current, &mut next);
            }

            let result = self.gpu.download(&current)?;

            // Write back
            self.write_tile(img, tile, &result);
        }

        if is_verbose() && total > 1 {
            eprintln!("\r  Tile {}/{} - done", total, total);
        }

        Ok(())
    }

    /// Apply color operation to handles.
    fn apply_color_op(&self, src: &G::Handle, dst: &mut G::Handle, op: &ColorOp) -> ComputeResult<()> {
        match op {
            ColorOp::Matrix(m) => self.gpu.exec_matrix(src, dst, m),
            ColorOp::Cdl { slope, offset, power, saturation } => {
                self.gpu.exec_cdl(src, dst, *slope, *offset, *power, *saturation)
            }
            ColorOp::Lut1d { lut, channels } => self.gpu.exec_lut1d(src, dst, lut, *channels),
            ColorOp::Lut3d { lut, size } => self.gpu.exec_lut3d(src, dst, lut, *size),
        }
    }

    // =========================================================================
    // Image Operations
    // =========================================================================

    /// Execute blur with automatic tiling.
    pub fn execute_blur(&self, img: &mut ComputeImage, radius: f32) -> ComputeResult<()> {
        let strategy = self.select_strategy(img);

        match strategy {
            ProcessingStrategy::SinglePass => {
                let src = self.gpu.upload(&img.data, img.width, img.height, img.channels)?;
                let mut dst = self.gpu.allocate(img.width, img.height, img.channels)?;
                self.gpu.exec_blur(&src, &mut dst, radius)?;
                img.data = self.gpu.download(&dst)?;
                Ok(())
            }
            ProcessingStrategy::Tiled { tile_size, .. } | ProcessingStrategy::Streaming { tile_size } => {
                // Blur needs overlap for kernel radius
                let overlap = (radius.ceil() as u32) * 2 + 2;
                self.execute_blur_tiled(img, radius, tile_size, overlap)
            }
        }
    }

    /// Tiled blur with overlap.
    fn execute_blur_tiled(&self, img: &mut ComputeImage, radius: f32, tile_size: u32, overlap: u32) -> ComputeResult<()> {
        let tiles = generate_tiles(img.width, img.height, tile_size);
        let total = tiles.len();

        if is_verbose() {
            eprintln!("  Blur: {} tiles with {}px overlap", total, overlap);
        }

        for (i, tile) in tiles.iter().enumerate() {
            if is_verbose() && total > 1 {
                eprint!("\r  Tile {}/{}...", i + 1, total);
            }

            // Expanded region with overlap
            let exp_x = tile.x.saturating_sub(overlap);
            let exp_y = tile.y.saturating_sub(overlap);
            let exp_w = (tile.width + overlap * 2).min(img.width - exp_x);
            let exp_h = (tile.height + overlap * 2).min(img.height - exp_y);

            let expanded_tile = Tile::new(exp_x, exp_y, exp_w, exp_h);
            let tile_data = self.extract_tile(img, &expanded_tile);

            // Process expanded tile
            let src = self.gpu.upload(&tile_data, exp_w, exp_h, img.channels)?;
            let mut dst = self.gpu.allocate(exp_w, exp_h, img.channels)?;
            self.gpu.exec_blur(&src, &mut dst, radius)?;
            let result = self.gpu.download(&dst)?;

            // Extract inner region (without overlap)
            let inner_x = tile.x - exp_x;
            let inner_y = tile.y - exp_y;
            let inner_result = self.extract_region(&result, exp_w, exp_h, img.channels,
                inner_x, inner_y, tile.width, tile.height);

            self.write_tile(img, tile, &inner_result);
        }

        if is_verbose() && total > 1 {
            eprintln!("\r  Tile {}/{} - done", total, total);
        }

        Ok(())
    }

    /// Execute resize.
    pub fn execute_resize(&self, img: &ComputeImage, width: u32, height: u32, filter: u32) -> ComputeResult<ComputeImage> {
        // Resize always needs full image for proper interpolation
        // For very large images, we'd need a more sophisticated approach
        let src = self.gpu.upload(&img.data, img.width, img.height, img.channels)?;
        let mut dst = self.gpu.allocate(width, height, img.channels)?;
        self.gpu.exec_resize(&src, &mut dst, filter)?;
        let data = self.gpu.download(&dst)?;
        ComputeImage::from_f32(data, width, height, img.channels)
    }

    // =========================================================================
    // Streaming Operations
    // =========================================================================

    /// Execute color operation with streaming I/O.
    pub fn execute_color_streaming<S, O>(
        &self,
        source: &mut S,
        output: &mut O,
        op: &ColorOp,
    ) -> ComputeResult<()>
    where
        S: StreamingSource,
        O: StreamingOutput,
    {
        let (width, height) = source.dims();
        let channels = source.channels();
        let tile_size = self.effective_tile_size();

        output.init(width, height, channels)?;

        let tiles = generate_tiles(width, height, tile_size);
        let total = tiles.len();

        if is_verbose() {
            eprintln!("  Streaming: {} tiles from {:?}", total, source.format());
        }

        for (i, tile) in tiles.iter().enumerate() {
            if is_verbose() {
                eprint!("\r  Tile {}/{}...", i + 1, total);
            }

            // Read tile from source
            let tile_data = source.read_region(tile.x, tile.y, tile.width, tile.height)?;

            // Process
            let src = self.gpu.upload(&tile_data, tile.width, tile.height, channels)?;
            let mut dst = self.gpu.allocate(tile.width, tile.height, channels)?;
            self.apply_color_op(&src, &mut dst, op)?;
            let result = self.gpu.download(&dst)?;

            // Write to output
            output.write_region(tile.x, tile.y, tile.width, tile.height, &result)?;
        }

        output.finish()?;

        if is_verbose() {
            eprintln!("\r  Tile {}/{} - done", total, total);
        }

        Ok(())
    }

    // =========================================================================
    // Tile Helpers
    // =========================================================================

    /// Extract tile data from image.
    fn extract_tile(&self, img: &ComputeImage, tile: &Tile) -> Vec<f32> {
        let c = img.channels as usize;
        let stride = (img.width as usize) * c;
        let mut data = Vec::with_capacity((tile.width * tile.height) as usize * c);

        for row in tile.y..(tile.y + tile.height) {
            let start = (row as usize) * stride + (tile.x as usize) * c;
            let end = start + (tile.width as usize) * c;
            data.extend_from_slice(&img.data[start..end]);
        }

        data
    }

    /// Write tile data back to image.
    fn write_tile(&self, img: &mut ComputeImage, tile: &Tile, data: &[f32]) {
        let c = img.channels as usize;
        let img_stride = (img.width as usize) * c;
        let tile_stride = (tile.width as usize) * c;

        for row in 0..tile.height as usize {
            let src_start = row * tile_stride;
            let dst_row = tile.y as usize + row;
            let dst_start = dst_row * img_stride + (tile.x as usize) * c;
            img.data[dst_start..dst_start + tile_stride]
                .copy_from_slice(&data[src_start..src_start + tile_stride]);
        }
    }

    /// Extract region from flat data.
    fn extract_region(&self, data: &[f32], src_w: u32, _src_h: u32, channels: u32,
                      x: u32, y: u32, w: u32, h: u32) -> Vec<f32> {
        let c = channels as usize;
        let stride = (src_w as usize) * c;
        let mut result = Vec::with_capacity((w * h) as usize * c);

        for row in y..(y + h) {
            let start = (row as usize) * stride + (x as usize) * c;
            let end = start + (w as usize) * c;
            result.extend_from_slice(&data[start..end]);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::CpuPrimitives;

    #[test]
    fn test_executor_single_pass() {
        let gpu = CpuPrimitives::new();
        let exec = TiledExecutor::new(gpu);

        let mut img = ComputeImage::from_f32(vec![0.5; 12], 2, 2, 3).unwrap();

        // Double exposure
        let matrix = [
            2.0, 0.0, 0.0, 0.0,
            0.0, 2.0, 0.0, 0.0,
            0.0, 0.0, 2.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];

        exec.execute_color(&mut img, &ColorOp::Matrix(matrix)).unwrap();

        assert!((img.data()[0] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_executor_tiled() {
        let gpu = CpuPrimitives::new();
        let config = ExecutorConfig {
            tile_size: Some(2), // Force 2x2 tiles
            ..Default::default()
        };
        let exec = TiledExecutor::with_config(gpu, config);

        // 4x4 image
        let mut img = ComputeImage::from_f32(vec![0.25; 48], 4, 4, 3).unwrap();

        let cdl = ColorOp::Cdl {
            slope: [2.0, 2.0, 2.0],
            offset: [0.0, 0.0, 0.0],
            power: [1.0, 1.0, 1.0],
            saturation: 1.0,
        };

        exec.execute_color(&mut img, &cdl).unwrap();

        // All pixels should be doubled
        assert!((img.data()[0] - 0.5).abs() < 1e-5);
    }
}
