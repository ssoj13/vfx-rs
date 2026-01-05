//! Geometric transformation functions for ImageBuf.
//!
//! This module provides functions for geometric image transformations:
//! - [`crop`] / [`cut`] - Extract regions
//! - [`flip`] / [`flop`] / [`transpose`] - Mirror operations
//! - [`rotate90`] / [`rotate180`] / [`rotate270`] - Right-angle rotations
//! - [`resize`] - Scale images

use crate::imagebuf::{ImageBuf, InitializePixels, WrapMode};
use vfx_core::{ImageSpec, Roi3D};

/// Crops an image to the specified ROI, keeping data outside ROI as black.
///
/// The returned image has the same pixel data window as src, but pixels
/// outside the ROI are set to zero.
///
/// # Arguments
///
/// * `src` - Source image
/// * `roi` - Region to keep (rest becomes black)
pub fn crop(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::Yes);
    crop_into(&mut dst, src, Some(roi));
    dst
}

/// Crops src into dst.
pub fn crop_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);
                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Cuts out a region from an image, resizing to match the ROI.
///
/// Unlike crop, cut actually resizes the output to match the ROI dimensions.
///
/// # Arguments
///
/// * `src` - Source image
/// * `roi` - Region to extract
pub fn cut(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());

    let mut spec = src.spec().clone();
    spec.width = roi.width() as u32;
    spec.height = roi.height() as u32;
    spec.x = roi.xbegin;
    spec.y = roi.ybegin;

    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    let nch = src.nchannels() as usize;
    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);
                dst.setpixel(x, y, z, &pixel);
            }
        }
    }

    dst
}

/// Flips an image vertically (mirror over horizontal axis).
///
/// # Arguments
///
/// * `src` - Source image
/// * `roi` - Optional region (defaults to entire image)
pub fn flip(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    flip_into(&mut dst, src, Some(roi));
    dst
}

/// Flips src vertically into dst.
pub fn flip_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let mut pixel = vec![0.0f32; nch];
    let ymax = roi.yend - 1;

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                let src_y = ymax - (y - roi.ybegin);
                src.getpixel(x, src_y, z, &mut pixel, WrapMode::Black);
                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Flops an image horizontally (mirror over vertical axis).
///
/// # Arguments
///
/// * `src` - Source image
/// * `roi` - Optional region (defaults to entire image)
pub fn flop(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    flop_into(&mut dst, src, Some(roi));
    dst
}

/// Flops src horizontally into dst.
pub fn flop_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let mut pixel = vec![0.0f32; nch];
    let xmax = roi.xend - 1;

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                let src_x = xmax - (x - roi.xbegin);
                src.getpixel(src_x, y, z, &mut pixel, WrapMode::Black);
                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Transposes an image (swaps x and y axes).
///
/// # Arguments
///
/// * `src` - Source image
/// * `roi` - Optional region (defaults to entire image)
pub fn transpose(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());

    // Swap width and height
    let mut spec = src.spec().clone();
    spec.width = roi.height() as u32;
    spec.height = roi.width() as u32;

    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    transpose_into(&mut dst, src, Some(roi));
    dst
}

/// Transposes src into dst.
pub fn transpose_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);
                // Swap x and y
                let dst_x = y - roi.ybegin;
                let dst_y = x - roi.xbegin;
                dst.setpixel(dst_x, dst_y, z, &pixel);
            }
        }
    }
}

/// Rotates an image 90 degrees clockwise.
///
/// # Arguments
///
/// * `src` - Source image
/// * `roi` - Optional region (defaults to entire image)
pub fn rotate90(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());

    // Swap width and height
    let mut spec = src.spec().clone();
    spec.width = roi.height() as u32;
    spec.height = roi.width() as u32;

    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    rotate90_into(&mut dst, src, Some(roi));
    dst
}

/// Rotates src 90 degrees clockwise into dst.
pub fn rotate90_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let mut pixel = vec![0.0f32; nch];
    let w = roi.width();

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);
                // 90° clockwise: (x, y) -> (h - 1 - y, x)
                let dst_x = roi.yend - 1 - y;
                let dst_y = x - roi.xbegin;
                dst.setpixel(dst_x, dst_y, z, &pixel);
            }
        }
    }
}

/// Rotates an image 180 degrees.
///
/// # Arguments
///
/// * `src` - Source image
/// * `roi` - Optional region (defaults to entire image)
pub fn rotate180(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    rotate180_into(&mut dst, src, Some(roi));
    dst
}

/// Rotates src 180 degrees into dst.
pub fn rotate180_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);
                let dst_x = roi.xend - 1 - (x - roi.xbegin);
                let dst_y = roi.yend - 1 - (y - roi.ybegin);
                dst.setpixel(dst_x, dst_y, z, &pixel);
            }
        }
    }
}

/// Rotates an image 270 degrees clockwise (90 degrees counter-clockwise).
///
/// # Arguments
///
/// * `src` - Source image
/// * `roi` - Optional region (defaults to entire image)
pub fn rotate270(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());

    // Swap width and height
    let mut spec = src.spec().clone();
    spec.width = roi.height() as u32;
    spec.height = roi.width() as u32;

    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    rotate270_into(&mut dst, src, Some(roi));
    dst
}

/// Rotates src 270 degrees clockwise into dst.
pub fn rotate270_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);
                // 270° clockwise (90° ccw): (x, y) -> (y, w - 1 - x)
                let dst_x = y - roi.ybegin;
                let dst_y = roi.xend - 1 - x;
                dst.setpixel(dst_x, dst_y, z, &pixel);
            }
        }
    }
}

/// Filter type for resizing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResizeFilter {
    /// Nearest neighbor (fastest, blocky)
    Nearest,
    /// Bilinear interpolation (default, smooth)
    #[default]
    Bilinear,
    /// Bicubic interpolation (sharper)
    Bicubic,
    /// Lanczos 3-lobe (high quality)
    Lanczos3,
}

/// Resizes an image to a new resolution.
///
/// # Arguments
///
/// * `src` - Source image
/// * `width` - New width
/// * `height` - New height
/// * `filter` - Interpolation filter to use
/// * `roi` - Optional region of source to resize (defaults to entire image)
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::{resize, ResizeFilter};
///
/// let half_size = resize(&large_image, 960, 540, ResizeFilter::Bilinear, None);
/// ```
pub fn resize(src: &ImageBuf, width: u32, height: u32, filter: ResizeFilter, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());

    let mut spec = src.spec().clone();
    spec.width = width;
    spec.height = height;

    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    resize_into(&mut dst, src, filter, Some(roi));
    dst
}

/// Resizes src into dst using the specified filter.
pub fn resize_into(dst: &mut ImageBuf, src: &ImageBuf, filter: ResizeFilter, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let src_w = roi.width() as f32;
    let src_h = roi.height() as f32;
    let dst_w = dst.width() as f32;
    let dst_h = dst.height() as f32;
    let nch = src.nchannels() as usize;

    let scale_x = src_w / dst_w;
    let scale_y = src_h / dst_h;

    let dst_roi = dst.roi();
    let mut pixel = vec![0.0f32; nch];

    for z in dst_roi.zbegin..dst_roi.zend {
        for y in dst_roi.ybegin..dst_roi.yend {
            for x in dst_roi.xbegin..dst_roi.xend {
                // Map dst coords to src coords
                let src_x = roi.xbegin as f32 + ((x - dst_roi.xbegin) as f32 + 0.5) * scale_x;
                let src_y = roi.ybegin as f32 + ((y - dst_roi.ybegin) as f32 + 0.5) * scale_y;

                match filter {
                    ResizeFilter::Nearest => {
                        let sx = src_x.floor() as i32;
                        let sy = src_y.floor() as i32;
                        src.getpixel(sx, sy, z, &mut pixel, WrapMode::Black);
                    }
                    ResizeFilter::Bilinear => {
                        src.interppixel(src_x, src_y, &mut pixel, WrapMode::Black);
                    }
                    ResizeFilter::Bicubic => {
                        src.interppixel_bicubic(src_x, src_y, &mut pixel, WrapMode::Black);
                    }
                    ResizeFilter::Lanczos3 => {
                        // Simplified - use bicubic for now
                        // Full Lanczos would require more complex implementation
                        src.interppixel_bicubic(src_x, src_y, &mut pixel, WrapMode::Black);
                    }
                }

                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Resample an image without filtering (point sampling).
///
/// This is faster than resize but may produce aliasing.
pub fn resample(src: &ImageBuf, width: u32, height: u32, roi: Option<Roi3D>) -> ImageBuf {
    resize(src, width, height, ResizeFilter::Nearest, roi)
}

/// Fit an image into the specified dimensions, maintaining aspect ratio.
///
/// The image will be scaled to fit entirely within the target dimensions,
/// with letterboxing/pillarboxing as needed.
pub fn fit(src: &ImageBuf, width: u32, height: u32, filter: ResizeFilter, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let src_w = roi.width() as f32;
    let src_h = roi.height() as f32;
    let dst_w = width as f32;
    let dst_h = height as f32;

    let scale_x = dst_w / src_w;
    let scale_y = dst_h / src_h;
    let scale = scale_x.min(scale_y);

    let new_w = (src_w * scale).round() as u32;
    let new_h = (src_h * scale).round() as u32;

    resize(src, new_w, new_h, filter, Some(roi))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flip() {
        let spec = ImageSpec::gray(4, 4);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        // Set first row to 1.0, rest to 0.0
        for x in 0..4 {
            src.setpixel(x, 0, 0, &[1.0]);
            for y in 1..4 {
                src.setpixel(x, y, 0, &[0.0]);
            }
        }

        let flipped = flip(&src, None);

        // After flip, last row should be 1.0
        let mut pixel = [0.0f32];
        flipped.getpixel(0, 3, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 1.0).abs() < 0.001);

        flipped.getpixel(0, 0, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_flop() {
        let spec = ImageSpec::gray(4, 4);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        // Set first column to 1.0
        for y in 0..4 {
            src.setpixel(0, y, 0, &[1.0]);
            for x in 1..4 {
                src.setpixel(x, y, 0, &[0.0]);
            }
        }

        let flopped = flop(&src, None);

        // After flop, last column should be 1.0
        let mut pixel = [0.0f32];
        flopped.getpixel(3, 0, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 1.0).abs() < 0.001);

        flopped.getpixel(0, 0, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_rotate180() {
        let spec = ImageSpec::gray(4, 4);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        // Set top-left corner pixel
        src.setpixel(0, 0, 0, &[1.0]);

        let rotated = rotate180(&src, None);

        // Should now be at bottom-right
        let mut pixel = [0.0f32];
        rotated.getpixel(3, 3, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 1.0).abs() < 0.001);

        rotated.getpixel(0, 0, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_resize() {
        let spec = ImageSpec::gray(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        // Fill with 1.0
        for y in 0..10 {
            for x in 0..10 {
                src.setpixel(x, y, 0, &[1.0]);
            }
        }

        let small = resize(&src, 5, 5, ResizeFilter::Bilinear, None);
        assert_eq!(small.width(), 5);
        assert_eq!(small.height(), 5);

        // Should still be ~1.0
        let mut pixel = [0.0f32];
        small.getpixel(2, 2, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_crop() {
        let spec = ImageSpec::gray(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        // Fill entire image with 1.0
        for y in 0..10 {
            for x in 0..10 {
                src.setpixel(x, y, 0, &[1.0]);
            }
        }

        // Crop to center 4x4
        let roi = Roi3D::new_2d(3, 7, 3, 7);
        let cropped = crop(&src, Some(roi));

        // Inside ROI should be 1.0
        let mut pixel = [0.0f32];
        cropped.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 1.0).abs() < 0.001);

        // Outside ROI should be 0.0
        cropped.getpixel(0, 0, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.0).abs() < 0.001);
    }
}
