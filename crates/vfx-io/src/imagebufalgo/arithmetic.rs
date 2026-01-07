//! Arithmetic operations for ImageBuf.
//!
//! This module provides pixel-wise arithmetic operations:
//! - [`add`] / [`sub`] / [`mul`] / [`div`] - Basic arithmetic
//! - [`abs`] / [`absdiff`] - Absolute value operations
//! - [`pow`] - Power function
//! - [`clamp`] - Value clamping
//! - [`invert`] - Color inversion
//! - [`over`] - Alpha compositing

use crate::imagebuf::{ImageBuf, InitializePixels, WrapMode};
use vfx_core::{ImageSpec, Roi3D};

/// Represents either an image or a constant value.
pub enum ImageOrConst<'a> {
    /// Reference to an ImageBuf
    Image(&'a ImageBuf),
    /// Constant value(s) for all pixels
    Const(Vec<f32>),
}

impl<'a> From<&'a ImageBuf> for ImageOrConst<'a> {
    fn from(img: &'a ImageBuf) -> Self {
        ImageOrConst::Image(img)
    }
}

impl<'a> From<f32> for ImageOrConst<'a> {
    fn from(v: f32) -> Self {
        ImageOrConst::Const(vec![v])
    }
}

impl<'a> From<&'a [f32]> for ImageOrConst<'a> {
    fn from(v: &'a [f32]) -> Self {
        ImageOrConst::Const(v.to_vec())
    }
}

impl<'a> From<Vec<f32>> for ImageOrConst<'a> {
    fn from(v: Vec<f32>) -> Self {
        ImageOrConst::Const(v)
    }
}

/// Gets value from ImageOrConst at given position.
fn get_value<'a>(ioc: &'a ImageOrConst, x: i32, y: i32, z: i32, nch: usize, pixel: &mut [f32]) {
    match ioc {
        ImageOrConst::Image(img) => {
            img.getpixel(x, y, z, pixel, WrapMode::Black);
        }
        ImageOrConst::Const(vals) => {
            for (c, p) in pixel.iter_mut().enumerate().take(nch) {
                *p = vals.get(c).copied().unwrap_or_else(|| vals.last().copied().unwrap_or(0.0));
            }
        }
    }
}

/// Gets the ROI for an ImageOrConst.
fn get_roi<'a>(ioc: &'a ImageOrConst) -> Option<Roi3D> {
    match ioc {
        ImageOrConst::Image(img) => Some(img.roi()),
        ImageOrConst::Const(_) => None,
    }
}

/// Gets the number of channels for an ImageOrConst.
fn get_nch<'a>(ioc: &'a ImageOrConst, default: usize) -> usize {
    match ioc {
        ImageOrConst::Image(img) => img.nchannels() as usize,
        ImageOrConst::Const(vals) => vals.len().max(default),
    }
}

/// Adds two images or values pixel-wise: dst = A + B
///
/// # Arguments
///
/// * `a` - First operand (image or constant)
/// * `b` - Second operand (image or constant)
/// * `roi` - Optional region (defaults to union of A and B)
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::add;
///
/// let brightened = add(&image, 0.1);  // Add 0.1 to all channels
/// let combined = add(&img1, &img2);   // Add two images
/// ```
pub fn add<'a, A: Into<ImageOrConst<'a>>, B: Into<ImageOrConst<'a>>>(
    a: A,
    b: B,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let a = a.into();
    let b = b.into();

    let roi = roi
        .or_else(|| get_roi(&a))
        .or_else(|| get_roi(&b))
        .unwrap_or_else(|| Roi3D::new_2d_with_channels(0, 1, 0, 1, 0, 1));

    let nch = get_nch(&a, 1).max(get_nch(&b, 1));
    let spec = ImageSpec::from_roi_nchannels(&roi, nch as u32);

    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    add_into(&mut dst, a, b, Some(roi));
    dst
}

/// Adds A and B into dst.
pub fn add_into<'a, A: Into<ImageOrConst<'a>>, B: Into<ImageOrConst<'a>>>(
    dst: &mut ImageBuf,
    a: A,
    b: B,
    roi: Option<Roi3D>,
) {
    let a = a.into();
    let b = b.into();
    let roi = roi.unwrap_or_else(|| dst.roi());
    let nch = dst.nchannels() as usize;

    let mut pa = vec![0.0f32; nch];
    let mut pb = vec![0.0f32; nch];
    let mut result = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                get_value(&a, x, y, z, nch, &mut pa);
                get_value(&b, x, y, z, nch, &mut pb);

                for c in 0..nch {
                    result[c] = pa[c] + pb[c];
                }

                dst.setpixel(x, y, z, &result);
            }
        }
    }
}

/// Subtracts two images or values pixel-wise: dst = A - B
pub fn sub<'a, A: Into<ImageOrConst<'a>>, B: Into<ImageOrConst<'a>>>(
    a: A,
    b: B,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let a = a.into();
    let b = b.into();

    let roi = roi
        .or_else(|| get_roi(&a))
        .or_else(|| get_roi(&b))
        .unwrap_or_else(|| Roi3D::new_2d_with_channels(0, 1, 0, 1, 0, 1));

    let nch = get_nch(&a, 1).max(get_nch(&b, 1));
    let spec = ImageSpec::from_roi_nchannels(&roi, nch as u32);

    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    sub_into(&mut dst, a, b, Some(roi));
    dst
}

/// Subtracts B from A into dst.
pub fn sub_into<'a, A: Into<ImageOrConst<'a>>, B: Into<ImageOrConst<'a>>>(
    dst: &mut ImageBuf,
    a: A,
    b: B,
    roi: Option<Roi3D>,
) {
    let a = a.into();
    let b = b.into();
    let roi = roi.unwrap_or_else(|| dst.roi());
    let nch = dst.nchannels() as usize;

    let mut pa = vec![0.0f32; nch];
    let mut pb = vec![0.0f32; nch];
    let mut result = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                get_value(&a, x, y, z, nch, &mut pa);
                get_value(&b, x, y, z, nch, &mut pb);

                for c in 0..nch {
                    result[c] = pa[c] - pb[c];
                }

                dst.setpixel(x, y, z, &result);
            }
        }
    }
}

/// Multiplies two images or values pixel-wise: dst = A * B
pub fn mul<'a, A: Into<ImageOrConst<'a>>, B: Into<ImageOrConst<'a>>>(
    a: A,
    b: B,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let a = a.into();
    let b = b.into();

    let roi = roi
        .or_else(|| get_roi(&a))
        .or_else(|| get_roi(&b))
        .unwrap_or_else(|| Roi3D::new_2d_with_channels(0, 1, 0, 1, 0, 1));

    let nch = get_nch(&a, 1).max(get_nch(&b, 1));
    let spec = ImageSpec::from_roi_nchannels(&roi, nch as u32);

    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    mul_into(&mut dst, a, b, Some(roi));
    dst
}

/// Multiplies A and B into dst.
pub fn mul_into<'a, A: Into<ImageOrConst<'a>>, B: Into<ImageOrConst<'a>>>(
    dst: &mut ImageBuf,
    a: A,
    b: B,
    roi: Option<Roi3D>,
) {
    let a = a.into();
    let b = b.into();
    let roi = roi.unwrap_or_else(|| dst.roi());
    let nch = dst.nchannels() as usize;

    let mut pa = vec![0.0f32; nch];
    let mut pb = vec![0.0f32; nch];
    let mut result = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                get_value(&a, x, y, z, nch, &mut pa);
                get_value(&b, x, y, z, nch, &mut pb);

                for c in 0..nch {
                    result[c] = pa[c] * pb[c];
                }

                dst.setpixel(x, y, z, &result);
            }
        }
    }
}

/// Divides two images or values pixel-wise: dst = A / B
pub fn div<'a, A: Into<ImageOrConst<'a>>, B: Into<ImageOrConst<'a>>>(
    a: A,
    b: B,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let a = a.into();
    let b = b.into();

    let roi = roi
        .or_else(|| get_roi(&a))
        .or_else(|| get_roi(&b))
        .unwrap_or_else(|| Roi3D::new_2d_with_channels(0, 1, 0, 1, 0, 1));

    let nch = get_nch(&a, 1).max(get_nch(&b, 1));
    let spec = ImageSpec::from_roi_nchannels(&roi, nch as u32);

    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    div_into(&mut dst, a, b, Some(roi));
    dst
}

/// Divides A by B into dst.
pub fn div_into<'a, A: Into<ImageOrConst<'a>>, B: Into<ImageOrConst<'a>>>(
    dst: &mut ImageBuf,
    a: A,
    b: B,
    roi: Option<Roi3D>,
) {
    let a = a.into();
    let b = b.into();
    let roi = roi.unwrap_or_else(|| dst.roi());
    let nch = dst.nchannels() as usize;

    let mut pa = vec![0.0f32; nch];
    let mut pb = vec![0.0f32; nch];
    let mut result = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                get_value(&a, x, y, z, nch, &mut pa);
                get_value(&b, x, y, z, nch, &mut pb);

                for c in 0..nch {
                    result[c] = if pb[c].abs() > 1e-10 {
                        pa[c] / pb[c]
                    } else {
                        0.0
                    };
                }

                dst.setpixel(x, y, z, &result);
            }
        }
    }
}

/// Computes absolute value of each pixel: dst = |A|
pub fn abs(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    abs_into(&mut dst, src, Some(roi));
    dst
}

/// Computes absolute value of src into dst.
pub fn abs_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                for p in pixel.iter_mut() {
                    *p = p.abs();
                }

                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Computes absolute difference: dst = |A - B|
pub fn absdiff<'a, A: Into<ImageOrConst<'a>>, B: Into<ImageOrConst<'a>>>(
    a: A,
    b: B,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let a = a.into();
    let b = b.into();

    let roi = roi
        .or_else(|| get_roi(&a))
        .or_else(|| get_roi(&b))
        .unwrap_or_else(|| Roi3D::new_2d_with_channels(0, 1, 0, 1, 0, 1));

    let nch = get_nch(&a, 1).max(get_nch(&b, 1));
    let spec = ImageSpec::from_roi_nchannels(&roi, nch as u32);

    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    absdiff_into(&mut dst, a, b, Some(roi));
    dst
}

/// Computes |A - B| into dst.
pub fn absdiff_into<'a, A: Into<ImageOrConst<'a>>, B: Into<ImageOrConst<'a>>>(
    dst: &mut ImageBuf,
    a: A,
    b: B,
    roi: Option<Roi3D>,
) {
    let a = a.into();
    let b = b.into();
    let roi = roi.unwrap_or_else(|| dst.roi());
    let nch = dst.nchannels() as usize;

    let mut pa = vec![0.0f32; nch];
    let mut pb = vec![0.0f32; nch];
    let mut result = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                get_value(&a, x, y, z, nch, &mut pa);
                get_value(&b, x, y, z, nch, &mut pb);

                for c in 0..nch {
                    result[c] = (pa[c] - pb[c]).abs();
                }

                dst.setpixel(x, y, z, &result);
            }
        }
    }
}

/// Raises pixel values to a power: dst = A^B
pub fn pow(src: &ImageBuf, exponent: &[f32], roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    pow_into(&mut dst, src, exponent, Some(roi));
    dst
}

/// Computes A^B into dst.
pub fn pow_into(dst: &mut ImageBuf, src: &ImageBuf, exponent: &[f32], roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                for (c, p) in pixel.iter_mut().enumerate().take(nch) {
                    let exp = exponent.get(c).copied().unwrap_or_else(|| exponent.last().copied().unwrap_or(1.0));
                    *p = p.powf(exp);
                }

                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Clamps pixel values to a range: dst = clamp(A, min, max)
pub fn clamp(src: &ImageBuf, min_val: &[f32], max_val: &[f32], roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    clamp_into(&mut dst, src, min_val, max_val, Some(roi));
    dst
}

/// Clamps src values into dst.
pub fn clamp_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    min_val: &[f32],
    max_val: &[f32],
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                for (c, p) in pixel.iter_mut().enumerate().take(nch) {
                    let min = min_val.get(c).copied().unwrap_or_else(|| min_val.last().copied().unwrap_or(0.0));
                    let max = max_val.get(c).copied().unwrap_or_else(|| max_val.last().copied().unwrap_or(1.0));
                    *p = p.clamp(min, max);
                }

                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Inverts pixel values: dst = 1 - A
pub fn invert(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    invert_into(&mut dst, src, Some(roi));
    dst
}

/// Inverts src values into dst.
pub fn invert_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;
    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                for p in pixel.iter_mut() {
                    *p = 1.0 - *p;
                }

                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Alpha compositing: dst = A over B
///
/// Performs standard Porter-Duff "over" compositing where A is composited
/// over B using A's alpha channel.
///
/// # Arguments
///
/// * `a` - Foreground image (must have alpha channel)
/// * `b` - Background image
/// * `roi` - Optional region
pub fn over(a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));
    let mut dst = ImageBuf::new(a.spec().clone(), InitializePixels::No);
    over_into(&mut dst, a, b, Some(roi));
    dst
}

/// Composites A over B into dst.
pub fn over_into(dst: &mut ImageBuf, a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));
    let nch = a.nchannels() as usize;

    // Find alpha channel
    let alpha_ch = a.spec().alpha_channel;
    let alpha_idx = if alpha_ch >= 0 { alpha_ch as usize } else { nch - 1 };

    let mut pa = vec![0.0f32; nch];
    let mut pb = vec![0.0f32; nch];
    let mut result = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                a.getpixel(x, y, z, &mut pa, WrapMode::Black);
                b.getpixel(x, y, z, &mut pb, WrapMode::Black);

                let alpha_a = pa.get(alpha_idx).copied().unwrap_or(1.0);
                let alpha_b = pb.get(alpha_idx).copied().unwrap_or(1.0);

                // Porter-Duff over: C_out = C_a * alpha_a + C_b * alpha_b * (1 - alpha_a)
                // alpha_out = alpha_a + alpha_b * (1 - alpha_a)
                let one_minus_alpha_a = 1.0 - alpha_a;

                for c in 0..nch {
                    if c == alpha_idx {
                        // Alpha channel
                        result[c] = alpha_a + alpha_b * one_minus_alpha_a;
                    } else {
                        // Color channels
                        result[c] = pa[c] * alpha_a + pb[c] * alpha_b * one_minus_alpha_a;

                        // Unpremultiply if output alpha is non-zero
                        let out_alpha = alpha_a + alpha_b * one_minus_alpha_a;
                        if out_alpha > 0.0 {
                            result[c] /= out_alpha;
                        }
                    }
                }

                dst.setpixel(x, y, z, &result);
            }
        }
    }
}

/// Computes the maximum of two images pixel-wise: dst = max(A, B)
pub fn max<'a, A: Into<ImageOrConst<'a>>, B: Into<ImageOrConst<'a>>>(
    a: A,
    b: B,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let a = a.into();
    let b = b.into();

    let roi = roi
        .or_else(|| get_roi(&a))
        .or_else(|| get_roi(&b))
        .unwrap_or_else(|| Roi3D::new_2d_with_channels(0, 1, 0, 1, 0, 1));

    let nch = get_nch(&a, 1).max(get_nch(&b, 1));
    let spec = ImageSpec::from_roi_nchannels(&roi, nch as u32);

    let mut dst = ImageBuf::new(spec, InitializePixels::No);

    let mut pa = vec![0.0f32; nch];
    let mut pb = vec![0.0f32; nch];
    let mut result = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                get_value(&a, x, y, z, nch, &mut pa);
                get_value(&b, x, y, z, nch, &mut pb);

                for c in 0..nch {
                    result[c] = pa[c].max(pb[c]);
                }

                dst.setpixel(x, y, z, &result);
            }
        }
    }

    dst
}

/// Computes the minimum of two images pixel-wise: dst = min(A, B)
pub fn min<'a, A: Into<ImageOrConst<'a>>, B: Into<ImageOrConst<'a>>>(
    a: A,
    b: B,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let a = a.into();
    let b = b.into();

    let roi = roi
        .or_else(|| get_roi(&a))
        .or_else(|| get_roi(&b))
        .unwrap_or_else(|| Roi3D::new_2d_with_channels(0, 1, 0, 1, 0, 1));

    let nch = get_nch(&a, 1).max(get_nch(&b, 1));
    let spec = ImageSpec::from_roi_nchannels(&roi, nch as u32);

    let mut dst = ImageBuf::new(spec, InitializePixels::No);

    let mut pa = vec![0.0f32; nch];
    let mut pb = vec![0.0f32; nch];
    let mut result = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                get_value(&a, x, y, z, nch, &mut pa);
                get_value(&b, x, y, z, nch, &mut pb);

                for c in 0..nch {
                    result[c] = pa[c].min(pb[c]);
                }

                dst.setpixel(x, y, z, &result);
            }
        }
    }

    dst
}

/// Multiply-add: dst = A * B + C
///
/// Computes the fused multiply-add operation pixel-wise.
pub fn mad<'a, A: Into<ImageOrConst<'a>>, B: Into<ImageOrConst<'a>>, C: Into<ImageOrConst<'a>>>(
    a: A,
    b: B,
    c: C,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let a = a.into();
    let b = b.into();
    let c = c.into();

    let roi = roi
        .or_else(|| get_roi(&a))
        .or_else(|| get_roi(&b))
        .or_else(|| get_roi(&c))
        .unwrap_or_else(|| Roi3D::new_2d_with_channels(0, 1, 0, 1, 0, 1));

    let nch = get_nch(&a, 1).max(get_nch(&b, 1)).max(get_nch(&c, 1));
    let spec = ImageSpec::from_roi_nchannels(&roi, nch as u32);

    let mut dst = ImageBuf::new(spec, InitializePixels::No);

    let mut pa = vec![0.0f32; nch];
    let mut pb = vec![0.0f32; nch];
    let mut pc = vec![0.0f32; nch];
    let mut result = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                get_value(&a, x, y, z, nch, &mut pa);
                get_value(&b, x, y, z, nch, &mut pb);
                get_value(&c, x, y, z, nch, &mut pc);

                for ch in 0..nch {
                    result[ch] = pa[ch].mul_add(pb[ch], pc[ch]);
                }

                dst.setpixel(x, y, z, &result);
            }
        }
    }

    dst
}

/// Normalize RGB vectors to unit length.
///
/// Treats each pixel's RGB channels as a 3D vector and normalizes it to unit length.
/// Useful for normalizing normal maps and direction vectors.
///
/// # Arguments
/// * `src` - Source image (must have 3 or 4 channels)
/// * `in_center` - Value to subtract before normalizing (default 0.0)
/// * `out_center` - Value to add after normalizing (default 0.0)
/// * `scale` - Scale factor for normalized vector (default 1.0)
/// * `roi` - Region of interest (or None for full image)
///
/// # Example
/// ```ignore
/// use vfx_io::imagebuf::{ImageBuf, InitializePixels};
/// use vfx_io::imagebufalgo::normalize;
/// use vfx_core::ImageSpec;
///
/// let spec = ImageSpec::rgb(100, 100);
/// let normals = ImageBuf::new(spec, InitializePixels::No);
/// let normalized = normalize(&normals, 0.5, 0.5, 1.0, None);
/// ```
pub fn normalize(src: &ImageBuf, in_center: f32, out_center: f32, scale: f32, roi: Option<Roi3D>) -> ImageBuf {
    let nch = src.nchannels() as usize;
    if nch != 3 && nch != 4 {
        // Return empty image for invalid input
        return ImageBuf::new(ImageSpec::rgb(1, 1), InitializePixels::Yes);
    }

    let roi = roi.unwrap_or_else(|| src.roi());
    let spec = ImageSpec::from_roi_nchannels(&roi, nch as u32);
    let mut dst = ImageBuf::new(spec, InitializePixels::No);

    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                // Treat RGB as 3D vector
                let vx = pixel[0] - in_center;
                let vy = pixel[1] - in_center;
                let vz = pixel[2] - in_center;

                let length = (vx * vx + vy * vy + vz * vz).sqrt();
                let s = if length > 0.0 { scale / length } else { 0.0 };

                pixel[0] = vx * s + out_center;
                pixel[1] = vy * s + out_center;
                pixel[2] = vz * s + out_center;
                // Alpha channel (if present) is preserved unchanged

                dst.setpixel(x, y, z, &pixel);
            }
        }
    }

    dst
}

/// Normalize RGB vectors to unit length, storing result in provided buffer.
///
/// See [`normalize`] for details.
pub fn normalize_into(dst: &mut ImageBuf, src: &ImageBuf, in_center: f32, out_center: f32, scale: f32, roi: Option<Roi3D>) {
    let nch = src.nchannels() as usize;
    if nch != 3 && nch != 4 {
        return;
    }

    let roi = roi.unwrap_or_else(|| src.roi());
    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                let vx = pixel[0] - in_center;
                let vy = pixel[1] - in_center;
                let vz = pixel[2] - in_center;

                let length = (vx * vx + vy * vy + vz * vz).sqrt();
                let s = if length > 0.0 { scale / length } else { 0.0 };

                pixel[0] = vx * s + out_center;
                pixel[1] = vy * s + out_center;
                pixel[2] = vz * s + out_center;

                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vfx_core::ImageSpec;

    #[test]
    fn test_add_images() {
        let spec = ImageSpec::gray(10, 10);
        let mut a = ImageBuf::new(spec.clone(), InitializePixels::No);
        let mut b = ImageBuf::new(spec, InitializePixels::No);

        for y in 0..10 {
            for x in 0..10 {
                a.setpixel(x, y, 0, &[0.3]);
                b.setpixel(x, y, 0, &[0.2]);
            }
        }

        let result = add(&a, &b, None);
        let mut pixel = [0.0f32];
        result.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_add_constant() {
        let spec = ImageSpec::gray(10, 10);
        let mut a = ImageBuf::new(spec, InitializePixels::No);

        for y in 0..10 {
            for x in 0..10 {
                a.setpixel(x, y, 0, &[0.3]);
            }
        }

        let result = add(&a, 0.2f32, None);
        let mut pixel = [0.0f32];
        result.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_mul() {
        let spec = ImageSpec::gray(10, 10);
        let mut a = ImageBuf::new(spec, InitializePixels::No);

        for y in 0..10 {
            for x in 0..10 {
                a.setpixel(x, y, 0, &[0.5]);
            }
        }

        let result = mul(&a, 2.0f32, None);
        let mut pixel = [0.0f32];
        result.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_clamp() {
        let spec = ImageSpec::gray(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        src.setpixel(0, 0, 0, &[-0.5]);  // Below min
        src.setpixel(1, 0, 0, &[0.5]);   // In range
        src.setpixel(2, 0, 0, &[1.5]);   // Above max

        let result = clamp(&src, &[0.0], &[1.0], None);

        let mut pixel = [0.0f32];
        result.getpixel(0, 0, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.0).abs() < 0.001);

        result.getpixel(1, 0, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.5).abs() < 0.001);

        result.getpixel(2, 0, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_invert() {
        let spec = ImageSpec::gray(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        for y in 0..10 {
            for x in 0..10 {
                src.setpixel(x, y, 0, &[0.3]);
            }
        }

        let result = invert(&src, None);
        let mut pixel = [0.0f32];
        result.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_pow() {
        let spec = ImageSpec::gray(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        for y in 0..10 {
            for x in 0..10 {
                src.setpixel(x, y, 0, &[0.5]);
            }
        }

        // 0.5^2 = 0.25
        let result = pow(&src, &[2.0], None);
        let mut pixel = [0.0f32];
        result.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.25).abs() < 0.001);
    }
}
