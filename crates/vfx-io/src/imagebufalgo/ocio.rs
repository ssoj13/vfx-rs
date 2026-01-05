//! OCIO color conversion functions for ImageBuf.
//!
//! This module provides OIIO-compatible color conversion functions that use
//! OpenColorIO for color space transforms.
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::imagebufalgo::{colorconvert, ociodisplay, ociolook};
//! use vfx_io::ColorConfig;
//!
//! let config = ColorConfig::aces_1_3();
//!
//! // Convert between color spaces
//! let result = colorconvert(&src, "ACEScg", "sRGB", Some(&config), None);
//!
//! // Apply display transform
//! let display = ociodisplay(&src, "sRGB", "Film", "ACEScg", Some(&config), None);
//!
//! // Apply look
//! let graded = ociolook(&src, "FilmGrade", "ACEScg", "ACEScg", Some(&config), None);
//! ```

use crate::colorconfig::ColorConfig;
use crate::imagebuf::{ImageBuf, InitializePixels, WrapMode};
use vfx_core::Roi3D;

/// Converts an image from one color space to another.
///
/// # Arguments
///
/// * `src` - Source image
/// * `from` - Source color space name
/// * `to` - Destination color space name
/// * `config` - Optional ColorConfig (uses default if None)
/// * `roi` - Optional region (defaults to entire image)
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::colorconvert;
///
/// let srgb = colorconvert(&linear, "ACEScg", "sRGB", None, None);
/// ```
pub fn colorconvert(
    src: &ImageBuf,
    from: &str,
    to: &str,
    config: Option<&ColorConfig>,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    colorconvert_into(&mut dst, src, from, to, config, Some(roi));
    dst
}

/// Converts an image from one color space to another, writing to dst.
///
/// # Arguments
///
/// * `dst` - Destination image
/// * `src` - Source image
/// * `from` - Source color space name
/// * `to` - Destination color space name
/// * `config` - Optional ColorConfig (uses default if None)
/// * `roi` - Optional region (defaults to entire image)
pub fn colorconvert_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    from: &str,
    to: &str,
    config: Option<&ColorConfig>,
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| src.roi());

    // Get or create config
    let default_config;
    let cfg = match config {
        Some(c) => c,
        None => {
            default_config = ColorConfig::new();
            &default_config
        }
    };

    // Create processor
    let processor = match cfg.processor(from, to) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("colorconvert failed to create processor: {}", e);
            // Fall back to copy
            copy_pixels(dst, src, roi);
            return;
        }
    };

    // Apply transform
    apply_processor(dst, src, &processor, roi);
}

/// Applies a display transform to an image.
///
/// # Arguments
///
/// * `src` - Source image
/// * `display` - Display name (e.g., "sRGB", "Rec.709")
/// * `view` - View name (e.g., "Film", "Raw")
/// * `from` - Source color space name
/// * `config` - Optional ColorConfig (uses default if None)
/// * `roi` - Optional region
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::ociodisplay;
///
/// let display = ociodisplay(&linear, "sRGB", "Film", "ACEScg", None, None);
/// ```
pub fn ociodisplay(
    src: &ImageBuf,
    display: &str,
    view: &str,
    from: &str,
    config: Option<&ColorConfig>,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    ociodisplay_into(&mut dst, src, display, view, from, config, Some(roi));
    dst
}

/// Applies a display transform, writing to dst.
pub fn ociodisplay_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    display: &str,
    view: &str,
    from: &str,
    config: Option<&ColorConfig>,
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| src.roi());

    // Get or create config
    let default_config;
    let cfg = match config {
        Some(c) => c,
        None => {
            default_config = ColorConfig::new();
            &default_config
        }
    };

    // Create display processor
    let processor = match cfg.display_processor(from, display, view) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("ociodisplay failed to create processor: {}", e);
            copy_pixels(dst, src, roi);
            return;
        }
    };

    apply_processor(dst, src, &processor, roi);
}

/// Applies a look transform to an image.
///
/// # Arguments
///
/// * `src` - Source image
/// * `looks` - Look specification (e.g., "+FilmGrade", "-ContrastBoost")
/// * `from` - Source color space name
/// * `to` - Destination color space name
/// * `config` - Optional ColorConfig
/// * `roi` - Optional region
///
/// # Look Syntax
///
/// - `+LookName` - Apply look forward
/// - `-LookName` - Apply look inverse
/// - Multiple looks separated by commas: `+GradeA, +GradeB`
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::ociolook;
///
/// let graded = ociolook(&src, "+ShowLUT", "ACEScg", "ACEScg", None, None);
/// ```
pub fn ociolook(
    src: &ImageBuf,
    looks: &str,
    from: &str,
    to: &str,
    config: Option<&ColorConfig>,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    ociolook_into(&mut dst, src, looks, from, to, config, Some(roi));
    dst
}

/// Applies a look transform, writing to dst.
pub fn ociolook_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    looks: &str,
    from: &str,
    to: &str,
    config: Option<&ColorConfig>,
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| src.roi());

    // Get or create config
    let default_config;
    let cfg = match config {
        Some(c) => c,
        None => {
            default_config = ColorConfig::new();
            &default_config
        }
    };

    // Create look processor
    let processor = match cfg.processor_with_looks(from, to, looks) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("ociolook failed to create processor: {}", e);
            copy_pixels(dst, src, roi);
            return;
        }
    };

    apply_processor(dst, src, &processor, roi);
}

/// Applies a file-based transform (LUT file).
///
/// # Arguments
///
/// * `src` - Source image
/// * `filename` - Path to LUT file (.cube, .csp, .clf, etc.)
/// * `inverse` - Apply inverse transform
/// * `config` - Optional ColorConfig (used for resolving paths)
/// * `roi` - Optional region
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::ociofiletransform;
///
/// let graded = ociofiletransform(&src, "grade.cube", false, None, None);
/// ```
pub fn ociofiletransform(
    src: &ImageBuf,
    filename: &str,
    inverse: bool,
    config: Option<&ColorConfig>,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut dst = ImageBuf::new(src.spec().clone(), InitializePixels::No);
    ociofiletransform_into(&mut dst, src, filename, inverse, config, Some(roi));
    dst
}

/// Applies a file-based transform, writing to dst.
pub fn ociofiletransform_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    filename: &str,
    inverse: bool,
    _config: Option<&ColorConfig>,
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| src.roi());

    // Create file transform
    use vfx_ocio::{FileTransform, Interpolation, Processor, Transform, TransformDirection};
    use std::path::PathBuf;

    let direction = if inverse {
        TransformDirection::Inverse
    } else {
        TransformDirection::Forward
    };

    let transform = Transform::FileTransform(FileTransform {
        src: PathBuf::from(filename),
        ccc_id: None,
        interpolation: Interpolation::Linear,
        direction,
    });

    let processor = match Processor::from_transform(&transform, TransformDirection::Forward) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("ociofiletransform failed: {}", e);
            copy_pixels(dst, src, roi);
            return;
        }
    };

    apply_processor(dst, src, &processor, roi);
}

/// Applies an OCIO processor to an image.
fn apply_processor(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    processor: &vfx_ocio::Processor,
    roi: Roi3D,
) {
    let src_nch = src.nchannels() as usize;
    let dst_nch = dst.nchannels() as usize;
    let min_nch = src_nch.min(dst_nch);

    // Process pixel by pixel
    let mut src_pixel = vec![0.0f32; src_nch];
    let mut dst_pixel = vec![0.0f32; dst_nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                // Get source pixel
                src.getpixel(x, y, z, &mut src_pixel, WrapMode::Black);

                // Copy to destination buffer (for processing)
                for i in 0..min_nch {
                    dst_pixel[i] = src_pixel[i];
                }

                // Apply color transform to RGB channels only
                if min_nch >= 3 {
                    // Extract RGB as a slice for processing
                    let rgb = [dst_pixel[0], dst_pixel[1], dst_pixel[2]];
                    let mut pixels = [rgb];
                    processor.apply_rgb(&mut pixels);
                    dst_pixel[0] = pixels[0][0];
                    dst_pixel[1] = pixels[0][1];
                    dst_pixel[2] = pixels[0][2];
                } else if min_nch >= 1 {
                    // For single-channel, process as gray (replicate)
                    let gray = dst_pixel[0];
                    let mut pixels = [[gray, gray, gray]];
                    processor.apply_rgb(&mut pixels);
                    // Take luminance
                    dst_pixel[0] = pixels[0][0] * 0.2126 + pixels[0][1] * 0.7152 + pixels[0][2] * 0.0722;
                }

                // Preserve alpha if present
                if min_nch >= 4 && src_nch >= 4 {
                    dst_pixel[3] = src_pixel[3];
                }

                dst.setpixel(x, y, z, &dst_pixel);
            }
        }
    }
}

/// Simple pixel copy fallback.
fn copy_pixels(dst: &mut ImageBuf, src: &ImageBuf, roi: Roi3D) {
    let src_nch = src.nchannels() as usize;
    let dst_nch = dst.nchannels() as usize;
    let min_nch = src_nch.min(dst_nch);

    let mut src_pixel = vec![0.0f32; src_nch];
    let mut dst_pixel = vec![0.0f32; dst_nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut src_pixel, WrapMode::Black);
                for i in 0..min_nch {
                    dst_pixel[i] = src_pixel[i];
                }
                dst.setpixel(x, y, z, &dst_pixel);
            }
        }
    }
}

/// Converts between color spaces in-place.
///
/// This is a convenience function that modifies the source image directly.
pub fn colorconvert_inplace(
    img: &mut ImageBuf,
    from: &str,
    to: &str,
    config: Option<&ColorConfig>,
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| img.roi());

    // Get or create config
    let default_config;
    let cfg = match config {
        Some(c) => c,
        None => {
            default_config = ColorConfig::new();
            &default_config
        }
    };

    // Create processor
    let processor = match cfg.processor(from, to) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("colorconvert_inplace failed to create processor: {}", e);
            return;
        }
    };

    let nch = img.nchannels() as usize;
    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                img.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                if nch >= 3 {
                    let mut pixels = [[pixel[0], pixel[1], pixel[2]]];
                    processor.apply_rgb(&mut pixels);
                    pixel[0] = pixels[0][0];
                    pixel[1] = pixels[0][1];
                    pixel[2] = pixels[0][2];
                }

                img.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Determines if two color spaces are equivalent.
///
/// Returns true if the two names refer to the same color space
/// (either directly or through aliases/roles).
pub fn equivalent_colorspace(name1: &str, name2: &str, config: Option<&ColorConfig>) -> bool {
    if name1.eq_ignore_ascii_case(name2) {
        return true;
    }

    let default_config;
    let cfg = match config {
        Some(c) => c,
        None => {
            default_config = ColorConfig::new();
            &default_config
        }
    };

    // Check if both resolve to the same color space
    let cs1 = cfg.inner().colorspace(name1);
    let cs2 = cfg.inner().colorspace(name2);

    match (cs1, cs2) {
        (Some(c1), Some(c2)) => c1.name().eq_ignore_ascii_case(c2.name()),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vfx_core::ImageSpec;

    #[test]
    fn test_colorconvert() {
        let spec = ImageSpec::rgb(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);
        src.setpixel(0, 0, 0, &[0.18, 0.18, 0.18]);

        let config = ColorConfig::aces_1_3();
        let dst = colorconvert(&src, "ACEScg", "sRGB", Some(&config), None);

        let mut pixel = [0.0f32; 3];
        dst.getpixel(0, 0, 0, &mut pixel, WrapMode::Black);

        // 0.18 ACEScg should convert to approximately 0.46 sRGB
        assert!(pixel[0] > 0.4 && pixel[0] < 0.5);
    }

    #[test]
    fn test_colorconvert_preserves_alpha() {
        let spec = ImageSpec::rgba(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);
        src.setpixel(0, 0, 0, &[0.5, 0.5, 0.5, 0.8]);

        let config = ColorConfig::aces_1_3();
        let dst = colorconvert(&src, "ACEScg", "sRGB", Some(&config), None);

        let mut pixel = [0.0f32; 4];
        dst.getpixel(0, 0, 0, &mut pixel, WrapMode::Black);

        // Alpha should be preserved
        assert!((pixel[3] - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_equivalent_colorspace() {
        let config = ColorConfig::aces_1_3();

        // Same name
        assert!(equivalent_colorspace("ACEScg", "ACEScg", Some(&config)));
        assert!(equivalent_colorspace("ACEScg", "acescg", Some(&config)));

        // Role resolves to color space
        assert!(equivalent_colorspace("scene_linear", "ACEScg", Some(&config)));
    }

    #[test]
    fn test_ociodisplay() {
        let spec = ImageSpec::rgb(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);
        src.setpixel(0, 0, 0, &[0.18, 0.18, 0.18]);

        let config = ColorConfig::aces_1_3();

        // This may fail if display doesn't exist, but shouldn't crash
        let _dst = ociodisplay(&src, "sRGB", "Film", "ACEScg", Some(&config), None);
    }

    #[test]
    fn test_colorconvert_inplace() {
        let spec = ImageSpec::rgb(10, 10);
        let mut img = ImageBuf::new(spec, InitializePixels::No);
        img.setpixel(0, 0, 0, &[0.18, 0.18, 0.18]);

        let config = ColorConfig::aces_1_3();
        colorconvert_inplace(&mut img, "ACEScg", "sRGB", Some(&config), None);

        let mut pixel = [0.0f32; 3];
        img.getpixel(0, 0, 0, &mut pixel, WrapMode::Black);

        // Should be converted
        assert!(pixel[0] > 0.4 && pixel[0] < 0.5);
    }
}
