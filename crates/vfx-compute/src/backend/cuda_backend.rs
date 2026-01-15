//! CUDA GPU compute backend for color/image processing.
//!
//! Requires the `cuda` feature and NVIDIA GPU with CUDA support.

use std::sync::Arc;

use cudarc::driver::{CudaContext, CudaFunction, CudaModule, CudaSlice, CudaStream, LaunchConfig, PushKernelArg};

use super::gpu_primitives::{GpuPrimitives, ImageHandle, AsAny};
use super::tiling::GpuLimits;

use crate::{ComputeError, ComputeResult};

// =============================================================================
// CUDA Kernel Source
// =============================================================================

/// CUDA PTX kernel source for color/image operations.
///
/// Compiled at runtime via NVRTC. Contains:
/// - `color_matrix`: 4x4 matrix color transform
/// - `cdl_kernel`: ASC CDL (slope/offset/power/sat)
/// - `lut1d_kernel`: 1D LUT interpolation per channel
/// - `lut3d_kernel`: 3D LUT trilinear interpolation
/// - `lut3d_tetra_kernel`: 3D LUT tetrahedral interpolation (higher quality)
/// - `resize_kernel`: Bilinear resize
/// - `blur_h_kernel`/`blur_v_kernel`: Separable Gaussian blur
/// - `flip_h_kernel`/`flip_v_kernel`: Horizontal/vertical flip
/// - `rotate90_kernel`: 90° rotations (n * 90°)
/// - `composite_over_kernel`: Alpha-over compositing
/// - `blend_kernel`: Photoshop blend modes
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

    // Saturation (Rec.709 luma coefficients - see vfx_core::pixel::REC709_LUMA_*)
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
// 3D LUT Tetrahedral Interpolation (higher quality)
// ============================================================================
__global__ void lut3d_tetra_kernel(
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

    int r0 = min((int)r, s - 2); r0 = max(r0, 0);
    int g0 = min((int)g, s - 2); g0 = max(g0, 0);
    int b0 = min((int)b, s - 2); b0 = max(b0, 0);
    int r1 = r0 + 1;
    int g1 = g0 + 1;
    int b1 = b0 + 1;

    float fr = r - (float)r0;
    float fg = g - (float)g0;
    float fb = b - (float)b0;

    for (int ch = 0; ch < 3; ch++) {
        // Fetch all 8 corners
        float c000 = lut3d_sample(lut, r0, g0, b0, ch, s);
        float c100 = lut3d_sample(lut, r1, g0, b0, ch, s);
        float c010 = lut3d_sample(lut, r0, g1, b0, ch, s);
        float c110 = lut3d_sample(lut, r1, g1, b0, ch, s);
        float c001 = lut3d_sample(lut, r0, g0, b1, ch, s);
        float c101 = lut3d_sample(lut, r1, g0, b1, ch, s);
        float c011 = lut3d_sample(lut, r0, g1, b1, ch, s);
        float c111 = lut3d_sample(lut, r1, g1, b1, ch, s);

        // Tetrahedral interpolation: 6 tetrahedra based on fr,fg,fb ordering
        float result;
        if (fr > fg) {
            if (fg > fb) {
                // fr > fg > fb: tetrahedron (0,0,0)-(1,0,0)-(1,1,0)-(1,1,1)
                result = c000 + fr*(c100-c000) + fg*(c110-c100) + fb*(c111-c110);
            } else if (fr > fb) {
                // fr > fb > fg: tetrahedron (0,0,0)-(1,0,0)-(1,0,1)-(1,1,1)
                result = c000 + fr*(c100-c000) + fb*(c101-c100) + fg*(c111-c101);
            } else {
                // fb > fr > fg: tetrahedron (0,0,0)-(0,0,1)-(1,0,1)-(1,1,1)
                result = c000 + fb*(c001-c000) + fr*(c101-c001) + fg*(c111-c101);
            }
        } else {
            if (fr > fb) {
                // fg > fr > fb: tetrahedron (0,0,0)-(0,1,0)-(1,1,0)-(1,1,1)
                result = c000 + fg*(c010-c000) + fr*(c110-c010) + fb*(c111-c110);
            } else if (fg > fb) {
                // fg > fb > fr: tetrahedron (0,0,0)-(0,1,0)-(0,1,1)-(1,1,1)
                result = c000 + fg*(c010-c000) + fb*(c011-c010) + fr*(c111-c011);
            } else {
                // fb > fg > fr: tetrahedron (0,0,0)-(0,0,1)-(0,1,1)-(1,1,1)
                result = c000 + fb*(c001-c000) + fg*(c011-c001) + fr*(c111-c011);
            }
        }
        dst[base + ch] = result;
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

// ============================================================================
// Hue Curves (Hue vs Hue/Sat/Lum)
// ============================================================================

// RGB to HSL conversion
__device__ void rgb_to_hsl(float r, float g, float b, float* h, float* s, float* l) {
    float mx = fmaxf(fmaxf(r, g), b);
    float mn = fminf(fminf(r, g), b);
    *l = (mx + mn) * 0.5f;

    if (mx - mn < 1e-6f) {
        *h = 0.0f;
        *s = 0.0f;
        return;
    }

    float d = mx - mn;
    *s = (*l > 0.5f) ? d / (2.0f - mx - mn) : d / (mx + mn);

    if (mx == r) {
        *h = (g - b) / d;
        if (g < b) *h += 6.0f;
    } else if (mx == g) {
        *h = (b - r) / d + 2.0f;
    } else {
        *h = (r - g) / d + 4.0f;
    }
    *h /= 6.0f;
}

__device__ float hue_to_rgb(float p, float q, float t) {
    if (t < 0.0f) t += 1.0f;
    if (t > 1.0f) t -= 1.0f;
    if (t < 1.0f/6.0f) return p + (q - p) * 6.0f * t;
    if (t < 0.5f) return q;
    if (t < 2.0f/3.0f) return p + (q - p) * (2.0f/3.0f - t) * 6.0f;
    return p;
}

__device__ void hsl_to_rgb(float h, float s, float l, float* r, float* g, float* b) {
    if (s < 1e-6f) {
        *r = *g = *b = l;
        return;
    }
    float q = (l < 0.5f) ? l * (1.0f + s) : l + s - l * s;
    float p = 2.0f * l - q;
    *r = hue_to_rgb(p, q, h + 1.0f/3.0f);
    *g = hue_to_rgb(p, q, h);
    *b = hue_to_rgb(p, q, h - 1.0f/3.0f);
}

// Sample 1D LUT with linear interpolation (hue wraps around)
__device__ float sample_hue_lut(const float* lut, int lut_size, float hue) {
    float h = hue - floorf(hue);  // wrap to 0-1
    float pos = h * (float)(lut_size - 1);
    int i0 = (int)pos;
    int i1 = (i0 + 1) % lut_size;
    float f = pos - (float)i0;
    return lut[i0] + f * (lut[i1] - lut[i0]);
}

__global__ void hue_curves_kernel(
    const float* __restrict__ src,
    float* __restrict__ dst,
    const float* __restrict__ hue_vs_hue,   // hue shift LUT
    const float* __restrict__ hue_vs_sat,   // sat multiplier LUT  
    const float* __restrict__ hue_vs_lum,   // lum offset LUT
    int total, int c, int lut_size
) {
    int px = blockIdx.x * blockDim.x + threadIdx.x;
    if (px >= total) return;

    int base = px * c;
    float r = src[base];
    float g = src[base + 1];
    float b = src[base + 2];

    // RGB to HSL
    float h, s, l;
    rgb_to_hsl(r, g, b, &h, &s, &l);

    // Apply curves
    float hue_shift = sample_hue_lut(hue_vs_hue, lut_size, h);
    float sat_mult = sample_hue_lut(hue_vs_sat, lut_size, h);
    float lum_offset = sample_hue_lut(hue_vs_lum, lut_size, h);

    float new_h = h + hue_shift;
    new_h = new_h - floorf(new_h);  // wrap
    float new_s = fminf(fmaxf(s * sat_mult, 0.0f), 1.0f);
    float new_l = fminf(fmaxf(l + lum_offset, 0.0f), 1.0f);

    // HSL to RGB
    hsl_to_rgb(new_h, new_s, new_l, &r, &g, &b);

    dst[base] = r;
    dst[base + 1] = g;
    dst[base + 2] = b;
    if (c >= 4) dst[base + 3] = src[base + 3];
}

} // extern "C"
"#;

// =============================================================================
// CUDA Handle
// =============================================================================

/// CUDA buffer handle for image data.
///
/// Holds a GPU-allocated slice of f32 values representing an image.
/// Memory is managed by cudarc and freed when this handle is dropped.
pub struct CudaImage {
    /// Device-side buffer containing pixel data (planar RGB/RGBA).
    buffer: CudaSlice<f32>,
    /// Image width in pixels.
    width: u32,
    /// Image height in pixels.
    height: u32,
    /// Number of channels (3=RGB, 4=RGBA).
    channels: u32,
}


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
    /// CUDA context - kept alive for the lifetime of primitives.
    #[allow(dead_code)]
    ctx: Arc<CudaContext>,
    stream: Arc<CudaStream>,
    /// Compiled CUDA module - kept alive for kernel lifetime.
    #[allow(dead_code)]
    module: Arc<CudaModule>,
    k_matrix: CudaFunction,
    k_cdl: CudaFunction,
    k_lut1d: CudaFunction,
    k_lut3d: CudaFunction,
    k_lut3d_tetra: CudaFunction,
    k_resize: CudaFunction,
    k_blur_h: CudaFunction,
    k_blur_v: CudaFunction,
    // Transform ops
    k_flip_h: CudaFunction,
    k_flip_v: CudaFunction,
    k_rotate90: CudaFunction,
    // Composite ops
    k_composite_over: CudaFunction,
    k_blend: CudaFunction,
    // Grading ops
    k_hue_curves: CudaFunction,
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

        // Load all kernel functions
        let load_err = |name: &str, e: cudarc::driver::result::DriverError| {
            ComputeError::ShaderCompilation(format!("Failed to load {name}: {e:?}"))
        };

        let k_matrix = module.load_function("color_matrix").map_err(|e| load_err("color_matrix", e))?;
        let k_cdl = module.load_function("cdl_kernel").map_err(|e| load_err("cdl_kernel", e))?;
        let k_lut1d = module.load_function("lut1d_kernel").map_err(|e| load_err("lut1d_kernel", e))?;
        let k_lut3d = module.load_function("lut3d_kernel").map_err(|e| load_err("lut3d_kernel", e))?;
        let k_lut3d_tetra = module.load_function("lut3d_tetra_kernel").map_err(|e| load_err("lut3d_tetra_kernel", e))?;
        let k_resize = module.load_function("resize_kernel").map_err(|e| load_err("resize_kernel", e))?;
        let k_blur_h = module.load_function("blur_h_kernel").map_err(|e| load_err("blur_h_kernel", e))?;
        let k_blur_v = module.load_function("blur_v_kernel").map_err(|e| load_err("blur_v_kernel", e))?;
        let k_flip_h = module.load_function("flip_h_kernel").map_err(|e| load_err("flip_h_kernel", e))?;
        let k_flip_v = module.load_function("flip_v_kernel").map_err(|e| load_err("flip_v_kernel", e))?;
        let k_rotate90 = module.load_function("rotate90_kernel").map_err(|e| load_err("rotate90_kernel", e))?;
        let k_composite_over = module.load_function("composite_over_kernel").map_err(|e| load_err("composite_over_kernel", e))?;
        let k_blend = module.load_function("blend_kernel").map_err(|e| load_err("blend_kernel", e))?;
        let k_hue_curves = module.load_function("hue_curves_kernel").map_err(|e| load_err("hue_curves_kernel", e))?;

        Ok(Self {
            ctx,
            stream,
            module,
            k_matrix,
            k_cdl,
            k_lut1d,
            k_lut3d,
            k_lut3d_tetra,
            k_resize,
            k_blur_h,
            k_blur_v,
            k_flip_h,
            k_flip_v,
            k_rotate90,
            k_composite_over,
            k_blend,
            k_hue_curves,
            limits,
        })
    }

    /// Check if CUDA is available.
    pub fn is_available() -> bool {
        CudaContext::new(0).is_ok()
    }

    /// Create launch config for 1D kernel (pixel-parallel).
    ///
    /// Uses 256 threads per block for simple per-pixel operations.
    fn launch_1d(&self, total: u32) -> LaunchConfig {
        let block = 256u32;
        let grid = total.div_ceil(block);
        LaunchConfig {
            block_dim: (block, 1, 1),
            grid_dim: (grid, 1, 1),
            shared_mem_bytes: 0,
        }
    }

    /// Create launch config for 2D kernel (image-parallel).
    ///
    /// Uses 16x16 thread blocks for spatial operations (blur, resize).
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
        // Pass matrix elements individually
        builder.arg(&matrix[0]); builder.arg(&matrix[1]); builder.arg(&matrix[2]); builder.arg(&matrix[3]);
        builder.arg(&matrix[4]); builder.arg(&matrix[5]); builder.arg(&matrix[6]); builder.arg(&matrix[7]);
        builder.arg(&matrix[8]); builder.arg(&matrix[9]); builder.arg(&matrix[10]); builder.arg(&matrix[11]);
        builder.arg(&matrix[12]); builder.arg(&matrix[13]); builder.arg(&matrix[14]); builder.arg(&matrix[15]);

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

    fn exec_lut3d_tetrahedral(&self, src: &Self::Handle, dst: &mut Self::Handle,
                              lut: &[f32], size: u32) -> ComputeResult<()> {
        let total = (src.width * src.height) as i32;
        let c = src.channels as i32;
        let s = size as i32;

        let lut_buf = self.stream.clone_htod(lut).map_err(|e| {
            ComputeError::BufferCreation(format!("LUT upload failed: {e:?}"))
        })?;

        let cfg = self.launch_1d(total as u32);
        let mut builder = self.stream.launch_builder(&self.k_lut3d_tetra);
        builder.arg(&src.buffer);
        builder.arg(&dst.buffer);
        builder.arg(&lut_buf);
        builder.arg(&total);
        builder.arg(&c);
        builder.arg(&s);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("LUT3D tetrahedral failed: {e:?}"))
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

    fn exec_hue_curves(&self, src: &Self::Handle, dst: &mut Self::Handle,
                       hue_vs_hue: &[f32], hue_vs_sat: &[f32], hue_vs_lum: &[f32],
                       lut_size: u32) -> ComputeResult<()> {
        let total = (src.width * src.height) as i32;
        let c = src.channels as i32;
        let lut_sz = lut_size as i32;

        // Upload LUTs to GPU
        let lut_hue = self.stream.clone_htod(hue_vs_hue).map_err(|e| {
            ComputeError::BufferCreation(format!("HueCurve LUT upload failed: {e:?}"))
        })?;
        let lut_sat = self.stream.clone_htod(hue_vs_sat).map_err(|e| {
            ComputeError::BufferCreation(format!("HueCurve LUT upload failed: {e:?}"))
        })?;
        let lut_lum = self.stream.clone_htod(hue_vs_lum).map_err(|e| {
            ComputeError::BufferCreation(format!("HueCurve LUT upload failed: {e:?}"))
        })?;

        let cfg = self.launch_1d(total as u32);
        let mut builder = self.stream.launch_builder(&self.k_hue_curves);
        builder.arg(&src.buffer);
        builder.arg(&dst.buffer);
        builder.arg(&lut_hue);
        builder.arg(&lut_sat);
        builder.arg(&lut_lum);
        builder.arg(&total);
        builder.arg(&c);
        builder.arg(&lut_sz);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("HueCurves failed: {e:?}"))
        })?;

        Ok(())
    }

    fn exec_flip_h(&self, handle: &mut Self::Handle) -> ComputeResult<()> {
        let (w, h, c) = (handle.width as i32, handle.height as i32, handle.channels as i32);
        
        let dst: CudaSlice<f32> = self.stream.alloc_zeros((w * h * c) as usize).map_err(|e| {
            ComputeError::BufferCreation(format!("{e:?}"))
        })?;

        let cfg = self.launch_2d(handle.width, handle.height);
        let mut builder = self.stream.launch_builder(&self.k_flip_h);
        builder.arg(&handle.buffer);
        builder.arg(&dst);
        builder.arg(&w); builder.arg(&h); builder.arg(&c);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("Flip H failed: {e:?}"))
        })?;

        handle.buffer = dst;
        Ok(())
    }

    fn exec_flip_v(&self, handle: &mut Self::Handle) -> ComputeResult<()> {
        let (w, h, c) = (handle.width as i32, handle.height as i32, handle.channels as i32);
        
        let dst: CudaSlice<f32> = self.stream.alloc_zeros((w * h * c) as usize).map_err(|e| {
            ComputeError::BufferCreation(format!("{e:?}"))
        })?;

        let cfg = self.launch_2d(handle.width, handle.height);
        let mut builder = self.stream.launch_builder(&self.k_flip_v);
        builder.arg(&handle.buffer);
        builder.arg(&dst);
        builder.arg(&w); builder.arg(&h); builder.arg(&c);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("Flip V failed: {e:?}"))
        })?;

        handle.buffer = dst;
        Ok(())
    }

    fn exec_rotate_90(&self, src: &Self::Handle, n: u32) -> ComputeResult<Self::Handle> {
        let n = n % 4;
        if n == 0 {
            // No rotation - copy
            let data = self.download(src)?;
            return self.upload(&data, src.width, src.height, src.channels);
        }

        let (sw, sh, c) = (src.width as i32, src.height as i32, src.channels as i32);
        let (dw, dh) = if n % 2 == 1 { (sh, sw) } else { (sw, sh) };
        let n_i32 = n as i32;
        
        let dst: CudaSlice<f32> = self.stream.alloc_zeros((dw * dh * c) as usize).map_err(|e| {
            ComputeError::BufferCreation(format!("{e:?}"))
        })?;

        let cfg = self.launch_2d(dw as u32, dh as u32);
        let mut builder = self.stream.launch_builder(&self.k_rotate90);
        builder.arg(&src.buffer);
        builder.arg(&dst);
        builder.arg(&sw); builder.arg(&sh);
        builder.arg(&dw); builder.arg(&dh);
        builder.arg(&c); builder.arg(&n_i32);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("Rotate90 failed: {e:?}"))
        })?;

        Ok(CudaImage {
            buffer: dst,
            width: dw as u32,
            height: dh as u32,
            channels: src.channels,
        })
    }

    fn exec_composite_over(&self, fg: &Self::Handle, bg: &mut Self::Handle) -> ComputeResult<()> {
        let total = (fg.width * fg.height) as i32;
        let c = fg.channels as i32;

        let cfg = self.launch_1d(total as u32);
        let mut builder = self.stream.launch_builder(&self.k_composite_over);
        builder.arg(&fg.buffer);
        builder.arg(&bg.buffer);
        builder.arg(&total); builder.arg(&c);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("Composite Over failed: {e:?}"))
        })?;
        Ok(())
    }

    fn exec_blend(&self, fg: &Self::Handle, bg: &mut Self::Handle, mode: u32, opacity: f32) -> ComputeResult<()> {
        let total = (fg.width * fg.height) as i32;
        let c = fg.channels as i32;
        let m = mode as i32;

        let cfg = self.launch_1d(total as u32);
        let mut builder = self.stream.launch_builder(&self.k_blend);
        builder.arg(&fg.buffer);
        builder.arg(&bg.buffer);
        builder.arg(&total); builder.arg(&c);
        builder.arg(&m); builder.arg(&opacity);

        #[allow(unsafe_code)]
        unsafe { builder.launch(cfg) }.map_err(|e| {
            ComputeError::OperationFailed(format!("Blend failed: {e:?}"))
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
// VRAM Detection
// =============================================================================

/// Query available VRAM from CUDA driver.
///
/// Returns 60% of free memory to leave headroom for other allocations.
/// Falls back to 4GB if query fails.
fn query_available_memory() -> u64 {
    use cudarc::driver::sys as cuda_sys;

    let mut free: usize = 0;
    let mut total: usize = 0;

    #[allow(unsafe_code)]
    let result = unsafe {
        cuda_sys::cuMemGetInfo_v2(&raw mut free, &raw mut total)
    };

    if result == cuda_sys::CUresult::CUDA_SUCCESS {
        // 60% safety margin for driver overhead and other allocations
        (free as f64 * 0.6) as u64
    } else {
        4 * 1024 * 1024 * 1024 // 4GB fallback
    }
}
