//! GPU primitives abstraction for unified backend implementation.

use crate::GpuResult;
use super::GpuLimits;

/// Handle to an image in GPU memory.
pub trait ImageHandle: Send + Sync + AsAny {
    /// Image dimensions (width, height, channels).
    fn dimensions(&self) -> (u32, u32, u32);

    /// Width.
    fn width(&self) -> u32 { self.dimensions().0 }

    /// Height.
    fn height(&self) -> u32 { self.dimensions().1 }

    /// Channel count.
    fn channels(&self) -> u32 { self.dimensions().2 }

    /// Size in bytes of GPU memory used.
    fn size_bytes(&self) -> u64 {
        let (w, h, c) = self.dimensions();
        (w as u64) * (h as u64) * (c as u64) * 4 // f32
    }
}

/// Helper trait for downcasting.
pub trait AsAny: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Parameters for GPU kernels.
#[derive(Clone, Debug)]
pub struct KernelParams {
    /// Image dimensions [width, height, channels, 0].
    pub dims: [u32; 4],
    /// Operation-specific parameters.
    pub params: [f32; 16],
}

impl KernelParams {
    pub fn new(width: u32, height: u32, channels: u32) -> Self {
        Self {
            dims: [width, height, channels, 0],
            params: [0.0; 16],
        }
    }
}

/// Core GPU operations abstraction.
pub trait GpuPrimitives: Send + Sync {
    /// Backend-specific image handle type.
    type Handle: ImageHandle;

    /// Upload image data to GPU.
    fn upload(&self, data: &[f32], width: u32, height: u32, channels: u32) -> GpuResult<Self::Handle>;

    /// Download image data from GPU.
    fn download(&self, handle: &Self::Handle) -> GpuResult<Vec<f32>>;

    /// Allocate output buffer.
    fn allocate(&self, width: u32, height: u32, channels: u32) -> GpuResult<Self::Handle>;

    /// Execute color matrix kernel.
    fn exec_matrix(&self, src: &Self::Handle, dst: &mut Self::Handle, matrix: &[f32; 16]) -> GpuResult<()>;

    /// Execute CDL kernel.
    fn exec_cdl(&self, src: &Self::Handle, dst: &mut Self::Handle,
                slope: [f32; 3], offset: [f32; 3], power: [f32; 3], sat: f32) -> GpuResult<()>;

    /// Execute 1D LUT kernel.
    fn exec_lut1d(&self, src: &Self::Handle, dst: &mut Self::Handle,
                  lut: &[f32], channels: u32) -> GpuResult<()>;

    /// Execute 3D LUT kernel.
    fn exec_lut3d(&self, src: &Self::Handle, dst: &mut Self::Handle,
                  lut: &[f32], size: u32) -> GpuResult<()>;

    /// Execute resize kernel.
    fn exec_resize(&self, src: &Self::Handle, dst: &mut Self::Handle, filter: u32) -> GpuResult<()>;

    /// Execute blur kernel.
    fn exec_blur(&self, src: &Self::Handle, dst: &mut Self::Handle, radius: f32) -> GpuResult<()>;

    /// Get GPU limits.
    fn limits(&self) -> &GpuLimits;

    /// Backend name.
    fn name(&self) -> &'static str;
}
