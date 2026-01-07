//! Geometric transformation functions for ImageBuf.
//!
//! This module provides functions for geometric image transformations:
//! - [`crop`] / [`cut`] - Extract regions
//! - [`flip`] / [`flop`] / [`transpose`] - Mirror operations
//! - [`rotate90`] / [`rotate180`] / [`rotate270`] - Right-angle rotations
//! - [`resize`] - Scale images

use crate::imagebuf::{ImageBuf, InitializePixels, WrapMode};
use vfx_core::Roi3D;

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

/// Pastes one image onto another at a specified location.
///
/// # Arguments
///
/// * `dst` - Destination image (modified in place)
/// * `xbegin` - X coordinate in dst where paste begins
/// * `ybegin` - Y coordinate in dst where paste begins
/// * `zbegin` - Z coordinate in dst where paste begins
/// * `chbegin` - Channel offset in dst
/// * `src` - Source image to paste
/// * `roi` - Optional region of src to paste (defaults to entire src)
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::paste;
///
/// // Paste src into dst at position (100, 100)
/// paste(&mut dst, 100, 100, 0, 0, &src, None);
/// ```
pub fn paste(
    dst: &mut ImageBuf,
    xbegin: i32,
    ybegin: i32,
    zbegin: i32,
    chbegin: i32,
    src: &ImageBuf,
    roi: Option<Roi3D>,
) -> bool {
    let roi = roi.unwrap_or_else(|| src.roi());
    paste_into(dst, xbegin, ybegin, zbegin, chbegin, src, Some(roi))
}

/// Pastes src into dst at specified location.
pub fn paste_into(
    dst: &mut ImageBuf,
    xbegin: i32,
    ybegin: i32,
    zbegin: i32,
    chbegin: i32,
    src: &ImageBuf,
    roi: Option<Roi3D>,
) -> bool {
    let roi = roi.unwrap_or_else(|| src.roi());
    let src_nch = src.nchannels() as usize;
    let dst_nch = dst.nchannels() as usize;
    let chbegin_usize = chbegin.max(0) as usize;

    let mut src_pixel = vec![0.0f32; src_nch];
    let mut dst_pixel = vec![0.0f32; dst_nch];

    let dst_roi = dst.roi();

    for src_z in roi.zbegin..roi.zend {
        let dst_z = zbegin + (src_z - roi.zbegin);
        if dst_z < dst_roi.zbegin || dst_z >= dst_roi.zend {
            continue;
        }

        for src_y in roi.ybegin..roi.yend {
            let dst_y = ybegin + (src_y - roi.ybegin);
            if dst_y < dst_roi.ybegin || dst_y >= dst_roi.yend {
                continue;
            }

            for src_x in roi.xbegin..roi.xend {
                let dst_x = xbegin + (src_x - roi.xbegin);
                if dst_x < dst_roi.xbegin || dst_x >= dst_roi.xend {
                    continue;
                }

                src.getpixel(src_x, src_y, src_z, &mut src_pixel, WrapMode::Black);
                dst.getpixel(dst_x, dst_y, dst_z, &mut dst_pixel, WrapMode::Black);

                // Copy channels from src to dst with offset
                for c in 0..src_nch {
                    let dst_c = chbegin_usize + c;
                    if dst_c < dst_nch {
                        dst_pixel[dst_c] = src_pixel[c];
                    }
                }

                dst.setpixel(dst_x, dst_y, dst_z, &dst_pixel);
            }
        }
    }

    true
}

/// Rotates an image by an arbitrary angle (in radians).
///
/// The image is rotated around its center. Pixels that fall outside
/// the image bounds after rotation are set to black.
///
/// # Arguments
///
/// * `src` - Source image
/// * `angle` - Rotation angle in radians (positive = counter-clockwise)
/// * `filter` - Interpolation filter for sampling
/// * `roi` - Optional region (defaults to entire image)
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::{rotate, ResizeFilter};
/// use std::f32::consts::PI;
///
/// let rotated = rotate(&src, PI / 4.0, ResizeFilter::Bilinear, None); // 45 degrees
/// ```
pub fn rotate(
    src: &ImageBuf,
    angle: f32,
    filter: ResizeFilter,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::Yes);
    rotate_into(&mut dst, src, angle, filter, Some(roi));
    dst
}

/// Rotates src by arbitrary angle into dst.
pub fn rotate_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    angle: f32,
    filter: ResizeFilter,
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let mut pixel = vec![0.0f32; nch];

    // Center of rotation
    let cx = (roi.xbegin + roi.xend) as f32 / 2.0;
    let cy = (roi.ybegin + roi.yend) as f32 / 2.0;

    let cos_a = angle.cos();
    let sin_a = angle.sin();

    let dst_roi = dst.roi();

    for z in dst_roi.zbegin..dst_roi.zend {
        for y in dst_roi.ybegin..dst_roi.yend {
            for x in dst_roi.xbegin..dst_roi.xend {
                // Map dst coords back to src coords (inverse rotation)
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;

                let src_x = cx + dx * cos_a + dy * sin_a;
                let src_y = cy - dx * sin_a + dy * cos_a;

                // Check bounds
                if src_x < roi.xbegin as f32 || src_x >= roi.xend as f32 ||
                   src_y < roi.ybegin as f32 || src_y >= roi.yend as f32 {
                    // Outside source bounds - leave as black
                    continue;
                }

                match filter {
                    ResizeFilter::Nearest => {
                        let sx = src_x.floor() as i32;
                        let sy = src_y.floor() as i32;
                        src.getpixel(sx, sy, z, &mut pixel, WrapMode::Black);
                    }
                    ResizeFilter::Bilinear => {
                        src.interppixel(src_x, src_y, &mut pixel, WrapMode::Black);
                    }
                    ResizeFilter::Bicubic | ResizeFilter::Lanczos3 => {
                        src.interppixel_bicubic(src_x, src_y, &mut pixel, WrapMode::Black);
                    }
                }

                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Circularly shifts an image by the specified amount.
///
/// Pixels that shift off one edge wrap around to the opposite edge.
///
/// # Arguments
///
/// * `src` - Source image
/// * `xshift` - Horizontal shift (positive = right)
/// * `yshift` - Vertical shift (positive = down)
/// * `zshift` - Depth shift
/// * `roi` - Optional region (defaults to entire image)
pub fn circular_shift(
    src: &ImageBuf,
    xshift: i32,
    yshift: i32,
    zshift: i32,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    circular_shift_into(&mut dst, src, xshift, yshift, zshift, Some(roi));
    dst
}

/// Circularly shifts src into dst.
pub fn circular_shift_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    xshift: i32,
    yshift: i32,
    zshift: i32,
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let mut pixel = vec![0.0f32; nch];

    let w = roi.width();
    let h = roi.height();
    let d = roi.depth();

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                // Calculate source position with wrapping
                let src_x = ((x - roi.xbegin - xshift) % w + w) % w + roi.xbegin;
                let src_y = ((y - roi.ybegin - yshift) % h + h) % h + roi.ybegin;
                let src_z = ((z - roi.zbegin - zshift) % d + d) % d + roi.zbegin;

                src.getpixel(src_x, src_y, src_z, &mut pixel, WrapMode::Black);
                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Applies an affine warp transformation using a 3x3 matrix.
///
/// # Arguments
///
/// * `src` - Source image
/// * `matrix` - 3x3 transformation matrix (row-major)
/// * `filter` - Interpolation filter
/// * `roi` - Optional output region
///
/// The matrix should be the INVERSE of the transform you want to apply,
/// as we sample from src for each dst pixel.
pub fn warp(
    src: &ImageBuf,
    matrix: &[f32; 9],
    filter: ResizeFilter,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::Yes);
    warp_into(&mut dst, src, matrix, filter, Some(roi));
    dst
}

/// Applies affine warp into dst.
pub fn warp_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    matrix: &[f32; 9],
    filter: ResizeFilter,
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let mut pixel = vec![0.0f32; nch];

    let dst_roi = dst.roi();

    // Matrix elements (row-major 3x3)
    let m00 = matrix[0];
    let m01 = matrix[1];
    let m02 = matrix[2];
    let m10 = matrix[3];
    let m11 = matrix[4];
    let m12 = matrix[5];
    let m20 = matrix[6];
    let m21 = matrix[7];
    let m22 = matrix[8];

    for z in dst_roi.zbegin..dst_roi.zend {
        for y in dst_roi.ybegin..dst_roi.yend {
            for x in dst_roi.xbegin..dst_roi.xend {
                let xf = x as f32;
                let yf = y as f32;

                // Apply inverse transform (homogeneous coords)
                let w = m20 * xf + m21 * yf + m22;
                if w.abs() < 1e-10 {
                    continue;
                }
                let src_x = (m00 * xf + m01 * yf + m02) / w;
                let src_y = (m10 * xf + m11 * yf + m12) / w;

                // Check bounds
                if src_x < roi.xbegin as f32 || src_x >= roi.xend as f32 ||
                   src_y < roi.ybegin as f32 || src_y >= roi.yend as f32 {
                    continue;
                }

                match filter {
                    ResizeFilter::Nearest => {
                        let sx = src_x.floor() as i32;
                        let sy = src_y.floor() as i32;
                        src.getpixel(sx, sy, z, &mut pixel, WrapMode::Black);
                    }
                    ResizeFilter::Bilinear => {
                        src.interppixel(src_x, src_y, &mut pixel, WrapMode::Black);
                    }
                    ResizeFilter::Bicubic | ResizeFilter::Lanczos3 => {
                        src.interppixel_bicubic(src_x, src_y, &mut pixel, WrapMode::Black);
                    }
                }

                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Reorients an image based on EXIF orientation tag.
///
/// # Arguments
///
/// * `src` - Source image
/// * `orientation` - EXIF orientation value (1-8)
///
/// Orientation values:
/// - 1: Normal
/// - 2: Flip horizontal
/// - 3: Rotate 180
/// - 4: Flip vertical
/// - 5: Transpose (flip + rotate90)
/// - 6: Rotate 90 CW
/// - 7: Transverse (flip + rotate270)
/// - 8: Rotate 270 CW
pub fn reorient(src: &ImageBuf, orientation: u8) -> ImageBuf {
    match orientation {
        1 => src.clone(),
        2 => flop(src, None),
        3 => rotate180(src, None),
        4 => flip(src, None),
        5 => transpose(src, None),
        6 => rotate90(src, None),
        7 => {
            let flipped = flip(src, None);
            rotate90(&flipped, None)
        },
        8 => rotate270(src, None),
        _ => src.clone(),
    }
}

/// Reorients an image using its embedded EXIF orientation metadata.
///
/// Reads the "Orientation" attribute from the image and applies the appropriate
/// transform to make the image upright. After reorienting, the returned image
/// will have orientation 1 (normal).
///
/// This is the recommended function for automatically fixing image orientation
/// from cameras and phones that embed EXIF orientation tags.
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebuf::ImageBuf;
/// use vfx_io::imagebufalgo::reorient_auto;
///
/// let photo = ImageBuf::read("photo.jpg").unwrap();
/// let oriented = reorient_auto(&photo);
/// ```
pub fn reorient_auto(src: &ImageBuf) -> ImageBuf {
    let orientation = src.orientation() as u8;
    if orientation == 1 || orientation == 0 {
        // Already oriented correctly
        src.clone()
    } else {
        reorient(src, orientation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vfx_core::ImageSpec;

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
