//! Compute backends for GPU image processing.
//!
//! Provides CPU (rayon) and wgpu backends with automatic selection.

mod gpu_primitives;
mod tiling;
mod detect;
mod cpu_backend;

#[cfg(feature = "wgpu")]
mod wgpu_backend;

pub use gpu_primitives::{GpuPrimitives, ImageHandle, KernelParams};
pub use tiling::{GpuLimits, Tile, generate_tiles, ProcessingStrategy, TileWorkflow};
pub use detect::{detect_backends, select_best_backend, describe_backends, BackendInfo};
pub use cpu_backend::{CpuBackend, CpuPrimitives};

#[cfg(feature = "wgpu")]
pub use wgpu_backend::{WgpuBackend, WgpuPrimitives};

#[cfg(not(feature = "wgpu"))]
use crate::ComputeError;
use crate::ComputeResult;


/// Available compute backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backend {
    /// Auto-select best available (wgpu > CPU).
    #[default]
    Auto,
    /// CPU backend using rayon for parallelization.
    Cpu,
    /// wgpu backend (Vulkan/Metal/DX12).
    Wgpu,
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

/// Create a backend instance.
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
    }
}
