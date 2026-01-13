//! Compute backends for GPU/CPU image processing.
//!
//! This module provides the core backend infrastructure:
//!
//! - [`GpuPrimitives`] - Core trait for backend implementations
//! - [`TiledExecutor`] - Automatic tiling for large images
//! - [`Backend`] - Backend selection enum
//! - Detection utilities for VRAM and backend availability
//!
//! # Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────┐
//! │        TiledExecutor<G: GpuPrimitives>           │
//! │  (automatic tiling, caching, cluster opt)       │
//! ├────────────────┬────────────────┬────────────────┤
//! │  CpuPrimitives  │  WgpuPrimitives │ CudaPrimitives │
//! │    (rayon)      │  (compute shdr) │   (cudarc)     │
//! └────────────────┴────────────────┴────────────────┘
//! ```
//!
//! # Backend Selection
//!
//! Backends are selected by priority:
//! 1. **CUDA** (priority 150) - If NVIDIA GPU with CUDA toolkit
//! 2. **wgpu discrete** (priority 100) - Discrete GPU via Vulkan/Metal/DX12
//! 3. **wgpu integrated** (priority 50) - Integrated GPU
//! 4. **CPU** (priority 10) - Always available fallback
//!
//! Override with `VFX_BACKEND=cpu|wgpu|cuda` environment variable.

mod gpu_primitives;
mod tiling;
mod detect;
mod cpu_backend;
mod executor;
pub mod streaming;
mod memory;
mod vram;
mod cache;
mod cluster;
mod planner;

#[cfg(feature = "wgpu")]
mod wgpu_backend;

#[cfg(feature = "cuda")]
mod cuda_backend;

// Core types
pub use gpu_primitives::{GpuPrimitives, ImageHandle, KernelParams, AsAny};
pub use tiling::{GpuLimits, Tile, generate_tiles, ProcessingStrategy, TileWorkflow};
pub use detect::{detect_backends, select_best_backend, describe_backends, BackendInfo};

// Memory management
pub use memory::{
    available_memory, system_memory, processing_budget, cache_budget,
    cache_disabled, tile_size_override, backend_override,
    image_memory, processing_memory, format_bytes,
    BYTES_PER_PIXEL, SAFE_MEMORY_FRACTION,
};

// VRAM detection
pub use vram::{detect_vram, total_vram, free_vram, available_vram, VramInfo};
#[cfg(feature = "wgpu")]
pub use vram::is_software_renderer;

// Region cache
pub use cache::{RegionCache, RegionKey, CachedRegion};

// Tile clustering
pub use cluster::{
    SourceRegion, TileTriple, TileCluster, ClusterConfig,
    cluster_tiles, analyze_source_region, compute_savings,
};

// Execution planner
pub use planner::{Planner, ExecutionPlan, Constraints};

// Executor
pub use executor::{TiledExecutor, ExecutorConfig, ColorOp, ImageOp, set_verbose};

// Streaming
pub use streaming::{
    StreamingSource, StreamingOutput, StreamingFormat,
    MemorySource, MemoryOutput,
    should_stream, estimate_memory,
};
#[cfg(feature = "io")]
pub use streaming::{ExrStreamingSource, ExrStreamingOutput};

// Backends
pub use cpu_backend::{CpuBackend, CpuPrimitives, CpuImage};

#[cfg(feature = "wgpu")]
pub use wgpu_backend::{WgpuPrimitives, WgpuImage};

#[cfg(feature = "cuda")]
pub use cuda_backend::{CudaPrimitives, CudaImage};

use crate::{ComputeResult, ComputeError};

/// Available compute backends for image processing.
///
/// Use [`Backend::Auto`] for automatic best-available selection,
/// or specify a particular backend explicitly.
///
/// # Environment Override
///
/// Set `VFX_BACKEND` environment variable to override:
/// - `VFX_BACKEND=cpu` - Force CPU
/// - `VFX_BACKEND=wgpu` - Force wgpu
/// - `VFX_BACKEND=cuda` - Force CUDA
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backend {
    /// Auto-select best available backend by priority.
    ///
    /// Selection order: CUDA > wgpu (discrete) > wgpu (integrated) > CPU
    #[default]
    Auto,
    
    /// CPU backend using rayon for parallel processing.
    ///
    /// Always available. Uses all CPU cores via work-stealing.
    Cpu,
    
    /// GPU via wgpu (Vulkan, Metal, or DX12).
    ///
    /// Requires compatible GPU. Falls back to CPU if unavailable.
    Wgpu,
    
    /// NVIDIA CUDA backend.
    ///
    /// Requires NVIDIA GPU and CUDA toolkit. Highest performance for NVIDIA hardware.
    Cuda,
}

impl Backend {
    /// Check if this backend is available on current system.
    pub fn is_available(&self) -> bool {
        match self {
            Self::Auto => true,
            Self::Cpu => true,
            #[cfg(feature = "wgpu")]
            Self::Wgpu => WgpuPrimitives::is_available(),
            #[cfg(not(feature = "wgpu"))]
            Self::Wgpu => false,
            #[cfg(feature = "cuda")]
            Self::Cuda => CudaPrimitives::is_available(),
            #[cfg(not(feature = "cuda"))]
            Self::Cuda => false,
        }
    }

    /// Get human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Cpu => "cpu",
            Self::Wgpu => "wgpu",
            Self::Cuda => "cuda",
        }
    }
}

/// Blend mode for compositing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum BlendMode {
    #[default]
    Normal = 0,
    Multiply = 1,
    Screen = 2,
    Add = 3,
    Subtract = 4,
    Overlay = 5,
    SoftLight = 6,
    HardLight = 7,
    Difference = 8,
}

// =============================================================================
// Macro for AnyExecutor dispatch (reduces ~200 lines of duplication)
// =============================================================================

/// Dispatch method to all executor variants.
macro_rules! dispatch_executor {
    // Simple method with no arguments (returns ref)
    ($self:ident, $method:ident) => {
        match $self {
            AnyExecutor::Cpu(e) => e.$method(),
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => e.$method(),
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => e.$method(),
        }
    };
    // Method with arguments
    ($self:ident, $method:ident, $($arg:expr),+) => {
        match $self {
            AnyExecutor::Cpu(e) => e.$method($($arg),+),
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => e.$method($($arg),+),
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => e.$method($($arg),+),
        }
    };
}

/// Dispatch GPU primitive operation with upload/exec/download pattern.
macro_rules! dispatch_gpu_op {
    // In-place operation (upload -> exec -> download back to same image)
    ($self:ident, $img:ident, $op:ident) => {
        match $self {
            AnyExecutor::Cpu(e) => {
                let mut handle = e.gpu().upload($img.data(), $img.width, $img.height, $img.channels)?;
                e.gpu().$op(&mut handle)?;
                $img.set_data(e.gpu().download(&handle)?);
                Ok(())
            }
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => {
                let mut handle = e.gpu().upload($img.data(), $img.width, $img.height, $img.channels)?;
                e.gpu().$op(&mut handle)?;
                $img.set_data(e.gpu().download(&handle)?);
                Ok(())
            }
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => {
                let mut handle = e.gpu().upload($img.data(), $img.width, $img.height, $img.channels)?;
                e.gpu().$op(&mut handle)?;
                $img.set_data(e.gpu().download(&handle)?);
                Ok(())
            }
        }
    };
    // Two-image operation (fg + bg -> bg modified)
    ($self:ident, fg=$fg:ident, bg=$bg:ident, $op:ident $(, $arg:expr)*) => {
        match $self {
            AnyExecutor::Cpu(e) => {
                let fg_h = e.gpu().upload($fg.data(), $fg.width, $fg.height, $fg.channels)?;
                let mut bg_h = e.gpu().upload($bg.data(), $bg.width, $bg.height, $bg.channels)?;
                e.gpu().$op(&fg_h, &mut bg_h $(, $arg)* )?;
                $bg.set_data(e.gpu().download(&bg_h)?);
                Ok(())
            }
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => {
                let fg_h = e.gpu().upload($fg.data(), $fg.width, $fg.height, $fg.channels)?;
                let mut bg_h = e.gpu().upload($bg.data(), $bg.width, $bg.height, $bg.channels)?;
                e.gpu().$op(&fg_h, &mut bg_h $(, $arg)* )?;
                $bg.set_data(e.gpu().download(&bg_h)?);
                Ok(())
            }
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => {
                let fg_h = e.gpu().upload($fg.data(), $fg.width, $fg.height, $fg.channels)?;
                let mut bg_h = e.gpu().upload($bg.data(), $bg.width, $bg.height, $bg.channels)?;
                e.gpu().$op(&fg_h, &mut bg_h $(, $arg)* )?;
                $bg.set_data(e.gpu().download(&bg_h)?);
                Ok(())
            }
        }
    };
}

// =============================================================================
// AnyExecutor - Unified executor with dynamic dispatch
// =============================================================================

/// Executor type enum for dynamic dispatch across all backends.
///
/// Provides a unified API for image processing regardless of backend.
/// Use [`create_executor`] to create the best available backend.
pub enum AnyExecutor {
    /// CPU backend (always available).
    Cpu(TiledExecutor<CpuPrimitives>),
    /// wgpu backend (Vulkan/Metal/DX12).
    #[cfg(feature = "wgpu")]
    Wgpu(TiledExecutor<WgpuPrimitives>),
    /// CUDA backend (NVIDIA GPUs).
    #[cfg(feature = "cuda")]
    Cuda(TiledExecutor<CudaPrimitives>),
}

impl AnyExecutor {
    // =========================================================================
    // Basic Accessors
    // =========================================================================

    /// Get backend name.
    pub fn name(&self) -> &'static str {
        dispatch_executor!(self, name)
    }

    /// Get GPU limits.
    pub fn limits(&self) -> &GpuLimits {
        dispatch_executor!(self, limits)
    }

    // =========================================================================
    // Color Operations (use TiledExecutor methods directly)
    // =========================================================================

    /// Execute color operation with automatic tiling.
    pub fn execute_color(&self, img: &mut crate::ComputeImage, op: &ColorOp) -> ComputeResult<()> {
        dispatch_executor!(self, execute_color, img, op)
    }

    /// Execute multiple color operations without GPU round-trips.
    pub fn execute_color_chain(&self, img: &mut crate::ComputeImage, ops: &[ColorOp]) -> ComputeResult<()> {
        dispatch_executor!(self, execute_color_chain, img, ops)
    }

    // =========================================================================
    // Image Operations
    // =========================================================================

    /// Execute blur with automatic tiling.
    pub fn execute_blur(&self, img: &mut crate::ComputeImage, radius: f32) -> ComputeResult<()> {
        dispatch_executor!(self, execute_blur, img, radius)
    }

    /// Execute resize.
    pub fn execute_resize(&self, img: &crate::ComputeImage, width: u32, height: u32, filter: u32) -> ComputeResult<crate::ComputeImage> {
        dispatch_executor!(self, execute_resize, img, width, height, filter)
    }

    /// Execute composite over.
    pub fn execute_composite_over(&self, fg: &crate::ComputeImage, bg: &mut crate::ComputeImage) -> ComputeResult<()> {
        dispatch_gpu_op!(self, fg=fg, bg=bg, exec_composite_over)
    }

    /// Execute blend.
    pub fn execute_blend(&self, fg: &crate::ComputeImage, bg: &mut crate::ComputeImage, mode: u32, opacity: f32) -> ComputeResult<()> {
        dispatch_gpu_op!(self, fg=fg, bg=bg, exec_blend, mode, opacity)
    }

    /// Execute flip horizontal.
    pub fn execute_flip_h(&self, img: &mut crate::ComputeImage) -> ComputeResult<()> {
        dispatch_gpu_op!(self, img, exec_flip_h)
    }

    /// Execute flip vertical.
    pub fn execute_flip_v(&self, img: &mut crate::ComputeImage) -> ComputeResult<()> {
        dispatch_gpu_op!(self, img, exec_flip_v)
    }

    /// Execute rotate 90 degrees clockwise.
    pub fn execute_rotate_90(&self, img: &crate::ComputeImage, n: u32) -> ComputeResult<crate::ComputeImage> {
        match self {
            AnyExecutor::Cpu(e) => e.execute_rotate_90(img, n),
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => e.execute_rotate_90(img, n),
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => e.execute_rotate_90(img, n),
        }
    }

    /// Crop region.
    pub fn execute_crop(&self, img: &crate::ComputeImage, x: u32, y: u32, w: u32, h: u32) -> ComputeResult<crate::ComputeImage> {
        let src_w = img.width as usize;
        let src_h = img.height as usize;
        let c = img.channels as usize;

        if x as usize + w as usize > src_w || y as usize + h as usize > src_h {
            return Err(ComputeError::InvalidDimensions(w, h));
        }

        let mut data = vec![0.0f32; (w as usize) * (h as usize) * c];

        for row in 0..h as usize {
            let src_row = (y as usize + row) * src_w * c + (x as usize) * c;
            let dst_row = row * (w as usize) * c;
            data[dst_row..dst_row + (w as usize) * c]
                .copy_from_slice(&img.data()[src_row..src_row + (w as usize) * c]);
        }

        crate::ComputeImage::from_f32(data, w, h, img.channels)
    }

    // =========================================================================
    // Streaming Operations
    // =========================================================================

    /// Execute color operation with streaming I/O.
    ///
    /// Processes image tile-by-tile from source to output without
    /// loading the entire image into memory.
    pub fn execute_color_streaming<S, O>(
        &self,
        source: &mut S,
        output: &mut O,
        op: &ColorOp,
    ) -> ComputeResult<()>
    where
        S: streaming::StreamingSource,
        O: streaming::StreamingOutput,
    {
        match self {
            AnyExecutor::Cpu(e) => e.execute_color_streaming(source, output, op),
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => e.execute_color_streaming(source, output, op),
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => e.execute_color_streaming(source, output, op),
        }
    }

    /// Execute chained color operations with streaming I/O.
    pub fn execute_color_chain_streaming<S, O>(
        &self,
        source: &mut S,
        output: &mut O,
        ops: &[ColorOp],
    ) -> ComputeResult<()>
    where
        S: streaming::StreamingSource,
        O: streaming::StreamingOutput,
    {
        match self {
            AnyExecutor::Cpu(e) => e.execute_color_chain_streaming(source, output, ops),
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => e.execute_color_chain_streaming(source, output, ops),
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => e.execute_color_chain_streaming(source, output, ops),
        }
    }
}

/// Create a TiledExecutor for the specified backend.
///
/// This is the preferred way to create processing backends, as it provides
/// automatic tiling and streaming support.
pub fn create_executor(backend: Backend) -> ComputeResult<AnyExecutor> {
    create_executor_with_config(backend, ExecutorConfig::default())
}

/// Create a TiledExecutor with custom config.
pub fn create_executor_with_config(backend: Backend, config: ExecutorConfig) -> ComputeResult<AnyExecutor> {
    match backend {
        Backend::Auto => {
            let best = select_best_backend();
            create_executor_with_config(best, config)
        }
        Backend::Cpu => {
            let gpu = CpuPrimitives::new();
            Ok(AnyExecutor::Cpu(TiledExecutor::with_config(gpu, config)))
        }
        Backend::Wgpu => {
            #[cfg(feature = "wgpu")]
            {
                let gpu = WgpuPrimitives::new()?;
                Ok(AnyExecutor::Wgpu(TiledExecutor::with_config(gpu, config)))
            }
            #[cfg(not(feature = "wgpu"))]
            {
                Err(ComputeError::BackendNotAvailable(
                    "wgpu feature not enabled".to_string()
                ))
            }
        }
        Backend::Cuda => {
            #[cfg(feature = "cuda")]
            {
                let gpu = CudaPrimitives::new()?;
                Ok(AnyExecutor::Cuda(TiledExecutor::with_config(gpu, config)))
            }
            #[cfg(not(feature = "cuda"))]
            {
                Err(ComputeError::BackendNotAvailable(
                    "cuda feature not enabled".to_string()
                ))
            }
        }
    }
}
