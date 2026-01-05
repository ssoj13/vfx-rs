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

// =============================================================================
// Kernel Generation (OIIO Parity)
// =============================================================================

/// Kernel types for `make_kernel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelType {
    /// Gaussian kernel (isotropic blur)
    Gaussian,
    /// Box/averaging kernel
    Box,
    /// Sharpening kernel
    Sharpen,
    /// Laplacian edge detection
    Laplacian,
    /// Laplacian of Gaussian (LoG)
    LaplacianOfGaussian,
    /// Sobel X gradient
    SobelX,
    /// Sobel Y gradient
    SobelY,
    /// Prewitt X gradient
    PrewittX,
    /// Prewitt Y gradient
    PrewittY,
    /// Emboss effect
    Emboss,
    /// Unsharp mask
    Unsharp,
    /// Disk kernel (circular)
    Disk,
}

/// Creates a convolution kernel of the specified type.
///
/// This is the OIIO-compatible `make_kernel` function.
///
/// # Arguments
///
/// * `kernel_type` - Type of kernel to generate
/// * `width` - Kernel width (odd number recommended)
/// * `height` - Kernel height (odd number recommended)
/// * `param` - Additional parameter (meaning depends on kernel type)
///
/// For Gaussian: param = sigma
/// For Sharpen: param = strength
/// For Unsharp: param = sigma
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::filters::{make_kernel, KernelType};
///
/// let gaussian = make_kernel(KernelType::Gaussian, 5, 5, 1.0);
/// let sharpen = make_kernel(KernelType::Sharpen, 3, 3, 1.0);
/// ```
pub fn make_kernel(kernel_type: KernelType, width: u32, height: u32, param: f32) -> Vec<f32> {
    let w = width as i32;
    let h = height as i32;
    let hw = w / 2;
    let hh = h / 2;
    let total = (w * h) as usize;

    match kernel_type {
        KernelType::Gaussian => {
            let sigma = if param <= 0.0 { 1.0 } else { param };
            let sigma2 = 2.0 * sigma * sigma;
            let mut kernel = vec![0.0f32; total];
            let mut sum = 0.0f32;

            for y in -hh..=hh {
                for x in -hw..=hw {
                    let idx = ((y + hh) * w + (x + hw)) as usize;
                    let d2 = (x * x + y * y) as f32;
                    kernel[idx] = (-d2 / sigma2).exp();
                    sum += kernel[idx];
                }
            }

            // Normalize
            for k in &mut kernel {
                *k /= sum;
            }
            kernel
        }

        KernelType::Box => {
            let n = total as f32;
            vec![1.0 / n; total]
        }

        KernelType::Sharpen => {
            if w == 3 && h == 3 {
                let amount = if param <= 0.0 { 1.0 } else { param };
                let center = 1.0 + 4.0 * amount;
                vec![
                    0.0, -amount, 0.0,
                    -amount, center, -amount,
                    0.0, -amount, 0.0,
                ]
            } else {
                // For larger kernels, create LoG-based sharpening
                let gaussian = make_kernel(KernelType::Gaussian, width, height, param);
                let center_idx = total / 2;
                let mut kernel = gaussian.iter().map(|&g| -g * param).collect::<Vec<_>>();
                kernel[center_idx] += 1.0 + param;
                kernel
            }
        }

        KernelType::Laplacian => {
            if w == 3 && h == 3 {
                vec![
                    0.0, -1.0, 0.0,
                    -1.0, 4.0, -1.0,
                    0.0, -1.0, 0.0,
                ]
            } else {
                // Extended Laplacian
                let mut kernel = vec![-1.0f32; total];
                let center_idx = total / 2;
                kernel[center_idx] = (total - 1) as f32;
                kernel
            }
        }

        KernelType::LaplacianOfGaussian => {
            let sigma = if param <= 0.0 { 1.0 } else { param };
            let sigma2 = sigma * sigma;
            let sigma4 = sigma2 * sigma2;
            let mut kernel = vec![0.0f32; total];
            let mut sum = 0.0f32;

            for y in -hh..=hh {
                for x in -hw..=hw {
                    let idx = ((y + hh) * w + (x + hw)) as usize;
                    let d2 = (x * x + y * y) as f32;
                    let g = (-d2 / (2.0 * sigma2)).exp();
                    kernel[idx] = ((d2 - 2.0 * sigma2) / sigma4) * g;
                    sum += kernel[idx];
                }
            }

            // Normalize to zero-sum
            let mean = sum / total as f32;
            for k in &mut kernel {
                *k -= mean;
            }
            kernel
        }

        KernelType::SobelX => {
            if w == 3 && h == 3 {
                vec![
                    -1.0, 0.0, 1.0,
                    -2.0, 0.0, 2.0,
                    -1.0, 0.0, 1.0,
                ]
            } else {
                // Extended Sobel using separable filters
                let mut kernel = vec![0.0f32; total];
                for y in -hh..=hh {
                    for x in -hw..=hw {
                        let idx = ((y + hh) * w + (x + hw)) as usize;
                        // Approximate extended Sobel
                        let smooth = 1.0 / (1 + y.abs()) as f32;
                        kernel[idx] = x as f32 * smooth;
                    }
                }
                kernel
            }
        }

        KernelType::SobelY => {
            if w == 3 && h == 3 {
                vec![
                    -1.0, -2.0, -1.0,
                    0.0, 0.0, 0.0,
                    1.0, 2.0, 1.0,
                ]
            } else {
                let mut kernel = vec![0.0f32; total];
                for y in -hh..=hh {
                    for x in -hw..=hw {
                        let idx = ((y + hh) * w + (x + hw)) as usize;
                        let smooth = 1.0 / (1 + x.abs()) as f32;
                        kernel[idx] = y as f32 * smooth;
                    }
                }
                kernel
            }
        }

        KernelType::PrewittX => {
            if w == 3 && h == 3 {
                vec![
                    -1.0, 0.0, 1.0,
                    -1.0, 0.0, 1.0,
                    -1.0, 0.0, 1.0,
                ]
            } else {
                let mut kernel = vec![0.0f32; total];
                for y in -hh..=hh {
                    for x in -hw..=hw {
                        let idx = ((y + hh) * w + (x + hw)) as usize;
                        kernel[idx] = x.signum() as f32;
                    }
                }
                kernel
            }
        }

        KernelType::PrewittY => {
            if w == 3 && h == 3 {
                vec![
                    -1.0, -1.0, -1.0,
                    0.0, 0.0, 0.0,
                    1.0, 1.0, 1.0,
                ]
            } else {
                let mut kernel = vec![0.0f32; total];
                for y in -hh..=hh {
                    for x in -hw..=hw {
                        let idx = ((y + hh) * w + (x + hw)) as usize;
                        kernel[idx] = y.signum() as f32;
                    }
                }
                kernel
            }
        }

        KernelType::Emboss => {
            if w == 3 && h == 3 {
                vec![
                    -2.0, -1.0, 0.0,
                    -1.0, 1.0, 1.0,
                    0.0, 1.0, 2.0,
                ]
            } else {
                let mut kernel = vec![0.0f32; total];
                for y in -hh..=hh {
                    for x in -hw..=hw {
                        let idx = ((y + hh) * w + (x + hw)) as usize;
                        kernel[idx] = (x + y) as f32 / (hw + hh) as f32;
                    }
                }
                let center_idx = total / 2;
                kernel[center_idx] = 1.0;
                kernel
            }
        }

        KernelType::Unsharp => {
            let sigma = if param <= 0.0 { 1.0 } else { param };
            let gaussian = make_kernel(KernelType::Gaussian, width, height, sigma);
            let center_idx = total / 2;
            let mut kernel: Vec<f32> = gaussian.iter().map(|&g| -g).collect();
            kernel[center_idx] += 2.0;
            kernel
        }

        KernelType::Disk => {
            let radius = (hw.min(hh)) as f32;
            let mut kernel = vec![0.0f32; total];
            let mut count = 0.0f32;

            for y in -hh..=hh {
                for x in -hw..=hw {
                    let idx = ((y + hh) * w + (x + hw)) as usize;
                    let d = ((x * x + y * y) as f32).sqrt();
                    if d <= radius {
                        kernel[idx] = 1.0;
                        count += 1.0;
                    }
                }
            }

            // Normalize
            if count > 0.0 {
                for k in &mut kernel {
                    *k /= count;
                }
            }
            kernel
        }
    }
}

/// Creates a named kernel string (for OIIO compatibility).
///
/// Accepts kernel names like "gaussian", "box", "laplacian", etc.
pub fn make_kernel_from_name(name: &str, width: u32, height: u32, param: f32) -> Option<Vec<f32>> {
    let kernel_type = match name.to_lowercase().as_str() {
        "gaussian" | "gauss" => KernelType::Gaussian,
        "box" | "average" => KernelType::Box,
        "sharpen" | "sharp" => KernelType::Sharpen,
        "laplacian" | "laplace" => KernelType::Laplacian,
        "log" | "laplacianofgaussian" => KernelType::LaplacianOfGaussian,
        "sobel_x" | "sobelx" => KernelType::SobelX,
        "sobel_y" | "sobely" => KernelType::SobelY,
        "prewitt_x" | "prewittx" => KernelType::PrewittX,
        "prewitt_y" | "prewitty" => KernelType::PrewittY,
        "emboss" => KernelType::Emboss,
        "unsharp" => KernelType::Unsharp,
        "disk" => KernelType::Disk,
        _ => return None,
    };

    Some(make_kernel(kernel_type, width, height, param))
}

// =============================================================================
// Iterated Morphological Operations
// =============================================================================

/// Applies morphological dilation with iteration support.
///
/// # Arguments
///
/// * `src` - Source image
/// * `size` - Structuring element size
/// * `iterations` - Number of times to apply the operation
/// * `roi` - Optional region of interest
pub fn dilate_n(src: &ImageBuf, size: u32, iterations: u32, roi: Option<Roi3D>) -> ImageBuf {
    if iterations == 0 {
        return src.clone();
    }

    let mut result = dilate(src, size, roi);
    for _ in 1..iterations {
        result = dilate(&result, size, roi);
    }
    result
}

/// Applies morphological erosion with iteration support.
pub fn erode_n(src: &ImageBuf, size: u32, iterations: u32, roi: Option<Roi3D>) -> ImageBuf {
    if iterations == 0 {
        return src.clone();
    }

    let mut result = erode(src, size, roi);
    for _ in 1..iterations {
        result = erode(&result, size, roi);
    }
    result
}

/// Morphological gradient (dilation - erosion).
///
/// Highlights edges in the image.
pub fn morph_gradient(src: &ImageBuf, size: u32, roi: Option<Roi3D>) -> ImageBuf {
    let dilated = dilate(src, size, roi);
    let eroded = erode(src, size, roi);

    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let mut result = ImageBuf::new(src.spec().clone(), InitializePixels::No);

    let mut d_pixel = vec![0.0f32; nch];
    let mut e_pixel = vec![0.0f32; nch];
    let mut out_pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                dilated.getpixel(x, y, z, &mut d_pixel, WrapMode::Clamp);
                eroded.getpixel(x, y, z, &mut e_pixel, WrapMode::Clamp);
                for c in 0..nch {
                    out_pixel[c] = d_pixel[c] - e_pixel[c];
                }
                result.setpixel(x, y, z, &out_pixel);
            }
        }
    }

    result
}

/// Top-hat transform (src - opening).
///
/// Extracts bright features smaller than the structuring element.
pub fn top_hat(src: &ImageBuf, size: u32, roi: Option<Roi3D>) -> ImageBuf {
    let opened = morph_open(src, size, roi);
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let mut result = ImageBuf::new(src.spec().clone(), InitializePixels::No);

    let mut s_pixel = vec![0.0f32; nch];
    let mut o_pixel = vec![0.0f32; nch];
    let mut out_pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut s_pixel, WrapMode::Clamp);
                opened.getpixel(x, y, z, &mut o_pixel, WrapMode::Clamp);
                for c in 0..nch {
                    out_pixel[c] = s_pixel[c] - o_pixel[c];
                }
                result.setpixel(x, y, z, &out_pixel);
            }
        }
    }

    result
}

/// Black-hat transform (closing - src).
///
/// Extracts dark features smaller than the structuring element.
pub fn black_hat(src: &ImageBuf, size: u32, roi: Option<Roi3D>) -> ImageBuf {
    let closed = morph_close(src, size, roi);
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let mut result = ImageBuf::new(src.spec().clone(), InitializePixels::No);

    let mut s_pixel = vec![0.0f32; nch];
    let mut c_pixel = vec![0.0f32; nch];
    let mut out_pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut s_pixel, WrapMode::Clamp);
                closed.getpixel(x, y, z, &mut c_pixel, WrapMode::Clamp);
                for c in 0..nch {
                    out_pixel[c] = c_pixel[c] - s_pixel[c];
                }
                result.setpixel(x, y, z, &out_pixel);
            }
        }
    }

    result
}

// =============================================================================
// Convolution with Border Mode
// =============================================================================

/// Applies convolution with explicit border handling.
///
/// # Arguments
///
/// * `src` - Source image
/// * `kernel` - Convolution kernel
/// * `kw`, `kh` - Kernel dimensions
/// * `border` - Border handling mode
/// * `roi` - Optional region of interest
pub fn convolve_with_border(
    src: &ImageBuf,
    kernel: &[f32],
    kw: u32,
    kh: u32,
    border: WrapMode,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    convolve_with_border_into(&mut dst, src, kernel, kw, kh, border, Some(roi));
    dst
}

/// Applies convolution with border mode into existing ImageBuf.
pub fn convolve_with_border_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    kernel: &[f32],
    kw: u32,
    kh: u32,
    border: WrapMode,
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
                        src.getpixel(sx, sy, z, &mut src_pixel, border);

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

/// Bilateral filter for edge-preserving smoothing.
///
/// Smooths the image while preserving edges by considering both
/// spatial proximity and intensity similarity.
///
/// # Arguments
///
/// * `src` - Source image
/// * `sigma_spatial` - Spatial standard deviation (size of blur)
/// * `sigma_range` - Range standard deviation (edge sensitivity)
/// * `roi` - Optional region of interest
pub fn bilateral(
    src: &ImageBuf,
    sigma_spatial: f32,
    sigma_range: f32,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    bilateral_into(&mut dst, src, sigma_spatial, sigma_range, Some(roi));
    dst
}

/// Applies bilateral filter into existing ImageBuf.
pub fn bilateral_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    sigma_spatial: f32,
    sigma_range: f32,
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;

    let radius = (sigma_spatial * 3.0).ceil() as i32;
    let spatial_coeff = -0.5 / (sigma_spatial * sigma_spatial);
    let range_coeff = -0.5 / (sigma_range * sigma_range);

    let mut center_pixel = vec![0.0f32; nch];
    let mut neighbor_pixel = vec![0.0f32; nch];
    let mut sum = vec![0.0f32; nch];
    let mut weight_sum = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut center_pixel, WrapMode::Clamp);

                // Reset accumulators
                for c in 0..nch {
                    sum[c] = 0.0;
                    weight_sum[c] = 0.0;
                }

                // Apply bilateral filter
                for ky in -radius..=radius {
                    for kx in -radius..=radius {
                        src.getpixel(x + kx, y + ky, z, &mut neighbor_pixel, WrapMode::Clamp);

                        let spatial_dist = (kx * kx + ky * ky) as f32;
                        let spatial_weight = (spatial_coeff * spatial_dist).exp();

                        for c in 0..nch {
                            let range_dist = (center_pixel[c] - neighbor_pixel[c]).powi(2);
                            let range_weight = (range_coeff * range_dist).exp();
                            let weight = spatial_weight * range_weight;

                            sum[c] += neighbor_pixel[c] * weight;
                            weight_sum[c] += weight;
                        }
                    }
                }

                // Normalize
                for c in 0..nch {
                    center_pixel[c] = if weight_sum[c] > 0.0 {
                        sum[c] / weight_sum[c]
                    } else {
                        center_pixel[c]
                    };
                }

                dst.setpixel(x, y, z, &center_pixel);
            }
        }
    }
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
