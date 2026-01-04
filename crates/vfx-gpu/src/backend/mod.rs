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
pub use tiling::{GpuLimits, Tile, generate_tiles};
pub use detect::{detect_backends, select_best_backend, BackendInfo};
pub use cpu_backend::{CpuBackend, CpuPrimitives};

#[cfg(feature = "wgpu")]
pub use wgpu_backend::{WgpuBackend, WgpuPrimitives};

use crate::GpuResult;



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

/// Trait for color/image processing backends.
pub trait ProcessingBackend: Send + Sync {
    /// Backend name.
    fn name(&self) -> &'static str;
    
    /// Available memory in bytes.
    fn available_memory(&self) -> u64;
    
    /// GPU limits for tiling decisions.
    fn limits(&self) -> &GpuLimits;
    
    /// Upload image to GPU memory.
    fn upload(&self, data: &[f32], width: u32, height: u32, channels: u32) -> GpuResult<Box<dyn ImageHandle>>;
    
    /// Download image from GPU.
    fn download(&self, handle: &dyn ImageHandle) -> GpuResult<Vec<f32>>;
    
    /// Apply 4x4 color matrix transform.
    fn apply_matrix(&self, handle: &mut dyn ImageHandle, matrix: &[f32; 16]) -> GpuResult<()>;
    
    /// Apply CDL (slope, offset, power, saturation).
    fn apply_cdl(&self, handle: &mut dyn ImageHandle, slope: [f32; 3], offset: [f32; 3], power: [f32; 3], sat: f32) -> GpuResult<()>;
    
    /// Apply 1D LUT.
    fn apply_lut1d(&self, handle: &mut dyn ImageHandle, lut: &[f32], channels: u32) -> GpuResult<()>;
    
    /// Apply 3D LUT.
    fn apply_lut3d(&self, handle: &mut dyn ImageHandle, lut: &[f32], size: u32) -> GpuResult<()>;
    
    /// Resize image.
    fn resize(&self, handle: &dyn ImageHandle, width: u32, height: u32, filter: u32) -> GpuResult<Box<dyn ImageHandle>>;
    
    /// Apply Gaussian blur.
    fn blur(&self, handle: &mut dyn ImageHandle, radius: f32) -> GpuResult<()>;
}

/// Create a backend instance.
pub fn create_backend(backend: Backend) -> GpuResult<Box<dyn ProcessingBackend>> {
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
                Err(GpuError::BackendNotAvailable(
                    "wgpu feature not enabled".to_string()
                ))
            }
        }
    }
}
