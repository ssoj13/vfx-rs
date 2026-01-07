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
//!     ├── TileCluster ──> groups tiles for PCIe optimization
//!     │
//!     └── execute_*() ──> dispatches to strategy
//!             │
//!             ├── FullSource    ─┐
//!             ├── RegionCached  ─┼── all use G::upload/exec_*/download
//!             ├── AdaptiveTiled ─┤
//!             └── Streaming     ─┘
//! ```
//!
//! # Strategies
//!
//! - **FullSource** (≤40% VRAM): Process entire image at once
//! - **RegionCached** (40-80% VRAM): Cache regions for pan/zoom
//! - **AdaptiveTiled** (>80% VRAM): Process in clustered tiles
//! - **Streaming**: Stream from/to disk for huge images

use std::sync::atomic::{AtomicBool, Ordering};

use super::gpu_primitives::{GpuPrimitives, ImageHandle};
use super::tiling::{GpuLimits, ProcessingStrategy, Tile, generate_tiles};
use super::streaming::{StreamingSource, StreamingOutput};
use super::planner::{Planner, ExecutionPlan, Constraints};
use super::cluster::TileCluster;
use super::cache::RegionCache;
use super::memory;
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

fn log(msg: &str) {
    if is_verbose() {
        eprintln!("  {}", msg);
    }
}

fn log_progress(current: usize, total: usize) {
    if is_verbose() && total > 1 {
        eprint!("\r  Tile {}/{}...", current + 1, total);
    }
}

fn log_done(total: usize) {
    if is_verbose() && total > 1 {
        eprintln!("\r  Tile {}/{} - done", total, total);
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
    /// Crop region.
    Crop { x: u32, y: u32, w: u32, h: u32 },
    /// Flip horizontal.
    FlipH,
    /// Flip vertical.
    FlipV,
    /// Rotate 90° clockwise (n times).
    Rotate90(u32),
}

impl ImageOp {
    /// Get kernel radius for operations that need overlap.
    pub fn kernel_radius(&self) -> u32 {
        match self {
            ImageOp::Blur(r) => (*r).ceil() as u32 + 1,
            _ => 0,
        }
    }
}

// =============================================================================
// Executor Configuration
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
            enable_cache: !memory::cache_disabled(),
            cache_budget: None,
            kernel_radius: 0,
            enable_clustering: true,
        }
    }
}

impl ExecutorConfig {
    /// Create config for viewer mode (pan/zoom optimization).
    pub fn viewer() -> Self {
        Self {
            enable_cache: true,
            ..Default::default()
        }
    }

    /// Create config for batch processing (maximum throughput).
    pub fn batch() -> Self {
        Self {
            enable_cache: false,
            enable_clustering: true,
            ..Default::default()
        }
    }
}

// =============================================================================
// Cached Handle Wrapper
// =============================================================================

/// Wrapper for GPU handle with region info for caching.
#[allow(dead_code)]  // TODO: Integrate cache lookup in execute methods
struct CachedHandle<H> {
    handle: H,
    width: u32,
    height: u32,
    channels: u32,
}

// =============================================================================
// Tiled Executor
// =============================================================================

/// Unified tiled executor for GPU backends.
///
/// Wraps any `GpuPrimitives` implementation and provides:
/// - Automatic strategy selection via Planner
/// - Region caching for viewer optimization
/// - Tile clustering for PCIe bandwidth savings
pub struct TiledExecutor<G: GpuPrimitives> {
    gpu: G,
    config: ExecutorConfig,
    planner: Planner,
    /// Region cache for viewer mode (stores GPU handles).
    cache: Option<RegionCache<CachedHandle<G::Handle>>>,
}

impl<G: GpuPrimitives> TiledExecutor<G> {
    /// Create new executor with default config.
    pub fn new(gpu: G) -> Self {
        Self::with_config(gpu, ExecutorConfig::default())
    }

    /// Create with custom config.
    pub fn with_config(gpu: G, config: ExecutorConfig) -> Self {
        let limits = gpu.limits();
        
        // Build planner constraints from GPU limits
        let constraints = Constraints {
            max_tile_dim: limits.max_tile_dim,
            memory_budget: limits.available_memory,
            min_tile_dim: 256,
            kernel_radius: config.kernel_radius,
        };
        let planner = Planner::new(constraints);
        
        // Create cache if enabled
        let cache = if config.enable_cache {
            let budget = config.cache_budget.unwrap_or_else(memory::cache_budget);
            Some(RegionCache::with_budget(budget))
        } else {
            None
        };
        
        Self { gpu, config, planner, cache }
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

    /// Get mutable reference to config.
    pub fn config_mut(&mut self) -> &mut ExecutorConfig {
        &mut self.config
    }

    /// Clear region cache.
    pub fn clear_cache(&mut self) {
        if let Some(ref mut cache) = self.cache {
            cache.clear();
        }
    }

    /// Get cache statistics.
    pub fn cache_stats(&self) -> Option<(u64, u64, f64)> {
        self.cache.as_ref().map(|c| (c.hits(), c.misses(), c.hit_ratio()))
    }

    // =========================================================================
    // Strategy Selection
    // =========================================================================

    /// Create execution plan for image.
    pub fn plan(&self, img: &ComputeImage) -> ExecutionPlan {
        self.plan_with_kernel(img, self.config.kernel_radius)
    }

    /// Create execution plan with specific kernel radius.
    pub fn plan_with_kernel(&self, img: &ComputeImage, kernel_radius: u32) -> ExecutionPlan {
        if self.config.force_streaming {
            let tile_size = self.config.tile_size.unwrap_or(2048);
            return ExecutionPlan {
                strategy: ProcessingStrategy::Streaming { tile_size },
                tiles: vec![],
                clusters: vec![],
                tile_size,
                total_memory: 0,
                clustering_savings: 0.0,
            };
        }
        
        // Use planner for intelligent strategy selection
        let mut planner = self.planner.clone();
        if kernel_radius > 0 {
            planner.constraints_mut().kernel_radius = kernel_radius;
        }
        if let Some(ts) = self.config.tile_size {
            // Override with user-specified tile size
            let mut plan = planner.plan(img.width, img.height, img.channels);
            plan.tile_size = ts;
            return plan;
        }
        
        planner.plan(img.width, img.height, img.channels)
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
        
        let plan = self.plan(img);

        if is_verbose() {
            log(&format!("Backend: {}", self.gpu.name()));
            log(&format!("Image: {}x{} ({} ch)", img.width, img.height, img.channels));
            log(&format!("Strategy: {:?}", plan.strategy));
            if plan.clustering_savings > 0.0 {
                log(&format!("Clustering savings: {:.1}%", plan.clustering_savings * 100.0));
            }
        }

        match plan.strategy {
            ProcessingStrategy::SinglePass => self.execute_color_chain_single(img, ops),
            ProcessingStrategy::Tiled { tile_size, .. } => {
                if self.config.enable_clustering && !plan.clusters.is_empty() {
                    self.execute_color_chain_clustered(img, ops, &plan.clusters)
                } else {
                    self.execute_color_chain_tiled(img, ops, tile_size)
                }
            }
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
            log(&format!("Tiles: {} ({}x{} grid, {}px), {} ops", total, grid_x, grid_y, tile_size, ops.len()));
        }

        for (i, tile) in tiles.iter().enumerate() {
            log_progress(i, total);

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

        log_done(total);
        Ok(())
    }

    /// Clustered color operations - optimizes PCIe bandwidth.
    fn execute_color_chain_clustered(&self, img: &mut ComputeImage, ops: &[ColorOp], clusters: &[TileCluster]) -> ComputeResult<()> {
        let total_clusters = clusters.len();
        let total_tiles: usize = clusters.iter().map(|c| c.tiles.len()).sum();
        
        if is_verbose() {
            log(&format!("Clusters: {} containing {} tiles", total_clusters, total_tiles));
        }

        for (ci, cluster) in clusters.iter().enumerate() {
            log_progress(ci, total_clusters);

            // Upload the entire cluster source region once
            let src = &cluster.source_region;
            let region_data = self.extract_region_from_image(
                img, src.x, src.y, src.w, src.h
            );
            
            let mut current = self.gpu.upload(&region_data, src.w, src.h, img.channels)?;
            let mut next = self.gpu.allocate(src.w, src.h, img.channels)?;

            // Apply all ops to the entire region
            for op in ops {
                self.apply_color_op(&current, &mut next, op)?;
                std::mem::swap(&mut current, &mut next);
            }

            let result = self.gpu.download(&current)?;

            // Write back each tile from the processed region
            for tile in &cluster.tiles {
                // Calculate tile offset within the cluster region
                let local_x = tile.x - src.x;
                let local_y = tile.y - src.y;
                
                let tile_data = self.extract_region(
                    &result, src.w, src.h, img.channels,
                    local_x, local_y, tile.width, tile.height
                );
                
                self.write_tile(img, tile, &tile_data);
            }
        }

        log_done(total_clusters);
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
        let kernel_radius = (radius.ceil() as u32) + 1;
        let plan = self.plan_with_kernel(img, kernel_radius);

        match plan.strategy {
            ProcessingStrategy::SinglePass => {
                let src = self.gpu.upload(&img.data, img.width, img.height, img.channels)?;
                let mut dst = self.gpu.allocate(img.width, img.height, img.channels)?;
                self.gpu.exec_blur(&src, &mut dst, radius)?;
                img.data = self.gpu.download(&dst)?;
                Ok(())
            }
            ProcessingStrategy::Tiled { tile_size, .. } | ProcessingStrategy::Streaming { tile_size } => {
                let overlap = kernel_radius * 2 + 2;
                self.execute_blur_tiled(img, radius, tile_size, overlap)
            }
        }
    }

    /// Tiled blur with overlap.
    fn execute_blur_tiled(&self, img: &mut ComputeImage, radius: f32, tile_size: u32, overlap: u32) -> ComputeResult<()> {
        let tiles = generate_tiles(img.width, img.height, tile_size);
        let total = tiles.len();

        if is_verbose() {
            log(&format!("Blur: {} tiles with {}px overlap", total, overlap));
        }

        for (i, tile) in tiles.iter().enumerate() {
            log_progress(i, total);

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

        log_done(total);
        Ok(())
    }

    /// Execute resize.
    pub fn execute_resize(&self, img: &ComputeImage, width: u32, height: u32, filter: u32) -> ComputeResult<ComputeImage> {
        // Resize needs full image for proper interpolation
        // For very large images, we could use pyramid approach
        let src = self.gpu.upload(&img.data, img.width, img.height, img.channels)?;
        let mut dst = self.gpu.allocate(width, height, img.channels)?;
        self.gpu.exec_resize(&src, &mut dst, filter)?;
        let data = self.gpu.download(&dst)?;
        ComputeImage::from_f32(data, width, height, img.channels)
    }

    /// Execute crop (simple, doesn't need GPU).
    pub fn execute_crop(&self, img: &ComputeImage, x: u32, y: u32, w: u32, h: u32) -> ComputeResult<ComputeImage> {
        let data = self.extract_region_from_image(img, x, y, w, h);
        ComputeImage::from_f32(data, w, h, img.channels)
    }

    /// Execute flip horizontal.
    pub fn execute_flip_h(&self, img: &mut ComputeImage) -> ComputeResult<()> {
        let mut handle = self.gpu.upload(&img.data, img.width, img.height, img.channels)?;
        self.gpu.exec_flip_h(&mut handle)?;
        img.data = self.gpu.download(&handle)?;
        Ok(())
    }

    /// Execute flip vertical.
    pub fn execute_flip_v(&self, img: &mut ComputeImage) -> ComputeResult<()> {
        let mut handle = self.gpu.upload(&img.data, img.width, img.height, img.channels)?;
        self.gpu.exec_flip_v(&mut handle)?;
        img.data = self.gpu.download(&handle)?;
        Ok(())
    }

    /// Execute rotate 90° (n times clockwise).
    pub fn execute_rotate_90(&self, img: &ComputeImage, n: u32) -> ComputeResult<ComputeImage> {
        let handle = self.gpu.upload(&img.data, img.width, img.height, img.channels)?;
        let rotated = self.gpu.exec_rotate_90(&handle, n)?;
        let (w, h, c) = rotated.dimensions();
        let data = self.gpu.download(&rotated)?;
        ComputeImage::from_f32(data, w, h, c)
    }

    /// Execute composite (fg over bg).
    pub fn execute_composite_over(&self, fg: &ComputeImage, bg: &mut ComputeImage) -> ComputeResult<()> {
        let plan = self.plan(bg);
        
        match plan.strategy {
            ProcessingStrategy::SinglePass => {
                let fg_handle = self.gpu.upload(&fg.data, fg.width, fg.height, fg.channels)?;
                let mut bg_handle = self.gpu.upload(&bg.data, bg.width, bg.height, bg.channels)?;
                self.gpu.exec_composite_over(&fg_handle, &mut bg_handle)?;
                bg.data = self.gpu.download(&bg_handle)?;
                Ok(())
            }
            ProcessingStrategy::Tiled { tile_size, .. } | ProcessingStrategy::Streaming { tile_size } => {
                self.execute_composite_over_tiled(fg, bg, tile_size)
            }
        }
    }

    /// Tiled composite.
    fn execute_composite_over_tiled(&self, fg: &ComputeImage, bg: &mut ComputeImage, tile_size: u32) -> ComputeResult<()> {
        let tiles = generate_tiles(bg.width, bg.height, tile_size);
        let total = tiles.len();

        for (i, tile) in tiles.iter().enumerate() {
            log_progress(i, total);

            let fg_data = self.extract_tile(fg, tile);
            let bg_data = self.extract_tile(bg, tile);

            let fg_handle = self.gpu.upload(&fg_data, tile.width, tile.height, fg.channels)?;
            let mut bg_handle = self.gpu.upload(&bg_data, tile.width, tile.height, bg.channels)?;
            
            self.gpu.exec_composite_over(&fg_handle, &mut bg_handle)?;
            
            let result = self.gpu.download(&bg_handle)?;
            self.write_tile(bg, tile, &result);
        }

        log_done(total);
        Ok(())
    }

    /// Execute blend with mode.
    pub fn execute_blend(&self, fg: &ComputeImage, bg: &mut ComputeImage, mode: u32, opacity: f32) -> ComputeResult<()> {
        let plan = self.plan(bg);
        
        match plan.strategy {
            ProcessingStrategy::SinglePass => {
                let fg_handle = self.gpu.upload(&fg.data, fg.width, fg.height, fg.channels)?;
                let mut bg_handle = self.gpu.upload(&bg.data, bg.width, bg.height, bg.channels)?;
                self.gpu.exec_blend(&fg_handle, &mut bg_handle, mode, opacity)?;
                bg.data = self.gpu.download(&bg_handle)?;
                Ok(())
            }
            ProcessingStrategy::Tiled { tile_size, .. } | ProcessingStrategy::Streaming { tile_size } => {
                self.execute_blend_tiled(fg, bg, mode, opacity, tile_size)
            }
        }
    }

    /// Tiled blend.
    fn execute_blend_tiled(&self, fg: &ComputeImage, bg: &mut ComputeImage, mode: u32, opacity: f32, tile_size: u32) -> ComputeResult<()> {
        let tiles = generate_tiles(bg.width, bg.height, tile_size);
        let total = tiles.len();

        for (i, tile) in tiles.iter().enumerate() {
            log_progress(i, total);

            let fg_data = self.extract_tile(fg, tile);
            let bg_data = self.extract_tile(bg, tile);

            let fg_handle = self.gpu.upload(&fg_data, tile.width, tile.height, fg.channels)?;
            let mut bg_handle = self.gpu.upload(&bg_data, tile.width, tile.height, bg.channels)?;
            
            self.gpu.exec_blend(&fg_handle, &mut bg_handle, mode, opacity)?;
            
            let result = self.gpu.download(&bg_handle)?;
            self.write_tile(bg, tile, &result);
        }

        log_done(total);
        Ok(())
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
        self.execute_color_chain_streaming(source, output, std::slice::from_ref(op))
    }

    /// Execute chained color operations with streaming I/O.
    pub fn execute_color_chain_streaming<S, O>(
        &self,
        source: &mut S,
        output: &mut O,
        ops: &[ColorOp],
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
            log(&format!("Streaming: {} tiles from {:?}", total, source.format()));
        }

        for (i, tile) in tiles.iter().enumerate() {
            log_progress(i, total);

            // Read tile from source
            let tile_data = source.read_region(tile.x, tile.y, tile.width, tile.height)?;

            // Process with chained ops
            let mut current = self.gpu.upload(&tile_data, tile.width, tile.height, channels)?;
            let mut next = self.gpu.allocate(tile.width, tile.height, channels)?;

            for op in ops {
                self.apply_color_op(&current, &mut next, op)?;
                std::mem::swap(&mut current, &mut next);
            }

            let result = self.gpu.download(&current)?;

            // Write to output
            output.write_region(tile.x, tile.y, tile.width, tile.height, &result)?;
        }

        output.finish()?;

        log_done(total);
        Ok(())
    }

    // =========================================================================
    // Tile Helpers
    // =========================================================================

    /// Extract tile data from image.
    fn extract_tile(&self, img: &ComputeImage, tile: &Tile) -> Vec<f32> {
        self.extract_region_from_image(img, tile.x, tile.y, tile.width, tile.height)
    }

    /// Extract region from image.
    fn extract_region_from_image(&self, img: &ComputeImage, x: u32, y: u32, w: u32, h: u32) -> Vec<f32> {
        let c = img.channels as usize;
        let stride = (img.width as usize) * c;
        let mut data = Vec::with_capacity((w * h) as usize * c);

        for row in y..(y + h) {
            let start = (row as usize) * stride + (x as usize) * c;
            let end = start + (w as usize) * c;
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

    #[test]
    fn test_executor_config_presets() {
        let viewer = ExecutorConfig::viewer();
        assert!(viewer.enable_cache);

        let batch = ExecutorConfig::batch();
        assert!(!batch.enable_cache);
        assert!(batch.enable_clustering);
    }
}
