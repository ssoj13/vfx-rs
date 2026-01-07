//! Compositing operations for ImageBuf.
//!
//! This module provides Porter-Duff compositing operations and blend modes:
//! - [`over`] - A over B (standard alpha composite)
//! - [`under`] - A under B (B over A)
//! - [`in_op`] - A in B (A masked by B's alpha)
//! - [`out`] - A out B (A masked by inverse of B's alpha)
//! - [`atop`] - A atop B (A over B only where B has coverage)
//! - [`xor`] - A xor B (non-overlapping parts)
//! - [`screen`] - Screen blend mode
//! - [`overlay`] - Overlay blend mode
//! - [`multiply`] - Multiply blend mode
//! - [`add`] - Additive blend mode
//! - [`hardlight`] - Hard light blend mode
//! - [`softlight`] - Soft light blend mode
//! - [`difference`] - Difference blend mode
//! - [`exclusion`] - Exclusion blend mode
//! - [`colordodge`] - Color dodge blend mode
//! - [`colorburn`] - Color burn blend mode

use crate::imagebuf::{ImageBuf, InitializePixels, WrapMode};
use vfx_core::Roi3D;

// ============================================================================
// Porter-Duff Compositing Operations
// ============================================================================

/// Porter-Duff "over" compositing: A over B
///
/// The standard alpha composite operation.
/// Result = A + B * (1 - alpha_A)
pub fn over(a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));
    let spec = a.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    over_into(&mut dst, a, b, Some(roi));
    dst
}

/// Porter-Duff "over" into existing destination.
pub fn over_into(dst: &mut ImageBuf, a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) {
    composite_op(dst, a, b, roi, |pa, pb, alpha_a, alpha_b| {
        let inv_alpha_a = 1.0 - alpha_a;
        let out_alpha = alpha_a + alpha_b * inv_alpha_a;
        if out_alpha > 0.0 {
            (pa * alpha_a + pb * alpha_b * inv_alpha_a) / out_alpha
        } else {
            0.0
        }
    }, |alpha_a, alpha_b| alpha_a + alpha_b * (1.0 - alpha_a));
}

/// Porter-Duff "under" compositing: A under B (equivalent to B over A)
pub fn under(a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    over(b, a, roi)
}

/// Porter-Duff "under" into existing destination.
pub fn under_into(dst: &mut ImageBuf, a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) {
    over_into(dst, b, a, roi);
}

/// Porter-Duff "in" compositing: A in B
///
/// Result = A masked by B's alpha
pub fn in_op(a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));
    let spec = a.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    in_into(&mut dst, a, b, Some(roi));
    dst
}

/// Porter-Duff "in" into existing destination.
pub fn in_into(dst: &mut ImageBuf, a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) {
    composite_op(dst, a, b, roi, |pa, _pb, _alpha_a, alpha_b| {
        pa * alpha_b
    }, |alpha_a, alpha_b| alpha_a * alpha_b);
}

/// Porter-Duff "out" compositing: A out B
///
/// Result = A masked by (1 - B's alpha)
pub fn out(a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));
    let spec = a.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    out_into(&mut dst, a, b, Some(roi));
    dst
}

/// Porter-Duff "out" into existing destination.
pub fn out_into(dst: &mut ImageBuf, a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) {
    composite_op(dst, a, b, roi, |pa, _pb, _alpha_a, alpha_b| {
        pa * (1.0 - alpha_b)
    }, |alpha_a, alpha_b| alpha_a * (1.0 - alpha_b));
}

/// Porter-Duff "atop" compositing: A atop B
///
/// A over B, but only where B has coverage
pub fn atop(a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));
    let spec = a.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    atop_into(&mut dst, a, b, Some(roi));
    dst
}

/// Porter-Duff "atop" into existing destination.
pub fn atop_into(dst: &mut ImageBuf, a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) {
    composite_op(dst, a, b, roi, |pa, pb, alpha_a, alpha_b| {
        pa * alpha_b + pb * (1.0 - alpha_a)
    }, |_alpha_a, alpha_b| alpha_b);
}

/// Porter-Duff "xor" compositing: A xor B
///
/// Non-overlapping parts of both images
pub fn xor(a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));
    let spec = a.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    xor_into(&mut dst, a, b, Some(roi));
    dst
}

/// Porter-Duff "xor" into existing destination.
pub fn xor_into(dst: &mut ImageBuf, a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) {
    composite_op(dst, a, b, roi, |pa, pb, alpha_a, alpha_b| {
        pa * (1.0 - alpha_b) + pb * (1.0 - alpha_a)
    }, |alpha_a, alpha_b| alpha_a + alpha_b - 2.0 * alpha_a * alpha_b);
}

// ============================================================================
// Blend Modes
// ============================================================================

/// Screen blend mode: 1 - (1-A) * (1-B)
///
/// Lightens the image. Black has no effect, white produces white.
pub fn screen(a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));
    let spec = a.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    screen_into(&mut dst, a, b, Some(roi));
    dst
}

/// Screen blend into existing destination.
pub fn screen_into(dst: &mut ImageBuf, a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) {
    blend_op(dst, a, b, roi, |pa, pb| {
        1.0 - (1.0 - pa) * (1.0 - pb)
    });
}

/// Multiply blend mode: A * B
///
/// Darkens the image. White has no effect, black produces black.
pub fn multiply(a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));
    let spec = a.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    multiply_into(&mut dst, a, b, Some(roi));
    dst
}

/// Multiply blend into existing destination.
pub fn multiply_into(dst: &mut ImageBuf, a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) {
    blend_op(dst, a, b, roi, |pa, pb| pa * pb);
}

/// Overlay blend mode.
///
/// Combines multiply and screen. Dark areas get darker, light areas get lighter.
pub fn overlay(a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));
    let spec = a.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    overlay_into(&mut dst, a, b, Some(roi));
    dst
}

/// Overlay blend into existing destination.
pub fn overlay_into(dst: &mut ImageBuf, a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) {
    blend_op(dst, a, b, roi, |pa, pb| {
        if pb < 0.5 {
            2.0 * pa * pb
        } else {
            1.0 - 2.0 * (1.0 - pa) * (1.0 - pb)
        }
    });
}

/// Hard light blend mode.
///
/// Similar to overlay but uses the top layer for the condition check.
pub fn hardlight(a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));
    let spec = a.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    hardlight_into(&mut dst, a, b, Some(roi));
    dst
}

/// Hard light blend into existing destination.
pub fn hardlight_into(dst: &mut ImageBuf, a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) {
    blend_op(dst, a, b, roi, |pa, pb| {
        if pa < 0.5 {
            2.0 * pa * pb
        } else {
            1.0 - 2.0 * (1.0 - pa) * (1.0 - pb)
        }
    });
}

/// Soft light blend mode.
///
/// Gentler version of hard light.
pub fn softlight(a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));
    let spec = a.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    softlight_into(&mut dst, a, b, Some(roi));
    dst
}

/// Soft light blend into existing destination.
pub fn softlight_into(dst: &mut ImageBuf, a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) {
    blend_op(dst, a, b, roi, |pa, pb| {
        if pa < 0.5 {
            pb - (1.0 - 2.0 * pa) * pb * (1.0 - pb)
        } else {
            let d = if pb <= 0.25 {
                ((16.0 * pb - 12.0) * pb + 4.0) * pb
            } else {
                pb.sqrt()
            };
            pb + (2.0 * pa - 1.0) * (d - pb)
        }
    });
}

/// Difference blend mode: |A - B|
///
/// Subtractive blend, useful for comparing images.
pub fn difference(a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));
    let spec = a.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    difference_into(&mut dst, a, b, Some(roi));
    dst
}

/// Difference blend into existing destination.
pub fn difference_into(dst: &mut ImageBuf, a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) {
    blend_op(dst, a, b, roi, |pa, pb| (pa - pb).abs());
}

/// Exclusion blend mode: A + B - 2*A*B
///
/// Similar to difference but lower contrast.
pub fn exclusion(a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));
    let spec = a.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    exclusion_into(&mut dst, a, b, Some(roi));
    dst
}

/// Exclusion blend into existing destination.
pub fn exclusion_into(dst: &mut ImageBuf, a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) {
    blend_op(dst, a, b, roi, |pa, pb| pa + pb - 2.0 * pa * pb);
}

/// Color dodge blend mode: B / (1 - A)
///
/// Brightens the image.
pub fn colordodge(a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));
    let spec = a.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    colordodge_into(&mut dst, a, b, Some(roi));
    dst
}

/// Color dodge blend into existing destination.
pub fn colordodge_into(dst: &mut ImageBuf, a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) {
    blend_op(dst, a, b, roi, |pa, pb| {
        if pa >= 1.0 {
            1.0
        } else {
            (pb / (1.0 - pa)).min(1.0)
        }
    });
}

/// Color burn blend mode: 1 - (1 - B) / A
///
/// Darkens the image.
pub fn colorburn(a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));
    let spec = a.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    colorburn_into(&mut dst, a, b, Some(roi));
    dst
}

/// Color burn blend into existing destination.
pub fn colorburn_into(dst: &mut ImageBuf, a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) {
    blend_op(dst, a, b, roi, |pa, pb| {
        if pa <= 0.0 {
            0.0
        } else {
            (1.0 - (1.0 - pb) / pa).max(0.0)
        }
    });
}

/// Additive blend mode: A + B
///
/// Simple addition (clamped to valid range).
pub fn add_blend(a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));
    let spec = a.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    add_blend_into(&mut dst, a, b, Some(roi));
    dst
}

/// Additive blend into existing destination.
pub fn add_blend_into(dst: &mut ImageBuf, a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) {
    blend_op(dst, a, b, roi, |pa, pb| pa + pb);
}

// ============================================================================
// Generic Compositing Operation
// ============================================================================

/// Generic compositing operation.
fn composite_op<F, G>(
    dst: &mut ImageBuf,
    a: &ImageBuf,
    b: &ImageBuf,
    roi: Option<Roi3D>,
    color_blend: F,
    alpha_blend: G,
)
where
    F: Fn(f32, f32, f32, f32) -> f32,
    G: Fn(f32, f32) -> f32,
{
    let roi = roi.unwrap_or_else(|| dst.roi());
    let nch = dst.nchannels() as usize;
    let alpha_ch_a = a.spec().alpha_channel;
    let alpha_ch_b = b.spec().alpha_channel;

    let mut pixel_a = vec![0.0f32; nch];
    let mut pixel_b = vec![0.0f32; nch];
    let mut result = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                a.getpixel(x, y, z, &mut pixel_a, WrapMode::Black);
                b.getpixel(x, y, z, &mut pixel_b, WrapMode::Black);

                let alpha_a = if alpha_ch_a >= 0 && (alpha_ch_a as usize) < pixel_a.len() {
                    pixel_a[alpha_ch_a as usize]
                } else {
                    1.0
                };
                let alpha_b = if alpha_ch_b >= 0 && (alpha_ch_b as usize) < pixel_b.len() {
                    pixel_b[alpha_ch_b as usize]
                } else {
                    1.0
                };

                for c in 0..nch {
                    if (alpha_ch_a >= 0 && c == alpha_ch_a as usize) ||
                       (alpha_ch_b >= 0 && c == alpha_ch_b as usize) {
                        // Alpha channel
                        result[c] = alpha_blend(alpha_a, alpha_b);
                    } else {
                        // Color channel
                        result[c] = color_blend(pixel_a[c], pixel_b[c], alpha_a, alpha_b);
                    }
                }

                dst.setpixel(x, y, z, &result);
            }
        }
    }
}

/// Generic blend operation (simpler, no alpha handling).
fn blend_op<F>(
    dst: &mut ImageBuf,
    a: &ImageBuf,
    b: &ImageBuf,
    roi: Option<Roi3D>,
    blend: F,
)
where
    F: Fn(f32, f32) -> f32,
{
    let roi = roi.unwrap_or_else(|| dst.roi());
    let nch = dst.nchannels() as usize;

    let mut pixel_a = vec![0.0f32; nch];
    let mut pixel_b = vec![0.0f32; nch];
    let mut result = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                a.getpixel(x, y, z, &mut pixel_a, WrapMode::Black);
                b.getpixel(x, y, z, &mut pixel_b, WrapMode::Black);

                for c in 0..nch {
                    result[c] = blend(pixel_a[c], pixel_b[c]);
                }

                dst.setpixel(x, y, z, &result);
            }
        }
    }
}

// ============================================================================
// Z-Depth Compositing
// ============================================================================

/// Z-depth based compositing (zover).
///
/// Composites two images based on their Z-depth channels. The pixel with the
/// smaller Z value (closer to camera) is composited "over" the further one.
///
/// Both images must have an alpha channel and a Z channel.
///
/// # Arguments
/// * `a` - First image
/// * `b` - Second image
/// * `z_zeroisinf` - If true, treat Z=0 as infinity (far away)
/// * `roi` - Region of interest
///
/// # Example
/// ```ignore
/// use vfx_io::imagebuf::ImageBuf;
/// use vfx_io::imagebufalgo::zover;
///
/// let fg = ImageBuf::read("fg.exr").unwrap();  // Has RGBA + Z
/// let bg = ImageBuf::read("bg.exr").unwrap();  // Has RGBA + Z
/// let composite = zover(&fg, &bg, false, None);
/// ```
pub fn zover(a: &ImageBuf, b: &ImageBuf, z_zeroisinf: bool, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));
    let spec = a.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    zover_into(&mut dst, a, b, z_zeroisinf, Some(roi));
    dst
}

/// Z-depth compositing into existing destination.
pub fn zover_into(dst: &mut ImageBuf, a: &ImageBuf, b: &ImageBuf, z_zeroisinf: bool, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| dst.roi());
    let nch = dst.nchannels() as usize;

    // Find alpha and Z channel indices
    // Default to channel 3 for alpha (RGBA) and channel 4 for Z (RGBAZ)
    let alpha_channel = if nch >= 4 { 3 } else { nch.saturating_sub(1) };
    let z_channel = if nch >= 5 { 4 } else { nch }; // If no Z channel, use out of bounds
    let has_z = z_channel < nch;

    let mut pixel_a = vec![0.0f32; nch];
    let mut pixel_b = vec![0.0f32; nch];
    let mut result = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                a.getpixel(x, y, z, &mut pixel_a, WrapMode::Black);
                b.getpixel(x, y, z, &mut pixel_b, WrapMode::Black);

                // Determine which is closer based on Z
                let a_is_closer = if has_z {
                    let mut az = pixel_a[z_channel];
                    let mut bz = pixel_b[z_channel];

                    if z_zeroisinf {
                        if az == 0.0 { az = f32::MAX; }
                        if bz == 0.0 { bz = f32::MAX; }
                    }

                    az <= bz
                } else {
                    true // Default: A over B
                };

                if a_is_closer {
                    // A over B
                    let alpha = pixel_a[alpha_channel].clamp(0.0, 1.0);
                    let one_minus_alpha = 1.0 - alpha;

                    for c in 0..nch {
                        result[c] = pixel_a[c] + one_minus_alpha * pixel_b[c];
                    }

                    if has_z {
                        result[z_channel] = if alpha != 0.0 { pixel_a[z_channel] } else { pixel_b[z_channel] };
                    }
                } else {
                    // B over A
                    let alpha = pixel_b[alpha_channel].clamp(0.0, 1.0);
                    let one_minus_alpha = 1.0 - alpha;

                    for c in 0..nch {
                        result[c] = pixel_b[c] + one_minus_alpha * pixel_a[c];
                    }

                    if has_z {
                        result[z_channel] = if alpha != 0.0 { pixel_b[z_channel] } else { pixel_a[z_channel] };
                    }
                }

                dst.setpixel(x, y, z, &result);
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_image(r: f32, g: f32, b: f32, a: f32) -> ImageBuf {
        let mut spec = ImageSpec::rgba(10, 10);
        spec.alpha_channel = 3;
        let mut buf = ImageBuf::new(spec, InitializePixels::No);
        for y in 0..10 {
            for x in 0..10 {
                buf.setpixel(x, y, 0, &[r, g, b, a]);
            }
        }
        buf
    }

    #[test]
    fn test_over() {
        let a = make_test_image(1.0, 0.0, 0.0, 0.5); // Red at 50%
        let b = make_test_image(0.0, 1.0, 0.0, 1.0); // Green at 100%

        let result = over(&a, &b, None);

        let mut pixel = [0.0f32; 4];
        result.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);

        // Red over green: should be mix of red and green
        // out_alpha = 0.5 + 1.0 * 0.5 = 1.0
        // r = (1.0 * 0.5 + 0.0 * 1.0 * 0.5) / 1.0 = 0.5
        // g = (0.0 * 0.5 + 1.0 * 1.0 * 0.5) / 1.0 = 0.5
        assert!((pixel[0] - 0.5).abs() < 0.01);
        assert!((pixel[1] - 0.5).abs() < 0.01);
        assert!((pixel[3] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_in_op() {
        let a = make_test_image(1.0, 0.0, 0.0, 1.0); // Red at 100%
        let b = make_test_image(0.0, 1.0, 0.0, 0.5); // Green at 50%

        let result = in_op(&a, &b, None);

        let mut pixel = [0.0f32; 4];
        result.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);

        // A "in" B: A masked by B's alpha
        // Result alpha = 1.0 * 0.5 = 0.5
        assert!((pixel[0] - 0.5).abs() < 0.01); // Red * 0.5
        assert!((pixel[3] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_screen() {
        let spec = ImageSpec::gray(10, 10);
        let mut a = ImageBuf::new(spec.clone(), InitializePixels::No);
        let mut b = ImageBuf::new(spec, InitializePixels::No);

        a.setpixel(5, 5, 0, &[0.5]);
        b.setpixel(5, 5, 0, &[0.5]);

        let result = screen(&a, &b, None);

        let mut pixel = [0.0f32];
        result.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);

        // Screen: 1 - (1-0.5) * (1-0.5) = 1 - 0.25 = 0.75
        assert!((pixel[0] - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_multiply() {
        let spec = ImageSpec::gray(10, 10);
        let mut a = ImageBuf::new(spec.clone(), InitializePixels::No);
        let mut b = ImageBuf::new(spec, InitializePixels::No);

        a.setpixel(5, 5, 0, &[0.5]);
        b.setpixel(5, 5, 0, &[0.5]);

        let result = multiply(&a, &b, None);

        let mut pixel = [0.0f32];
        result.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);

        // Multiply: 0.5 * 0.5 = 0.25
        assert!((pixel[0] - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_difference() {
        let spec = ImageSpec::gray(10, 10);
        let mut a = ImageBuf::new(spec.clone(), InitializePixels::No);
        let mut b = ImageBuf::new(spec, InitializePixels::No);

        a.setpixel(5, 5, 0, &[0.8]);
        b.setpixel(5, 5, 0, &[0.3]);

        let result = difference(&a, &b, None);

        let mut pixel = [0.0f32];
        result.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);

        // Difference: |0.8 - 0.3| = 0.5
        assert!((pixel[0] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_overlay() {
        let spec = ImageSpec::gray(10, 10);
        let mut a = ImageBuf::new(spec.clone(), InitializePixels::No);
        let mut b = ImageBuf::new(spec, InitializePixels::No);

        // Test with base < 0.5
        a.setpixel(0, 0, 0, &[0.5]);
        b.setpixel(0, 0, 0, &[0.3]);

        // Test with base > 0.5
        a.setpixel(1, 0, 0, &[0.5]);
        b.setpixel(1, 0, 0, &[0.7]);

        let result = overlay(&a, &b, None);

        let mut pixel = [0.0f32];

        // b < 0.5: 2 * 0.5 * 0.3 = 0.3
        result.getpixel(0, 0, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.3).abs() < 0.01);

        // b > 0.5: 1 - 2 * (1 - 0.5) * (1 - 0.7) = 1 - 2 * 0.5 * 0.3 = 0.7
        result.getpixel(1, 0, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.7).abs() < 0.01);
    }
}
