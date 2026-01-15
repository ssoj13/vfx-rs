//! Parallel image processing operations using Rayon.
//!
//! This module provides parallelized versions of image operations
//! for better performance on multi-core systems.
//!
//! # Example
//!
//! ```rust
//! use vfx_ops::parallel;
//!
//! let src = vec![0.5f32; 1920 * 1080 * 4];
//! let blurred = parallel::box_blur(&src, 1920, 1080, 4, 5).unwrap();
//! ```

use crate::{OpsError, OpsResult};
use rayon::prelude::*;

/// Parallel box blur using separable passes.
///
/// Significantly faster than single-threaded version for large images.
///
/// # Example
///
/// ```rust
/// use vfx_ops::parallel::box_blur;
///
/// let src = vec![0.5f32; 256 * 256 * 4];
/// let blurred = box_blur(&src, 256, 256, 4, 3).unwrap();
/// ```
pub fn box_blur(
    src: &[f32],
    width: usize,
    height: usize,
    channels: usize,
    radius: usize,
) -> OpsResult<Vec<f32>> {
    // Validate dimensions
    if width == 0 || height == 0 || channels == 0 {
        return Err(OpsError::InvalidDimensions(
            "width, height, and channels must be > 0".into(),
        ));
    }
    
    // Check for overflow in size calculation
    let expected = width
        .checked_mul(height)
        .and_then(|v| v.checked_mul(channels))
        .ok_or_else(|| OpsError::InvalidDimensions(
            "image dimensions overflow".into(),
        ))?;
    
    if src.len() != expected {
        return Err(OpsError::InvalidDimensions(format!(
            "expected {} pixels, got {}",
            expected,
            src.len()
        )));
    }
    
    // Validate radius - kernel_size = 2 * radius + 1, must not overflow
    let _kernel_size = radius
        .checked_mul(2)
        .and_then(|v| v.checked_add(1))
        .ok_or_else(|| OpsError::InvalidDimensions(
            "radius too large, causes overflow".into(),
        ))?;

    let temp = blur_horizontal_par(src, width, height, channels, radius);
    let result = blur_vertical_par(&temp, width, height, channels, radius);

    Ok(result)
}

/// Parallel horizontal blur pass.
fn blur_horizontal_par(
    src: &[f32],
    width: usize,
    height: usize,
    channels: usize,
    radius: usize,
) -> Vec<f32> {
    let kernel_size = 2 * radius + 1;
    let inv_size = 1.0 / kernel_size as f32;

    let mut dst = vec![0.0f32; width * height * channels];

    dst.par_chunks_mut(width * channels)
        .enumerate()
        .for_each(|(y, row)| {
            for c in 0..channels {
                let mut sum = 0.0f32;
                // Initialize window
                for kx in 0..=radius {
                    let sx = kx.min(width - 1);
                    sum += src[(y * width + sx) * channels + c];
                }
                sum += src[y * width * channels + c] * radius as f32;

                for x in 0..width {
                    row[x * channels + c] = sum * inv_size;

                    let left = (x as isize - radius as isize).max(0) as usize;
                    let right = (x + radius + 1).min(width - 1);

                    sum -= src[(y * width + left) * channels + c];
                    sum += src[(y * width + right) * channels + c];
                }
            }
        });

    dst
}

/// Parallel vertical blur pass.
/// 
/// Uses transpose-blur-transpose approach for safe parallel processing.
fn blur_vertical_par(
    src: &[f32],
    width: usize,
    height: usize,
    channels: usize,
    radius: usize,
) -> Vec<f32> {
    // Transpose so columns become rows (enables safe parallel row processing)
    let transposed = transpose(src, width, height, channels);
    
    // Apply horizontal blur on transposed data (effectively vertical blur)
    let blurred = blur_horizontal_par(&transposed, height, width, channels, radius);
    
    // Transpose back to original orientation
    transpose(&blurred, height, width, channels)
}

/// Transpose image data: rows become columns.
/// 
/// For image of size (width, height) with C channels:
/// Input:  pixel at (x, y) is at index (y * width + x) * C
/// Output: pixel at (x, y) is at index (x * height + y) * C
fn transpose(src: &[f32], width: usize, height: usize, channels: usize) -> Vec<f32> {
    let mut dst = vec![0.0f32; width * height * channels];
    
    // Process rows in parallel
    dst.par_chunks_mut(height * channels)
        .enumerate()
        .for_each(|(x, col)| {
            for y in 0..height {
                let src_idx = (y * width + x) * channels;
                let dst_idx = y * channels;
                for c in 0..channels {
                    col[dst_idx + c] = src[src_idx + c];
                }
            }
        });
    
    dst
}

/// Parallel resize using separable passes.
///
/// # Example
///
/// ```rust
/// use vfx_ops::parallel::resize;
/// use vfx_ops::Filter;
///
/// let src = vec![0.5f32; 256 * 256 * 4];
/// let resized = resize(&src, 256, 256, 4, 512, 512, Filter::Lanczos3).unwrap();
/// ```
pub fn resize(
    src: &[f32],
    src_w: usize,
    src_h: usize,
    channels: usize,
    dst_w: usize,
    dst_h: usize,
    filter: crate::Filter,
) -> OpsResult<Vec<f32>> {
    let expected = src_w * src_h * channels;
    if src.len() != expected {
        return Err(OpsError::InvalidDimensions(format!(
            "expected {} pixels, got {}",
            expected,
            src.len()
        )));
    }
    if dst_w == 0 || dst_h == 0 {
        return Err(OpsError::InvalidDimensions(
            "destination size must be > 0".into(),
        ));
    }

    let temp = resize_horizontal_par(src, src_w, src_h, channels, dst_w, filter);
    let result = resize_vertical_par(&temp, dst_w, src_h, channels, dst_h, filter);

    Ok(result)
}

/// Parallel horizontal resize.
fn resize_horizontal_par(
    src: &[f32],
    src_w: usize,
    src_h: usize,
    channels: usize,
    dst_w: usize,
    filter: crate::Filter,
) -> Vec<f32> {
    let scale = src_w as f32 / dst_w as f32;
    let support = filter.support() * scale.max(1.0);

    let mut dst = vec![0.0f32; dst_w * src_h * channels];

    dst.par_chunks_mut(dst_w * channels)
        .enumerate()
        .for_each(|(y, row)| {
            for x in 0..dst_w {
                let center = (x as f32 + 0.5) * scale - 0.5;
                let left = ((center - support).floor() as isize).max(0) as usize;
                let right = ((center + support).ceil() as usize).min(src_w - 1);

                let mut sum = vec![0.0f32; channels];
                let mut weight_sum = 0.0f32;

                for sx in left..=right {
                    let dist = (sx as f32 - center) / scale.max(1.0);
                    let w = filter.weight(dist);
                    weight_sum += w;

                    let src_idx = (y * src_w + sx) * channels;
                    for c in 0..channels {
                        sum[c] += src[src_idx + c] * w;
                    }
                }

                if weight_sum > 0.0 {
                    for c in 0..channels {
                        row[x * channels + c] = sum[c] / weight_sum;
                    }
                }
            }
        });

    dst
}

/// Parallel vertical resize.
fn resize_vertical_par(
    src: &[f32],
    src_w: usize,
    src_h: usize,
    channels: usize,
    dst_h: usize,
    filter: crate::Filter,
) -> Vec<f32> {
    let scale = src_h as f32 / dst_h as f32;
    let support = filter.support() * scale.max(1.0);

    let mut dst = vec![0.0f32; src_w * dst_h * channels];

    dst.par_chunks_mut(src_w * channels)
        .enumerate()
        .for_each(|(y, row)| {
            let center = (y as f32 + 0.5) * scale - 0.5;
            let top = ((center - support).floor() as isize).max(0) as usize;
            let bottom = ((center + support).ceil() as usize).min(src_h - 1);

            for x in 0..src_w {
                let mut sum = vec![0.0f32; channels];
                let mut weight_sum = 0.0f32;

                for sy in top..=bottom {
                    let dist = (sy as f32 - center) / scale.max(1.0);
                    let w = filter.weight(dist);
                    weight_sum += w;

                    let src_idx = (sy * src_w + x) * channels;
                    for c in 0..channels {
                        sum[c] += src[src_idx + c] * w;
                    }
                }

                if weight_sum > 0.0 {
                    for c in 0..channels {
                        row[x * channels + c] = sum[c] / weight_sum;
                    }
                }
            }
        });

    dst
}

/// Parallel composite (over operation).
///
/// # Example
///
/// ```rust
/// use vfx_ops::parallel::over;
///
/// let fg = vec![1.0, 0.0, 0.0, 0.5]; // Semi-transparent red
/// let bg = vec![0.0, 0.0, 1.0, 1.0]; // Opaque blue
/// let result = over(&fg, &bg, 1, 1).unwrap();
/// ```
pub fn over(
    fg: &[f32],
    bg: &[f32],
    width: usize,
    height: usize,
) -> OpsResult<Vec<f32>> {
    let size = width * height * 4;
    if fg.len() != size || bg.len() != size {
        return Err(OpsError::SizeMismatch(format!(
            "expected {} pixels, got fg={}, bg={}",
            size,
            fg.len(),
            bg.len()
        )));
    }

    let mut result = vec![0.0f32; size];

    result
        .par_chunks_mut(4)
        .enumerate()
        .for_each(|(i, out)| {
            let idx = i * 4;
            let fg_px = [fg[idx], fg[idx + 1], fg[idx + 2], fg[idx + 3]];
            let bg_px = [bg[idx], bg[idx + 1], bg[idx + 2], bg[idx + 3]];

            let fg_a = fg_px[3];
            let bg_a = bg_px[3];
            let out_a = fg_a + bg_a * (1.0 - fg_a);

            if out_a < 1e-8 {
                out.copy_from_slice(&[0.0, 0.0, 0.0, 0.0]);
            } else {
                let inv_out_a = 1.0 / out_a;
                out[0] = (fg_px[0] * fg_a + bg_px[0] * bg_a * (1.0 - fg_a)) * inv_out_a;
                out[1] = (fg_px[1] * fg_a + bg_px[1] * bg_a * (1.0 - fg_a)) * inv_out_a;
                out[2] = (fg_px[2] * fg_a + bg_px[2] * bg_a * (1.0 - fg_a)) * inv_out_a;
                out[3] = out_a;
            }
        });

    Ok(result)
}

/// Parallel convolution.
///
/// # Example
///
/// ```rust
/// use vfx_ops::parallel::convolve;
/// use vfx_ops::filter::Kernel;
///
/// let src = vec![0.5f32; 64 * 64 * 3];
/// let kernel = Kernel::gaussian(5, 1.5);
/// let result = convolve(&src, 64, 64, 3, &kernel).unwrap();
/// ```
pub fn convolve(
    src: &[f32],
    width: usize,
    height: usize,
    channels: usize,
    kernel: &crate::filter::Kernel,
) -> OpsResult<Vec<f32>> {
    let expected = width * height * channels;
    if src.len() != expected {
        return Err(OpsError::InvalidDimensions(format!(
            "expected {} pixels, got {}",
            expected,
            src.len()
        )));
    }

    let (rx, ry) = kernel.radius();
    let mut dst = vec![0.0f32; expected];

    dst.par_chunks_mut(width * channels)
        .enumerate()
        .for_each(|(y, row)| {
            for x in 0..width {
                let mut sums = vec![0.0f32; channels];

                for ky in 0..kernel.height {
                    for kx in 0..kernel.width {
                        let sx = (x as isize + kx as isize - rx as isize)
                            .max(0)
                            .min(width as isize - 1) as usize;
                        let sy = (y as isize + ky as isize - ry as isize)
                            .max(0)
                            .min(height as isize - 1) as usize;

                        let src_idx = (sy * width + sx) * channels;
                        let kw = kernel.data[ky * kernel.width + kx];

                        for c in 0..channels {
                            sums[c] += src[src_idx + c] * kw;
                        }
                    }
                }

                for c in 0..channels {
                    row[x * channels + c] = sums[c];
                }
            }
        });

    Ok(dst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_blur() {
        let src = vec![0.5f32; 64 * 64 * 4];
        let result = box_blur(&src, 64, 64, 4, 2).unwrap();
        assert_eq!(result.len(), src.len());

        // Constant image stays constant
        for v in result {
            assert!((v - 0.5).abs() < 0.01);
        }
    }

    #[test]
    fn test_parallel_resize() {
        let src = vec![0.5f32; 32 * 32 * 4];
        let result = resize(&src, 32, 32, 4, 64, 64, crate::Filter::Bilinear).unwrap();
        assert_eq!(result.len(), 64 * 64 * 4);

        for v in result {
            assert!((v - 0.5).abs() < 0.01);
        }
    }

    #[test]
    fn test_parallel_over() {
        let fg = vec![1.0, 0.0, 0.0, 0.5, 0.0, 1.0, 0.0, 0.5];
        let bg = vec![0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0];
        let result = over(&fg, &bg, 2, 1).unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_parallel_convolve() {
        let src = vec![0.5f32; 32 * 32 * 3];
        let kernel = crate::filter::Kernel::box_blur(3);
        let result = convolve(&src, 32, 32, 3, &kernel).unwrap();
        assert_eq!(result.len(), src.len());
    }
}
