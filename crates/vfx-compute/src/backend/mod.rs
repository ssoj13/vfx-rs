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
pub use wgpu_backend::{WgpuBackend, WgpuPrimitives, WgpuImage};

#[cfg(feature = "cuda")]
pub use cuda_backend::{CudaBackend, CudaPrimitives, CudaImage};

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
            Self::Wgpu => WgpuBackend::is_available(),
            #[cfg(not(feature = "wgpu"))]
            Self::Wgpu => false,
            #[cfg(feature = "cuda")]
            Self::Cuda => CudaBackend::is_available(),
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

/// Executor type enum for dynamic dispatch.
pub enum AnyExecutor {
    Cpu(TiledExecutor<CpuPrimitives>),
    #[cfg(feature = "wgpu")]
    Wgpu(TiledExecutor<WgpuPrimitives>),
    #[cfg(feature = "cuda")]
    Cuda(TiledExecutor<CudaPrimitives>),
}

impl AnyExecutor {
    /// Get backend name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Cpu(e) => e.name(),
            #[cfg(feature = "wgpu")]
            Self::Wgpu(e) => e.name(),
            #[cfg(feature = "cuda")]
            Self::Cuda(e) => e.name(),
        }
    }

    /// Get GPU limits.
    pub fn limits(&self) -> &GpuLimits {
        match self {
            Self::Cpu(e) => e.limits(),
            #[cfg(feature = "wgpu")]
            Self::Wgpu(e) => e.limits(),
            #[cfg(feature = "cuda")]
            Self::Cuda(e) => e.limits(),
        }
    }

    /// Execute color operation with automatic tiling.
    pub fn execute_color(&self, img: &mut crate::ComputeImage, op: &ColorOp) -> ComputeResult<()> {
        match self {
            Self::Cpu(e) => e.execute_color(img, op),
            #[cfg(feature = "wgpu")]
            Self::Wgpu(e) => e.execute_color(img, op),
            #[cfg(feature = "cuda")]
            Self::Cuda(e) => e.execute_color(img, op),
        }
    }

    /// Execute multiple color operations without GPU round-trips.
    ///
    /// More efficient than calling `execute_color()` multiple times.
    pub fn execute_color_chain(&self, img: &mut crate::ComputeImage, ops: &[ColorOp]) -> ComputeResult<()> {
        match self {
            Self::Cpu(e) => e.execute_color_chain(img, ops),
            #[cfg(feature = "wgpu")]
            Self::Wgpu(e) => e.execute_color_chain(img, ops),
            #[cfg(feature = "cuda")]
            Self::Cuda(e) => e.execute_color_chain(img, ops),
        }
    }

    /// Execute blur with automatic tiling.
    pub fn execute_blur(&self, img: &mut crate::ComputeImage, radius: f32) -> ComputeResult<()> {
        match self {
            Self::Cpu(e) => e.execute_blur(img, radius),
            #[cfg(feature = "wgpu")]
            Self::Wgpu(e) => e.execute_blur(img, radius),
            #[cfg(feature = "cuda")]
            Self::Cuda(e) => e.execute_blur(img, radius),
        }
    }

    /// Execute resize.
    pub fn execute_resize(&self, img: &crate::ComputeImage, width: u32, height: u32, filter: u32) -> ComputeResult<crate::ComputeImage> {
        match self {
            Self::Cpu(e) => e.execute_resize(img, width, height, filter),
            #[cfg(feature = "wgpu")]
            Self::Wgpu(e) => e.execute_resize(img, width, height, filter),
            #[cfg(feature = "cuda")]
            Self::Cuda(e) => e.execute_resize(img, width, height, filter),
        }
    }

    /// Execute composite over.
    pub fn execute_composite_over(&self, fg: &crate::ComputeImage, bg: &mut crate::ComputeImage) -> ComputeResult<()> {
        match self {
            Self::Cpu(e) => {
                let fg_handle = e.gpu().upload(&fg.data, fg.width, fg.height, fg.channels)?;
                let mut bg_handle = e.gpu().upload(&bg.data, bg.width, bg.height, bg.channels)?;
                e.gpu().exec_composite_over(&fg_handle, &mut bg_handle)?;
                bg.data = e.gpu().download(&bg_handle)?;
                Ok(())
            }
            #[cfg(feature = "wgpu")]
            Self::Wgpu(e) => {
                let fg_handle = e.gpu().upload(&fg.data, fg.width, fg.height, fg.channels)?;
                let mut bg_handle = e.gpu().upload(&bg.data, bg.width, bg.height, bg.channels)?;
                e.gpu().exec_composite_over(&fg_handle, &mut bg_handle)?;
                bg.data = e.gpu().download(&bg_handle)?;
                Ok(())
            }
            #[cfg(feature = "cuda")]
            Self::Cuda(e) => {
                let fg_handle = e.gpu().upload(&fg.data, fg.width, fg.height, fg.channels)?;
                let mut bg_handle = e.gpu().upload(&bg.data, bg.width, bg.height, bg.channels)?;
                e.gpu().exec_composite_over(&fg_handle, &mut bg_handle)?;
                bg.data = e.gpu().download(&bg_handle)?;
                Ok(())
            }
        }
    }

    /// Execute blend.
    pub fn execute_blend(&self, fg: &crate::ComputeImage, bg: &mut crate::ComputeImage, mode: u32, opacity: f32) -> ComputeResult<()> {
        match self {
            Self::Cpu(e) => {
                let fg_handle = e.gpu().upload(&fg.data, fg.width, fg.height, fg.channels)?;
                let mut bg_handle = e.gpu().upload(&bg.data, bg.width, bg.height, bg.channels)?;
                e.gpu().exec_blend(&fg_handle, &mut bg_handle, mode, opacity)?;
                bg.data = e.gpu().download(&bg_handle)?;
                Ok(())
            }
            #[cfg(feature = "wgpu")]
            Self::Wgpu(e) => {
                let fg_handle = e.gpu().upload(&fg.data, fg.width, fg.height, fg.channels)?;
                let mut bg_handle = e.gpu().upload(&bg.data, bg.width, bg.height, bg.channels)?;
                e.gpu().exec_blend(&fg_handle, &mut bg_handle, mode, opacity)?;
                bg.data = e.gpu().download(&bg_handle)?;
                Ok(())
            }
            #[cfg(feature = "cuda")]
            Self::Cuda(e) => {
                let fg_handle = e.gpu().upload(&fg.data, fg.width, fg.height, fg.channels)?;
                let mut bg_handle = e.gpu().upload(&bg.data, bg.width, bg.height, bg.channels)?;
                e.gpu().exec_blend(&fg_handle, &mut bg_handle, mode, opacity)?;
                bg.data = e.gpu().download(&bg_handle)?;
                Ok(())
            }
        }
    }

    /// Execute flip horizontal.
    pub fn execute_flip_h(&self, img: &mut crate::ComputeImage) -> ComputeResult<()> {
        match self {
            Self::Cpu(e) => {
                let mut handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                e.gpu().exec_flip_h(&mut handle)?;
                img.data = e.gpu().download(&handle)?;
                Ok(())
            }
            #[cfg(feature = "wgpu")]
            Self::Wgpu(e) => {
                let mut handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                e.gpu().exec_flip_h(&mut handle)?;
                img.data = e.gpu().download(&handle)?;
                Ok(())
            }
            #[cfg(feature = "cuda")]
            Self::Cuda(e) => {
                let mut handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                e.gpu().exec_flip_h(&mut handle)?;
                img.data = e.gpu().download(&handle)?;
                Ok(())
            }
        }
    }

    /// Execute flip vertical.
    pub fn execute_flip_v(&self, img: &mut crate::ComputeImage) -> ComputeResult<()> {
        match self {
            Self::Cpu(e) => {
                let mut handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                e.gpu().exec_flip_v(&mut handle)?;
                img.data = e.gpu().download(&handle)?;
                Ok(())
            }
            #[cfg(feature = "wgpu")]
            Self::Wgpu(e) => {
                let mut handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                e.gpu().exec_flip_v(&mut handle)?;
                img.data = e.gpu().download(&handle)?;
                Ok(())
            }
            #[cfg(feature = "cuda")]
            Self::Cuda(e) => {
                let mut handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                e.gpu().exec_flip_v(&mut handle)?;
                img.data = e.gpu().download(&handle)?;
                Ok(())
            }
        }
    }

    /// Execute rotate 90 degrees clockwise.
    pub fn execute_rotate_90(&self, img: &crate::ComputeImage, n: u32) -> ComputeResult<crate::ComputeImage> {
        match self {
            Self::Cpu(e) => {
                let handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                let rotated = e.gpu().exec_rotate_90(&handle, n)?;
                let (w, h, c) = rotated.dimensions();
                let data = e.gpu().download(&rotated)?;
                crate::ComputeImage::from_f32(data, w, h, c)
            }
            #[cfg(feature = "wgpu")]
            Self::Wgpu(e) => {
                let handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                let rotated = e.gpu().exec_rotate_90(&handle, n)?;
                let (w, h, c) = rotated.dimensions();
                let data = e.gpu().download(&rotated)?;
                crate::ComputeImage::from_f32(data, w, h, c)
            }
            #[cfg(feature = "cuda")]
            Self::Cuda(e) => {
                let handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                let rotated = e.gpu().exec_rotate_90(&handle, n)?;
                let (w, h, c) = rotated.dimensions();
                let data = e.gpu().download(&rotated)?;
                crate::ComputeImage::from_f32(data, w, h, c)
            }
        }
    }

    /// Crop region (CPU-only, since GpuPrimitives doesn't have exec_crop).
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
                .copy_from_slice(&img.data[src_row..src_row + (w as usize) * c]);
        }
        
        crate::ComputeImage::from_f32(data, w, h, img.channels)
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
