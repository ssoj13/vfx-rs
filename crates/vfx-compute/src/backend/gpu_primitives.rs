//! GPU primitives abstraction for unified backend implementation.

use crate::ComputeResult;
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
    fn upload(&self, data: &[f32], width: u32, height: u32, channels: u32) -> ComputeResult<Self::Handle>;

    /// Download image data from GPU.
    fn download(&self, handle: &Self::Handle) -> ComputeResult<Vec<f32>>;

    /// Allocate output buffer.
    fn allocate(&self, width: u32, height: u32, channels: u32) -> ComputeResult<Self::Handle>;

    /// Execute color matrix kernel.
    fn exec_matrix(&self, src: &Self::Handle, dst: &mut Self::Handle, matrix: &[f32; 16]) -> ComputeResult<()>;

    /// Execute CDL kernel.
    fn exec_cdl(&self, src: &Self::Handle, dst: &mut Self::Handle,
                slope: [f32; 3], offset: [f32; 3], power: [f32; 3], sat: f32) -> ComputeResult<()>;

    /// Execute 1D LUT kernel.
    fn exec_lut1d(&self, src: &Self::Handle, dst: &mut Self::Handle,
                  lut: &[f32], channels: u32) -> ComputeResult<()>;

    /// Execute 3D LUT kernel (trilinear interpolation).
    fn exec_lut3d(&self, src: &Self::Handle, dst: &mut Self::Handle,
                  lut: &[f32], size: u32) -> ComputeResult<()>;

    /// Execute 3D LUT with tetrahedral interpolation (higher quality).
    ///
    /// Tetrahedral interpolation splits the unit cube into 6 tetrahedra
    /// and interpolates using 4 vertices instead of 8. This produces
    /// smoother results, especially with LUTs containing sharp transitions.
    fn exec_lut3d_tetrahedral(&self, src: &Self::Handle, dst: &mut Self::Handle,
                              lut: &[f32], size: u32) -> ComputeResult<()> {
        // Default: fallback to trilinear
        self.exec_lut3d(src, dst, lut, size)
    }

    /// Execute resize kernel.
    fn exec_resize(&self, src: &Self::Handle, dst: &mut Self::Handle, filter: u32) -> ComputeResult<()>;

    /// Execute blur kernel.
    fn exec_blur(&self, src: &Self::Handle, dst: &mut Self::Handle, radius: f32) -> ComputeResult<()>;

    /// Execute hue curves (Hue vs Hue/Sat/Lum).
    ///
    /// Applies three baked LUTs for hue-based adjustments:
    /// - `hue_vs_hue`: hue shift per input hue
    /// - `hue_vs_sat`: saturation multiplier per input hue
    /// - `hue_vs_lum`: luminance offset per input hue
    ///
    /// Each LUT has `lut_size` entries covering hue 0-1.
    fn exec_hue_curves(&self, src: &Self::Handle, dst: &mut Self::Handle,
                       hue_vs_hue: &[f32], hue_vs_sat: &[f32], hue_vs_lum: &[f32],
                       lut_size: u32) -> ComputeResult<()>;

    // =========================================================================
    // Transform Operations
    // =========================================================================

    /// Execute flip horizontal.
    fn exec_flip_h(&self, handle: &mut Self::Handle) -> ComputeResult<()>;

    /// Execute flip vertical.
    fn exec_flip_v(&self, handle: &mut Self::Handle) -> ComputeResult<()>;

    /// Execute rotate 90° clockwise (n times).
    fn exec_rotate_90(&self, src: &Self::Handle, n: u32) -> ComputeResult<Self::Handle>;

    // =========================================================================
    // Composite Operations
    // =========================================================================

    /// Execute Porter-Duff Over composite.
    fn exec_composite_over(&self, fg: &Self::Handle, bg: &mut Self::Handle) -> ComputeResult<()>;

    /// Execute blend with mode and opacity.
    fn exec_blend(&self, fg: &Self::Handle, bg: &mut Self::Handle, mode: u32, opacity: f32) -> ComputeResult<()>;

    // =========================================================================
    // Info
    // =========================================================================

    /// Get GPU limits.
    fn limits(&self) -> &GpuLimits;

    /// Backend name.
    fn name(&self) -> &'static str;
}
