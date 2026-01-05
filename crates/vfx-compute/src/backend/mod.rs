//! Compute backends for GPU image processing.
//!
//! Provides CPU (rayon), wgpu, and CUDA backends with automatic selection.
//!
//! # Architecture
//!
//! ```text
//! TiledExecutor<G: GpuPrimitives>
//!     +-- CpuPrimitives  (rayon parallelization)
//!     +-- WgpuPrimitives (Vulkan/Metal/DX12)
//!     +-- CudaPrimitives (NVIDIA CUDA)
//! ```
//!
//! All backends use the same `TiledExecutor` for automatic tiling and streaming.

mod gpu_primitives;
mod tiling;
mod detect;
mod cpu_backend;
mod executor;
pub mod streaming;

#[cfg(feature = "wgpu")]
mod wgpu_backend;

#[cfg(feature = "cuda")]
mod cuda_backend;

// Core types
pub use gpu_primitives::{GpuPrimitives, ImageHandle, KernelParams, AsAny};
pub use tiling::{GpuLimits, Tile, generate_tiles, ProcessingStrategy, TileWorkflow};
pub use detect::{detect_backends, select_best_backend, describe_backends, BackendInfo};

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

use crate::ComputeResult;
#[cfg(not(feature = "wgpu"))]
use crate::ComputeError;

/// Available compute backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backend {
    /// Auto-select best available (CUDA > wgpu > CPU).
    #[default]
    Auto,
    /// CPU backend using rayon for parallelization.
    Cpu,
    /// wgpu backend (Vulkan/Metal/DX12).
    Wgpu,
    /// NVIDIA CUDA backend.
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

/// Trait for color/image processing backends.
///
/// NOTE: This trait is being phased out in favor of `TiledExecutor<G: GpuPrimitives>`.
/// New code should use the executor pattern for automatic tiling support.
pub trait ProcessingBackend: Send + Sync {
    /// Backend name.
    fn name(&self) -> &'static str;

    /// Available memory in bytes.
    fn available_memory(&self) -> u64;

    /// GPU limits for tiling decisions.
    fn limits(&self) -> &GpuLimits;

    /// Upload image to GPU memory.
    fn upload(&self, data: &[f32], width: u32, height: u32, channels: u32) -> ComputeResult<Box<dyn ImageHandle>>;

    /// Download image from GPU.
    fn download(&self, handle: &dyn ImageHandle) -> ComputeResult<Vec<f32>>;

    // === Color operations ===

    /// Apply 4x4 color matrix transform.
    fn apply_matrix(&self, handle: &mut dyn ImageHandle, matrix: &[f32; 16]) -> ComputeResult<()>;

    /// Apply CDL (slope, offset, power, saturation).
    fn apply_cdl(&self, handle: &mut dyn ImageHandle, slope: [f32; 3], offset: [f32; 3], power: [f32; 3], sat: f32) -> ComputeResult<()>;

    /// Apply 1D LUT.
    fn apply_lut1d(&self, handle: &mut dyn ImageHandle, lut: &[f32], channels: u32) -> ComputeResult<()>;

    /// Apply 3D LUT.
    fn apply_lut3d(&self, handle: &mut dyn ImageHandle, lut: &[f32], size: u32) -> ComputeResult<()>;

    // === Image operations ===

    /// Resize image.
    fn resize(&self, handle: &dyn ImageHandle, width: u32, height: u32, filter: u32) -> ComputeResult<Box<dyn ImageHandle>>;

    /// Apply Gaussian blur.
    fn blur(&self, handle: &mut dyn ImageHandle, radius: f32) -> ComputeResult<()>;

    // === Composite operations ===

    /// Porter-Duff Over: fg over bg.
    fn composite_over(&self, fg: &dyn ImageHandle, bg: &mut dyn ImageHandle) -> ComputeResult<()>;

    /// Blend with mode.
    fn blend(&self, fg: &dyn ImageHandle, bg: &mut dyn ImageHandle, mode: BlendMode, opacity: f32) -> ComputeResult<()>;

    // === Transform operations ===

    /// Crop region.
    fn crop(&self, handle: &dyn ImageHandle, x: u32, y: u32, w: u32, h: u32) -> ComputeResult<Box<dyn ImageHandle>>;

    /// Flip horizontal.
    fn flip_h(&self, handle: &mut dyn ImageHandle) -> ComputeResult<()>;

    /// Flip vertical.
    fn flip_v(&self, handle: &mut dyn ImageHandle) -> ComputeResult<()>;

    /// Rotate 90 degrees clockwise (n times).
    fn rotate_90(&self, handle: &dyn ImageHandle, n: u32) -> ComputeResult<Box<dyn ImageHandle>>;
}

/// Create a ProcessingBackend instance (legacy API).
///
/// For new code, prefer using `create_executor()` which provides automatic tiling.
pub fn create_backend(backend: Backend) -> ComputeResult<Box<dyn ProcessingBackend>> {
    match backend {
        Backend::Auto => {
            let best = select_best_backend();
            create_backend(best)
        }
        Backend::Cpu => Ok(Box::new(CpuBackend::new())),
        Backend::Wgpu => {
            #[cfg(feature = "wgpu")]
            {
                Ok(Box::new(WgpuBackend::new()?))
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
                Ok(Box::new(CudaBackend::new()?))
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
