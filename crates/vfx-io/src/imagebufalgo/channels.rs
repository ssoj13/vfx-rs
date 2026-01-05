//! Channel manipulation functions for ImageBuf.
//!
//! This module provides functions for manipulating image channels:
//! - [`channels`] - Shuffle/reorder/extract channels
//! - [`channel_append`] - Append channels from one image to another
//! - [`channel_sum`] - Weighted sum of channels (e.g., luminance)

use crate::imagebuf::{ImageBuf, InitializePixels, WrapMode};
use vfx_core::{ImageSpec, Roi3D};

/// Creates a new image with reordered/selected channels.
///
/// # Arguments
///
/// * `src` - Source image
/// * `channel_order` - New channel order (indices into src channels, or -1 for fill value)
/// * `channel_values` - Values to use for -1 channel indices
/// * `roi` - Optional region (defaults to entire image)
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::channels;
///
/// // Extract just RGB from RGBA
/// let rgb = channels(&rgba_image, &[0, 1, 2], &[], None);
///
/// // Swap red and blue
/// let bgr = channels(&rgb_image, &[2, 1, 0], &[], None);
///
/// // Add alpha channel filled with 1.0
/// let rgba = channels(&rgb_image, &[0, 1, 2, -1], &[1.0], None);
/// ```
pub fn channels(
    src: &ImageBuf,
    channel_order: &[i32],
    channel_values: &[f32],
    roi: Option<Roi3D>,
) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let new_nch = channel_order.len();

    let mut spec = src.spec().clone();
    spec.nchannels = new_nch.min(255) as u8;

    // Build new channel names
    let src_names = &spec.channel_names;
    let new_names: Vec<String> = channel_order
        .iter()
        .enumerate()
        .map(|(i, &ch)| {
            if ch >= 0 && (ch as usize) < src_names.len() {
                src_names[ch as usize].clone()
            } else {
                format!("channel{}", i)
            }
        })
        .collect();
    spec.channel_names = new_names;

    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    channels_into(&mut dst, src, channel_order, channel_values, Some(roi));
    dst
}

/// Shuffles channels from src into dst.
pub fn channels_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    channel_order: &[i32],
    channel_values: &[f32],
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let src_nch = src.nchannels() as usize;
    let dst_nch = dst.nchannels() as usize;

    let mut src_pixel = vec![0.0f32; src_nch];
    let mut dst_pixel = vec![0.0f32; dst_nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut src_pixel, WrapMode::Black);

                for (dst_ch, &src_ch) in channel_order.iter().enumerate() {
                    if dst_ch >= dst_nch {
                        break;
                    }

                    if src_ch >= 0 && (src_ch as usize) < src_nch {
                        dst_pixel[dst_ch] = src_pixel[src_ch as usize];
                    } else {
                        // Use fill value
                        let fill_idx = (-src_ch - 1) as usize;
                        dst_pixel[dst_ch] = channel_values.get(fill_idx).copied().unwrap_or(0.0);
                    }
                }

                dst.setpixel(x, y, z, &dst_pixel);
            }
        }
    }
}

/// Appends channels from image B to image A.
///
/// # Arguments
///
/// * `a` - First image (channels come first)
/// * `b` - Second image (channels come after A's)
/// * `roi` - Optional region (defaults to union of A and B)
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::channel_append;
///
/// // Combine RGB with a separate alpha channel
/// let rgba = channel_append(&rgb, &alpha, None);
/// ```
pub fn channel_append(a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));

    let a_nch = a.nchannels() as usize;
    let b_nch = b.nchannels() as usize;
    let total_nch = a_nch + b_nch;

    let mut spec = a.spec().clone();
    spec.nchannels = total_nch.min(255) as u8;

    // Combine channel names
    let a_names = &a.spec().channel_names;
    let b_names = &b.spec().channel_names;
    let mut new_names: Vec<String> = a_names.clone();
    new_names.extend(b_names.clone());
    spec.channel_names = new_names;

    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    channel_append_into(&mut dst, a, b, Some(roi));
    dst
}

/// Appends channels from B to A into dst.
pub fn channel_append_into(dst: &mut ImageBuf, a: &ImageBuf, b: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));

    let a_nch = a.nchannels() as usize;
    let b_nch = b.nchannels() as usize;
    let dst_nch = dst.nchannels() as usize;

    let mut a_pixel = vec![0.0f32; a_nch];
    let mut b_pixel = vec![0.0f32; b_nch];
    let mut dst_pixel = vec![0.0f32; dst_nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                a.getpixel(x, y, z, &mut a_pixel, WrapMode::Black);
                b.getpixel(x, y, z, &mut b_pixel, WrapMode::Black);

                // Copy A's channels
                for (i, &v) in a_pixel.iter().enumerate() {
                    if i < dst_nch {
                        dst_pixel[i] = v;
                    }
                }

                // Copy B's channels
                for (i, &v) in b_pixel.iter().enumerate() {
                    let dst_idx = a_nch + i;
                    if dst_idx < dst_nch {
                        dst_pixel[dst_idx] = v;
                    }
                }

                dst.setpixel(x, y, z, &dst_pixel);
            }
        }
    }
}

/// Computes weighted sum of channels, producing a single-channel result.
///
/// # Arguments
///
/// * `src` - Source image
/// * `weights` - Weight for each channel (last weight repeats for missing)
/// * `roi` - Optional region
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::channel_sum;
///
/// // Compute luminance from RGB
/// let luma = channel_sum(&rgb, &[0.2126, 0.7152, 0.0722], None);
/// ```
pub fn channel_sum(src: &ImageBuf, weights: &[f32], roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());

    let mut spec = src.spec().clone();
    spec.nchannels = 1;
    spec.channel_names = vec!["Y".to_string()];

    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    channel_sum_into(&mut dst, src, weights, Some(roi));
    dst
}

/// Computes weighted sum of channels into dst.
pub fn channel_sum_into(dst: &mut ImageBuf, src: &ImageBuf, weights: &[f32], roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let src_nch = src.nchannels() as usize;

    // Expand weights
    let w: Vec<f32> = (0..src_nch)
        .map(|c| weights.get(c).copied().unwrap_or_else(|| weights.last().copied().unwrap_or(1.0)))
        .collect();

    let mut src_pixel = vec![0.0f32; src_nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut src_pixel, WrapMode::Black);

                let sum: f32 = src_pixel.iter().zip(w.iter()).map(|(&v, &wt)| v * wt).sum();
                dst.setpixel(x, y, z, &[sum]);
            }
        }
    }
}

/// Extracts a single channel from an image.
///
/// # Arguments
///
/// * `src` - Source image
/// * `channel` - Channel index to extract
/// * `roi` - Optional region
pub fn extract_channel(src: &ImageBuf, channel: usize, roi: Option<Roi3D>) -> ImageBuf {
    channels(src, &[channel as i32], &[], roi)
}

/// Flattens a multi-channel image to a single channel by averaging.
pub fn flatten(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let nch = src.nchannels() as usize;
    let weights = vec![1.0 / nch as f32; nch];
    channel_sum(src, &weights, roi)
}

/// Creates an alpha channel from an existing image.
///
/// Returns the alpha channel if it exists, otherwise returns a fully opaque channel.
pub fn get_alpha(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let spec = src.spec();
    let alpha_ch = spec.alpha_channel;

    if alpha_ch >= 0 {
        extract_channel(src, alpha_ch as usize, roi)
    } else {
        // No alpha, create fully opaque
        let roi = roi.unwrap_or_else(|| src.roi());
        let mut new_spec = ImageSpec::gray(roi.width() as u32, roi.height() as u32);
        new_spec.channel_names = vec!["A".to_string()];
        let buf = ImageBuf::new(new_spec, InitializePixels::No);
        super::patterns::fill_into(&mut { buf.clone() }, &[1.0], Some(roi));
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channels_extract() {
        let spec = ImageSpec::rgba(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);
        src.setpixel(0, 0, 0, &[1.0, 0.5, 0.25, 1.0]);

        // Extract just R, G
        let rg = channels(&src, &[0, 1], &[], None);
        assert_eq!(rg.nchannels(), 2);

        let mut pixel = [0.0f32; 2];
        rg.getpixel(0, 0, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 1.0).abs() < 0.001);
        assert!((pixel[1] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_channels_swap() {
        let spec = ImageSpec::rgb(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);
        src.setpixel(0, 0, 0, &[1.0, 0.5, 0.25]);

        // Swap to BGR
        let bgr = channels(&src, &[2, 1, 0], &[], None);

        let mut pixel = [0.0f32; 3];
        bgr.getpixel(0, 0, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.25).abs() < 0.001); // B
        assert!((pixel[1] - 0.5).abs() < 0.001);  // G
        assert!((pixel[2] - 1.0).abs() < 0.001);  // R
    }

    #[test]
    fn test_channel_append() {
        let spec_rgb = ImageSpec::rgb(10, 10);
        let spec_a = ImageSpec::gray(10, 10);

        let mut rgb = ImageBuf::new(spec_rgb, InitializePixels::No);
        let mut alpha = ImageBuf::new(spec_a, InitializePixels::No);

        rgb.setpixel(0, 0, 0, &[1.0, 0.5, 0.25]);
        alpha.setpixel(0, 0, 0, &[0.8]);

        let rgba = channel_append(&rgb, &alpha, None);
        assert_eq!(rgba.nchannels(), 4);

        let mut pixel = [0.0f32; 4];
        rgba.getpixel(0, 0, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 1.0).abs() < 0.001);
        assert!((pixel[1] - 0.5).abs() < 0.001);
        assert!((pixel[2] - 0.25).abs() < 0.001);
        assert!((pixel[3] - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_channel_sum() {
        let spec = ImageSpec::rgb(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);
        src.setpixel(0, 0, 0, &[0.2, 0.5, 0.3]);

        // Equal weights
        let luma = channel_sum(&src, &[1.0 / 3.0], None);
        assert_eq!(luma.nchannels(), 1);

        let mut pixel = [0.0f32; 1];
        luma.getpixel(0, 0, 0, &mut pixel, WrapMode::Black);
        // (0.2 + 0.5 + 0.3) / 3 = 0.333...
        assert!((pixel[0] - 0.333).abs() < 0.01);
    }
}
