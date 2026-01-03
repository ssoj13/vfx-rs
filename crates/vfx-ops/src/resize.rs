//! Image resize and resampling operations.
//!
//! Provides high-quality image scaling using various interpolation filters.
//!
//! # Filters
//!
//! - [`Filter::Nearest`] - Fastest, no interpolation (blocky)
//! - [`Filter::Bilinear`] - Linear interpolation (smooth but blurry)
//! - [`Filter::Bicubic`] - Cubic interpolation (sharper than bilinear)
//! - [`Filter::Lanczos3`] - High-quality sinc-based (best for downscaling)
//!
//! # Example
//!
//! ```rust
//! use vfx_ops::resize::{resize_f32, Filter};
//!
//! let src: Vec<f32> = vec![0.0; 64 * 64 * 4]; // 64x64 RGBA
//! let dst = resize_f32(&src, 64, 64, 4, 128, 128, Filter::Lanczos3);
//! ```

use crate::{OpsError, OpsResult};

/// Resampling filter for resize operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filter {
    /// Nearest-neighbor (fastest, no interpolation).
    Nearest,
    /// Bilinear interpolation (smooth, fast).
    Bilinear,
    /// Bicubic interpolation (sharper than bilinear).
    #[default]
    Bicubic,
    /// Lanczos-3 (high quality, best for downscaling).
    Lanczos3,
}

impl Filter {
    /// Returns the support radius for this filter.
    #[inline]
    pub fn support(&self) -> f32 {
        match self {
            Filter::Nearest => 0.5,
            Filter::Bilinear => 1.0,
            Filter::Bicubic => 2.0,
            Filter::Lanczos3 => 3.0,
        }
    }

    /// Evaluates the filter kernel at position x.
    #[inline]
    pub fn weight(&self, x: f32) -> f32 {
        match self {
            Filter::Nearest => nearest_weight(x),
            Filter::Bilinear => bilinear_weight(x),
            Filter::Bicubic => bicubic_weight(x),
            Filter::Lanczos3 => lanczos_weight(x, 3.0),
        }
    }
}

/// Nearest-neighbor weight function.
#[inline]
fn nearest_weight(x: f32) -> f32 {
    if x.abs() < 0.5 { 1.0 } else { 0.0 }
}

/// Bilinear (triangle) weight function.
#[inline]
fn bilinear_weight(x: f32) -> f32 {
    let ax = x.abs();
    if ax < 1.0 { 1.0 - ax } else { 0.0 }
}

/// Bicubic (Mitchell-Netravali) weight function.
#[inline]
fn bicubic_weight(x: f32) -> f32 {
    // Mitchell-Netravali with B=1/3, C=1/3
    const B: f32 = 1.0 / 3.0;
    const C: f32 = 1.0 / 3.0;

    let ax = x.abs();
    if ax < 1.0 {
        ((12.0 - 9.0 * B - 6.0 * C) * ax * ax * ax
            + (-18.0 + 12.0 * B + 6.0 * C) * ax * ax
            + (6.0 - 2.0 * B))
            / 6.0
    } else if ax < 2.0 {
        ((-B - 6.0 * C) * ax * ax * ax
            + (6.0 * B + 30.0 * C) * ax * ax
            + (-12.0 * B - 48.0 * C) * ax
            + (8.0 * B + 24.0 * C))
            / 6.0
    } else {
        0.0
    }
}

/// Lanczos weight function.
#[inline]
fn lanczos_weight(x: f32, a: f32) -> f32 {
    let ax = x.abs();
    if ax < 1e-8 {
        1.0
    } else if ax < a {
        let pi_x = std::f32::consts::PI * ax;
        let pi_x_a = pi_x / a;
        (pi_x.sin() / pi_x) * (pi_x_a.sin() / pi_x_a)
    } else {
        0.0
    }
}

/// Resizes f32 image data.
///
/// # Arguments
///
/// * `src` - Source pixel data
/// * `src_w` - Source width
/// * `src_h` - Source height
/// * `channels` - Number of channels (3 or 4)
/// * `dst_w` - Destination width
/// * `dst_h` - Destination height
/// * `filter` - Resampling filter
///
/// # Returns
///
/// Resized pixel data as Vec<f32>.
///
/// # Example
///
/// ```rust
/// use vfx_ops::resize::{resize_f32, Filter};
///
/// let src = vec![0.5f32; 16 * 16 * 4];
/// let dst = resize_f32(&src, 16, 16, 4, 32, 32, Filter::Bilinear).unwrap();
/// assert_eq!(dst.len(), 32 * 32 * 4);
/// ```
pub fn resize_f32(
    src: &[f32],
    src_w: usize,
    src_h: usize,
    channels: usize,
    dst_w: usize,
    dst_h: usize,
    filter: Filter,
) -> OpsResult<Vec<f32>> {
    // Validate inputs
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

    // Two-pass separable resize: horizontal then vertical
    let temp = resize_horizontal(src, src_w, src_h, channels, dst_w, filter);
    let result = resize_vertical(&temp, dst_w, src_h, channels, dst_h, filter);

    Ok(result)
}

/// Horizontal resize pass.
fn resize_horizontal(
    src: &[f32],
    src_w: usize,
    src_h: usize,
    channels: usize,
    dst_w: usize,
    filter: Filter,
) -> Vec<f32> {
    let mut dst = vec![0.0f32; dst_w * src_h * channels];
    let scale = src_w as f32 / dst_w as f32;
    let support = filter.support() * scale.max(1.0);

    for y in 0..src_h {
        for x in 0..dst_w {
            // Map destination x to source x
            let center = (x as f32 + 0.5) * scale - 0.5;
            let left = ((center - support).floor() as isize).max(0) as usize;
            let right = ((center + support).ceil() as usize).min(src_w - 1);

            // Accumulate weighted samples
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

            // Normalize and store
            let dst_idx = (y * dst_w + x) * channels;
            if weight_sum > 0.0 {
                for c in 0..channels {
                    dst[dst_idx + c] = sum[c] / weight_sum;
                }
            }
        }
    }

    dst
}

/// Vertical resize pass.
fn resize_vertical(
    src: &[f32],
    src_w: usize,
    src_h: usize,
    channels: usize,
    dst_h: usize,
    filter: Filter,
) -> Vec<f32> {
    let mut dst = vec![0.0f32; src_w * dst_h * channels];
    let scale = src_h as f32 / dst_h as f32;
    let support = filter.support() * scale.max(1.0);

    for y in 0..dst_h {
        // Map destination y to source y
        let center = (y as f32 + 0.5) * scale - 0.5;
        let top = ((center - support).floor() as isize).max(0) as usize;
        let bottom = ((center + support).ceil() as usize).min(src_h - 1);

        for x in 0..src_w {
            // Accumulate weighted samples
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

            // Normalize and store
            let dst_idx = (y * src_w + x) * channels;
            if weight_sum > 0.0 {
                for c in 0..channels {
                    dst[dst_idx + c] = sum[c] / weight_sum;
                }
            }
        }
    }

    dst
}

/// Calculates the aspect-preserving dimensions for a target size.
///
/// # Example
///
/// ```rust
/// use vfx_ops::resize::fit_dimensions;
///
/// // Fit 1920x1080 into 640x480 box
/// let (w, h) = fit_dimensions(1920, 1080, 640, 480);
/// assert_eq!((w, h), (640, 360)); // Letterboxed
/// ```
pub fn fit_dimensions(
    src_w: usize,
    src_h: usize,
    max_w: usize,
    max_h: usize,
) -> (usize, usize) {
    let scale_w = max_w as f32 / src_w as f32;
    let scale_h = max_h as f32 / src_h as f32;
    let scale = scale_w.min(scale_h);

    let new_w = ((src_w as f32 * scale).round() as usize).max(1);
    let new_h = ((src_h as f32 * scale).round() as usize).max(1);

    (new_w, new_h)
}

/// Calculates dimensions that fill the target (may crop).
///
/// # Example
///
/// ```rust
/// use vfx_ops::resize::fill_dimensions;
///
/// // Fill 640x480 with 1920x1080 (crop sides)
/// let (w, h) = fill_dimensions(1920, 1080, 640, 480);
/// assert_eq!((w, h), (853, 480));
/// ```
pub fn fill_dimensions(
    src_w: usize,
    src_h: usize,
    min_w: usize,
    min_h: usize,
) -> (usize, usize) {
    let scale_w = min_w as f32 / src_w as f32;
    let scale_h = min_h as f32 / src_h as f32;
    let scale = scale_w.max(scale_h);

    let new_w = ((src_w as f32 * scale).round() as usize).max(1);
    let new_h = ((src_h as f32 * scale).round() as usize).max(1);

    (new_w, new_h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_weights() {
        // Nearest at center
        assert!((Filter::Nearest.weight(0.0) - 1.0).abs() < 0.01);
        assert!((Filter::Nearest.weight(0.6) - 0.0).abs() < 0.01);

        // Bilinear at center
        assert!((Filter::Bilinear.weight(0.0) - 1.0).abs() < 0.01);
        assert!((Filter::Bilinear.weight(0.5) - 0.5).abs() < 0.01);

        // Lanczos at center
        assert!((Filter::Lanczos3.weight(0.0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_resize_identity() {
        // 2x2 RGBA image
        let src = vec![
            1.0, 0.0, 0.0, 1.0, // Red
            0.0, 1.0, 0.0, 1.0, // Green
            0.0, 0.0, 1.0, 1.0, // Blue
            1.0, 1.0, 1.0, 1.0, // White
        ];

        // Resize to same size should preserve values roughly
        let dst = resize_f32(&src, 2, 2, 4, 2, 2, Filter::Bilinear).unwrap();
        assert_eq!(dst.len(), 16);
    }

    #[test]
    fn test_resize_upscale() {
        let src = vec![0.5f32; 4 * 4 * 4]; // 4x4 RGBA
        let dst = resize_f32(&src, 4, 4, 4, 8, 8, Filter::Bilinear).unwrap();
        assert_eq!(dst.len(), 8 * 8 * 4);

        // Constant image should stay constant
        for v in dst {
            assert!((v - 0.5).abs() < 0.01);
        }
    }

    #[test]
    fn test_resize_downscale() {
        let src = vec![0.25f32; 64 * 64 * 3]; // 64x64 RGB
        let dst = resize_f32(&src, 64, 64, 3, 16, 16, Filter::Lanczos3).unwrap();
        assert_eq!(dst.len(), 16 * 16 * 3);
    }

    #[test]
    fn test_fit_dimensions() {
        // Wide image into square box
        assert_eq!(fit_dimensions(1920, 1080, 640, 640), (640, 360));

        // Tall image into square box
        assert_eq!(fit_dimensions(1080, 1920, 640, 640), (360, 640));

        // Already fits
        assert_eq!(fit_dimensions(320, 240, 640, 480), (640, 480));
    }

    #[test]
    fn test_fill_dimensions() {
        // Wide image to fill square
        assert_eq!(fill_dimensions(1920, 1080, 640, 640), (1138, 640));
    }
}
