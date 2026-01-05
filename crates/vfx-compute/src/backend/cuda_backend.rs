//! CUDA GPU compute backend for color/image processing.
//!
//! Requires the `cuda` feature and NVIDIA GPU with CUDA support.

use std::sync::Arc;

use cudarc::driver::{CudaContext, CudaFunction, CudaModule, CudaSlice, CudaStream, LaunchConfig, PushKernelArg};

use super::gpu_primitives::{GpuPrimitives, ImageHandle, KernelParams, AsAny};
use super::tiling::GpuLimits;
use super::{ProcessingBackend, BlendMode};
use crate::{ComputeError, ComputeResult};

// =============================================================================
// CUDA Kernel Source
// =============================================================================

const CUDA_KERNELS: &str = r#"
extern "C" {

// ============================================================================
// Color Matrix 4x4
// ============================================================================
__global__ void color_matrix(
    const float* __restrict__ src,
    float* __restrict__ dst,
    int total, int c,
    float m00, float m01, float m02, float m03,
    float m10, float m11, float m12, float m13,
    float m20, float m21, float m22, float m23,
    float m30, float m31, float m32, float m33
) {
    int px = blockIdx.x * blockDim.x + threadIdx.x;
    if (px >= total) return;

    int base = px * c;
    float r = src[base];
    float g = src[base + 1];
    float b = src[base + 2];
    float a = (c >= 4) ? src[base + 3] : 1.0f;

    dst[base]     = m00*r + m01*g + m02*b + m03*a;
    dst[base + 1] = m10*r + m11*g + m12*b + m13*a;
    dst[base + 2] = m20*r + m21*g + m22*b + m23*a;
    if (c >= 4) dst[base + 3] = m30*r + m31*g + m32*b + m33*a;
}

// ============================================================================
// CDL (Color Decision List)
// ============================================================================
__global__ void cdl_kernel(
    const float* __restrict__ src,
    float* __restrict__ dst,
    int total, int c,
    float slope_r, float slope_g, float slope_b,
    float offset_r, float offset_g, float offset_b,
    float power_r, float power_g, float power_b,
    float saturation
) {
    int px = blockIdx.x * blockDim.x + threadIdx.x;
    if (px >= total) return;

    int base = px * c;
    
    // CDL: out = (in * slope + offset) ^ power
    float r = powf(fmaxf(src[base]     * slope_r + offset_r, 0.0f), power_r);
    float g = powf(fmaxf(src[base + 1] * slope_g + offset_g, 0.0f), power_g);
    float b = powf(fmaxf(src[base + 2] * slope_b + offset_b, 0.0f), power_b);

    // Saturation
    if (saturation != 1.0f) {
        float luma = 0.2126f * r + 0.7152f * g + 0.0722f * b;
        r = luma + saturation * (r - luma);
        g = luma + saturation * (g - luma);
        b = luma + saturation * (b - luma);
    }

    dst[base]     = r;
    dst[base + 1] = g;
    dst[base + 2] = b;
    if (c >= 4) dst[base + 3] = src[base + 3];
}

// ============================================================================
// 1D LUT Interpolation
// ============================================================================
__global__ void lut1d_kernel(
    const float* __restrict__ src,
    float* __restrict__ dst,
    const float* __restrict__ lut,
    int total, int c, int lut_size
) {
    int px = blockIdx.x * blockDim.x + threadIdx.x;
    if (px >= total) return;

    int base = px * c;
    float scale = (float)(lut_size - 1);

    for (int ch = 0; ch < 3 && ch < c; ch++) {
        float v = fminf(fmaxf(src[base + ch], 0.0f), 1.0f) * scale;
        int i0 = (int)v;
        int i1 = min(i0 + 1, lut_size - 1);
        float f = v - (float)i0;

        float v0 = lut[i0 * 3 + ch];
        float v1 = lut[i1 * 3 + ch];
        dst[base + ch] = v0 + f * (v1 - v0);
    }
    if (c >= 4) dst[base + 3] = src[base + 3];
}

// ============================================================================
// 3D LUT Trilinear Interpolation
// ============================================================================
__device__ float lut3d_sample(const float* lut, int ri, int gi, int bi, int ch, int s) {
    return lut[(bi * s * s + gi * s + ri) * 3 + ch];
}

__global__ void lut3d_kernel(
    const float* __restrict__ src,
    float* __restrict__ dst,
    const float* __restrict__ lut,
    int total, int c, int s
) {
    int px = blockIdx.x * blockDim.x + threadIdx.x;
    if (px >= total) return;

    int base = px * c;
    float scale = (float)(s - 1);

    float r = fminf(fmaxf(src[base], 0.0f), 1.0f) * scale;
    float g = fminf(fmaxf(src[base + 1], 0.0f), 1.0f) * scale;
    float b = fminf(fmaxf(src[base + 2], 0.0f), 1.0f) * scale;

    int r0 = min((int)r, s - 1);
    int g0 = min((int)g, s - 1);
    int b0 = min((int)b, s - 1);
    int r1 = min(r0 + 1, s - 1);
    int g1 = min(g0 + 1, s - 1);
    int b1 = min(b0 + 1, s - 1);

    float fr = r - (float)r0;
    float fg = g - (float)g0;
    float fb = b - (float)b0;

    for (int ch = 0; ch < 3; ch++) {
        float c000 = lut3d_sample(lut, r0, g0, b0, ch, s);
        float c100 = lut3d_sample(lut, r1, g0, b0, ch, s);
        float c010 = lut3d_sample(lut, r0, g1, b0, ch, s);
        float c110 = lut3d_sample(lut, r1, g1, b0, ch, s);
        float c001 = lut3d_sample(lut, r0, g0, b1, ch, s);
        float c101 = lut3d_sample(lut, r1, g0, b1, ch, s);
        float c011 = lut3d_sample(lut, r0, g1, b1, ch, s);
        float c111 = lut3d_sample(lut, r1, g1, b1, ch, s);

        float c00 = c000 + fr * (c100 - c000);
        float c10 = c010 + fr * (c110 - c010);
        float c01 = c001 + fr * (c101 - c001);
        float c11 = c011 + fr * (c111 - c011);

        float cc0 = c00 + fg * (c10 - c00);
        float cc1 = c01 + fg * (c11 - c01);

        dst[base + ch] = cc0 + fb * (cc1 - cc0);
    }
    if (c >= 4) dst[base + 3] = src[base + 3];
}

// ============================================================================
// Bilinear Resize
// ============================================================================
__global__ void resize_kernel(
    const float* __restrict__ src,
    float* __restrict__ dst,
    int sw, int sh, int dw, int dh, int c
) {
    int dx = blockIdx.x * blockDim.x + threadIdx.x;
    int dy = blockIdx.y * blockDim.y + threadIdx.y;
    if (dx >= dw || dy >= dh) return;

    float sx_scale = (float)sw / (float)dw;
    float sy_scale = (float)sh / (float)dh;

    float fx = (float)dx * sx_scale;
    float fy = (float)dy * sy_scale;

    int x0 = min((int)fx, sw - 1);
    int y0 = min((int)fy, sh - 1);
    int x1 = min(x0 + 1, sw - 1);
    int y1 = min(y0 + 1, sh - 1);

    float ffx = fx - (float)x0;
    float ffy = fy - (float)y0;

    int dst_base = (dy * dw + dx) * c;

    for (int ch = 0; ch < c; ch++) {
        float c00 = src[(y0 * sw + x0) * c + ch];
        float c10 = src[(y0 * sw + x1) * c + ch];
        float c01 = src[(y1 * sw + x0) * c + ch];
        float c11 = src[(y1 * sw + x1) * c + ch];

        float top = c00 + ffx * (c10 - c00);
        float bot = c01 + ffx * (c11 - c01);
        dst[dst_base + ch] = top + ffy * (bot - top);
    }
}

// ============================================================================
// Gaussian Blur (Horizontal)
// ============================================================================
__global__ void blur_h_kernel(
    const float* __restrict__ src,
    float* __restrict__ dst,
    const float* __restrict__ kernel,
    int w, int h, int c, int radius
) {
    int px = blockIdx.x * blockDim.x + threadIdx.x;
    if (px >= w * h) return;

    int y = px / w;
    int x = px % w;
    int k_size = radius * 2 + 1;
    int base = (y * w + x) * c;

    for (int ch = 0; ch < c; ch++) {
        float acc = 0.0f;
        for (int ki = 0; ki < k_size; ki++) {
            int sx = min(max(x + ki - radius, 0), w - 1);
            acc += src[(y * w + sx) * c + ch] * kernel[ki];
        }
        dst[base + ch] = acc;
    }
}

// ============================================================================
// Gaussian Blur (Vertical)
// ============================================================================
__global__ void blur_v_kernel(
    const float* __restrict__ src,
    float* __restrict__ dst,
    const float* __restrict__ kernel,
    int w, int h, int c, int radius
) {
    int px = blockIdx.x * blockDim.x + threadIdx.x;
    if (px >= w * h) return;

    int y = px / w;
    int x = px % w;
    int k_size = radius * 2 + 1;
    int base = (y * w + x) * c;

    for (int ch = 0; ch < c; ch++) {
        float acc = 0.0f;
        for (int ki = 0; ki < k_size; ki++) {
            int sy = min(max(y + ki - radius, 0), h - 1);
            acc += src[(sy * w + x) * c + ch] * kernel[ki];
        }
        dst[base + ch] = acc;
    }
}

// ============================================================================
// Porter-Duff Over Composite
// ============================================================================
__global__ void composite_over_kernel(
    const float* __restrict__ fg,
    float* __restrict__ bg,
    int total, int c
) {
    int px = blockIdx.x * blockDim.x + threadIdx.x;
    if (px >= total) return;

    int base = px * c;
    float fg_r = fg[base];
    float fg_g = fg[base + 1];
    float fg_b = fg[base + 2];
    float fg_a = (c >= 4) ? fg[base + 3] : 1.0f;

    float bg_r = bg[base];
    float bg_g = bg[base + 1];
    float bg_b = bg[base + 2];
    float bg_a = (c >= 4) ? bg[base + 3] : 1.0f;

    float inv_fg_a = 1.0f - fg_a;
    bg[base]     = fg_r * fg_a + bg_r * bg_a * inv_fg_a;
    bg[base + 1] = fg_g * fg_a + bg_g * bg_a * inv_fg_a;
    bg[base + 2] = fg_b * fg_a + bg_b * bg_a * inv_fg_a;
    if (c >= 4) bg[base + 3] = fg_a + bg_a * inv_fg_a;
}

// ============================================================================
// Flip Horizontal
// ============================================================================
__global__ void flip_h_kernel(
    const float* __restrict__ src,
    float* __restrict__ dst,
    int w, int h, int c
) {
    int x = blockIdx.x * blockDim.x + threadIdx.x;
    int y = blockIdx.y * blockDim.y + threadIdx.y;
    if (x >= w || y >= h) return;

    int src_idx = (y * w + (w - 1 - x)) * c;
    int dst_idx = (y * w + x) * c;
    for (int ch = 0; ch < c; ch++) {
        dst[dst_idx + ch] = src[src_idx + ch];
    }
}

// ============================================================================
// Flip Vertical
// ============================================================================
__global__ void flip_v_kernel(
    const float* __restrict__ src,
    float* __restrict__ dst,
    int w, int h, int c
) {
    int x = blockIdx.x * blockDim.x + threadIdx.x;
    int y = blockIdx.y * blockDim.y + threadIdx.y;
    if (x >= w || y >= h) return;

    int src_idx = ((h - 1 - y) * w + x) * c;
    int dst_idx = (y * w + x) * c;
    for (int ch = 0; ch < c; ch++) {
        dst[dst_idx + ch] = src[src_idx + ch];
    }
}

// ============================================================================
// Crop
// ============================================================================
__global__ void crop_kernel(
    const float* __restrict__ src,
    float* __restrict__ dst,
    int src_w, int dst_w, int dst_h, int crop_x, int crop_y, int c
) {
    int dx = blockIdx.x * blockDim.x + threadIdx.x;
    int dy = blockIdx.y * blockDim.y + threadIdx.y;
    if (dx >= dst_w || dy >= dst_h) return;

    int sx = crop_x + dx;
    int sy = crop_y + dy;
    int src_idx = (sy * src_w + sx) * c;
    int dst_idx = (dy * dst_w + dx) * c;
    for (int ch = 0; ch < c; ch++) {
        dst[dst_idx + ch] = src[src_idx + ch];
    }
}

// ============================================================================
// Rotate 90
// ============================================================================
__global__ void rotate90_kernel(
    const float* __restrict__ src,
    float* __restrict__ dst,
    int src_w, int src_h, int dst_w, int dst_h, int c, int n
) {
    int dx = blockIdx.x * blockDim.x + threadIdx.x;
    int dy = blockIdx.y * blockDim.y + threadIdx.y;
    if (dx >= dst_w || dy >= dst_h) return;

    int sx, sy;
    switch (n % 4) {
        case 1:  // 90 CW
            sx = dy;
            sy = src_h - 1 - dx;
            break;
        case 2:  // 180
            sx = src_w - 1 - dx;
            sy = src_h - 1 - dy;
            break;
        case 3:  // 270 CW
            sx = src_w - 1 - dy;
            sy = dx;
            break;
        default:  // 0
            sx = dx;
            sy = dy;
            break;
    }

    int src_idx = (sy * src_w + sx) * c;
    int dst_idx = (dy * dst_w + dx) * c;
    for (int ch = 0; ch < c; ch++) {
        dst[dst_idx + ch] = src[src_idx + ch];
    }
}

// ============================================================================
// Blend Modes
// ============================================================================
__device__ float blend_channel(float a, float b, int mode) {
    switch (mode) {
        case 0: return a;  // Normal
        case 1: return a * b;  // Multiply
        case 2: return 1.0f - (1.0f - a) * (1.0f - b);  // Screen
        case 3: return fminf(a + b, 1.0f);  // Add
        case 4: return fmaxf(b - a, 0.0f);  // Subtract
        case 5: return (b < 0.5f) ? 2.0f * a * b : 1.0f - 2.0f * (1.0f - a) * (1.0f - b);  // Overlay
        case 6: return (a < 0.5f) ? b - (1.0f - 2.0f * a) * b * (1.0f - b) : b + (2.0f * a - 1.0f) * (sqrtf(b) - b);  // SoftLight
        case 7: return (a < 0.5f) ? 2.0f * a * b : 1.0f - 2.0f * (1.0f - a) * (1.0f - b);  // HardLight
        case 8: return fabsf(a - b);  // Difference
        default: return a;
    }
}

__global__ void blend_kernel(
    const float* __restrict__ fg,
    float* __restrict__ bg,
    int total, int c, int mode, float opacity
) {
    int px = blockIdx.x * blockDim.x + threadIdx.x;
    if (px >= total) return;

    int base = px * c;
    for (int ch = 0; ch < 3 && ch < c; ch++) {
        float a = fg[base + ch];
        float b = bg[base + ch];
        float blended = blend_channel(a, b, mode);
        bg[base + ch] = b + opacity * (blended - b);
    }
}

} // extern "C"
"#;

// =============================================================================
// CUDA Handle
// =============================================================================

/// CUDA buffer handle for image data.
pub struct CudaImage {
    buffer: CudaSlice<f32>,
    width: u32,
    height: u32,
    channels: u32,
}

// Alias for backward compatibility
pub type CudaImageHandle = CudaImage;

impl AsAny for CudaImage {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl ImageHandle for CudaImage {
    fn dimensions(&self) -> (u32, u32, u32) {
        (self.width, self.height, self.channels)
    }
}

// =============================================================================
// CudaPrimitives (implements GpuPrimitives for TiledExecutor)
// =============================================================================

/// CUDA primitives for TiledExecutor.
///
/// Implements `GpuPrimitives` trait for use with `TiledExecutor<CudaPrimitives>`.
pub struct CudaPrimitives {
    ctx: Arc<CudaContext>,
    stream: Arc<CudaStream>,
    module: Arc<CudaModule>,
    k_matrix: CudaFunction,
    k_cdl: CudaFunction,
    k_lut1d: CudaFunction,
    k_lut3d: CudaFunction,
    k_resize: CudaFunction,
    k_blur_h: CudaFunction,
    k_blur_v: CudaFunction,
    limits: GpuLimits,
}

impl CudaPrimitives {
    /// Create new CUDA primitives.
    pub fn new() -> ComputeResult<Self> {
        let ctx = CudaContext::new(0).map_err(|e| {
            ComputeError::DeviceCreation(format!("CUDA init failed: {e:?}"))
        })?;

        let stream = ctx.default_stream();
        let available = query_available_memory();
        let limits = GpuLimits {
            max_tile_dim: 32768,
            max_buffer_bytes: available,
            total_memory: available,
            available_memory: available,
            detected: true,
        };

        let ptx = cudarc::nvrtc::compile_ptx(CUDA_KERNELS).map_err(|e| {
            ComputeError::ShaderCompilation(format!("CUDA kernel compile failed: {e:?}"))
        })?;

        let module = ctx.load_module(ptx).map_err(|e| {
            ComputeError::ShaderCompilation(format!("CUDA module load failed: {e:?}"))
        })?;

        let load_fn = |name: &str| -> ComputeResult<CudaFunction> {
            module.load_function(name).map_err(|e| {
                ComputeError::ShaderCompilation(format!("Failed to load {name}: {e:?}"))
            })
        };

        Ok(Self {
            ctx,
            stream,
            module,
            k_matrix: load_fn("color_matrix")?,
            k_cdl: load_fn("cdl_kernel")?,
            k_lut1d: load_fn("lut1d_kernel")?,
            k_lut3d: load_fn("lut3d_kernel")?,
            k_resize: load_fn("resize_kernel")?,
            k_blur_h: load_fn("blur_h_kernel")?,
            k_blur_v: load_fn("blur_v_kernel")?,
            limits,
        })
    }

    fn launch_1d(&self, total: u32) -> LaunchConfig {
        let block = 256u32;
        let grid = total.div_ceil(block);
        LaunchConfig {
            block_dim: (block, 1, 1),
            grid_dim: (grid, 1, 1),
            shared_mem_bytes: 0,
        }
    }

    fn launch_2d(&self, w: u32, h: u32) -> LaunchConfig {
        let block = 16u32;
        LaunchConfig {
            block_dim: (block, block, 1),
            grid_dim: (w.div_ceil(block), h.div_ceil(block), 1),
            shared_mem_bytes: 0,
        }
    }
}

impl GpuPrimitives for CudaPrimitives {
    type Handle = CudaImage;

    fn upload(&self, data: &[f32], width: u32, height: u32, channels: u32) -> ComputeResult<Self::Handle> {
        let buffer = self.stream.clone_htod(data).map_err(|e| {
            ComputeError::BufferCreation(format!("Upload failed: {e:?}"))
        })?;
        Ok(CudaImage { buffer, width, height, channels })
    }

    fn download(&self, handle: &Self::Handle) -> ComputeResult<Vec<f32>> {
        self.stream.clone_dtoh(&handle.buffer).map_err(|e| {
            ComputeError::OperationFailed(format!("Download failed: {e:?}"))
        })
    }

    fn allocate(&self, width: u32, height: u32, channels: u32) -> ComputeResult<Self::Handle> {
        let size = (width * height * channels) as usize;
        let buffer: CudaSlice<f32> = self.stream.alloc_zeros(size).map_err(|e| {
            ComputeError::BufferCreation(format!("Allocate failed: {e:?}"))
        })?;
        Ok(CudaImage { buffer, width, height, channels })
    }

    fn exec_matrix(&self, src: &Self::Handle, dst: &mut Self::Handle, matrix: &[f32; 16]) -> ComputeResult<()> {
        let total = (src.width * src.height) as i32;
        let c = src.channels as i32;

        let cfg = self.launch_1d(total as u32);
        let mut builder = self.stream.launch_builder(&self.k_matrix);
        builder.arg(&src.buffer);
        builder.arg(&dst.buffer);
        builder.arg(&total);
        builder.arg(&c);
        for &m in matrix {
            builder.arg(&m);
        }

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("Matrix failed: {e:?}"))
        })?;
        Ok(())
    }

    fn exec_cdl(&self, src: &Self::Handle, dst: &mut Self::Handle,
                slope: [f32; 3], offset: [f32; 3], power: [f32; 3], sat: f32) -> ComputeResult<()> {
        let total = (src.width * src.height) as i32;
        let c = src.channels as i32;

        let cfg = self.launch_1d(total as u32);
        let mut builder = self.stream.launch_builder(&self.k_cdl);
        builder.arg(&src.buffer);
        builder.arg(&dst.buffer);
        builder.arg(&total);
        builder.arg(&c);
        builder.arg(&slope[0]); builder.arg(&slope[1]); builder.arg(&slope[2]);
        builder.arg(&offset[0]); builder.arg(&offset[1]); builder.arg(&offset[2]);
        builder.arg(&power[0]); builder.arg(&power[1]); builder.arg(&power[2]);
        builder.arg(&sat);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("CDL failed: {e:?}"))
        })?;
        Ok(())
    }

    fn exec_lut1d(&self, src: &Self::Handle, dst: &mut Self::Handle,
                  lut: &[f32], _channels: u32) -> ComputeResult<()> {
        let total = (src.width * src.height) as i32;
        let c = src.channels as i32;
        let lut_size = (lut.len() / 3) as i32;

        let lut_buf = self.stream.clone_htod(lut).map_err(|e| {
            ComputeError::BufferCreation(format!("LUT upload failed: {e:?}"))
        })?;

        let cfg = self.launch_1d(total as u32);
        let mut builder = self.stream.launch_builder(&self.k_lut1d);
        builder.arg(&src.buffer);
        builder.arg(&dst.buffer);
        builder.arg(&lut_buf);
        builder.arg(&total);
        builder.arg(&c);
        builder.arg(&lut_size);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("LUT1D failed: {e:?}"))
        })?;
        Ok(())
    }

    fn exec_lut3d(&self, src: &Self::Handle, dst: &mut Self::Handle,
                  lut: &[f32], size: u32) -> ComputeResult<()> {
        let total = (src.width * src.height) as i32;
        let c = src.channels as i32;
        let s = size as i32;

        let lut_buf = self.stream.clone_htod(lut).map_err(|e| {
            ComputeError::BufferCreation(format!("LUT upload failed: {e:?}"))
        })?;

        let cfg = self.launch_1d(total as u32);
        let mut builder = self.stream.launch_builder(&self.k_lut3d);
        builder.arg(&src.buffer);
        builder.arg(&dst.buffer);
        builder.arg(&lut_buf);
        builder.arg(&total);
        builder.arg(&c);
        builder.arg(&s);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("LUT3D failed: {e:?}"))
        })?;
        Ok(())
    }

    fn exec_resize(&self, src: &Self::Handle, dst: &mut Self::Handle, _filter: u32) -> ComputeResult<()> {
        let (sw, sh, c) = (src.width as i32, src.height as i32, src.channels as i32);
        let (dw, dh) = (dst.width as i32, dst.height as i32);

        let cfg = self.launch_2d(dst.width, dst.height);
        let mut builder = self.stream.launch_builder(&self.k_resize);
        builder.arg(&src.buffer);
        builder.arg(&dst.buffer);
        builder.arg(&sw); builder.arg(&sh);
        builder.arg(&dw); builder.arg(&dh);
        builder.arg(&c);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("Resize failed: {e:?}"))
        })?;
        Ok(())
    }

    fn exec_blur(&self, src: &Self::Handle, dst: &mut Self::Handle, radius: f32) -> ComputeResult<()> {
        let (w, h, c) = (src.width as i32, src.height as i32, src.channels as i32);
        let r = radius.ceil() as i32;
        let sigma = radius / 3.0;

        // Gaussian kernel
        let kernel_size = (r * 2 + 1) as usize;
        let mut kernel = vec![0.0f32; kernel_size];
        let mut sum = 0.0f32;
        for i in 0..kernel_size {
            let x = (i as i32 - r) as f32;
            let g = (-x * x / (2.0 * sigma * sigma)).exp();
            kernel[i] = g;
            sum += g;
        }
        for k in &mut kernel { *k /= sum; }

        let kernel_buf = self.stream.clone_htod(&kernel).map_err(|e| {
            ComputeError::BufferCreation(format!("Kernel upload failed: {e:?}"))
        })?;

        // Temp buffer for horizontal pass
        let temp: CudaSlice<f32> = self.stream.alloc_zeros((w * h * c) as usize).map_err(|e| {
            ComputeError::BufferCreation(format!("{e:?}"))
        })?;

        // Horizontal pass
        let cfg = self.launch_2d(src.width, src.height);
        let mut builder = self.stream.launch_builder(&self.k_blur_h);
        builder.arg(&src.buffer);
        builder.arg(&temp);
        builder.arg(&kernel_buf);
        builder.arg(&w); builder.arg(&h); builder.arg(&c); builder.arg(&r);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("Blur H failed: {e:?}"))
        })?;

        // Vertical pass
        let mut builder = self.stream.launch_builder(&self.k_blur_v);
        builder.arg(&temp);
        builder.arg(&dst.buffer);
        builder.arg(&kernel_buf);
        builder.arg(&w); builder.arg(&h); builder.arg(&c); builder.arg(&r);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("Blur V failed: {e:?}"))
        })?;

        Ok(())
    }

    fn limits(&self) -> &GpuLimits {
        &self.limits
    }

    fn name(&self) -> &'static str {
        "CUDA"
    }
}

// =============================================================================
// CudaBackend (ProcessingBackend - legacy API)
// =============================================================================

/// CUDA GPU compute backend.
pub struct CudaBackend {
    #[allow(dead_code)]
    ctx: Arc<CudaContext>,
    stream: Arc<CudaStream>,
    #[allow(dead_code)]
    module: Arc<CudaModule>,
    
    // Kernels
    k_matrix: CudaFunction,
    k_cdl: CudaFunction,
    k_lut1d: CudaFunction,
    k_lut3d: CudaFunction,
    k_resize: CudaFunction,
    k_blur_h: CudaFunction,
    k_blur_v: CudaFunction,
    k_composite: CudaFunction,
    k_blend: CudaFunction,
    k_flip_h: CudaFunction,
    k_flip_v: CudaFunction,
    k_crop: CudaFunction,
    k_rotate: CudaFunction,
    
    limits: GpuLimits,
    device_name: String,
}

impl CudaBackend {
    /// Create new CUDA backend.
    pub fn new() -> ComputeResult<Self> {
        let ctx = CudaContext::new(0).map_err(|e| {
            ComputeError::DeviceCreation(format!("CUDA init failed: {e:?}"))
        })?;

        let stream = ctx.default_stream();
        let device_name = format!("CUDA Device {}", ctx.cu_device());

        // Query memory
        let available = query_available_memory();
        let limits = GpuLimits {
            max_tile_dim: 32768,
            max_buffer_bytes: available,
            total_memory: available,
            available_memory: available,
            detected: true,
        };

        // Compile PTX
        let ptx = cudarc::nvrtc::compile_ptx(CUDA_KERNELS).map_err(|e| {
            ComputeError::ShaderCompilation(format!("CUDA kernel compile failed: {e:?}"))
        })?;

        let module = ctx.load_module(ptx).map_err(|e| {
            ComputeError::ShaderCompilation(format!("CUDA module load failed: {e:?}"))
        })?;

        // Load all kernels
        let load_fn = |name: &str| -> ComputeResult<CudaFunction> {
            module.load_function(name).map_err(|e| {
                ComputeError::ShaderCompilation(format!("Failed to load {name}: {e:?}"))
            })
        };

        Ok(Self {
            ctx,
            stream,
            module,
            k_matrix: load_fn("color_matrix")?,
            k_cdl: load_fn("cdl_kernel")?,
            k_lut1d: load_fn("lut1d_kernel")?,
            k_lut3d: load_fn("lut3d_kernel")?,
            k_resize: load_fn("resize_kernel")?,
            k_blur_h: load_fn("blur_h_kernel")?,
            k_blur_v: load_fn("blur_v_kernel")?,
            k_composite: load_fn("composite_over_kernel")?,
            k_blend: load_fn("blend_kernel")?,
            k_flip_h: load_fn("flip_h_kernel")?,
            k_flip_v: load_fn("flip_v_kernel")?,
            k_crop: load_fn("crop_kernel")?,
            k_rotate: load_fn("rotate90_kernel")?,
            limits,
            device_name,
        })
    }

    /// Check if CUDA is available.
    pub fn is_available() -> bool {
        CudaContext::new(0).is_ok()
    }

    /// Get device name.
    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    fn launch_1d(&self, kernel: &CudaFunction, total: u32) -> LaunchConfig {
        let block = 256u32;
        let grid = total.div_ceil(block);
        LaunchConfig {
            block_dim: (block, 1, 1),
            grid_dim: (grid, 1, 1),
            shared_mem_bytes: 0,
        }
    }

    fn launch_2d(&self, w: u32, h: u32) -> LaunchConfig {
        let block = 16u32;
        LaunchConfig {
            block_dim: (block, block, 1),
            grid_dim: (w.div_ceil(block), h.div_ceil(block), 1),
            shared_mem_bytes: 0,
        }
    }

    fn downcast<'a>(&self, handle: &'a dyn ImageHandle) -> ComputeResult<&'a CudaImageHandle> {
        (handle as &dyn std::any::Any)
            .downcast_ref::<CudaImageHandle>()
            .ok_or_else(|| ComputeError::OperationFailed("Invalid handle type".into()))
    }

    fn downcast_mut<'a>(&self, handle: &'a mut dyn ImageHandle) -> ComputeResult<&'a mut CudaImageHandle> {
        (handle as &mut dyn std::any::Any)
            .downcast_mut::<CudaImageHandle>()
            .ok_or_else(|| ComputeError::OperationFailed("Invalid handle type".into()))
    }
}

impl ProcessingBackend for CudaBackend {
    fn name(&self) -> &'static str { "CUDA" }
    
    fn available_memory(&self) -> u64 { query_available_memory() }
    
    fn limits(&self) -> &GpuLimits { &self.limits }

    fn upload(&self, data: &[f32], width: u32, height: u32, channels: u32) -> ComputeResult<Box<dyn ImageHandle>> {
        let buffer = self.stream.clone_htod(data).map_err(|e| {
            ComputeError::BufferCreation(format!("Upload failed: {e:?}"))
        })?;
        Ok(Box::new(CudaImageHandle { buffer, width, height, channels }))
    }

    fn download(&self, handle: &dyn ImageHandle) -> ComputeResult<Vec<f32>> {
        let h = self.downcast(handle)?;
        self.stream.clone_dtoh(&h.buffer).map_err(|e| {
            ComputeError::OperationFailed(format!("Download failed: {e:?}"))
        })
    }

    fn apply_matrix(&self, handle: &mut dyn ImageHandle, matrix: &[f32; 16]) -> ComputeResult<()> {
        let h = self.downcast_mut(handle)?;
        let total = (h.width * h.height) as i32;
        let c = h.channels as i32;

        // Allocate output
        let out: CudaSlice<f32> = self.stream.alloc_zeros(total as usize * c as usize).map_err(|e| {
            ComputeError::BufferCreation(format!("{e:?}"))
        })?;

        let cfg = self.launch_1d(total as u32);
        let mut builder = self.stream.launch_builder(&self.k_matrix);
        builder.arg(&h.buffer);
        builder.arg(&out);
        builder.arg(&total);
        builder.arg(&c);
        for &v in matrix { builder.arg(&v); }

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("Kernel failed: {e:?}"))
        })?;

        h.buffer = out;
        Ok(())
    }

    fn apply_cdl(&self, handle: &mut dyn ImageHandle, slope: [f32; 3], offset: [f32; 3], power: [f32; 3], sat: f32) -> ComputeResult<()> {
        let h = self.downcast_mut(handle)?;
        let total = (h.width * h.height) as i32;
        let c = h.channels as i32;

        let out: CudaSlice<f32> = self.stream.alloc_zeros(total as usize * c as usize).map_err(|e| {
            ComputeError::BufferCreation(format!("{e:?}"))
        })?;

        let cfg = self.launch_1d(total as u32);
        let mut builder = self.stream.launch_builder(&self.k_cdl);
        builder.arg(&h.buffer);
        builder.arg(&out);
        builder.arg(&total);
        builder.arg(&c);
        builder.arg(&slope[0]); builder.arg(&slope[1]); builder.arg(&slope[2]);
        builder.arg(&offset[0]); builder.arg(&offset[1]); builder.arg(&offset[2]);
        builder.arg(&power[0]); builder.arg(&power[1]); builder.arg(&power[2]);
        builder.arg(&sat);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("Kernel failed: {e:?}"))
        })?;

        h.buffer = out;
        Ok(())
    }

    fn apply_lut1d(&self, handle: &mut dyn ImageHandle, lut: &[f32], channels: u32) -> ComputeResult<()> {
        let h = self.downcast_mut(handle)?;
        let total = (h.width * h.height) as i32;
        let c = h.channels as i32;
        let lut_size = (lut.len() / channels as usize) as i32;

        let lut_buf = self.stream.clone_htod(lut).map_err(|e| {
            ComputeError::BufferCreation(format!("{e:?}"))
        })?;
        let out: CudaSlice<f32> = self.stream.alloc_zeros(total as usize * c as usize).map_err(|e| {
            ComputeError::BufferCreation(format!("{e:?}"))
        })?;

        let cfg = self.launch_1d(total as u32);
        let mut builder = self.stream.launch_builder(&self.k_lut1d);
        builder.arg(&h.buffer);
        builder.arg(&out);
        builder.arg(&lut_buf);
        builder.arg(&total);
        builder.arg(&c);
        builder.arg(&lut_size);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("Kernel failed: {e:?}"))
        })?;

        h.buffer = out;
        Ok(())
    }

    fn apply_lut3d(&self, handle: &mut dyn ImageHandle, lut: &[f32], size: u32) -> ComputeResult<()> {
        let h = self.downcast_mut(handle)?;
        let total = (h.width * h.height) as i32;
        let c = h.channels as i32;
        let s = size as i32;

        let lut_buf = self.stream.clone_htod(lut).map_err(|e| {
            ComputeError::BufferCreation(format!("{e:?}"))
        })?;
        let out: CudaSlice<f32> = self.stream.alloc_zeros(total as usize * c as usize).map_err(|e| {
            ComputeError::BufferCreation(format!("{e:?}"))
        })?;

        let cfg = self.launch_1d(total as u32);
        let mut builder = self.stream.launch_builder(&self.k_lut3d);
        builder.arg(&h.buffer);
        builder.arg(&out);
        builder.arg(&lut_buf);
        builder.arg(&total);
        builder.arg(&c);
        builder.arg(&s);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("Kernel failed: {e:?}"))
        })?;

        h.buffer = out;
        Ok(())
    }

    fn resize(&self, handle: &dyn ImageHandle, width: u32, height: u32, _filter: u32) -> ComputeResult<Box<dyn ImageHandle>> {
        let h = self.downcast(handle)?;
        let sw = h.width as i32;
        let sh = h.height as i32;
        let dw = width as i32;
        let dh = height as i32;
        let c = h.channels as i32;

        let out: CudaSlice<f32> = self.stream.alloc_zeros((dw * dh * c) as usize).map_err(|e| {
            ComputeError::BufferCreation(format!("{e:?}"))
        })?;

        let cfg = self.launch_2d(width, height);
        let mut builder = self.stream.launch_builder(&self.k_resize);
        builder.arg(&h.buffer);
        builder.arg(&out);
        builder.arg(&sw);
        builder.arg(&sh);
        builder.arg(&dw);
        builder.arg(&dh);
        builder.arg(&c);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("Kernel failed: {e:?}"))
        })?;

        Ok(Box::new(CudaImageHandle { buffer: out, width, height, channels: h.channels }))
    }

    fn blur(&self, handle: &mut dyn ImageHandle, radius: f32) -> ComputeResult<()> {
        let h = self.downcast_mut(handle)?;
        let w = h.width as i32;
        let hh = h.height as i32;
        let c = h.channels as i32;
        let r = radius.ceil() as i32;
        let total = w * hh;

        // Generate Gaussian kernel
        let k_size = (r * 2 + 1) as usize;
        let sigma = radius / 3.0;
        let mut kernel = vec![0.0f32; k_size];
        let mut sum = 0.0f32;
        for i in 0..k_size {
            let x = (i as i32 - r) as f32;
            let g = (-x * x / (2.0 * sigma * sigma)).exp();
            kernel[i] = g;
            sum += g;
        }
        for k in &mut kernel { *k /= sum; }

        let kernel_buf = self.stream.clone_htod(&kernel).map_err(|e| {
            ComputeError::BufferCreation(format!("{e:?}"))
        })?;

        // Horizontal pass
        let temp: CudaSlice<f32> = self.stream.alloc_zeros((total * c) as usize).map_err(|e| {
            ComputeError::BufferCreation(format!("{e:?}"))
        })?;

        let cfg = self.launch_1d(total as u32);
        let mut builder = self.stream.launch_builder(&self.k_blur_h);
        builder.arg(&h.buffer);
        builder.arg(&temp);
        builder.arg(&kernel_buf);
        builder.arg(&w);
        builder.arg(&hh);
        builder.arg(&c);
        builder.arg(&r);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("Blur H failed: {e:?}"))
        })?;

        // Vertical pass
        let out: CudaSlice<f32> = self.stream.alloc_zeros((total * c) as usize).map_err(|e| {
            ComputeError::BufferCreation(format!("{e:?}"))
        })?;

        let mut builder = self.stream.launch_builder(&self.k_blur_v);
        builder.arg(&temp);
        builder.arg(&out);
        builder.arg(&kernel_buf);
        builder.arg(&w);
        builder.arg(&hh);
        builder.arg(&c);
        builder.arg(&r);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("Blur V failed: {e:?}"))
        })?;

        h.buffer = out;
        Ok(())
    }

    fn composite_over(&self, fg: &dyn ImageHandle, bg: &mut dyn ImageHandle) -> ComputeResult<()> {
        let fg_h = self.downcast(fg)?;
        let bg_h = self.downcast_mut(bg)?;
        let total = (bg_h.width * bg_h.height) as i32;
        let c = bg_h.channels as i32;

        let cfg = self.launch_1d(total as u32);
        let mut builder = self.stream.launch_builder(&self.k_composite);
        builder.arg(&fg_h.buffer);
        builder.arg(&bg_h.buffer);
        builder.arg(&total);
        builder.arg(&c);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("Composite failed: {e:?}"))
        })?;

        Ok(())
    }

    fn blend(&self, fg: &dyn ImageHandle, bg: &mut dyn ImageHandle, mode: BlendMode, opacity: f32) -> ComputeResult<()> {
        let fg_h = self.downcast(fg)?;
        let bg_h = self.downcast_mut(bg)?;
        let total = (bg_h.width * bg_h.height) as i32;
        let c = bg_h.channels as i32;
        let m = mode as i32;

        let cfg = self.launch_1d(total as u32);
        let mut builder = self.stream.launch_builder(&self.k_blend);
        builder.arg(&fg_h.buffer);
        builder.arg(&bg_h.buffer);
        builder.arg(&total);
        builder.arg(&c);
        builder.arg(&m);
        builder.arg(&opacity);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("Blend failed: {e:?}"))
        })?;

        Ok(())
    }

    fn crop(&self, handle: &dyn ImageHandle, x: u32, y: u32, w: u32, h: u32) -> ComputeResult<Box<dyn ImageHandle>> {
        let src = self.downcast(handle)?;
        let c = src.channels as i32;

        let out: CudaSlice<f32> = self.stream.alloc_zeros((w * h * src.channels) as usize).map_err(|e| {
            ComputeError::BufferCreation(format!("{e:?}"))
        })?;

        let cfg = self.launch_2d(w, h);
        let mut builder = self.stream.launch_builder(&self.k_crop);
        builder.arg(&src.buffer);
        builder.arg(&out);
        builder.arg(&(src.width as i32));
        builder.arg(&(w as i32));
        builder.arg(&(h as i32));
        builder.arg(&(x as i32));
        builder.arg(&(y as i32));
        builder.arg(&c);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("Crop failed: {e:?}"))
        })?;

        Ok(Box::new(CudaImageHandle { buffer: out, width: w, height: h, channels: src.channels }))
    }

    fn flip_h(&self, handle: &mut dyn ImageHandle) -> ComputeResult<()> {
        let h = self.downcast_mut(handle)?;
        let w = h.width as i32;
        let hh = h.height as i32;
        let c = h.channels as i32;

        let out: CudaSlice<f32> = self.stream.alloc_zeros((w * hh * c) as usize).map_err(|e| {
            ComputeError::BufferCreation(format!("{e:?}"))
        })?;

        let cfg = self.launch_2d(h.width, h.height);
        let mut builder = self.stream.launch_builder(&self.k_flip_h);
        builder.arg(&h.buffer);
        builder.arg(&out);
        builder.arg(&w);
        builder.arg(&hh);
        builder.arg(&c);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("FlipH failed: {e:?}"))
        })?;

        h.buffer = out;
        Ok(())
    }

    fn flip_v(&self, handle: &mut dyn ImageHandle) -> ComputeResult<()> {
        let h = self.downcast_mut(handle)?;
        let w = h.width as i32;
        let hh = h.height as i32;
        let c = h.channels as i32;

        let out: CudaSlice<f32> = self.stream.alloc_zeros((w * hh * c) as usize).map_err(|e| {
            ComputeError::BufferCreation(format!("{e:?}"))
        })?;

        let cfg = self.launch_2d(h.width, h.height);
        let mut builder = self.stream.launch_builder(&self.k_flip_v);
        builder.arg(&h.buffer);
        builder.arg(&out);
        builder.arg(&w);
        builder.arg(&hh);
        builder.arg(&c);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("FlipV failed: {e:?}"))
        })?;

        h.buffer = out;
        Ok(())
    }

    fn rotate_90(&self, handle: &dyn ImageHandle, n: u32) -> ComputeResult<Box<dyn ImageHandle>> {
        let src = self.downcast(handle)?;
        let n_mod = n % 4;
        
        let (dst_w, dst_h) = if n_mod % 2 == 1 {
            (src.height, src.width)
        } else {
            (src.width, src.height)
        };

        let c = src.channels as i32;
        let out: CudaSlice<f32> = self.stream.alloc_zeros((dst_w * dst_h * src.channels) as usize).map_err(|e| {
            ComputeError::BufferCreation(format!("{e:?}"))
        })?;

        let cfg = self.launch_2d(dst_w, dst_h);
        let mut builder = self.stream.launch_builder(&self.k_rotate);
        builder.arg(&src.buffer);
        builder.arg(&out);
        builder.arg(&(src.width as i32));
        builder.arg(&(src.height as i32));
        builder.arg(&(dst_w as i32));
        builder.arg(&(dst_h as i32));
        builder.arg(&c);
        builder.arg(&(n_mod as i32));

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("Rotate failed: {e:?}"))
        })?;

        Ok(Box::new(CudaImageHandle { buffer: out, width: dst_w, height: dst_h, channels: src.channels }))
    }
}

// =============================================================================
// VRAM Detection
// =============================================================================

fn query_available_memory() -> u64 {
    use cudarc::driver::sys as cuda_sys;

    let mut free: usize = 0;
    let mut total: usize = 0;

    #[allow(unsafe_code)]
    let result = unsafe {
        cuda_sys::cuMemGetInfo_v2(&raw mut free, &raw mut total)
    };

    if result == cuda_sys::CUresult::CUDA_SUCCESS {
        // 60% safety margin
        (free as f64 * 0.6) as u64
    } else {
        4 * 1024 * 1024 * 1024 // 4GB fallback
    }
}
