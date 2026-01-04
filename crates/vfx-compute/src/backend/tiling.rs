//! GPU tiling for large images.
//!
//! Provides VRAM-aware tile sizing for efficient GPU processing of large images.
//! Automatically adapts tile size based on available GPU memory to avoid OOM errors.
//!
//! # Example
//!
//! ```rust,ignore
//! use vfx_compute::backend::tiling::{GpuLimits, ProcessingStrategy};
//!
//! let limits = GpuLimits::detect(); // Auto-detect from wgpu
//! let strategy = ProcessingStrategy::recommend(8192, 8192, 4, &limits);
//!
//! match strategy {
//!     ProcessingStrategy::SinglePass => { /* Process whole image */ }
//!     ProcessingStrategy::Tiled { tile_size } => { /* Process in tiles */ }
//!     ProcessingStrategy::Streaming => { /* Use StreamingSource */ }
//! }
//! ```

/// Memory safety margins.
///
/// We reserve memory for:
/// - Intermediate buffers during processing
/// - GPU driver overhead
/// - Other concurrent GPU operations
const VRAM_SAFETY_MARGIN: f64 = 0.4; // Use max 60% of VRAM
const VRAM_TILE_OVERHEAD: f64 = 3.0; // src + dst + intermediate = 3x tile size

/// Default assumptions when GPU info unavailable.
const DEFAULT_VRAM_BYTES: u64 = 2 * 1024 * 1024 * 1024; // 2 GB
const DEFAULT_MAX_TEXTURE_DIM: u32 = 16384;
const DEFAULT_MAX_BUFFER_BYTES: u64 = 256 * 1024 * 1024; // 256 MB

/// GPU resource limits.
///
/// Contains detected or default GPU capabilities used for VRAM-aware tile sizing.
#[derive(Debug, Clone)]
pub struct GpuLimits {
    /// Maximum texture dimension (width or height).
    pub max_tile_dim: u32,
    /// Maximum buffer size in bytes.
    pub max_buffer_bytes: u64,
    /// Total GPU memory in bytes (detected or estimated).
    pub total_memory: u64,
    /// Available GPU memory in bytes (after safety margin).
    pub available_memory: u64,
    /// Whether values were auto-detected vs defaults.
    pub detected: bool,
}

impl Default for GpuLimits {
    fn default() -> Self {
        Self {
            max_tile_dim: DEFAULT_MAX_TEXTURE_DIM,
            max_buffer_bytes: DEFAULT_MAX_BUFFER_BYTES,
            total_memory: DEFAULT_VRAM_BYTES,
            available_memory: (DEFAULT_VRAM_BYTES as f64 * (1.0 - VRAM_SAFETY_MARGIN)) as u64,
            detected: false,
        }
    }
}

impl GpuLimits {
    /// Creates limits with specified VRAM (applies safety margin).
    pub fn with_vram(total_vram_bytes: u64) -> Self {
        let available = (total_vram_bytes as f64 * (1.0 - VRAM_SAFETY_MARGIN)) as u64;
        Self {
            max_tile_dim: DEFAULT_MAX_TEXTURE_DIM,
            max_buffer_bytes: DEFAULT_MAX_BUFFER_BYTES,
            total_memory: total_vram_bytes,
            available_memory: available,
            detected: true,
        }
    }

    /// Creates limits from wgpu adapter limits.
    #[cfg(feature = "wgpu")]
    pub fn from_wgpu_limits(limits: &wgpu::Limits, total_vram: Option<u64>) -> Self {
        let max_dim = limits.max_texture_dimension_2d;
        let max_buffer = limits.max_buffer_size;
        let total = total_vram.unwrap_or(DEFAULT_VRAM_BYTES);
        let available = (total as f64 * (1.0 - VRAM_SAFETY_MARGIN)) as u64;

        Self {
            max_tile_dim: max_dim,
            max_buffer_bytes: max_buffer,
            total_memory: total,
            available_memory: available,
            detected: true,
        }
    }

    /// Check if image needs tiling based on texture size limits.
    pub fn needs_tiling(&self, width: u32, height: u32) -> bool {
        width > self.max_tile_dim || height > self.max_tile_dim
    }

    /// Estimate memory required for processing an image.
    ///
    /// Accounts for source, destination, and intermediate buffers.
    pub fn estimate_memory(&self, width: u32, height: u32, channels: u32) -> u64 {
        let bytes_per_pixel = (channels as u64) * 4; // f32
        let image_bytes = (width as u64) * (height as u64) * bytes_per_pixel;
        (image_bytes as f64 * VRAM_TILE_OVERHEAD) as u64
    }

    /// Check if image fits in available memory with headroom.
    pub fn fits_memory(&self, width: u32, height: u32, channels: u32) -> bool {
        self.estimate_memory(width, height, channels) <= self.available_memory
    }

    /// Calculate optimal tile size based on available VRAM.
    ///
    /// Returns tile size that:
    /// 1. Fits within texture dimension limits
    /// 2. Fits within available VRAM (with overhead)
    /// 3. Is power-of-2 aligned for GPU efficiency
    pub fn optimal_tile_size(&self, width: u32, height: u32, channels: u32) -> u32 {
        let bytes_per_pixel = (channels as u64) * 4;

        // Max tile bytes = available / overhead factor
        let max_tile_bytes = (self.available_memory as f64 / VRAM_TILE_OVERHEAD) as u64;

        // Solve for tile dimension: tile^2 * bpp <= max_tile_bytes
        let max_tile_from_mem = ((max_tile_bytes / bytes_per_pixel) as f64).sqrt() as u32;

        // Clamp to texture limits
        let max_tile = max_tile_from_mem.min(self.max_tile_dim);

        // Round down to power of 2 for efficiency
        let tile = round_down_pow2(max_tile);

        // Minimum tile size to avoid too many tiles
        let min_tile = 256;
        let tile = tile.max(min_tile);

        // Don't exceed image dimensions
        tile.min(width).min(height)
    }

    /// Calculate tile size optimized for specific workflow.
    pub fn tile_size_for_workflow(&self, width: u32, height: u32, channels: u32, workflow: TileWorkflow) -> u32 {
        let base = self.optimal_tile_size(width, height, channels);

        match workflow {
            TileWorkflow::ColorTransform => base, // Standard
            TileWorkflow::Convolution { kernel_radius } => {
                // Need overlap for convolution kernels
                let overlap = kernel_radius * 2;
                (base - overlap).max(256)
            }
            TileWorkflow::Warp => {
                // Warp may sample outside tile boundaries
                // Use smaller tiles with more overlap
                (base / 2).max(512)
            }
            TileWorkflow::Composite => base, // Standard
        }
    }
}

/// Workflow type for tile size optimization.
#[derive(Debug, Clone, Copy)]
pub enum TileWorkflow {
    /// Simple per-pixel transforms (color matrix, LUT, CDL).
    ColorTransform,
    /// Convolution filters requiring neighbor access.
    Convolution { kernel_radius: u32 },
    /// Warp/distortion with arbitrary sampling.
    Warp,
    /// Layer compositing.
    Composite,
}

/// Processing strategy based on image size and GPU limits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProcessingStrategy {
    /// Process entire image in one pass (fits in VRAM).
    SinglePass,
    /// Process in tiles (too large for single pass but fits in RAM).
    Tiled {
        tile_size: u32,
        num_tiles: u32,
    },
    /// Use streaming I/O (too large for RAM).
    Streaming {
        tile_size: u32,
    },
}

impl ProcessingStrategy {
    /// Recommend processing strategy based on image size and GPU limits.
    ///
    /// Decision tree:
    /// 1. If fits VRAM with headroom -> SinglePass
    /// 2. If exceeds VRAM but fits RAM (< 80% system RAM) -> Tiled
    /// 3. Otherwise -> Streaming
    pub fn recommend(width: u32, height: u32, channels: u32, limits: &GpuLimits) -> Self {
        let required_vram = limits.estimate_memory(width, height, channels);

        // Check if fits in single pass
        if required_vram <= limits.available_memory && !limits.needs_tiling(width, height) {
            return Self::SinglePass;
        }

        let tile_size = limits.optimal_tile_size(width, height, channels);
        let num_tiles_x = (width + tile_size - 1) / tile_size;
        let num_tiles_y = (height + tile_size - 1) / tile_size;
        let num_tiles = num_tiles_x * num_tiles_y;

        // Estimate RAM usage (conservative: 2x for src + dst in RAM)
        let bytes_per_pixel = (channels as u64) * 4;
        let ram_bytes = (width as u64) * (height as u64) * bytes_per_pixel * 2;

        // Use streaming if >8GB RAM would be needed (conservative threshold)
        let streaming_threshold = 8 * 1024 * 1024 * 1024u64;

        if ram_bytes > streaming_threshold {
            Self::Streaming { tile_size }
        } else {
            Self::Tiled { tile_size, num_tiles }
        }
    }

    /// Recommend strategy with explicit RAM limit.
    pub fn recommend_with_ram(
        width: u32,
        height: u32,
        channels: u32,
        limits: &GpuLimits,
        available_ram: u64,
    ) -> Self {
        let required_vram = limits.estimate_memory(width, height, channels);

        if required_vram <= limits.available_memory && !limits.needs_tiling(width, height) {
            return Self::SinglePass;
        }

        let tile_size = limits.optimal_tile_size(width, height, channels);
        let num_tiles_x = (width + tile_size - 1) / tile_size;
        let num_tiles_y = (height + tile_size - 1) / tile_size;
        let num_tiles = num_tiles_x * num_tiles_y;

        // Estimate RAM: src + dst + working buffer
        let bytes_per_pixel = (channels as u64) * 4;
        let ram_bytes = (width as u64) * (height as u64) * bytes_per_pixel * 3;

        // Use 70% of available RAM as threshold
        let ram_threshold = (available_ram as f64 * 0.7) as u64;

        if ram_bytes > ram_threshold {
            Self::Streaming { tile_size }
        } else {
            Self::Tiled { tile_size, num_tiles }
        }
    }
}

/// Round down to nearest power of 2.
fn round_down_pow2(n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    1 << (31 - n.leading_zeros())
}

/// A tile region within an image.
#[derive(Debug, Clone, Copy)]
pub struct Tile {
    /// X offset in source image.
    pub x: u32,
    /// Y offset in source image.
    pub y: u32,
    /// Tile width.
    pub width: u32,
    /// Tile height.
    pub height: u32,
}

impl Tile {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }
    
    /// Full image as single tile.
    pub fn full(width: u32, height: u32) -> Self {
        Self::new(0, 0, width, height)
    }
}

/// Generate tiles for processing large images.
pub fn generate_tiles(width: u32, height: u32, tile_size: u32) -> Vec<Tile> {
    let mut tiles = Vec::new();
    
    let mut y = 0;
    while y < height {
        let th = tile_size.min(height - y);
        let mut x = 0;
        while x < width {
            let tw = tile_size.min(width - x);
            tiles.push(Tile::new(x, y, tw, th));
            x += tile_size;
        }
        y += tile_size;
    }
    
    tiles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_tiles() {
        let tiles = generate_tiles(1000, 1000, 512);
        assert_eq!(tiles.len(), 4); // 2x2 grid

        assert_eq!(tiles[0].x, 0);
        assert_eq!(tiles[0].width, 512);
        assert_eq!(tiles[1].x, 512);
        assert_eq!(tiles[1].width, 488);
    }

    #[test]
    fn test_single_tile() {
        let tiles = generate_tiles(256, 256, 512);
        assert_eq!(tiles.len(), 1);
        assert_eq!(tiles[0].width, 256);
    }

    #[test]
    fn test_round_down_pow2() {
        assert_eq!(round_down_pow2(1000), 512);
        assert_eq!(round_down_pow2(512), 512);
        assert_eq!(round_down_pow2(1024), 1024);
        assert_eq!(round_down_pow2(2000), 1024);
        assert_eq!(round_down_pow2(4096), 4096);
        assert_eq!(round_down_pow2(0), 0);
    }

    #[test]
    fn test_gpu_limits_default() {
        let limits = GpuLimits::default();
        assert!(!limits.detected);
        assert!(limits.available_memory < limits.total_memory);
    }

    #[test]
    fn test_gpu_limits_with_vram() {
        let limits = GpuLimits::with_vram(8 * 1024 * 1024 * 1024); // 8 GB
        assert!(limits.detected);
        assert_eq!(limits.total_memory, 8 * 1024 * 1024 * 1024);
        // Available should be 60% of total (40% safety margin)
        assert!(limits.available_memory < limits.total_memory);
    }

    #[test]
    fn test_optimal_tile_size() {
        let limits = GpuLimits::with_vram(4 * 1024 * 1024 * 1024); // 4 GB

        // Small image: tile = image size
        let tile = limits.optimal_tile_size(256, 256, 4);
        assert_eq!(tile, 256);

        // Large image: should be power of 2
        let tile = limits.optimal_tile_size(8192, 8192, 4);
        assert!(tile.is_power_of_two());
        assert!(tile >= 256);
    }

    #[test]
    fn test_processing_strategy_single_pass() {
        let limits = GpuLimits::with_vram(8 * 1024 * 1024 * 1024); // 8 GB

        // Small image should be single pass
        let strategy = ProcessingStrategy::recommend(512, 512, 4, &limits);
        assert_eq!(strategy, ProcessingStrategy::SinglePass);
    }

    #[test]
    fn test_processing_strategy_tiled() {
        let limits = GpuLimits::with_vram(2 * 1024 * 1024 * 1024); // 2 GB

        // Large image needs tiling
        let strategy = ProcessingStrategy::recommend(16384, 16384, 4, &limits);
        match strategy {
            ProcessingStrategy::Tiled { tile_size, num_tiles } => {
                assert!(tile_size >= 256);
                assert!(num_tiles > 1);
            }
            _ => panic!("Expected Tiled strategy"),
        }
    }

    #[test]
    fn test_processing_strategy_streaming() {
        let limits = GpuLimits::with_vram(2 * 1024 * 1024 * 1024); // 2 GB

        // Huge image needs streaming (>8GB RAM)
        let strategy = ProcessingStrategy::recommend(65536, 65536, 4, &limits);
        match strategy {
            ProcessingStrategy::Streaming { tile_size } => {
                assert!(tile_size >= 256);
            }
            _ => panic!("Expected Streaming strategy for 65k image"),
        }
    }

    #[test]
    fn test_estimate_memory() {
        let limits = GpuLimits::default();

        // 1024x1024 RGBA f32 = 16 MB, with 3x overhead = 48 MB
        let mem = limits.estimate_memory(1024, 1024, 4);
        assert_eq!(mem, 1024 * 1024 * 4 * 4 * 3); // w * h * channels * sizeof(f32) * overhead
    }
}
