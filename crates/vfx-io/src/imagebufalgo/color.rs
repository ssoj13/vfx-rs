//! Color manipulation and compositing functions for ImageBuf.
//!
//! This module provides functions for color operations:
//! - [`premult`] - Premultiply RGB by alpha
//! - [`unpremult`] - Divide RGB by alpha (unpremultiply)
//! - [`repremult`] - Undo unpremult and repremultiply
//! - [`saturate`] - Adjust color saturation
//! - [`contrast_remap`] - Contrast adjustment with optional sigmoidal curve
//! - [`color_map`] - Apply named color maps (heatmap, spectrum, etc.)
//! - [`colormatrixtransform`] - Apply 4x4 color matrix
//! - [`rangecompress`] / [`rangeexpand`] - Nonlinear range remapping
//! - [`srgb_to_linear`] / [`linear_to_srgb`] - sRGB gamma conversion

use crate::imagebuf::{ImageBuf, InitializePixels, WrapMode};
use vfx_core::{ImageSpec, Roi3D};
use vfx_core::pixel::{REC709_LUMA_R, REC709_LUMA_G, REC709_LUMA_B};

// ============================================================================
// Premultiply / Unpremultiply
// ============================================================================

/// Premultiply RGB channels by alpha: RGB *= A
///
/// This converts from "unassociated alpha" (straight alpha) to
/// "associated alpha" (premultiplied alpha).
///
/// No-op if there's no identified alpha channel.
pub fn premult(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let spec = src.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    premult_into(&mut dst, src, Some(roi));
    dst
}

/// Premultiply into existing destination buffer.
pub fn premult_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = dst.nchannels() as usize;
    let alpha_ch = src.spec().alpha_channel;

    if alpha_ch < 0 {
        // No alpha channel, just copy
        copy_pixels(dst, src, &roi);
        return;
    }

    let alpha_idx = alpha_ch as usize;
    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                let alpha = pixel.get(alpha_idx).copied().unwrap_or(1.0);

                // Multiply color channels by alpha
                for c in 0..nch {
                    if c != alpha_idx {
                        pixel[c] *= alpha;
                    }
                }

                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Unpremultiply RGB channels by alpha: RGB /= A
///
/// This converts from "associated alpha" (premultiplied) to
/// "unassociated alpha" (straight alpha).
///
/// No-op if there's no identified alpha channel.
pub fn unpremult(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let spec = src.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    unpremult_into(&mut dst, src, Some(roi));
    dst
}

/// Unpremultiply into existing destination buffer.
pub fn unpremult_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = dst.nchannels() as usize;
    let alpha_ch = src.spec().alpha_channel;

    if alpha_ch < 0 {
        copy_pixels(dst, src, &roi);
        return;
    }

    let alpha_idx = alpha_ch as usize;
    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                let alpha = pixel.get(alpha_idx).copied().unwrap_or(1.0);

                if alpha > 0.0 {
                    let inv_alpha = 1.0 / alpha;
                    for c in 0..nch {
                        if c != alpha_idx {
                            pixel[c] *= inv_alpha;
                        }
                    }
                }
                // If alpha == 0, leave RGB as-is (0/0 case)

                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Repremultiply: undo unpremult and repremultiply.
///
/// Useful when you've done operations on unpremultiplied data
/// and want to go back to premultiplied while preserving any
/// modifications to the alpha channel.
pub fn repremult(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let spec = src.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    repremult_into(&mut dst, src, Some(roi));
    dst
}

/// Repremultiply into existing destination buffer.
pub fn repremult_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    // Repremult is the same as premult for our purposes
    premult_into(dst, src, roi);
}

// ============================================================================
// Saturation
// ============================================================================

/// Adjust color saturation.
///
/// # Arguments
///
/// * `src` - Source image
/// * `scale` - Saturation scale: 0.0 = grayscale, 1.0 = unchanged, >1 = more saturated
/// * `firstchannel` - First channel to process (default 0 for RGB)
/// * `roi` - Optional region of interest
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::color::saturate;
///
/// // Desaturate to grayscale
/// let gray = saturate(&image, 0.0, 0, None);
///
/// // Increase saturation by 50%
/// let vivid = saturate(&image, 1.5, 0, None);
/// ```
pub fn saturate(
    src: &ImageBuf,
    scale: f32,
    firstchannel: usize,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let spec = src.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    saturate_into(&mut dst, src, scale, firstchannel, Some(roi));
    dst
}

/// Adjust saturation into existing destination buffer.
pub fn saturate_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    scale: f32,
    firstchannel: usize,
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = dst.nchannels() as usize;


    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                // Calculate luminance from the three channels
                let r = pixel.get(firstchannel).copied().unwrap_or(0.0);
                let g = pixel.get(firstchannel + 1).copied().unwrap_or(0.0);
                let b = pixel.get(firstchannel + 2).copied().unwrap_or(0.0);

                let lum = r * REC709_LUMA_R + g * REC709_LUMA_G + b * REC709_LUMA_B;

                // Interpolate between luminance and original color
                if firstchannel < nch {
                    pixel[firstchannel] = lum + (r - lum) * scale;
                }
                if firstchannel + 1 < nch {
                    pixel[firstchannel + 1] = lum + (g - lum) * scale;
                }
                if firstchannel + 2 < nch {
                    pixel[firstchannel + 2] = lum + (b - lum) * scale;
                }

                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

// ============================================================================
// Contrast Remap
// ============================================================================

/// Contrast remap with optional sigmoidal curve.
///
/// Transforms pixel values from domain [black, white] to range [min, max],
/// with optional sigmoidal contrast curve.
///
/// # Arguments
///
/// * `src` - Source image
/// * `black` - Input black point (maps to min)
/// * `white` - Input white point (maps to max)
/// * `min` - Output minimum
/// * `max` - Output maximum
/// * `scontrast` - Sigmoidal contrast (1.0 = linear, >1 = steeper curve)
/// * `sthresh` - Sigmoidal threshold (pivot point, default 0.5)
/// * `roi` - Optional region of interest
pub fn contrast_remap(
    src: &ImageBuf,
    black: f32,
    white: f32,
    min: f32,
    max: f32,
    scontrast: f32,
    sthresh: f32,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let spec = src.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    contrast_remap_into(&mut dst, src, black, white, min, max, scontrast, sthresh, Some(roi));
    dst
}

/// Contrast remap into existing destination buffer.
#[allow(clippy::too_many_arguments)]
pub fn contrast_remap_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    black: f32,
    white: f32,
    min: f32,
    max: f32,
    scontrast: f32,
    sthresh: f32,
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = dst.nchannels() as usize;

    let range = white - black;
    let out_range = max - min;

    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                for c in 0..nch {
                    let v = pixel[c];

                    // Step 1: Linear rescale [black, white] -> [0, 1]
                    let t = if range.abs() > 1e-10 {
                        (v - black) / range
                    } else {
                        // Binary threshold if black == white
                        if v >= black { 1.0 } else { 0.0 }
                    };

                    // Step 2: Apply sigmoidal contrast if scontrast != 1
                    let s = if (scontrast - 1.0).abs() < 1e-6 {
                        t
                    } else {
                        sigmoid_contrast(t, scontrast, sthresh)
                    };

                    // Step 3: Rescale [0, 1] -> [min, max]
                    pixel[c] = min + s * out_range;
                }

                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Sigmoidal contrast function.
fn sigmoid_contrast(x: f32, contrast: f32, threshold: f32) -> f32 {
    // Attempt to match OIIO's sigmoidal contrast
    // Using a smooth step function based on logistic curve
    let x_shifted = (x - threshold) * contrast;
    let sig = 1.0 / (1.0 + (-x_shifted).exp());

    // Normalize so that 0 maps to 0 and 1 maps to 1
    let sig_0 = 1.0 / (1.0 + (threshold * contrast).exp());
    let sig_1 = 1.0 / (1.0 + ((threshold - 1.0) * contrast).exp());

    (sig - sig_0) / (sig_1 - sig_0)
}

// ============================================================================
// Color Maps
// ============================================================================

/// Named color maps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMapName {
    /// Inferno colormap (perceptually uniform, dark-to-bright)
    #[default]
    Inferno,
    /// Viridis colormap (perceptually uniform, blue-green-yellow)
    Viridis,
    /// Turbo colormap (rainbow-like but perceptually improved)
    Turbo,
    /// Magma colormap (perceptually uniform, dark purple to bright yellow)
    Magma,
    /// Plasma colormap (perceptually uniform, purple to yellow)
    Plasma,
    /// Blue-Red diverging colormap
    BlueRed,
    /// Heat colormap (black-red-yellow-white)
    Heat,
    /// Spectrum/Rainbow colormap
    Spectrum,
}

impl ColorMapName {
    /// Parse color map name from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "inferno" => Some(Self::Inferno),
            "viridis" => Some(Self::Viridis),
            "turbo" => Some(Self::Turbo),
            "magma" => Some(Self::Magma),
            "plasma" => Some(Self::Plasma),
            "blue-red" | "bluered" => Some(Self::BlueRed),
            "heat" => Some(Self::Heat),
            "spectrum" | "rainbow" => Some(Self::Spectrum),
            _ => None,
        }
    }
}

/// Apply a named color map to a single-channel image.
///
/// Maps input values [0, 1] to RGB colors based on the selected colormap.
///
/// # Arguments
///
/// * `src` - Source image
/// * `srcchannel` - Source channel to use as input (or -1 for luminance)
/// * `map` - Color map to apply
/// * `roi` - Optional region of interest
pub fn color_map(
    src: &ImageBuf,
    srcchannel: i32,
    map: ColorMapName,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());

    // Output is always 3-channel RGB
    let mut spec = ImageSpec::new(
        roi.width() as u32,
        roi.height() as u32,
        3,
        vfx_core::DataFormat::F32,
    );
    spec.channel_names = vec!["R".to_string(), "G".to_string(), "B".to_string()];

    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    color_map_into(&mut dst, src, srcchannel, map, Some(roi));
    dst
}

/// Apply color map into existing destination buffer.
pub fn color_map_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    srcchannel: i32,
    map: ColorMapName,
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let src_nch = src.nchannels() as usize;


    let mut src_pixel = vec![0.0f32; src_nch];
    let mut dst_pixel = [0.0f32; 3];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut src_pixel, WrapMode::Black);

                // Get the input value
                let val = if srcchannel < 0 {
                    // Compute luminance
                    let r = src_pixel.first().copied().unwrap_or(0.0);
                    let g = src_pixel.get(1).copied().unwrap_or(r);
                    let b = src_pixel.get(2).copied().unwrap_or(r);
                    r * REC709_LUMA_R + g * REC709_LUMA_G + b * REC709_LUMA_B
                } else {
                    src_pixel.get(srcchannel as usize).copied().unwrap_or(0.0)
                };

                // Apply colormap
                apply_colormap(val, map, &mut dst_pixel);

                dst.setpixel(x, y, z, &dst_pixel);
            }
        }
    }
}

/// Apply a colormap to a value, producing RGB output.
fn apply_colormap(val: f32, map: ColorMapName, rgb: &mut [f32; 3]) {
    let t = val.clamp(0.0, 1.0);

    match map {
        ColorMapName::Heat => {
            // Black -> Red -> Yellow -> White
            if t < 0.333 {
                let s = t / 0.333;
                rgb[0] = s;
                rgb[1] = 0.0;
                rgb[2] = 0.0;
            } else if t < 0.666 {
                let s = (t - 0.333) / 0.333;
                rgb[0] = 1.0;
                rgb[1] = s;
                rgb[2] = 0.0;
            } else {
                let s = (t - 0.666) / 0.334;
                rgb[0] = 1.0;
                rgb[1] = 1.0;
                rgb[2] = s;
            }
        }
        ColorMapName::Spectrum => {
            // Rainbow: Red -> Orange -> Yellow -> Green -> Cyan -> Blue -> Violet
            let h = t * 300.0; // Hue from 0 to 300 degrees
            hsv_to_rgb(h, 1.0, 1.0, rgb);
        }
        ColorMapName::BlueRed => {
            // Diverging: Blue (0) -> White (0.5) -> Red (1)
            if t < 0.5 {
                let s = t * 2.0;
                rgb[0] = s;
                rgb[1] = s;
                rgb[2] = 1.0;
            } else {
                let s = (t - 0.5) * 2.0;
                rgb[0] = 1.0;
                rgb[1] = 1.0 - s;
                rgb[2] = 1.0 - s;
            }
        }
        ColorMapName::Inferno => {
            // Approximation of the inferno colormap
            inferno_colormap(t, rgb);
        }
        ColorMapName::Viridis => {
            // Approximation of the viridis colormap
            viridis_colormap(t, rgb);
        }
        ColorMapName::Turbo => {
            // Approximation of the turbo colormap
            turbo_colormap(t, rgb);
        }
        ColorMapName::Magma => {
            // Approximation of the magma colormap
            magma_colormap(t, rgb);
        }
        ColorMapName::Plasma => {
            // Approximation of the plasma colormap
            plasma_colormap(t, rgb);
        }
    }
}

/// Convert HSV to RGB.
fn hsv_to_rgb(h: f32, s: f32, v: f32, rgb: &mut [f32; 3]) {
    let c = v * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());

    let (r1, g1, b1) = if h_prime < 1.0 {
        (c, x, 0.0)
    } else if h_prime < 2.0 {
        (x, c, 0.0)
    } else if h_prime < 3.0 {
        (0.0, c, x)
    } else if h_prime < 4.0 {
        (0.0, x, c)
    } else if h_prime < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    let m = v - c;
    rgb[0] = r1 + m;
    rgb[1] = g1 + m;
    rgb[2] = b1 + m;
}

// Perceptually uniform colormaps (approximations)

fn inferno_colormap(t: f32, rgb: &mut [f32; 3]) {
    // Polynomial approximation of inferno
    rgb[0] = (-4.54 * t.powi(3) + 5.04 * t.powi(2) + 0.47 * t + 0.0).clamp(0.0, 1.0);
    rgb[1] = (-3.72 * t.powi(3) + 2.73 * t.powi(2) + 1.03 * t - 0.02).clamp(0.0, 1.0);
    rgb[2] = (4.26 * t.powi(3) - 5.67 * t.powi(2) + 1.71 * t + 0.03).clamp(0.0, 1.0);
}

fn viridis_colormap(t: f32, rgb: &mut [f32; 3]) {
    // Polynomial approximation of viridis
    rgb[0] = (0.28 + 0.14 * t - 0.68 * t.powi(2) + 1.78 * t.powi(3) - 0.53 * t.powi(4)).clamp(0.0, 1.0);
    rgb[1] = (0.0 + 1.41 * t - 0.89 * t.powi(2) + 0.48 * t.powi(3)).clamp(0.0, 1.0);
    rgb[2] = (0.33 + 1.26 * t - 2.98 * t.powi(2) + 1.74 * t.powi(3) - 0.31 * t.powi(4)).clamp(0.0, 1.0);
}

fn turbo_colormap(t: f32, rgb: &mut [f32; 3]) {
    // Simplified turbo approximation
    rgb[0] = (0.13 + 0.87 * (1.0 - ((t - 0.65).abs() * 3.0).min(1.0))).clamp(0.0, 1.0);
    rgb[1] = (0.13 + 0.87 * (1.0 - ((t - 0.50).abs() * 3.0).min(1.0))).clamp(0.0, 1.0);
    rgb[2] = (0.13 + 0.87 * (1.0 - ((t - 0.35).abs() * 3.0).min(1.0))).clamp(0.0, 1.0);
}

fn magma_colormap(t: f32, rgb: &mut [f32; 3]) {
    // Polynomial approximation
    rgb[0] = (-3.52 * t.powi(3) + 4.87 * t.powi(2) + 0.47 * t + 0.0).clamp(0.0, 1.0);
    rgb[1] = (-1.36 * t.powi(3) + 0.97 * t.powi(2) + 0.42 * t - 0.02).clamp(0.0, 1.0);
    rgb[2] = (4.98 * t.powi(3) - 6.33 * t.powi(2) + 1.89 * t + 0.14).clamp(0.0, 1.0);
}

fn plasma_colormap(t: f32, rgb: &mut [f32; 3]) {
    // Polynomial approximation
    rgb[0] = (0.05 + 0.82 * t + 0.21 * t.powi(2) - 0.08 * t.powi(3)).clamp(0.0, 1.0);
    rgb[1] = (0.0 + 0.63 * t - 0.41 * t.powi(2) + 0.78 * t.powi(3)).clamp(0.0, 1.0);
    rgb[2] = (0.53 - 0.15 * t - 0.85 * t.powi(2) + 0.55 * t.powi(3)).clamp(0.0, 1.0);
}

// ============================================================================
// Color Matrix Transform
// ============================================================================

/// Apply a 4x4 color matrix transformation.
///
/// Transforms each pixel as: color_out = color_in * matrix
/// Matrix is in row-major order (OpenGL style).
///
/// # Arguments
///
/// * `src` - Source image
/// * `matrix` - 4x4 color transformation matrix (16 floats, row-major)
/// * `unpremult` - If true, unpremultiply before transform, repremultiply after
/// * `roi` - Optional region of interest
pub fn colormatrixtransform(
    src: &ImageBuf,
    matrix: &[f32; 16],
    unpremult: bool,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let spec = src.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    colormatrixtransform_into(&mut dst, src, matrix, unpremult, Some(roi));
    dst
}

/// Apply color matrix transformation into existing destination.
pub fn colormatrixtransform_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    matrix: &[f32; 16],
    do_unpremult: bool,
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = dst.nchannels() as usize;
    let alpha_ch = src.spec().alpha_channel;

    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                let alpha = if alpha_ch >= 0 && (alpha_ch as usize) < nch {
                    pixel[alpha_ch as usize]
                } else {
                    1.0
                };

                // Unpremultiply if requested
                if do_unpremult && alpha > 0.0 {
                    let inv_alpha = 1.0 / alpha;
                    for c in 0..nch {
                        if alpha_ch < 0 || c != alpha_ch as usize {
                            pixel[c] *= inv_alpha;
                        }
                    }
                }

                // Apply matrix transform (assuming RGBA or at least RGB)
                let r = pixel.first().copied().unwrap_or(0.0);
                let g = pixel.get(1).copied().unwrap_or(0.0);
                let b = pixel.get(2).copied().unwrap_or(0.0);
                let a = pixel.get(3).copied().unwrap_or(1.0);

                // Matrix multiply: [r, g, b, a] * M
                let r_out = r * matrix[0] + g * matrix[4] + b * matrix[8] + a * matrix[12];
                let g_out = r * matrix[1] + g * matrix[5] + b * matrix[9] + a * matrix[13];
                let b_out = r * matrix[2] + g * matrix[6] + b * matrix[10] + a * matrix[14];
                let a_out = r * matrix[3] + g * matrix[7] + b * matrix[11] + a * matrix[15];

                if nch > 0 { pixel[0] = r_out; }
                if nch > 1 { pixel[1] = g_out; }
                if nch > 2 { pixel[2] = b_out; }
                if nch > 3 { pixel[3] = a_out; }

                // Repremultiply if we unpremultiplied
                if do_unpremult && alpha > 0.0 {
                    let new_alpha = if nch > 3 { pixel[3] } else { alpha };
                    for c in 0..nch.min(3) {
                        pixel[c] *= new_alpha;
                    }
                }

                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

// ============================================================================
// Range Compression / Expansion
// ============================================================================

/// Compress dynamic range for HDR processing.
///
/// Maps [0, infinity) to [0, 1) using logarithmic compression.
/// This is useful before operations that have artifacts with HDR values.
pub fn rangecompress(src: &ImageBuf, use_luma: bool, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let spec = src.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    rangecompress_into(&mut dst, src, use_luma, Some(roi));
    dst
}

/// Range compress into existing destination.
pub fn rangecompress_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    use_luma: bool,
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = dst.nchannels() as usize;

    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                if use_luma && nch >= 3 {
                    // Compress based on luminance (preserve hue)
                    let r = pixel[0];
                    let g = pixel[1];
                    let b = pixel[2];
                    let lum = r * REC709_LUMA_R + g * REC709_LUMA_G + b * REC709_LUMA_B;

                    if lum > 0.0 {
                        let compressed_lum = range_compress_value(lum);
                        let scale = compressed_lum / lum;
                        pixel[0] *= scale;
                        pixel[1] *= scale;
                        pixel[2] *= scale;
                    }
                } else {
                    // Compress each channel independently
                    for c in 0..nch {
                        pixel[c] = range_compress_value(pixel[c]);
                    }
                }

                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Expand dynamic range (inverse of rangecompress).
pub fn rangeexpand(src: &ImageBuf, use_luma: bool, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let spec = src.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    rangeexpand_into(&mut dst, src, use_luma, Some(roi));
    dst
}

/// Range expand into existing destination.
pub fn rangeexpand_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    use_luma: bool,
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = dst.nchannels() as usize;

    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                if use_luma && nch >= 3 {
                    let r = pixel[0];
                    let g = pixel[1];
                    let b = pixel[2];
                    let lum = r * REC709_LUMA_R + g * REC709_LUMA_G + b * REC709_LUMA_B;

                    if lum > 0.0 && lum < 1.0 {
                        let expanded_lum = range_expand_value(lum);
                        let scale = expanded_lum / lum;
                        pixel[0] *= scale;
                        pixel[1] *= scale;
                        pixel[2] *= scale;
                    }
                } else {
                    for c in 0..nch {
                        pixel[c] = range_expand_value(pixel[c]);
                    }
                }

                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Compress a single value from [0, infinity) to [0, 1).
fn range_compress_value(x: f32) -> f32 {
    if x <= 0.0 {
        0.0
    } else {
        // log1p(x) / (1 + log1p(x)) maps [0, inf) to [0, 1)
        let lx = (1.0 + x).ln();
        lx / (1.0 + lx)
    }
}

/// Expand a single value from [0, 1) to [0, infinity).
fn range_expand_value(y: f32) -> f32 {
    if y <= 0.0 {
        0.0
    } else if y >= 1.0 {
        // Avoid division by zero
        f32::MAX
    } else {
        // Inverse of range_compress_value
        let lx = y / (1.0 - y);
        lx.exp() - 1.0
    }
}

// ============================================================================
// sRGB Gamma Conversion
// ============================================================================

/// Convert sRGB to linear RGB.
///
/// Applies the inverse sRGB transfer function to convert from
/// sRGB gamma space to linear light.
pub fn srgb_to_linear(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let spec = src.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    srgb_to_linear_into(&mut dst, src, Some(roi));
    dst
}

/// Convert sRGB to linear into existing destination.
pub fn srgb_to_linear_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = dst.nchannels() as usize;
    let alpha_ch = src.spec().alpha_channel;

    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                for c in 0..nch {
                    // Don't convert alpha channel
                    if alpha_ch >= 0 && c == alpha_ch as usize {
                        continue;
                    }
                    pixel[c] = srgb_to_linear_value(pixel[c]);
                }

                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Convert linear RGB to sRGB.
///
/// Applies the sRGB transfer function to convert from
/// linear light to sRGB gamma space.
pub fn linear_to_srgb(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let spec = src.spec().clone();
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    linear_to_srgb_into(&mut dst, src, Some(roi));
    dst
}

/// Convert linear to sRGB into existing destination.
pub fn linear_to_srgb_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = dst.nchannels() as usize;
    let alpha_ch = src.spec().alpha_channel;

    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                for c in 0..nch {
                    if alpha_ch >= 0 && c == alpha_ch as usize {
                        continue;
                    }
                    pixel[c] = linear_to_srgb_value(pixel[c]);
                }

                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Convert a single sRGB value to linear.
fn srgb_to_linear_value(x: f32) -> f32 {
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

/// Convert a single linear value to sRGB.
fn linear_to_srgb_value(x: f32) -> f32 {
    if x <= 0.0031308 {
        x * 12.92
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Copy pixels from src to dst within the given ROI.
fn copy_pixels(dst: &mut ImageBuf, src: &ImageBuf, roi: &Roi3D) {
    let nch = dst.nchannels() as usize;
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use vfx_core::ImageSpec;

    #[test]
    fn test_premult_unpremult() {
        let mut spec = ImageSpec::rgba(10, 10);
        spec.alpha_channel = 3;

        let mut src = ImageBuf::new(spec, InitializePixels::No);

        // Set a pixel with unassociated alpha
        src.setpixel(5, 5, 0, &[1.0, 0.5, 0.0, 0.5]);

        // Premultiply
        let premultiplied = premult(&src, None);
        let mut pixel = [0.0f32; 4];
        premultiplied.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);

        // RGB should be halved
        assert!((pixel[0] - 0.5).abs() < 0.001);
        assert!((pixel[1] - 0.25).abs() < 0.001);
        assert!((pixel[2] - 0.0).abs() < 0.001);
        assert!((pixel[3] - 0.5).abs() < 0.001);

        // Unpremultiply should get back original
        let unpremultiplied = unpremult(&premultiplied, None);
        unpremultiplied.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);

        assert!((pixel[0] - 1.0).abs() < 0.001);
        assert!((pixel[1] - 0.5).abs() < 0.001);
        assert!((pixel[2] - 0.0).abs() < 0.001);
        assert!((pixel[3] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_saturate() {
        let spec = ImageSpec::rgb(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        // Set a colored pixel
        src.setpixel(5, 5, 0, &[1.0, 0.5, 0.0]);

        // Desaturate to grayscale
        let gray = saturate(&src, 0.0, 0, None);
        let mut pixel = [0.0f32; 3];
        gray.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);

        // All channels should be equal (luminance)
        assert!((pixel[0] - pixel[1]).abs() < 0.001);
        assert!((pixel[1] - pixel[2]).abs() < 0.001);
    }

    #[test]
    fn test_contrast_remap() {
        let spec = ImageSpec::gray(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        src.setpixel(0, 0, 0, &[0.0]);
        src.setpixel(1, 0, 0, &[0.5]);
        src.setpixel(2, 0, 0, &[1.0]);

        // Linear remap from [0,1] to [0.2, 0.8]
        let remapped = contrast_remap(&src, 0.0, 1.0, 0.2, 0.8, 1.0, 0.5, None);

        let mut pixel = [0.0f32];
        remapped.getpixel(0, 0, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.2).abs() < 0.001);

        remapped.getpixel(1, 0, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.5).abs() < 0.001);

        remapped.getpixel(2, 0, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_color_map_heat() {
        let spec = ImageSpec::gray(10, 1);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        // Gradient from 0 to 1
        for x in 0..10 {
            src.setpixel(x, 0, 0, &[x as f32 / 9.0]);
        }

        let mapped = color_map(&src, 0, ColorMapName::Heat, None);

        // Check that output is RGB
        assert_eq!(mapped.nchannels(), 3);

        // Black at 0
        let mut pixel = [0.0f32; 3];
        mapped.getpixel(0, 0, 0, &mut pixel, WrapMode::Black);
        assert!(pixel[0] < 0.2);
        assert!(pixel[1] < 0.1);
        assert!(pixel[2] < 0.1);

        // White at end
        mapped.getpixel(9, 0, 0, &mut pixel, WrapMode::Black);
        assert!(pixel[0] > 0.9);
        assert!(pixel[1] > 0.9);
        assert!(pixel[2] > 0.9);
    }

    #[test]
    fn test_srgb_linear_roundtrip() {
        let spec = ImageSpec::rgb(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        src.setpixel(5, 5, 0, &[0.5, 0.2, 0.8]);

        let linear = srgb_to_linear(&src, None);
        let back_to_srgb = linear_to_srgb(&linear, None);

        let mut original = [0.0f32; 3];
        let mut result = [0.0f32; 3];

        src.getpixel(5, 5, 0, &mut original, WrapMode::Black);
        back_to_srgb.getpixel(5, 5, 0, &mut result, WrapMode::Black);

        for c in 0..3 {
            assert!((original[c] - result[c]).abs() < 0.001);
        }
    }

    #[test]
    fn test_rangecompress_expand_roundtrip() {
        let spec = ImageSpec::rgb(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        // Test with HDR value
        src.setpixel(5, 5, 0, &[2.5, 0.5, 10.0]);

        let compressed = rangecompress(&src, false, None);
        let expanded = rangeexpand(&compressed, false, None);

        let mut original = [0.0f32; 3];
        let mut result = [0.0f32; 3];

        src.getpixel(5, 5, 0, &mut original, WrapMode::Black);
        expanded.getpixel(5, 5, 0, &mut result, WrapMode::Black);

        for c in 0..3 {
            assert!((original[c] - result[c]).abs() < 0.01);
        }
    }

    #[test]
    fn test_colormatrixtransform() {
        let spec = ImageSpec::rgb(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        src.setpixel(5, 5, 0, &[1.0, 0.0, 0.0]);

        // Identity matrix with red-blue swap
        #[rustfmt::skip]
        let swap_rb: [f32; 16] = [
            0.0, 0.0, 1.0, 0.0,  // R -> B
            0.0, 1.0, 0.0, 0.0,  // G -> G
            1.0, 0.0, 0.0, 0.0,  // B -> R
            0.0, 0.0, 0.0, 1.0,  // A -> A
        ];

        let result = colormatrixtransform(&src, &swap_rb, false, None);

        let mut pixel = [0.0f32; 3];
        result.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);

        // Red should become blue
        assert!((pixel[0] - 0.0).abs() < 0.001);
        assert!((pixel[1] - 0.0).abs() < 0.001);
        assert!((pixel[2] - 1.0).abs() < 0.001);
    }
}
