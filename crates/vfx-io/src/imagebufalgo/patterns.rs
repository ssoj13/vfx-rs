//! Pattern generation functions for ImageBuf.
//!
//! This module provides functions for generating image patterns:
//! - [`zero`] - Create an all-black image
//! - [`fill`] - Fill with solid color or gradient
//! - [`checker`] - Checkerboard pattern
//! - [`noise`] - Various noise patterns (gaussian, uniform, blue)

use crate::imagebuf::{ImageBuf, InitializePixels};
use vfx_core::{ImageSpec, Roi3D};

/// Creates an all-black image with the specified ROI.
///
/// # Arguments
///
/// * `roi` - Region defining the image size and channel count
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::zero;
/// use vfx_core::Roi3D;
///
/// let roi = Roi3D::new_2d_with_channels(0, 1920, 0, 1080, 0, 4);
/// let black = zero(roi);
/// ```
pub fn zero(roi: Roi3D) -> ImageBuf {
    let spec = ImageSpec::from_roi(&roi);
    ImageBuf::new(spec, InitializePixels::Yes)
}

/// Fills a destination image with zeros.
///
/// # Arguments
///
/// * `dst` - Destination image to fill
/// * `roi` - Optional region to fill (defaults to entire image)
pub fn zero_into(dst: &mut ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| dst.roi());
    fill_into(dst, &[0.0], Some(roi));
}

/// Creates a new image filled with the specified color.
///
/// # Arguments
///
/// * `values` - Channel values to fill with
/// * `roi` - Region defining the image size
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::fill;
/// use vfx_core::Roi3D;
///
/// let roi = Roi3D::new_2d_with_channels(0, 100, 0, 100, 0, 4);
/// let red = fill(&[1.0, 0.0, 0.0, 1.0], roi);
/// ```
pub fn fill(values: &[f32], roi: Roi3D) -> ImageBuf {
    let spec = ImageSpec::from_roi(&roi);
    let mut buf = ImageBuf::new(spec, InitializePixels::No);
    fill_into(&mut buf, values, None);
    buf
}

/// Fills a destination image with the specified color.
///
/// # Arguments
///
/// * `dst` - Destination image to fill
/// * `values` - Channel values to fill with (last value repeats for missing channels)
/// * `roi` - Optional region to fill (defaults to entire image)
pub fn fill_into(dst: &mut ImageBuf, values: &[f32], roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| dst.roi());
    let nch = dst.nchannels() as usize;

    // Expand values to fill all channels (last value repeats)
    let pixel: Vec<f32> = (0..nch)
        .map(|c| values.get(c).copied().unwrap_or_else(|| values.last().copied().unwrap_or(0.0)))
        .collect();

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Creates a gradient-filled image (top to bottom).
///
/// # Arguments
///
/// * `top` - Color at the top
/// * `bottom` - Color at the bottom
/// * `roi` - Region defining the image size
pub fn fill_gradient(top: &[f32], bottom: &[f32], roi: Roi3D) -> ImageBuf {
    let spec = ImageSpec::from_roi(&roi);
    let mut buf = ImageBuf::new(spec, InitializePixels::No);
    fill_gradient_into(&mut buf, top, bottom, Some(roi));
    buf
}

/// Fills a destination image with a vertical gradient.
pub fn fill_gradient_into(dst: &mut ImageBuf, top: &[f32], bottom: &[f32], roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| dst.roi());
    let nch = dst.nchannels() as usize;
    let height = (roi.yend - roi.ybegin) as f32;

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            let t = if height > 1.0 {
                (y - roi.ybegin) as f32 / (height - 1.0)
            } else {
                0.0
            };

            let pixel: Vec<f32> = (0..nch)
                .map(|c| {
                    let t_val = top.get(c).copied().unwrap_or_else(|| top.last().copied().unwrap_or(0.0));
                    let b_val = bottom.get(c).copied().unwrap_or_else(|| bottom.last().copied().unwrap_or(0.0));
                    t_val + (b_val - t_val) * t
                })
                .collect();

            for x in roi.xbegin..roi.xend {
                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Creates a four-corner gradient-filled image.
///
/// # Arguments
///
/// * `topleft` - Color at top-left corner
/// * `topright` - Color at top-right corner
/// * `bottomleft` - Color at bottom-left corner
/// * `bottomright` - Color at bottom-right corner
/// * `roi` - Region defining the image size
pub fn fill_corners(
    topleft: &[f32],
    topright: &[f32],
    bottomleft: &[f32],
    bottomright: &[f32],
    roi: Roi3D,
) -> ImageBuf {
    let spec = ImageSpec::from_roi(&roi);
    let mut buf = ImageBuf::new(spec, InitializePixels::No);
    fill_corners_into(&mut buf, topleft, topright, bottomleft, bottomright, Some(roi));
    buf
}

/// Fills a destination image with a four-corner gradient.
pub fn fill_corners_into(
    dst: &mut ImageBuf,
    topleft: &[f32],
    topright: &[f32],
    bottomleft: &[f32],
    bottomright: &[f32],
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| dst.roi());
    let nch = dst.nchannels() as usize;
    let width = (roi.xend - roi.xbegin) as f32;
    let height = (roi.yend - roi.ybegin) as f32;

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            let ty = if height > 1.0 {
                (y - roi.ybegin) as f32 / (height - 1.0)
            } else {
                0.0
            };

            for x in roi.xbegin..roi.xend {
                let tx = if width > 1.0 {
                    (x - roi.xbegin) as f32 / (width - 1.0)
                } else {
                    0.0
                };

                let pixel: Vec<f32> = (0..nch)
                    .map(|c| {
                        let tl = topleft.get(c).copied().unwrap_or_else(|| topleft.last().copied().unwrap_or(0.0));
                        let tr = topright.get(c).copied().unwrap_or_else(|| topright.last().copied().unwrap_or(0.0));
                        let bl = bottomleft.get(c).copied().unwrap_or_else(|| bottomleft.last().copied().unwrap_or(0.0));
                        let br = bottomright.get(c).copied().unwrap_or_else(|| bottomright.last().copied().unwrap_or(0.0));

                        // Bilinear interpolation
                        let top = tl + (tr - tl) * tx;
                        let bot = bl + (br - bl) * tx;
                        top + (bot - top) * ty
                    })
                    .collect();

                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Creates a checkerboard pattern image.
///
/// # Arguments
///
/// * `width` - Checker width in pixels
/// * `height` - Checker height in pixels
/// * `depth` - Checker depth in pixels (for 3D)
/// * `color1` - First checker color
/// * `color2` - Second checker color
/// * `offset` - Offset (x, y, z) for pattern origin
/// * `roi` - Region defining the image size
pub fn checker(
    check_width: i32,
    check_height: i32,
    check_depth: i32,
    color1: &[f32],
    color2: &[f32],
    offset: (i32, i32, i32),
    roi: Roi3D,
) -> ImageBuf {
    let spec = ImageSpec::from_roi(&roi);
    let mut buf = ImageBuf::new(spec, InitializePixels::No);
    checker_into(&mut buf, check_width, check_height, check_depth, color1, color2, offset, Some(roi));
    buf
}

/// Fills a destination image with a checkerboard pattern.
pub fn checker_into(
    dst: &mut ImageBuf,
    check_width: i32,
    check_height: i32,
    check_depth: i32,
    color1: &[f32],
    color2: &[f32],
    offset: (i32, i32, i32),
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| dst.roi());
    let nch = dst.nchannels() as usize;
    let (ox, oy, oz) = offset;

    // Expand colors
    let c1: Vec<f32> = (0..nch)
        .map(|c| color1.get(c).copied().unwrap_or_else(|| color1.last().copied().unwrap_or(0.0)))
        .collect();
    let c2: Vec<f32> = (0..nch)
        .map(|c| color2.get(c).copied().unwrap_or_else(|| color2.last().copied().unwrap_or(0.0)))
        .collect();

    let cw = check_width.max(1);
    let ch = check_height.max(1);
    let cd = check_depth.max(1);

    for z in roi.zbegin..roi.zend {
        let zcheck = ((z - oz).div_euclid(cd) & 1) != 0;

        for y in roi.ybegin..roi.yend {
            let ycheck = ((y - oy).div_euclid(ch) & 1) != 0;

            for x in roi.xbegin..roi.xend {
                let xcheck = ((x - ox).div_euclid(cw) & 1) != 0;

                let color = if xcheck ^ ycheck ^ zcheck { &c2 } else { &c1 };
                dst.setpixel(x, y, z, color);
            }
        }
    }
}

/// Noise type for noise generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NoiseType {
    /// Gaussian (normal distribution) noise
    #[default]
    Gaussian,
    /// Uniform (white) noise
    Uniform,
    /// Salt and pepper noise
    Salt,
    /// Blue noise (good spectral properties)
    Blue,
}

impl NoiseType {
    /// Parses noise type from string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "gaussian" | "normal" => Self::Gaussian,
            "uniform" | "white" => Self::Uniform,
            "salt" => Self::Salt,
            "blue" => Self::Blue,
            _ => Self::Gaussian,
        }
    }
}

/// Creates a noise image.
///
/// # Arguments
///
/// * `noise_type` - Type of noise to generate
/// * `a` - First parameter (mean for gaussian, min for uniform)
/// * `b` - Second parameter (stddev for gaussian, max for uniform)
/// * `mono` - If true, same noise value for all channels
/// * `seed` - Random seed
/// * `roi` - Region defining the image size
pub fn noise(
    noise_type: NoiseType,
    a: f32,
    b: f32,
    mono: bool,
    seed: u32,
    roi: Roi3D,
) -> ImageBuf {
    let spec = ImageSpec::from_roi(&roi);
    let mut buf = ImageBuf::new(spec, InitializePixels::No);
    noise_into(&mut buf, noise_type, a, b, mono, seed, Some(roi));
    buf
}

/// Fills a destination image with noise.
pub fn noise_into(
    dst: &mut ImageBuf,
    noise_type: NoiseType,
    a: f32,
    b: f32,
    mono: bool,
    seed: u32,
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| dst.roi());
    let nch = dst.nchannels() as usize;

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                let pixel: Vec<f32> = if mono {
                    let v = generate_noise(noise_type, a, b, seed, x, y, z, 0);
                    vec![v; nch]
                } else {
                    (0..nch)
                        .map(|c| generate_noise(noise_type, a, b, seed, x, y, z, c as i32))
                        .collect()
                };

                dst.setpixel(x, y, z, &pixel);
            }
        }
    }
}

/// Generates a single noise value using hash-based RNG.
fn generate_noise(noise_type: NoiseType, a: f32, b: f32, seed: u32, x: i32, y: i32, z: i32, c: i32) -> f32 {
    // Hash-based deterministic random
    let hash = hash_coords(seed, x, y, z, c);
    let r = hash_to_float(hash);

    match noise_type {
        NoiseType::Gaussian => {
            // Box-Muller transform for Gaussian
            let hash2 = hash_coords(seed.wrapping_add(1), x, y, z, c);
            let r2 = hash_to_float(hash2);
            let u1 = r.max(1e-10); // Avoid log(0)
            let u2 = r2;
            let mag = (-2.0 * u1.ln()).sqrt();
            let theta = 2.0 * std::f32::consts::PI * u2;
            a + b * mag * theta.cos()
        }
        NoiseType::Uniform => {
            a + (b - a) * r
        }
        NoiseType::Salt => {
            // a = salt value, b = probability
            if r < b { a } else { 0.0 }
        }
        NoiseType::Blue => {
            // Simplified blue noise - use offset hashing
            let r2 = hash_to_float(hash_coords(seed.wrapping_add(12345), x, y, z, c));
            let v = (r + r2) * 0.5;
            a + (b - a) * v
        }
    }
}

/// Hash coordinates to a u32.
fn hash_coords(seed: u32, x: i32, y: i32, z: i32, c: i32) -> u32 {
    // Simple FNV-1a-like hash
    let mut h = seed.wrapping_add(0x811c9dc5);
    h = h.wrapping_mul(0x01000193) ^ (x as u32);
    h = h.wrapping_mul(0x01000193) ^ (y as u32);
    h = h.wrapping_mul(0x01000193) ^ (z as u32);
    h = h.wrapping_mul(0x01000193) ^ (c as u32);
    h
}

/// Convert hash to float in [0, 1).
fn hash_to_float(hash: u32) -> f32 {
    (hash as f32) / (u32::MAX as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero() {
        let roi = Roi3D::new_2d_with_channels(0, 10, 0, 10, 0, 4);
        let buf = zero(roi);
        assert_eq!(buf.width(), 10);
        assert_eq!(buf.height(), 10);
        assert_eq!(buf.nchannels(), 4);

        let mut pixel = [0.0f32; 4];
        buf.getpixel(5, 5, 0, &mut pixel, crate::imagebuf::WrapMode::Black);
        assert!(pixel.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_fill() {
        let roi = Roi3D::new_2d_with_channels(0, 10, 0, 10, 0, 4);
        let buf = fill(&[1.0, 0.0, 0.0, 1.0], roi);

        let mut pixel = [0.0f32; 4];
        buf.getpixel(5, 5, 0, &mut pixel, crate::imagebuf::WrapMode::Black);
        assert!((pixel[0] - 1.0).abs() < 0.001);
        assert!((pixel[1] - 0.0).abs() < 0.001);
        assert!((pixel[2] - 0.0).abs() < 0.001);
        assert!((pixel[3] - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_checker() {
        let roi = Roi3D::new_2d_with_channels(0, 10, 0, 10, 0, 3);
        let buf = checker(2, 2, 1, &[1.0, 1.0, 1.0], &[0.0, 0.0, 0.0], (0, 0, 0), roi);

        let mut p1 = [0.0f32; 3];
        let mut p2 = [0.0f32; 3];
        buf.getpixel(0, 0, 0, &mut p1, crate::imagebuf::WrapMode::Black);
        buf.getpixel(2, 0, 0, &mut p2, crate::imagebuf::WrapMode::Black);

        // p1 and p2 should be different colors
        assert!((p1[0] - p2[0]).abs() > 0.5);
    }

    #[test]
    fn test_noise() {
        let roi = Roi3D::new_2d_with_channels(0, 10, 0, 10, 0, 1);
        let buf = noise(NoiseType::Uniform, 0.0, 1.0, false, 42, roi);

        // Just check it runs and produces something in range
        let mut pixel = [0.0f32];
        buf.getpixel(5, 5, 0, &mut pixel, crate::imagebuf::WrapMode::Black);
        assert!(pixel[0] >= 0.0 && pixel[0] <= 1.0);
    }
}
