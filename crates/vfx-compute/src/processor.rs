//! Unified compute processor combining color and image operations.
//!
//! # Configuration
//!
//! Use [`ProcessorBuilder`] for fine-grained control over processing:
//!
//! ```ignore
//! use vfx_compute::{ProcessorBuilder, Backend};
//!
//! let proc = ProcessorBuilder::new()
//!     .backend(Backend::Wgpu)
//!     .tile_size(2048)
//!     .ram_limit_mb(8192)
//!     .build()?;
//! ```
//!
//! # Automatic Strategy Selection
//!
//! The processor automatically chooses optimal processing strategy:
//! - **SinglePass**: Image fits in GPU memory
//! - **Tiled**: Image needs tiling but fits in RAM  
//! - **Streaming**: Image too large for RAM, use disk streaming

use crate::image::ComputeImage;
use crate::backend::{Backend, AnyExecutor, create_executor, ColorOp, GpuLimits, ProcessingStrategy};
use crate::color::Cdl;
use crate::ops::{ResizeFilter, BlendMode};
use crate::ComputeResult;

// ============================================================================
// Batch Operations
// ============================================================================

/// Individual operation for batching.
#[derive(Debug, Clone)]
pub enum BatchOp {
    /// 4x4 color matrix.
    Matrix([f32; 16]),
    /// CDL transform.
    Cdl(Cdl),
    /// 1D LUT.
    Lut1d { lut: Vec<f32>, channels: u32 },
    /// 3D LUT.
    Lut3d { lut: Vec<f32>, size: u32 },
}

/// Batch of color operations to apply without GPU round-trips.
///
/// # Example
/// ```ignore
/// let batch = ColorOpBatch::new()
///     .exposure(1.5)
///     .saturation(1.2)
///     .contrast(1.1);
/// processor.apply_color_ops(&mut img, &batch)?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct ColorOpBatch {
    pub(crate) ops: Vec<BatchOp>,
}

impl ColorOpBatch {
    /// Create empty batch.
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    /// Add matrix transform.
    pub fn matrix(mut self, m: &[f32; 16]) -> Self {
        self.ops.push(BatchOp::Matrix(*m));
        self
    }

    /// Add CDL transform.
    pub fn cdl(mut self, cdl: Cdl) -> Self {
        self.ops.push(BatchOp::Cdl(cdl));
        self
    }

    /// Add exposure adjustment (stops).
    pub fn exposure(mut self, stops: f32) -> Self {
        let mult = 2.0f32.powf(stops);
        let m = [
            mult, 0.0, 0.0, 0.0,
            0.0, mult, 0.0, 0.0,
            0.0, 0.0, mult, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        self.ops.push(BatchOp::Matrix(m));
        self
    }

    /// Add saturation adjustment.
    pub fn saturation(mut self, sat: f32) -> Self {
        self.ops.push(BatchOp::Cdl(Cdl {
            saturation: sat,
            ..Default::default()
        }));
        self
    }

    /// Add contrast adjustment.
    pub fn contrast(mut self, c: f32) -> Self {
        let offset = 0.5 * (1.0 - c);
        let m = [
            c, 0.0, 0.0, offset,
            0.0, c, 0.0, offset,
            0.0, 0.0, c, offset,
            0.0, 0.0, 0.0, 1.0,
        ];
        self.ops.push(BatchOp::Matrix(m));
        self
    }

    /// Add 1D LUT.
    pub fn lut1d(mut self, lut: Vec<f32>, channels: u32) -> Self {
        self.ops.push(BatchOp::Lut1d { lut, channels });
        self
    }

    /// Add 3D LUT.
    pub fn lut3d(mut self, lut: Vec<f32>, size: u32) -> Self {
        self.ops.push(BatchOp::Lut3d { lut, size });
        self
    }

    /// Check if batch is empty.
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Number of operations.
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// Convert to ColorOp vector for executor.
    fn to_color_ops(&self) -> Vec<ColorOp> {
        self.ops.iter().map(|op| match op {
            BatchOp::Matrix(m) => ColorOp::Matrix(*m),
            BatchOp::Cdl(cdl) => ColorOp::Cdl {
                slope: cdl.slope,
                offset: cdl.offset,
                power: cdl.power,
                saturation: cdl.saturation,
            },
            BatchOp::Lut1d { lut, channels } => ColorOp::Lut1d {
                lut: lut.clone(),
                channels: *channels,
            },
            BatchOp::Lut3d { lut, size } => ColorOp::Lut3d {
                lut: lut.clone(),
                size: *size,
            },
        }).collect()
    }
}

// ============================================================================
// Configuration
// ============================================================================

/// Default tile size for GPU processing.
pub const DEFAULT_TILE_SIZE: u32 = 4096;
/// Minimum tile size.
pub const MIN_TILE_SIZE: u32 = 256;
/// Maximum tile size.
pub const MAX_TILE_SIZE: u32 = 16384;
/// Default RAM percentage to use (80%).
pub const DEFAULT_RAM_PERCENT: u8 = 80;

/// Processing configuration.
///
/// Controls how the processor handles large images and memory management.
#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    /// Override tile size (None = auto-detect based on GPU).
    pub tile_size: Option<u32>,
    /// Force streaming mode even for small images.
    pub force_streaming: bool,
    /// Maximum RAM to use in bytes (None = use ram_percent).
    pub ram_limit: Option<u64>,
    /// Percentage of system RAM to use (default: 80).
    pub ram_percent: u8,
    /// Enable verbose output for debugging.
    pub verbose: bool,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            tile_size: None,
            force_streaming: false,
            ram_limit: None,
            ram_percent: DEFAULT_RAM_PERCENT,
            verbose: false,
        }
    }
}

impl ProcessorConfig {
    /// Get effective RAM limit based on config.
    pub fn effective_ram_limit(&self) -> u64 {
        if let Some(limit) = self.ram_limit {
            return limit;
        }
        // Estimate system RAM (default 16GB if can't detect)
        let system_ram = detect_system_ram().unwrap_or(16 * 1024 * 1024 * 1024);
        (system_ram as f64 * (self.ram_percent as f64 / 100.0)) as u64
    }

    /// Get effective tile size based on config and GPU limits.
    pub fn effective_tile_size(&self, limits: &GpuLimits) -> u32 {
        self.tile_size.unwrap_or_else(|| {
            // Use GPU optimal tile size, clamped to our limits
            limits.optimal_tile_size(16384, 16384, 4)
                .clamp(MIN_TILE_SIZE, MAX_TILE_SIZE)
        })
    }
}

/// Detect available system RAM in bytes.
fn detect_system_ram() -> Option<u64> {
    #[cfg(target_os = "windows")]
    {
        use std::mem::MaybeUninit;
        #[repr(C)]
        struct MEMORYSTATUSEX {
            dw_length: u32,
            dw_memory_load: u32,
            ull_total_phys: u64,
            ull_avail_phys: u64,
            ull_total_page_file: u64,
            ull_avail_page_file: u64,
            ull_total_virtual: u64,
            ull_avail_virtual: u64,
            ull_avail_extended_virtual: u64,
        }
        unsafe extern "system" {
            fn GlobalMemoryStatusEx(buffer: *mut MEMORYSTATUSEX) -> i32;
        }
        unsafe {
            let mut mem = MaybeUninit::<MEMORYSTATUSEX>::uninit();
            (*mem.as_mut_ptr()).dw_length = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            if GlobalMemoryStatusEx(mem.as_mut_ptr()) != 0 {
                return Some((*mem.as_ptr()).ull_total_phys);
            }
        }
        None
    }
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<u64>() {
                            return Some(kb * 1024);
                        }
                    }
                }
            }
        }
        None
    }
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("sysctl").args(["-n", "hw.memsize"]).output() {
            if let Ok(s) = String::from_utf8(output.stdout) {
                if let Ok(bytes) = s.trim().parse::<u64>() {
                    return Some(bytes);
                }
            }
        }
        None
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

// ============================================================================
// Builder
// ============================================================================

/// Builder for [`Processor`] with configuration options.
///
/// # Example
///
/// ```ignore
/// use vfx_compute::{ProcessorBuilder, Backend};
///
/// let proc = ProcessorBuilder::new()
///     .backend(Backend::Wgpu)
///     .tile_size(2048)
///     .ram_limit_mb(16384)
///     .verbose(true)
///     .build()?;
/// ```
#[derive(Debug, Clone)]
pub struct ProcessorBuilder {
    backend: Backend,
    config: ProcessorConfig,
}

impl Default for ProcessorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessorBuilder {
    /// Create new builder with default settings.
    pub fn new() -> Self {
        Self {
            backend: Backend::Auto,
            config: ProcessorConfig::default(),
        }
    }

    /// Set compute backend.
    pub fn backend(mut self, backend: Backend) -> Self {
        self.backend = backend;
        self
    }

    /// Set tile size for GPU processing.
    ///
    /// Must be between 256 and 16384. Default: auto-detect.
    pub fn tile_size(mut self, size: u32) -> Self {
        self.config.tile_size = Some(size.clamp(MIN_TILE_SIZE, MAX_TILE_SIZE));
        self
    }

    /// Force streaming mode regardless of image size.
    pub fn force_streaming(mut self, enabled: bool) -> Self {
        self.config.force_streaming = enabled;
        self
    }

    /// Set maximum RAM in bytes.
    pub fn ram_limit(mut self, bytes: u64) -> Self {
        self.config.ram_limit = Some(bytes);
        self
    }

    /// Set maximum RAM in megabytes.
    pub fn ram_limit_mb(mut self, mb: u64) -> Self {
        self.config.ram_limit = Some(mb * 1024 * 1024);
        self
    }

    /// Set RAM usage as percentage of system RAM (1-100).
    pub fn ram_percent(mut self, percent: u8) -> Self {
        self.config.ram_percent = percent.clamp(1, 100);
        self
    }

    /// Enable verbose output.
    pub fn verbose(mut self, enabled: bool) -> Self {
        self.config.verbose = enabled;
        self
    }

    /// Build the Processor.
    pub fn build(self) -> ComputeResult<Processor> {
        let executor = create_executor(self.backend)?;
        Ok(Processor {
            executor,
            config: self.config,
        })
    }
}

// ============================================================================
// Processor
// ============================================================================

/// Unified compute processor.
///
/// Combines color grading and image processing operations with automatic
/// backend selection (GPU when available, CPU fallback).
///
/// # Example
/// ```ignore
/// use vfx_compute::{Processor, Backend, ComputeImage};
///
/// let proc = Processor::auto()?;
/// let mut img = ComputeImage::from_f32(data, 1920, 1080, 3)?;
///
/// // Color operations
/// proc.apply_exposure(&mut img, 1.5)?;
/// proc.apply_saturation(&mut img, 1.2)?;
///
/// // Image operations  
/// let resized = proc.resize(&img, 960, 540, ResizeFilter::Bilinear)?;
/// ```
///
/// # Configuration
///
/// Use [`ProcessorBuilder`] for fine-grained control:
///
/// ```ignore
/// let proc = ProcessorBuilder::new()
///     .backend(Backend::Wgpu)
///     .tile_size(2048)
///     .ram_limit_mb(8192)
///     .build()?;
/// ```
pub struct Processor {
    executor: AnyExecutor,
    config: ProcessorConfig,
}

impl Processor {
    /// Create with specified backend and default config.
    pub fn new(backend: Backend) -> ComputeResult<Self> {
        Ok(Self {
            executor: create_executor(backend)?,
            config: ProcessorConfig::default(),
        })
    }

    /// Create with backend and custom config.
    pub fn with_config(backend: Backend, config: ProcessorConfig) -> ComputeResult<Self> {
        Ok(Self {
            executor: create_executor(backend)?,
            config,
        })
    }

    /// Create builder for fine-grained configuration.
    pub fn builder() -> ProcessorBuilder {
        ProcessorBuilder::new()
    }

    /// Create with auto-selected backend (GPU if available, else CPU).
    pub fn auto() -> ComputeResult<Self> {
        Self::new(Backend::Auto)
    }

    /// Create with CPU backend.
    pub fn cpu() -> ComputeResult<Self> {
        Self::new(Backend::Cpu)
    }

    /// Create with GPU backend (requires wgpu feature).
    #[cfg(feature = "wgpu")]
    pub fn gpu() -> ComputeResult<Self> {
        Self::new(Backend::Wgpu)
    }

    // =========================================================================
    // Info
    // =========================================================================

    /// Backend name ("cpu" or "wgpu").
    pub fn backend_name(&self) -> &'static str {
        self.executor.name()
    }

    /// Available memory in bytes.
    pub fn available_memory(&self) -> u64 {
        self.executor.limits().available_memory
    }

    /// Check if using GPU backend.
    pub fn is_gpu(&self) -> bool {
        self.executor.name() == "wgpu"
    }

    /// Get current configuration.
    pub fn config(&self) -> &ProcessorConfig {
        &self.config
    }

    /// Get GPU limits.
    pub fn limits(&self) -> &GpuLimits {
        self.executor.limits()
    }

    // =========================================================================
    // Strategy Selection
    // =========================================================================

    /// Recommend processing strategy for an image.
    ///
    /// Returns the optimal strategy based on:
    /// - Image dimensions and channels
    /// - GPU memory limits
    /// - Configured RAM limits
    /// - Force streaming flag
    ///
    /// # Example
    /// ```ignore
    /// let strategy = proc.recommend_strategy(&img);
    /// match strategy {
    ///     ProcessingStrategy::SinglePass => println!("Fast: fits GPU"),
    ///     ProcessingStrategy::Tiled { tile_size, .. } => println!("Tiled: {}px", tile_size),
    ///     ProcessingStrategy::Streaming { .. } => println!("Streaming: too large for RAM"),
    /// }
    /// ```
    pub fn recommend_strategy(&self, img: &ComputeImage) -> ProcessingStrategy {
        if self.config.force_streaming {
            let tile_size = self.config.effective_tile_size(self.executor.limits());
            return ProcessingStrategy::Streaming { tile_size };
        }
        
        ProcessingStrategy::recommend_with_ram(
            img.width,
            img.height,
            img.channels,
            self.executor.limits(),
            self.config.effective_ram_limit(),
        )
    }

    /// Check if image should use streaming (too large for RAM).
    pub fn should_stream(&self, img: &ComputeImage) -> bool {
        if self.config.force_streaming {
            return true;
        }
        matches!(
            self.recommend_strategy(img),
            ProcessingStrategy::Streaming { .. }
        )
    }

    /// Check if image needs tiling (too large for single GPU pass).
    pub fn needs_tiling(&self, img: &ComputeImage) -> bool {
        !matches!(
            self.recommend_strategy(img),
            ProcessingStrategy::SinglePass
        )
    }

    /// Get effective tile size for processing.
    pub fn effective_tile_size(&self) -> u32 {
        self.config.effective_tile_size(self.executor.limits())
    }

    /// Log strategy info if verbose mode enabled.
    #[allow(dead_code)]
    fn log_strategy(&self, img: &ComputeImage, op_name: &str) {
        if self.config.verbose {
            let strategy = self.recommend_strategy(img);
            eprintln!("[{}] {}x{} -> {:?}", op_name, img.width, img.height, strategy);
        }
    }

    // =========================================================================
    // Color Operations
    // =========================================================================

    /// Apply 4x4 color matrix transform.
    pub fn apply_matrix(&self, img: &mut ComputeImage, matrix: &[f32; 16]) -> ComputeResult<()> {
        let op = ColorOp::Matrix(*matrix);
        self.executor.execute_color(img, &op)
    }

    /// Apply CDL (Color Decision List) transform.
    pub fn apply_cdl(&self, img: &mut ComputeImage, cdl: &Cdl) -> ComputeResult<()> {
        let op = ColorOp::Cdl {
            slope: cdl.slope,
            offset: cdl.offset,
            power: cdl.power,
            saturation: cdl.saturation,
        };
        self.executor.execute_color(img, &op)
    }

    /// Apply 1D LUT.
    pub fn apply_lut1d(&self, img: &mut ComputeImage, lut: &[f32], channels: u32) -> ComputeResult<()> {
        let op = ColorOp::Lut1d {
            lut: lut.to_vec(),
            channels,
        };
        self.executor.execute_color(img, &op)
    }

    /// Apply 3D LUT with trilinear interpolation.
    pub fn apply_lut3d(&self, img: &mut ComputeImage, lut: &[f32], size: u32) -> ComputeResult<()> {
        let op = ColorOp::Lut3d {
            lut: lut.to_vec(),
            size,
        };
        self.executor.execute_color(img, &op)
    }

    /// Apply multiple color operations without GPU round-trips.
    ///
    /// More efficient than calling individual methods when applying
    /// multiple transforms (e.g., exposure + saturation + matrix).
    ///
    /// # Example
    /// ```ignore
    /// use vfx_compute::{Processor, ColorOpBatch};
    ///
    /// let batch = ColorOpBatch::new()
    ///     .exposure(1.5)
    ///     .saturation(1.2)
    ///     .matrix(&my_matrix);
    /// proc.apply_color_ops(&mut img, &batch)?;
    /// ```
    pub fn apply_color_ops(&self, img: &mut ComputeImage, batch: &ColorOpBatch) -> ComputeResult<()> {
        if batch.ops.is_empty() {
            return Ok(());
        }

        let ops = batch.to_color_ops();
        self.executor.execute_color_chain(img, &ops)
    }

    /// Apply exposure adjustment (in stops).
    /// 
    /// +1.0 = 2x brighter, -1.0 = 2x darker.
    pub fn apply_exposure(&self, img: &mut ComputeImage, stops: f32) -> ComputeResult<()> {
        let mult = 2.0f32.powf(stops);
        let matrix = [
            mult, 0.0, 0.0, 0.0,
            0.0, mult, 0.0, 0.0,
            0.0, 0.0, mult, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        self.apply_matrix(img, &matrix)
    }

    /// Apply saturation adjustment.
    ///
    /// 1.0 = no change, 0.0 = grayscale, 2.0 = double saturation.
    pub fn apply_saturation(&self, img: &mut ComputeImage, sat: f32) -> ComputeResult<()> {
        let cdl = Cdl {
            saturation: sat,
            ..Default::default()
        };
        self.apply_cdl(img, &cdl)
    }

    /// Apply contrast adjustment.
    ///
    /// 1.0 = no change, 0.5 = less contrast, 2.0 = more contrast.
    pub fn apply_contrast(&self, img: &mut ComputeImage, contrast: f32) -> ComputeResult<()> {
        let offset = 0.5 * (1.0 - contrast);
        let matrix = [
            contrast, 0.0, 0.0, offset,
            0.0, contrast, 0.0, offset,
            0.0, 0.0, contrast, offset,
            0.0, 0.0, 0.0, 1.0,
        ];
        self.apply_matrix(img, &matrix)
    }

    // =========================================================================
    // Image Operations
    // =========================================================================

    /// Resize image with specified filter.
    pub fn resize(&self, img: &ComputeImage, width: u32, height: u32, filter: ResizeFilter) -> ComputeResult<ComputeImage> {
        self.executor.execute_resize(img, width, height, filter as u32)
    }

    /// Resize to half dimensions (useful for mipmap generation).
    pub fn resize_half(&self, img: &ComputeImage) -> ComputeResult<ComputeImage> {
        self.resize(img, img.width / 2, img.height / 2, ResizeFilter::Bilinear)
    }

    /// Apply Gaussian blur.
    pub fn blur(&self, img: &mut ComputeImage, radius: f32) -> ComputeResult<()> {
        self.executor.execute_blur(img, radius)
    }

    /// Apply sharpening (unsharp mask).
    ///
    /// Amount 1.0 = moderate sharpening.
    pub fn sharpen(&self, img: &mut ComputeImage, amount: f32) -> ComputeResult<()> {
        let original = img.data.clone();
        
        // Small blur
        self.executor.execute_blur(img, 1.0)?;
        let blurred = std::mem::take(&mut img.data);
        
        // Unsharp mask: sharp = original + amount * (original - blur)
        img.data = original.iter()
            .zip(blurred.iter())
            .map(|(o, b)| o + amount * (o - b))
            .collect();
        
        Ok(())
    }

    // =========================================================================
    // Composite Operations
    // =========================================================================

    /// Porter-Duff Over: foreground over background.
    pub fn composite_over(&self, fg: &ComputeImage, bg: &mut ComputeImage) -> ComputeResult<()> {
        self.executor.execute_composite_over(fg, bg)
    }

    /// Blend with mode and opacity.
    pub fn blend(&self, fg: &ComputeImage, bg: &mut ComputeImage, mode: BlendMode, opacity: f32) -> ComputeResult<()> {
        self.executor.execute_blend(fg, bg, mode as u32, opacity)
    }

    // =========================================================================
    // Transform Operations
    // =========================================================================

    /// Crop region from image.
    pub fn crop(&self, img: &ComputeImage, x: u32, y: u32, w: u32, h: u32) -> ComputeResult<ComputeImage> {
        self.executor.execute_crop(img, x, y, w, h)
    }

    /// Flip horizontal.
    pub fn flip_h(&self, img: &mut ComputeImage) -> ComputeResult<()> {
        self.executor.execute_flip_h(img)
    }

    /// Flip vertical.
    pub fn flip_v(&self, img: &mut ComputeImage) -> ComputeResult<()> {
        self.executor.execute_flip_v(img)
    }

    /// Rotate 90 degrees clockwise (n times).
    pub fn rotate_90(&self, img: &ComputeImage, n: u32) -> ComputeResult<ComputeImage> {
        self.executor.execute_rotate_90(img, n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_auto() {
        let proc = Processor::auto().unwrap();
        println!("Backend: {}", proc.backend_name());
        // CPU backend may report 0 on some systems
        let _mem = proc.available_memory();
    }

    #[test]
    fn test_processor_exposure() {
        let proc = Processor::cpu().unwrap();
        let mut img = ComputeImage::from_f32(vec![0.5, 0.5, 0.5], 1, 1, 3).unwrap();
        
        proc.apply_exposure(&mut img, 1.0).unwrap();
        
        // +1 stop = 2x brightness
        assert!((img.data()[0] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_processor_contrast() {
        let proc = Processor::cpu().unwrap();
        let mut img = ComputeImage::from_f32(vec![0.5, 0.5, 0.5], 1, 1, 3).unwrap();
        
        // No change at contrast=1.0
        proc.apply_contrast(&mut img, 1.0).unwrap();
        assert!((img.data()[0] - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_color_op_batch() {
        let proc = Processor::cpu().unwrap();
        let mut img = ComputeImage::from_f32(vec![0.25, 0.25, 0.25], 1, 1, 3).unwrap();
        
        // Batch: +1 stop exposure (2x) + saturation (no change for gray)
        let batch = ColorOpBatch::new()
            .exposure(1.0)
            .saturation(1.0);
        
        proc.apply_color_ops(&mut img, &batch).unwrap();
        
        // Should be 0.5 (doubled)
        assert!((img.data()[0] - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_color_op_batch_chained() {
        let proc = Processor::cpu().unwrap();
        let mut img = ComputeImage::from_f32(vec![0.125, 0.125, 0.125], 1, 1, 3).unwrap();
        
        // Chain: +1 stop (2x) + +1 stop (2x) = 4x total
        let batch = ColorOpBatch::new()
            .exposure(1.0)
            .exposure(1.0);
        
        proc.apply_color_ops(&mut img, &batch).unwrap();
        
        // 0.125 * 4 = 0.5
        assert!((img.data()[0] - 0.5).abs() < 1e-5);
    }
}
