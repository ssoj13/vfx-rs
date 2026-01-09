//! SIMD-optimized color processing operations.
//!
//! Auto-selects: AVX2 > SSE4.1 > NEON > scalar.

#![allow(unsafe_op_in_unsafe_fn)]

/// SIMD capability level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimdLevel {
    Scalar,
    Sse41,
    Avx2,
    #[allow(dead_code)]
    Neon,
}

impl SimdLevel {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub fn detect() -> Self {
        if is_x86_feature_detected!("avx2") {
            SimdLevel::Avx2
        } else if is_x86_feature_detected!("sse4.1") {
            SimdLevel::Sse41
        } else {
            SimdLevel::Scalar
        }
    }

    #[cfg(target_arch = "aarch64")]
    pub fn detect() -> Self {
        SimdLevel::Neon
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
    pub fn detect() -> Self {
        SimdLevel::Scalar
    }
}

/// Matrix multiply: out = M * rgb (row-major matrix)
/// out.r = m00*r + m01*g + m02*b + m03
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub fn matrix_multiply_simd(pixels: &mut [[f32; 3]], matrix: &[f32; 16]) {
    let level = SimdLevel::detect();
    match level {
        SimdLevel::Avx2 | SimdLevel::Sse41 => unsafe { matrix_sse41(pixels, matrix) },
        _ => matrix_multiply_scalar(pixels, matrix),
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub fn range_clamp_simd(pixels: &mut [[f32; 3]], min: f32, max: f32) {
    let level = SimdLevel::detect();
    match level {
        SimdLevel::Avx2 | SimdLevel::Sse41 => unsafe { clamp_sse41(pixels, min, max) },
        _ => range_clamp_scalar(pixels, min, max),
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse4.1")]
unsafe fn matrix_sse41(pixels: &mut [[f32; 3]], matrix: &[f32; 16]) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    // Load matrix rows
    let row0 = _mm_loadu_ps(matrix.as_ptr()); // [m00, m01, m02, m03]
    let row1 = _mm_loadu_ps(matrix.as_ptr().add(4)); // [m10, m11, m12, m13]
    let row2 = _mm_loadu_ps(matrix.as_ptr().add(8)); // [m20, m21, m22, m23]

    for pixel in pixels.iter_mut() {
        // Load [r, g, b, 1.0] for dot product with offset
        let rgba = _mm_set_ps(1.0, pixel[2], pixel[1], pixel[0]);

        // Dot product: sum of element-wise multiply
        // _mm_dp_ps mask 0xF1 = multiply all 4, store in element 0
        let out_r = _mm_dp_ps(rgba, row0, 0xF1);
        let out_g = _mm_dp_ps(rgba, row1, 0xF1);
        let out_b = _mm_dp_ps(rgba, row2, 0xF1);

        pixel[0] = _mm_cvtss_f32(out_r);
        pixel[1] = _mm_cvtss_f32(out_g);
        pixel[2] = _mm_cvtss_f32(out_b);
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse4.1")]
unsafe fn clamp_sse41(pixels: &mut [[f32; 3]], min: f32, max: f32) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    let vmin = _mm_set1_ps(min);
    let vmax = _mm_set1_ps(max);

    for pixel in pixels.iter_mut() {
        let mut v = _mm_set_ps(0.0, pixel[2], pixel[1], pixel[0]);
        v = _mm_max_ps(v, vmin);
        v = _mm_min_ps(v, vmax);

        pixel[0] = _mm_cvtss_f32(v);
        pixel[1] = _mm_cvtss_f32(_mm_shuffle_ps(v, v, 1));
        pixel[2] = _mm_cvtss_f32(_mm_shuffle_ps(v, v, 2));
    }
}

// ARM NEON
#[cfg(target_arch = "aarch64")]
pub fn matrix_multiply_simd(pixels: &mut [[f32; 3]], matrix: &[f32; 16]) {
    // NEON doesn't have efficient horizontal sum, use scalar
    matrix_multiply_scalar(pixels, matrix);
}

#[cfg(target_arch = "aarch64")]
pub fn range_clamp_simd(pixels: &mut [[f32; 3]], min: f32, max: f32) {
    use std::arch::aarch64::*;

    unsafe {
        let vmin = vdupq_n_f32(min);
        let vmax = vdupq_n_f32(max);

        for pixel in pixels.iter_mut() {
            let mut v = vld1q_f32([pixel[0], pixel[1], pixel[2], 0.0].as_ptr());
            v = vmaxq_f32(v, vmin);
            v = vminq_f32(v, vmax);

            let mut out = [0.0f32; 4];
            vst1q_f32(out.as_mut_ptr(), v);
            pixel[0] = out[0];
            pixel[1] = out[1];
            pixel[2] = out[2];
        }
    }
}

// Scalar fallback
#[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
pub fn matrix_multiply_simd(pixels: &mut [[f32; 3]], m: &[f32; 16]) {
    matrix_multiply_scalar(pixels, m);
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
pub fn range_clamp_simd(pixels: &mut [[f32; 3]], min: f32, max: f32) {
    range_clamp_scalar(pixels, min, max);
}

/// Scalar matrix multiply
pub fn matrix_multiply_scalar(pixels: &mut [[f32; 3]], m: &[f32; 16]) {
    for pixel in pixels.iter_mut() {
        let (r, g, b) = (pixel[0], pixel[1], pixel[2]);
        pixel[0] = m[0] * r + m[1] * g + m[2] * b + m[3];
        pixel[1] = m[4] * r + m[5] * g + m[6] * b + m[7];
        pixel[2] = m[8] * r + m[9] * g + m[10] * b + m[11];
    }
}

/// Scalar range clamp
pub fn range_clamp_scalar(pixels: &mut [[f32; 3]], min: f32, max: f32) {
    for pixel in pixels.iter_mut() {
        pixel[0] = pixel[0].clamp(min, max);
        pixel[1] = pixel[1].clamp(min, max);
        pixel[2] = pixel[2].clamp(min, max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_simd_level() {
        let level = SimdLevel::detect();
        println!("SIMD: {:?}", level);
    }

    #[test]
    fn matrix_identity() {
        #[rustfmt::skip]
        let identity = [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        let mut pixels = [[0.5, 0.3, 0.8]];
        let expected = pixels;

        matrix_multiply_simd(&mut pixels, &identity);
        assert!((pixels[0][0] - expected[0][0]).abs() < 1e-6);
        assert!((pixels[0][1] - expected[0][1]).abs() < 1e-6);
        assert!((pixels[0][2] - expected[0][2]).abs() < 1e-6);
    }

    #[test]
    fn matrix_scale() {
        #[rustfmt::skip]
        let scale = [
            2.0, 0.0, 0.0, 0.0,
            0.0, 2.0, 0.0, 0.0,
            0.0, 0.0, 2.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        let mut pixels = [[0.5, 0.3, 0.2]];
        matrix_multiply_simd(&mut pixels, &scale);
        assert!((pixels[0][0] - 1.0).abs() < 1e-6);
        assert!((pixels[0][1] - 0.6).abs() < 1e-6);
        assert!((pixels[0][2] - 0.4).abs() < 1e-6);
    }

    #[test]
    fn matrix_offset() {
        #[rustfmt::skip]
        let offset = [
            1.0, 0.0, 0.0, 0.1,
            0.0, 1.0, 0.0, 0.2,
            0.0, 0.0, 1.0, 0.3,
            0.0, 0.0, 0.0, 1.0,
        ];
        let mut pixels = [[0.0, 0.0, 0.0]];
        matrix_multiply_simd(&mut pixels, &offset);
        assert!((pixels[0][0] - 0.1).abs() < 1e-6);
        assert!((pixels[0][1] - 0.2).abs() < 1e-6);
        assert!((pixels[0][2] - 0.3).abs() < 1e-6);
    }

    #[test]
    fn range_clamp() {
        let mut pixels = [[-0.5, 0.5, 1.5]];
        range_clamp_simd(&mut pixels, 0.0, 1.0);
        assert_eq!(pixels[0], [0.0, 0.5, 1.0]);
    }

    #[test]
    fn simd_scalar_match() {
        #[rustfmt::skip]
        let m = [
            0.4124564, 0.3575761, 0.1804375, 0.0,
            0.2126729, 0.7151522, 0.0721750, 0.0,
            0.0193339, 0.1191920, 0.9503041, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        let input = [[0.5, 0.3, 0.8]];
        let mut simd = input;
        let mut scalar = input;

        matrix_multiply_simd(&mut simd, &m);
        matrix_multiply_scalar(&mut scalar, &m);

        println!("SIMD:   {:?}", simd[0]);
        println!("Scalar: {:?}", scalar[0]);

        assert!((simd[0][0] - scalar[0][0]).abs() < 1e-5, "R mismatch");
        assert!((simd[0][1] - scalar[0][1]).abs() < 1e-5, "G mismatch");
        assert!((simd[0][2] - scalar[0][2]).abs() < 1e-5, "B mismatch");
    }
}
