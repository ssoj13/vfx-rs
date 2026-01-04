//! FFT-based image operations.
//!
//! Uses Fast Fourier Transform for efficient convolution and frequency-domain
//! operations on large kernels.
//!
//! # Operations
//!
//! - [`fft_convolve`] - FFT-based convolution (faster for large kernels)
//! - [`fft_blur`] - Gaussian blur via frequency domain
//! - [`fft_sharpen`] - Unsharp mask via frequency domain
//!
//! # When to Use FFT
//!
//! FFT convolution is faster than direct convolution when kernel radius > ~10.
//! For small kernels, use [`filter::convolve`](super::filter::convolve).
//!
//! # Example
//!
//! ```rust,ignore
//! use vfx_ops::fft::{fft_blur, fft_convolve};
//!
//! // Fast blur for large radius
//! let blurred = fft_blur(&image, width, height, 3, 50.0)?;
//! ```

use crate::{OpsError, OpsResult};
use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;

/// FFT-based convolution.
///
/// More efficient than direct convolution for large kernels.
/// Uses 2D FFT with zero-padding.
///
/// # Arguments
///
/// * `src` - Source pixel data (single channel)
/// * `width` - Image width
/// * `height` - Image height
/// * `kernel` - Convolution kernel (must be smaller than image)
/// * `kw` - Kernel width
/// * `kh` - Kernel height
///
/// # Example
///
/// ```rust,ignore
/// use vfx_ops::fft::fft_convolve;
///
/// let src = vec![0.5f32; 256 * 256];
/// let kernel = vec![1.0 / 25.0; 25]; // 5x5 box blur
/// let result = fft_convolve(&src, 256, 256, &kernel, 5, 5)?;
/// ```
pub fn fft_convolve(
    src: &[f32],
    width: usize,
    height: usize,
    kernel: &[f32],
    kw: usize,
    kh: usize,
) -> OpsResult<Vec<f32>> {
    if src.len() != width * height {
        return Err(OpsError::InvalidDimensions(format!(
            "expected {} pixels, got {}",
            width * height,
            src.len()
        )));
    }
    if kernel.len() != kw * kh {
        return Err(OpsError::InvalidParameter(format!(
            "kernel size mismatch: {} vs {}x{}",
            kernel.len(),
            kw,
            kh
        )));
    }

    // Padded dimensions (power of 2 for efficiency)
    let pw = (width + kw - 1).next_power_of_two();
    let ph = (height + kh - 1).next_power_of_two();

    // Create padded buffers
    let mut img_complex = vec![Complex::new(0.0f32, 0.0); pw * ph];
    let mut kern_complex = vec![Complex::new(0.0f32, 0.0); pw * ph];

    // Copy image with zero-padding
    for y in 0..height {
        for x in 0..width {
            img_complex[y * pw + x] = Complex::new(src[y * width + x], 0.0);
        }
    }

    // Copy kernel centered (wrap-around for circular convolution)
    let kx_off = kw / 2;
    let ky_off = kh / 2;
    for ky in 0..kh {
        for kx in 0..kw {
            let tx = if kx < kx_off {
                pw - (kx_off - kx)
            } else {
                kx - kx_off
            };
            let ty = if ky < ky_off {
                ph - (ky_off - ky)
            } else {
                ky - ky_off
            };
            kern_complex[ty * pw + tx] = Complex::new(kernel[ky * kw + kx], 0.0);
        }
    }

    // Create FFT planner
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(pw);
    let ifft = planner.plan_fft_inverse(pw);

    // Row-wise FFT
    for y in 0..ph {
        let row = &mut img_complex[y * pw..(y + 1) * pw];
        fft.process(row);
    }
    for y in 0..ph {
        let row = &mut kern_complex[y * pw..(y + 1) * pw];
        fft.process(row);
    }

    // Column-wise FFT
    let mut col_buf = vec![Complex::new(0.0f32, 0.0); ph];
    for x in 0..pw {
        for y in 0..ph {
            col_buf[y] = img_complex[y * pw + x];
        }
        let fft_col = planner.plan_fft_forward(ph);
        fft_col.process(&mut col_buf);
        for y in 0..ph {
            img_complex[y * pw + x] = col_buf[y];
        }
    }
    for x in 0..pw {
        for y in 0..ph {
            col_buf[y] = kern_complex[y * pw + x];
        }
        let fft_col = planner.plan_fft_forward(ph);
        fft_col.process(&mut col_buf);
        for y in 0..ph {
            kern_complex[y * pw + x] = col_buf[y];
        }
    }

    // Multiply in frequency domain
    for i in 0..img_complex.len() {
        img_complex[i] = img_complex[i] * kern_complex[i];
    }

    // Inverse column FFT
    for x in 0..pw {
        for y in 0..ph {
            col_buf[y] = img_complex[y * pw + x];
        }
        let ifft_col = planner.plan_fft_inverse(ph);
        ifft_col.process(&mut col_buf);
        for y in 0..ph {
            img_complex[y * pw + x] = col_buf[y];
        }
    }

    // Inverse row FFT
    for y in 0..ph {
        let row = &mut img_complex[y * pw..(y + 1) * pw];
        ifft.process(row);
    }

    // Normalize and extract result
    let scale = 1.0 / (pw * ph) as f32;
    let mut result = vec![0.0f32; width * height];
    for y in 0..height {
        for x in 0..width {
            result[y * width + x] = img_complex[y * pw + x].re * scale;
        }
    }

    Ok(result)
}

/// FFT-based Gaussian blur.
///
/// Efficient for large blur radii (> 10 pixels).
///
/// # Arguments
///
/// * `src` - Source pixel data
/// * `width` - Image width
/// * `height` - Image height
/// * `channels` - Number of channels (3 or 4)
/// * `sigma` - Blur radius in pixels
///
/// # Example
///
/// ```rust,ignore
/// use vfx_ops::fft::fft_blur;
///
/// let src = vec![0.5f32; 256 * 256 * 3];
/// let result = fft_blur(&src, 256, 256, 3, 20.0)?;
/// ```
pub fn fft_blur(
    src: &[f32],
    width: usize,
    height: usize,
    channels: usize,
    sigma: f32,
) -> OpsResult<Vec<f32>> {
    let expected = width * height * channels;
    if src.len() != expected {
        return Err(OpsError::InvalidDimensions(format!(
            "expected {} pixels, got {}",
            expected,
            src.len()
        )));
    }

    // Generate Gaussian kernel
    let ksize = ((sigma * 6.0).ceil() as usize) | 1; // Ensure odd
    let ksize = ksize.max(3);
    let half = ksize / 2;
    let sigma2 = 2.0 * sigma * sigma;

    let mut kernel = vec![0.0f32; ksize * ksize];
    let mut sum = 0.0f32;

    for ky in 0..ksize {
        for kx in 0..ksize {
            let dx = kx as f32 - half as f32;
            let dy = ky as f32 - half as f32;
            let w = (-(dx * dx + dy * dy) / sigma2).exp();
            kernel[ky * ksize + kx] = w;
            sum += w;
        }
    }

    // Normalize
    for w in &mut kernel {
        *w /= sum;
    }

    // Process each channel
    let mut result = vec![0.0f32; expected];

    for c in 0..channels {
        // Extract channel
        let channel: Vec<f32> = (0..width * height)
            .map(|i| src[i * channels + c])
            .collect();

        // FFT convolve
        let blurred = fft_convolve(&channel, width, height, &kernel, ksize, ksize)?;

        // Copy back
        for i in 0..width * height {
            result[i * channels + c] = blurred[i];
        }
    }

    Ok(result)
}

/// FFT-based unsharp mask sharpening.
///
/// Subtracts blurred version to enhance edges.
///
/// # Arguments
///
/// * `src` - Source pixel data
/// * `width` - Image width
/// * `height` - Image height
/// * `channels` - Number of channels
/// * `sigma` - Blur radius for mask
/// * `amount` - Sharpening strength (0.5-2.0 typical)
///
/// # Example
///
/// ```rust,ignore
/// use vfx_ops::fft::fft_sharpen;
///
/// let src = vec![0.5f32; 256 * 256 * 3];
/// let result = fft_sharpen(&src, 256, 256, 3, 3.0, 1.5)?;
/// ```
pub fn fft_sharpen(
    src: &[f32],
    width: usize,
    height: usize,
    channels: usize,
    sigma: f32,
    amount: f32,
) -> OpsResult<Vec<f32>> {
    let expected = width * height * channels;
    if src.len() != expected {
        return Err(OpsError::InvalidDimensions(format!(
            "expected {} pixels, got {}",
            expected,
            src.len()
        )));
    }

    let blurred = fft_blur(src, width, height, channels, sigma)?;

    let mut result = vec![0.0f32; expected];
    for i in 0..expected {
        // Unsharp mask: original + amount * (original - blurred)
        result[i] = src[i] + amount * (src[i] - blurred[i]);
    }

    Ok(result)
}

/// FFT-based high-pass filter.
///
/// Extracts high-frequency details by subtracting low-pass filtered version.
///
/// # Arguments
///
/// * `src` - Source pixel data
/// * `width` - Image width
/// * `height` - Image height
/// * `channels` - Number of channels
/// * `cutoff` - Cutoff frequency (larger = more detail preserved)
pub fn fft_highpass(
    src: &[f32],
    width: usize,
    height: usize,
    channels: usize,
    cutoff: f32,
) -> OpsResult<Vec<f32>> {
    let expected = width * height * channels;
    if src.len() != expected {
        return Err(OpsError::InvalidDimensions(format!(
            "expected {} pixels, got {}",
            expected,
            src.len()
        )));
    }

    // Low-pass = blur with sigma inversely related to cutoff
    let sigma = (width.max(height) as f32) / (cutoff * 2.0 * PI);
    let lowpass = fft_blur(src, width, height, channels, sigma.max(1.0))?;

    // High-pass = original - low-pass
    let mut result = vec![0.0f32; expected];
    for i in 0..expected {
        result[i] = src[i] - lowpass[i];
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_convolve_identity() {
        // Delta kernel (identity)
        let src = vec![0.5f32; 16 * 16];
        let kernel = vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]; // 3x3 identity
        let result = fft_convolve(&src, 16, 16, &kernel, 3, 3).unwrap();

        // Should be unchanged
        for v in result {
            assert!((v - 0.5).abs() < 0.01);
        }
    }

    #[test]
    fn test_fft_blur_constant() {
        let src = vec![0.5f32; 32 * 32 * 3];
        let result = fft_blur(&src, 32, 32, 3, 3.0).unwrap();

        // Constant image should remain constant
        for v in result {
            assert!((v - 0.5).abs() < 0.02);
        }
    }

    #[test]
    fn test_fft_sharpen() {
        let src = vec![0.5f32; 32 * 32 * 3];
        let result = fft_sharpen(&src, 32, 32, 3, 2.0, 1.0).unwrap();

        // Constant image should be mostly unchanged
        for v in result {
            assert!((v - 0.5).abs() < 0.1);
        }
    }

    #[test]
    fn test_fft_highpass_constant() {
        let src = vec![0.5f32; 32 * 32 * 1];
        let result = fft_highpass(&src, 32, 32, 1, 10.0).unwrap();

        // Constant image should have ~zero high-pass
        for v in result {
            assert!(v.abs() < 0.1);
        }
    }
}
