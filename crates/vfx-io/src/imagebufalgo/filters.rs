//! Image filtering operations.
//!
//! Provides convolution-based and morphological filters for image processing:
//!
//! - [`median`] - Median filter for noise reduction
//! - [`unsharp_mask`] - Unsharp masking for sharpening
//! - [`blur`] - Gaussian blur
//! - [`dilate`] - Morphological dilation
//! - [`erode`] - Morphological erosion
//! - [`laplacian`] - Edge detection using Laplacian
//! - [`sharpen`] - Simple sharpening filter
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::imagebufalgo::filters::{median, blur, unsharp_mask};
//!
//! // Remove salt-and-pepper noise
//! let denoised = median(&noisy_image, 3, None);
//!
//! // Apply Gaussian blur
//! let blurred = blur(&image, 2.0, None);
//!
//! // Sharpen with unsharp mask
//! let sharp = unsharp_mask(&image, 2.0, 1.5, 0.0, None);
//! ```

use crate::imagebuf::{ImageBuf, InitializePixels, WrapMode};
use vfx_core::Roi3D;

/// Applies a median filter to reduce noise.
///
/// The median filter replaces each pixel with the median value of its
/// neighborhood. This is effective at removing salt-and-pepper noise
/// while preserving edges.
///
/// # Arguments
///
/// * `src` - Source image
/// * `size` - Filter kernel size (e.g., 3 for 3x3)
/// * `roi` - Optional region of interest
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::filters::median;
///
/// let denoised = median(&noisy, 3, None);
/// ```
pub fn median(src: &ImageBuf, size: u32, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    median_into(&mut dst, src, size, Some(roi));
    dst
}

/// Applies median filter into an existing ImageBuf.
pub fn median_into(dst: &mut ImageBuf, src: &ImageBuf, size: u32, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let half = (size / 2) as i32;
    let kernel_size = (size * size) as usize;

    let mut neighborhood: Vec<f32> = vec![0.0; kernel_size];
    let mut src_pixel = vec![0.0f32; nch];
    let mut dst_pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                // Process each channel
                for c in 0..nch {
                    // Gather neighborhood values
                    let mut idx = 0;
                    for ky in -half..=half {
                        for kx in -half..=half {
                            let sx = x + kx;
                            let sy = y + ky;
                            src.getpixel(sx, sy, z, &mut src_pixel, WrapMode::Clamp);
                            neighborhood[idx] = src_pixel[c];
                            idx += 1;
                        }
                    }

                    // Sort and take median
                    neighborhood[..kernel_size].sort_by(|a, b| a.partial_cmp(b).unwrap());
                    dst_pixel[c] = neighborhood[kernel_size / 2];
                }

                dst.setpixel(x, y, z, &dst_pixel);
            }
        }
    }
}

/// Applies Gaussian blur to an image.
///
/// # Arguments
///
/// * `src` - Source image
/// * `sigma` - Standard deviation of Gaussian (larger = more blur)
/// * `roi` - Optional region of interest
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::filters::blur;
///
/// let blurred = blur(&image, 2.0, None);
/// ```
pub fn blur(src: &ImageBuf, sigma: f32, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    blur_into(&mut dst, src, sigma, Some(roi));
    dst
}

/// Applies Gaussian blur into an existing ImageBuf.
pub fn blur_into(dst: &mut ImageBuf, src: &ImageBuf, sigma: f32, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());

    // Determine kernel size from sigma (3*sigma covers 99.7% of Gaussian)
    let radius = (sigma * 3.0).ceil() as i32;
    let size = radius * 2 + 1;

    // Generate Gaussian kernel
    let mut kernel = vec![0.0f32; (size * size) as usize];
    let sigma2 = 2.0 * sigma * sigma;
    let mut sum = 0.0f32;

    for ky in -radius..=radius {
        for kx in -radius..=radius {
            let idx = ((ky + radius) * size + (kx + radius)) as usize;
            let d2 = (kx * kx + ky * ky) as f32;
            kernel[idx] = (-d2 / sigma2).exp();
            sum += kernel[idx];
        }
    }

    // Normalize kernel
    for k in kernel.iter_mut() {
        *k /= sum;
    }

    convolve_into(dst, src, &kernel, size as u32, size as u32, Some(roi));
}

/// Applies unsharp masking to sharpen an image.
///
/// Unsharp mask works by subtracting a blurred version of the image
/// from the original, then adding a weighted version back.
///
/// # Arguments
///
/// * `src` - Source image
/// * `sigma` - Blur sigma (larger = sharper result)
/// * `amount` - Strength of sharpening (1.0 = normal)
/// * `threshold` - Minimum difference to sharpen (0-1)
/// * `roi` - Optional region of interest
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::filters::unsharp_mask;
///
/// let sharp = unsharp_mask(&image, 2.0, 1.5, 0.0, None);
/// ```
pub fn unsharp_mask(
    src: &ImageBuf,
    sigma: f32,
    amount: f32,
    threshold: f32,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    unsharp_mask_into(&mut dst, src, sigma, amount, threshold, Some(roi));
    dst
}

/// Applies unsharp mask into an existing ImageBuf.
pub fn unsharp_mask_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    sigma: f32,
    amount: f32,
    threshold: f32,
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;

    // Create blurred version
    let blurred = blur(src, sigma, Some(roi));

    let mut src_pixel = vec![0.0f32; nch];
    let mut blur_pixel = vec![0.0f32; nch];
    let mut dst_pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut src_pixel, WrapMode::Clamp);
                blurred.getpixel(x, y, z, &mut blur_pixel, WrapMode::Clamp);

                for c in 0..nch {
                    let diff = src_pixel[c] - blur_pixel[c];
                    if diff.abs() >= threshold {
                        dst_pixel[c] = src_pixel[c] + amount * diff;
                    } else {
                        dst_pixel[c] = src_pixel[c];
                    }
                }

                dst.setpixel(x, y, z, &dst_pixel);
            }
        }
    }
}

/// Applies morphological dilation.
///
/// Dilation expands bright regions and shrinks dark regions.
/// Each pixel becomes the maximum value in its neighborhood.
///
/// # Arguments
///
/// * `src` - Source image
/// * `size` - Structuring element size (e.g., 3 for 3x3)
/// * `roi` - Optional region of interest
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::filters::dilate;
///
/// let dilated = dilate(&mask, 3, None);
/// ```
pub fn dilate(src: &ImageBuf, size: u32, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    dilate_into(&mut dst, src, size, Some(roi));
    dst
}

/// Applies morphological dilation into an existing ImageBuf.
pub fn dilate_into(dst: &mut ImageBuf, src: &ImageBuf, size: u32, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let half = (size / 2) as i32;

    let mut src_pixel = vec![0.0f32; nch];
    let mut dst_pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                // Initialize with minimum
                for c in 0..nch {
                    dst_pixel[c] = f32::NEG_INFINITY;
                }

                // Find maximum in neighborhood
                for ky in -half..=half {
                    for kx in -half..=half {
                        let sx = x + kx;
                        let sy = y + ky;
                        src.getpixel(sx, sy, z, &mut src_pixel, WrapMode::Clamp);
                        for c in 0..nch {
                            dst_pixel[c] = dst_pixel[c].max(src_pixel[c]);
                        }
                    }
                }

                dst.setpixel(x, y, z, &dst_pixel);
            }
        }
    }
}

/// Applies morphological erosion.
///
/// Erosion shrinks bright regions and expands dark regions.
/// Each pixel becomes the minimum value in its neighborhood.
///
/// # Arguments
///
/// * `src` - Source image
/// * `size` - Structuring element size (e.g., 3 for 3x3)
/// * `roi` - Optional region of interest
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::filters::erode;
///
/// let eroded = erode(&mask, 3, None);
/// ```
pub fn erode(src: &ImageBuf, size: u32, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    erode_into(&mut dst, src, size, Some(roi));
    dst
}

/// Applies morphological erosion into an existing ImageBuf.
pub fn erode_into(dst: &mut ImageBuf, src: &ImageBuf, size: u32, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let half = (size / 2) as i32;

    let mut src_pixel = vec![0.0f32; nch];
    let mut dst_pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                // Initialize with maximum
                for c in 0..nch {
                    dst_pixel[c] = f32::INFINITY;
                }

                // Find minimum in neighborhood
                for ky in -half..=half {
                    for kx in -half..=half {
                        let sx = x + kx;
                        let sy = y + ky;
                        src.getpixel(sx, sy, z, &mut src_pixel, WrapMode::Clamp);
                        for c in 0..nch {
                            dst_pixel[c] = dst_pixel[c].min(src_pixel[c]);
                        }
                    }
                }

                dst.setpixel(x, y, z, &dst_pixel);
            }
        }
    }
}

/// Morphological opening (erode followed by dilate).
///
/// Removes small bright spots while preserving overall shapes.
pub fn morph_open(src: &ImageBuf, size: u32, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let eroded = erode(src, size, Some(roi));
    dilate(&eroded, size, Some(roi))
}

/// Morphological closing (dilate followed by erode).
///
/// Fills small dark spots while preserving overall shapes.
pub fn morph_close(src: &ImageBuf, size: u32, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let dilated = dilate(src, size, Some(roi));
    erode(&dilated, size, Some(roi))
}

/// Applies Laplacian edge detection.
///
/// The Laplacian highlights regions of rapid intensity change.
///
/// # Arguments
///
/// * `src` - Source image
/// * `roi` - Optional region of interest
pub fn laplacian(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);

    // Standard Laplacian kernel
    #[rustfmt::skip]
    let kernel: [f32; 9] = [
        0.0, -1.0, 0.0,
        -1.0, 4.0, -1.0,
        0.0, -1.0, 0.0,
    ];

    convolve_into(&mut dst, src, &kernel, 3, 3, Some(roi));
    dst
}

/// Applies a simple sharpening filter.
///
/// # Arguments
///
/// * `src` - Source image
/// * `amount` - Sharpening strength (0.0 = none, 1.0 = normal)
/// * `roi` - Optional region of interest
pub fn sharpen(src: &ImageBuf, amount: f32, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);

    // Sharpening kernel: identity + amount * (identity - blur)
    let center = 1.0 + 4.0 * amount;
    #[rustfmt::skip]
    let kernel: [f32; 9] = [
        0.0, -amount, 0.0,
        -amount, center, -amount,
        0.0, -amount, 0.0,
    ];

    convolve_into(&mut dst, src, &kernel, 3, 3, Some(roi));
    dst
}

/// Applies a generic convolution kernel.
///
/// # Arguments
///
/// * `src` - Source image
/// * `kernel` - Convolution kernel (row-major)
/// * `kw` - Kernel width
/// * `kh` - Kernel height
/// * `roi` - Optional region of interest
pub fn convolve(
    src: &ImageBuf,
    kernel: &[f32],
    kw: u32,
    kh: u32,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    convolve_into(&mut dst, src, kernel, kw, kh, Some(roi));
    dst
}

/// Applies convolution into an existing ImageBuf.
pub fn convolve_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    kernel: &[f32],
    kw: u32,
    kh: u32,
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let half_w = (kw / 2) as i32;
    let half_h = (kh / 2) as i32;

    let mut src_pixel = vec![0.0f32; nch];
    let mut dst_pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                // Reset accumulator
                for c in 0..nch {
                    dst_pixel[c] = 0.0;
                }

                // Apply kernel
                let mut ki = 0;
                for ky in -half_h..=half_h {
                    for kx in -half_w..=half_w {
                        let sx = x + kx;
                        let sy = y + ky;
                        src.getpixel(sx, sy, z, &mut src_pixel, WrapMode::Clamp);

                        for c in 0..nch {
                            dst_pixel[c] += src_pixel[c] * kernel[ki];
                        }
                        ki += 1;
                    }
                }

                dst.setpixel(x, y, z, &dst_pixel);
            }
        }
    }
}

/// Box blur (uniform averaging filter).
///
/// Faster than Gaussian blur but produces "boxy" artifacts.
pub fn box_blur(src: &ImageBuf, size: u32, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    box_blur_into(&mut dst, src, size, Some(roi));
    dst
}

/// Applies box blur into an existing ImageBuf.
pub fn box_blur_into(dst: &mut ImageBuf, src: &ImageBuf, size: u32, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let half = (size / 2) as i32;
    let n = ((half * 2 + 1) * (half * 2 + 1)) as f32;

    let mut src_pixel = vec![0.0f32; nch];
    let mut dst_pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                // Reset accumulator
                for c in 0..nch {
                    dst_pixel[c] = 0.0;
                }

                // Sum neighborhood
                for ky in -half..=half {
                    for kx in -half..=half {
                        let sx = x + kx;
                        let sy = y + ky;
                        src.getpixel(sx, sy, z, &mut src_pixel, WrapMode::Clamp);
                        for c in 0..nch {
                            dst_pixel[c] += src_pixel[c];
                        }
                    }
                }

                // Average
                for c in 0..nch {
                    dst_pixel[c] /= n;
                }

                dst.setpixel(x, y, z, &dst_pixel);
            }
        }
    }
}

/// Sobel edge detection (returns gradient magnitude).
pub fn sobel(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    sobel_into(&mut dst, src, Some(roi));
    dst
}

/// Applies Sobel edge detection into an existing ImageBuf.
pub fn sobel_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;

    // Sobel kernels
    #[rustfmt::skip]
    let sobel_x: [f32; 9] = [
        -1.0, 0.0, 1.0,
        -2.0, 0.0, 2.0,
        -1.0, 0.0, 1.0,
    ];
    #[rustfmt::skip]
    let sobel_y: [f32; 9] = [
        -1.0, -2.0, -1.0,
        0.0, 0.0, 0.0,
        1.0, 2.0, 1.0,
    ];

    let mut src_pixel = vec![0.0f32; nch];
    let mut dst_pixel = vec![0.0f32; nch];
    let mut gx = vec![0.0f32; nch];
    let mut gy = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                // Reset gradients
                for c in 0..nch {
                    gx[c] = 0.0;
                    gy[c] = 0.0;
                }

                // Apply Sobel kernels
                let mut ki = 0;
                for ky in -1..=1 {
                    for kx in -1..=1 {
                        let sx = x + kx;
                        let sy = y + ky;
                        src.getpixel(sx, sy, z, &mut src_pixel, WrapMode::Clamp);
                        for c in 0..nch {
                            gx[c] += src_pixel[c] * sobel_x[ki];
                            gy[c] += src_pixel[c] * sobel_y[ki];
                        }
                        ki += 1;
                    }
                }

                // Gradient magnitude
                for c in 0..nch {
                    dst_pixel[c] = (gx[c] * gx[c] + gy[c] * gy[c]).sqrt();
                }

                dst.setpixel(x, y, z, &dst_pixel);
            }
        }
    }
}

// ============================================================================
// Hole Filling
// ============================================================================

/// Fill holes in alpha channel using push-pull algorithm.
///
/// This function fills transparent (alpha = 0) regions with colors from
/// neighboring pixels using a multi-resolution pyramid approach:
///
/// 1. **Push phase**: Build an image pyramid by repeatedly downscaling
/// 2. **Pull phase**: Blend upscaled levels back with original, filling holes
///
/// The result preserves fully opaque regions while smoothly interpolating
/// colors into transparent areas based on surrounding pixels.
///
/// # Arguments
/// * `src` - Source image with alpha channel
/// * `roi` - Region of interest (or None for full image)
///
/// # Example
/// ```ignore
/// use vfx_io::imagebuf::ImageBuf;
/// use vfx_io::imagebufalgo::fillholes_pushpull;
///
/// let img_with_holes = ImageBuf::read("cutout.exr").unwrap();
/// let filled = fillholes_pushpull(&img_with_holes, None);
/// ```
pub fn fillholes_pushpull(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;

    // Need at least 2 channels (one color + alpha) or standard RGBA
    if nch < 2 {
        return src.clone();
    }

    // Assume alpha is last channel
    let alpha_channel = nch - 1;

    let width = roi.width() as usize;
    let height = roi.height() as usize;

    // Create working copy as the top of pyramid
    let spec = src.spec().clone();
    let mut pyramid = vec![ImageBuf::new(spec, InitializePixels::No)];

    // Copy source to top of pyramid
    let mut pixel = vec![0.0f32; nch];
    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);
                pyramid[0].setpixel(x, y, z, &pixel);
            }
        }
    }

    // Push phase: build pyramid
    let mut w = width;
    let mut h = height;

    while w > 1 || h > 1 {
        w = (w + 1) / 2;
        h = (h + 1) / 2;

        let small_spec = vfx_core::ImageSpec::new(w as u32, h as u32, nch as u8, vfx_core::DataFormat::F32);
        let mut small = ImageBuf::new(small_spec, InitializePixels::Yes);

        let prev = pyramid.last().unwrap();
        let prev_w = prev.width() as i32;
        let prev_h = prev.height() as i32;

        // Downsample with box filter (average of 2x2)
        let mut samples = vec![0.0f32; nch];
        for y in 0..h {
            for x in 0..w {
                for c in 0..nch {
                    samples[c] = 0.0;
                }
                let mut total_weight = 0.0f32;

                // Sample 2x2 block from previous level
                for dy in 0..2i32 {
                    for dx in 0..2i32 {
                        let px = (x as i32 * 2 + dx).min(prev_w - 1);
                        let py = (y as i32 * 2 + dy).min(prev_h - 1);
                        prev.getpixel(px, py, 0, &mut pixel, WrapMode::Clamp);

                        let alpha = pixel[alpha_channel];
                        if alpha > 0.0 {
                            // Weight by alpha for proper blending
                            for c in 0..nch {
                                samples[c] += pixel[c] * alpha;
                            }
                            total_weight += alpha;
                        }
                    }
                }

                // Divide by total weight (renormalize)
                if total_weight > 0.0 {
                    for c in 0..nch {
                        samples[c] /= total_weight;
                    }
                }

                small.setpixel(x as i32, y as i32, 0, &samples);
            }
        }

        pyramid.push(small);
    }

    // Pull phase: composite from bottom up
    for i in (0..pyramid.len() - 1).rev() {
        let big = &pyramid[i];
        let big_w = big.width() as usize;
        let big_h = big.height() as usize;

        // Create upscaled version of smaller level
        let small = &pyramid[i + 1];

        let mut result = ImageBuf::new(big.spec().clone(), InitializePixels::No);
        let mut big_pixel = vec![0.0f32; nch];
        let mut small_pixel = vec![0.0f32; nch];
        let mut out_pixel = vec![0.0f32; nch];

        for y in 0..big_h {
            for x in 0..big_w {
                big.getpixel(x as i32, y as i32, 0, &mut big_pixel, WrapMode::Black);

                // Bilinear sample from small
                let sx = x as f32 / 2.0;
                let sy = y as f32 / 2.0;
                let sx0 = sx.floor() as i32;
                let sy0 = sy.floor() as i32;
                let fx = sx - sx0 as f32;
                let fy = sy - sy0 as f32;

                // Get 4 samples for bilinear interpolation
                let mut s00 = vec![0.0f32; nch];
                let mut s10 = vec![0.0f32; nch];
                let mut s01 = vec![0.0f32; nch];
                let mut s11 = vec![0.0f32; nch];

                small.getpixel(sx0, sy0, 0, &mut s00, WrapMode::Clamp);
                small.getpixel(sx0 + 1, sy0, 0, &mut s10, WrapMode::Clamp);
                small.getpixel(sx0, sy0 + 1, 0, &mut s01, WrapMode::Clamp);
                small.getpixel(sx0 + 1, sy0 + 1, 0, &mut s11, WrapMode::Clamp);

                // Bilinear interpolation
                for c in 0..nch {
                    let top = s00[c] * (1.0 - fx) + s10[c] * fx;
                    let bottom = s01[c] * (1.0 - fx) + s11[c] * fx;
                    small_pixel[c] = top * (1.0 - fy) + bottom * fy;
                }

                // Composite: big over upscaled small
                let alpha = big_pixel[alpha_channel].clamp(0.0, 1.0);
                let inv_alpha = 1.0 - alpha;

                for c in 0..nch {
                    out_pixel[c] = big_pixel[c] + small_pixel[c] * inv_alpha;
                }

                result.setpixel(x as i32, y as i32, 0, &out_pixel);
            }
        }

        pyramid[i] = result;
    }

    pyramid.into_iter().next().unwrap()
}

/// Fill holes into existing destination buffer.
pub fn fillholes_pushpull_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let result = fillholes_pushpull(src, roi);
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = result.nchannels() as usize;
    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                result.getpixel(x, y, z, &mut pixel, WrapMode::Black);
                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

// ============================================================================
// Kernel Generation
// ============================================================================

/// Create a filter kernel by name.
///
/// Generates various filter kernels for use with convolution operations.
///
/// # Supported kernels
/// - "box" - Box/average filter
/// - "gaussian" - Gaussian blur kernel
/// - "triangle" - Triangle/tent filter (linear interpolation)
/// - "laplacian" - Laplacian edge detection (3x3 only)
/// - "binomial" - Binomial filter (approximates Gaussian)
/// - "sharpen" - Simple sharpening kernel (3x3)
///
/// # Arguments
/// * `name` - Kernel name (case insensitive)
/// * `width` - Kernel width
/// * `height` - Kernel height
/// * `normalize` - Whether to normalize kernel to sum to 1.0
///
/// # Example
/// ```ignore
/// use vfx_io::imagebufalgo::make_kernel;
///
/// let gaussian = make_kernel("gaussian", 5.0, 5.0, true);
/// let laplacian = make_kernel("laplacian", 3.0, 3.0, false);
/// ```
pub fn make_kernel(name: &str, width: f32, height: f32, normalize: bool) -> ImageBuf {
    // Round up to odd size
    let w = (width.ceil() as usize).max(1) | 1;
    let h = (height.ceil() as usize).max(1) | 1;

    let spec = vfx_core::ImageSpec::gray(w as u32, h as u32);
    let mut kernel = ImageBuf::new(spec, InitializePixels::No);

    let name_lower = name.to_lowercase();

    match name_lower.as_str() {
        "gaussian" => {
            // Gaussian kernel
            let sigma_x = width / 6.0; // 3 sigma rule
            let sigma_y = height / 6.0;
            let cx = (w / 2) as f32;
            let cy = (h / 2) as f32;

            for y in 0..h {
                for x in 0..w {
                    let dx = x as f32 - cx;
                    let dy = y as f32 - cy;
                    let val = (-0.5 * ((dx * dx) / (sigma_x * sigma_x) + (dy * dy) / (sigma_y * sigma_y))).exp();
                    kernel.setpixel(x as i32, y as i32, 0, &[val]);
                }
            }
        }
        "box" => {
            // Box/average filter
            let val = 1.0;
            for y in 0..h {
                for x in 0..w {
                    kernel.setpixel(x as i32, y as i32, 0, &[val]);
                }
            }
        }
        "triangle" | "tent" => {
            // Triangle/tent filter
            let cx = (w / 2) as f32;
            let cy = (h / 2) as f32;
            let rx = cx + 0.5;
            let ry = cy + 0.5;

            for y in 0..h {
                for x in 0..w {
                    let dx = (x as f32 - cx).abs();
                    let dy = (y as f32 - cy).abs();
                    let wx = (1.0 - dx / rx).max(0.0);
                    let wy = (1.0 - dy / ry).max(0.0);
                    kernel.setpixel(x as i32, y as i32, 0, &[wx * wy]);
                }
            }
        }
        "laplacian" => {
            // Laplacian edge detection (only valid for 3x3)
            if w == 3 && h == 3 {
                let vals = [
                    0.0, 1.0, 0.0,
                    1.0, -4.0, 1.0,
                    0.0, 1.0, 0.0,
                ];
                for y in 0..3 {
                    for x in 0..3 {
                        kernel.setpixel(x as i32, y as i32, 0, &[vals[y * 3 + x]]);
                    }
                }
                // Laplacian sums to 0, don't normalize
                return kernel;
            } else {
                // For non-3x3, use discrete Laplacian approximation
                let cx = (w / 2) as f32;
                let cy = (h / 2) as f32;

                for y in 0..h {
                    for x in 0..w {
                        let dx = (x as f32 - cx).abs();
                        let dy = (y as f32 - cy).abs();
                        let dist = (dx * dx + dy * dy).sqrt();
                        let val = if dist < 0.5 {
                            -((w * h) as f32) + 1.0
                        } else {
                            1.0
                        };
                        kernel.setpixel(x as i32, y as i32, 0, &[val]);
                    }
                }
                return kernel; // Don't normalize
            }
        }
        "binomial" => {
            // Binomial filter (approximates Gaussian)
            let mut row_w = vec![1.0f32; w];
            let mut row_h = vec![1.0f32; h];

            // Build binomial coefficients for width
            for _ in 1..w {
                let mut new_row = vec![0.0f32; w];
                new_row[0] = row_w[0];
                for i in 1..w {
                    new_row[i] = row_w[i - 1] + row_w[i];
                }
                row_w = new_row;
            }

            // Build binomial coefficients for height
            if h != w {
                for _ in 1..h {
                    let mut new_row = vec![0.0f32; h];
                    new_row[0] = row_h[0];
                    for i in 1..h {
                        new_row[i] = row_h[i - 1] + row_h[i];
                    }
                    row_h = new_row;
                }
            } else {
                row_h = row_w.clone();
            }

            for y in 0..h {
                for x in 0..w {
                    kernel.setpixel(x as i32, y as i32, 0, &[row_w[x] * row_h[y]]);
                }
            }
        }
        "sharpen" => {
            // Simple sharpening kernel (3x3)
            if w >= 3 && h >= 3 {
                // Fill with zeros first
                for y in 0..h {
                    for x in 0..w {
                        kernel.setpixel(x as i32, y as i32, 0, &[0.0]);
                    }
                }
                // Set center 3x3 sharpen kernel
                let cx = w / 2;
                let cy = h / 2;
                let sharpen = [
                    0.0, -1.0, 0.0,
                    -1.0, 5.0, -1.0,
                    0.0, -1.0, 0.0,
                ];
                for dy in 0..3usize {
                    for dx in 0..3usize {
                        let x = (cx - 1 + dx) as i32;
                        let y = (cy - 1 + dy) as i32;
                        kernel.setpixel(x, y, 0, &[sharpen[dy * 3 + dx]]);
                    }
                }
                return kernel; // Don't normalize sharpen
            } else {
                // Fallback to box
                for y in 0..h {
                    for x in 0..w {
                        kernel.setpixel(x as i32, y as i32, 0, &[1.0]);
                    }
                }
            }
        }
        _ => {
            // Unknown kernel - use box filter
            for y in 0..h {
                for x in 0..w {
                    kernel.setpixel(x as i32, y as i32, 0, &[1.0]);
                }
            }
        }
    }

    // Normalize if requested
    if normalize {
        let mut sum = 0.0f32;
        let mut pixel = [0.0f32];

        for y in 0..h {
            for x in 0..w {
                kernel.getpixel(x as i32, y as i32, 0, &mut pixel, WrapMode::Black);
                sum += pixel[0];
            }
        }

        if sum != 0.0 && sum != 1.0 {
            for y in 0..h {
                for x in 0..w {
                    kernel.getpixel(x as i32, y as i32, 0, &mut pixel, WrapMode::Black);
                    pixel[0] /= sum;
                    kernel.setpixel(x as i32, y as i32, 0, &pixel);
                }
            }
        }
    }

    kernel
}

#[cfg(test)]
mod tests {
    use super::*;
    use vfx_core::ImageSpec;

    fn create_test_image() -> ImageBuf {
        let spec = ImageSpec::rgba(8, 8);
        let mut buf = ImageBuf::new(spec, InitializePixels::No);
        // Fill with gray
        for y in 0..8 {
            for x in 0..8 {
                buf.setpixel(x, y, 0, &[0.5f32, 0.5, 0.5, 1.0]);
            }
        }
        buf
    }

    #[test]
    fn test_median() {
        let src = create_test_image();
        let result = median(&src, 3, None);
        assert_eq!(result.width(), 8);
        assert_eq!(result.height(), 8);

        // Median of constant should be constant
        let mut pixel = [0.0f32; 4];
        result.getpixel(4, 4, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_blur() {
        let src = create_test_image();
        let result = blur(&src, 1.0, None);
        assert_eq!(result.width(), 8);

        // Blur of constant should be constant
        let mut pixel = [0.0f32; 4];
        result.getpixel(4, 4, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_dilate_erode() {
        let src = create_test_image();

        let dilated = dilate(&src, 3, None);
        let eroded = erode(&src, 3, None);

        // For constant image, dilate and erode should be the same
        let mut d_pixel = [0.0f32; 4];
        let mut e_pixel = [0.0f32; 4];
        dilated.getpixel(4, 4, 0, &mut d_pixel, WrapMode::Black);
        eroded.getpixel(4, 4, 0, &mut e_pixel, WrapMode::Black);

        assert!((d_pixel[0] - 0.5).abs() < 0.01);
        assert!((e_pixel[0] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_unsharp_mask() {
        let src = create_test_image();
        let result = unsharp_mask(&src, 1.0, 1.5, 0.0, None);
        assert_eq!(result.width(), 8);

        // For constant, unsharp should have no effect (no edges)
        let mut pixel = [0.0f32; 4];
        result.getpixel(4, 4, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_laplacian() {
        let src = create_test_image();
        let result = laplacian(&src, None);
        assert_eq!(result.width(), 8);

        // Laplacian of constant should be zero (no edges)
        let mut pixel = [0.0f32; 4];
        result.getpixel(4, 4, 0, &mut pixel, WrapMode::Black);
        assert!(pixel[0].abs() < 0.01);
    }

    #[test]
    fn test_sharpen() {
        let src = create_test_image();
        let result = sharpen(&src, 1.0, None);
        assert_eq!(result.width(), 8);
    }

    #[test]
    fn test_sobel() {
        let src = create_test_image();
        let result = sobel(&src, None);
        assert_eq!(result.width(), 8);

        // Sobel of constant should be zero (no edges)
        let mut pixel = [0.0f32; 4];
        result.getpixel(4, 4, 0, &mut pixel, WrapMode::Black);
        assert!(pixel[0].abs() < 0.01);
    }

    #[test]
    fn test_convolve() {
        let src = create_test_image();

        // Identity kernel
        let kernel = [0.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
        let result = convolve(&src, &kernel, 3, 3, None);

        let mut pixel = [0.0f32; 4];
        result.getpixel(4, 4, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_morph_open_close() {
        let src = create_test_image();

        let opened = morph_open(&src, 3, None);
        let closed = morph_close(&src, 3, None);

        assert_eq!(opened.width(), 8);
        assert_eq!(closed.width(), 8);
    }
}
