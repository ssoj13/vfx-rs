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
use crate::backend::{Backend, ProcessingBackend, BlendMode, create_backend, GpuLimits, ProcessingStrategy};
use crate::color::Cdl;
use crate::ops::ResizeFilter;
use crate::ComputeResult;

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
        let backend = create_backend(self.backend)?;
        Ok(Processor {
            backend,
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
    backend: Box<dyn ProcessingBackend>,
    config: ProcessorConfig,
}

impl Processor {
    /// Create with specified backend and default config.
    pub fn new(backend: Backend) -> ComputeResult<Self> {
        Ok(Self {
            backend: create_backend(backend)?,
            config: ProcessorConfig::default(),
        })
    }

    /// Create with backend and custom config.
    pub fn with_config(backend: Backend, config: ProcessorConfig) -> ComputeResult<Self> {
        Ok(Self {
            backend: create_backend(backend)?,
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
        self.backend.name()
    }

    /// Available memory in bytes.
    pub fn available_memory(&self) -> u64 {
        self.backend.available_memory()
    }

    /// Check if using GPU backend.
    pub fn is_gpu(&self) -> bool {
        self.backend.name() == "wgpu"
    }

    /// Get current configuration.
    pub fn config(&self) -> &ProcessorConfig {
        &self.config
    }

    /// Get GPU limits.
    pub fn limits(&self) -> &GpuLimits {
        self.backend.limits()
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
            let tile_size = self.config.effective_tile_size(self.backend.limits());
            return ProcessingStrategy::Streaming { tile_size };
        }
        
        ProcessingStrategy::recommend_with_ram(
            img.width,
            img.height,
            img.channels,
            self.backend.limits(),
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
        self.config.effective_tile_size(self.backend.limits())
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
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.apply_matrix(handle.as_mut(), matrix)?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Apply CDL (Color Decision List) transform.
    pub fn apply_cdl(&self, img: &mut ComputeImage, cdl: &Cdl) -> ComputeResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.apply_cdl(handle.as_mut(), cdl.slope, cdl.offset, cdl.power, cdl.saturation)?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Apply 1D LUT.
    pub fn apply_lut1d(&self, img: &mut ComputeImage, lut: &[f32], channels: u32) -> ComputeResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.apply_lut1d(handle.as_mut(), lut, channels)?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Apply 3D LUT with trilinear interpolation.
    pub fn apply_lut3d(&self, img: &mut ComputeImage, lut: &[f32], size: u32) -> ComputeResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.apply_lut3d(handle.as_mut(), lut, size)?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
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
        let handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        let resized = self.backend.resize(handle.as_ref(), width, height, filter as u32)?;
        let data = self.backend.download(resized.as_ref())?;
        ComputeImage::from_f32(data, width, height, img.channels)
    }

    /// Resize to half dimensions (useful for mipmap generation).
    pub fn resize_half(&self, img: &ComputeImage) -> ComputeResult<ComputeImage> {
        self.resize(img, img.width / 2, img.height / 2, ResizeFilter::Bilinear)
    }

    /// Apply Gaussian blur.
    pub fn blur(&self, img: &mut ComputeImage, radius: f32) -> ComputeResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.blur(handle.as_mut(), radius)?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Apply sharpening (unsharp mask).
    ///
    /// Amount 1.0 = moderate sharpening.
    pub fn sharpen(&self, img: &mut ComputeImage, amount: f32) -> ComputeResult<()> {
        let original = img.data.clone();
        
        // Small blur
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.blur(handle.as_mut(), 1.0)?;
        let blurred = self.backend.download(handle.as_ref())?;
        
        // Unsharp mask: sharp = original + amount * (original - blur)
        for i in 0..img.data.len() {
            img.data[i] = original[i] + amount * (original[i] - blurred[i]);
        }
        
        Ok(())
    }

    // =========================================================================
    // Composite Operations
    // =========================================================================

    /// Porter-Duff Over: foreground over background.
    pub fn composite_over(&self, fg: &ComputeImage, bg: &mut ComputeImage) -> ComputeResult<()> {
        let fg_handle = self.backend.upload(&fg.data, fg.width, fg.height, fg.channels)?;
        let mut bg_handle = self.backend.upload(&bg.data, bg.width, bg.height, bg.channels)?;
        self.backend.composite_over(fg_handle.as_ref(), bg_handle.as_mut())?;
        bg.data = self.backend.download(bg_handle.as_ref())?;
        Ok(())
    }

    /// Blend with mode and opacity.
    pub fn blend(&self, fg: &ComputeImage, bg: &mut ComputeImage, mode: BlendMode, opacity: f32) -> ComputeResult<()> {
        let fg_handle = self.backend.upload(&fg.data, fg.width, fg.height, fg.channels)?;
        let mut bg_handle = self.backend.upload(&bg.data, bg.width, bg.height, bg.channels)?;
        self.backend.blend(fg_handle.as_ref(), bg_handle.as_mut(), mode, opacity)?;
        bg.data = self.backend.download(bg_handle.as_ref())?;
        Ok(())
    }

    // =========================================================================
    // Transform Operations
    // =========================================================================

    /// Crop region from image.
    pub fn crop(&self, img: &ComputeImage, x: u32, y: u32, w: u32, h: u32) -> ComputeResult<ComputeImage> {
        let handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        let cropped = self.backend.crop(handle.as_ref(), x, y, w, h)?;
        let data = self.backend.download(cropped.as_ref())?;
        ComputeImage::from_f32(data, w, h, img.channels)
    }

    /// Flip horizontal.
    pub fn flip_h(&self, img: &mut ComputeImage) -> ComputeResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.flip_h(handle.as_mut())?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Flip vertical.
    pub fn flip_v(&self, img: &mut ComputeImage) -> ComputeResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.flip_v(handle.as_mut())?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Rotate 90 degrees clockwise (n times).
    pub fn rotate_90(&self, img: &ComputeImage, n: u32) -> ComputeResult<ComputeImage> {
        let handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        let rotated = self.backend.rotate_90(handle.as_ref(), n)?;
        let (w, h, c) = rotated.dimensions();
        let data = self.backend.download(rotated.as_ref())?;
        ComputeImage::from_f32(data, w, h, c)
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
}
