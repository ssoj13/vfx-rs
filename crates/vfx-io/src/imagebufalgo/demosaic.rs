//! Bayer demosaicing algorithms.
//!
//! Converts raw Bayer pattern sensor data to full RGB images.
//!
//! # Bayer Patterns
//!
//! Most digital camera sensors use a Bayer color filter array (CFA):
//! ```text
//! RGGB:     BGGR:     GRBG:     GBRG:
//! R G R G   B G B G   G R G R   G B G B
//! G B G B   G R G R   B G B G   R G R G
//! R G R G   B G B G   G R G R   G B G B
//! G B G B   G R G R   B G B G   R G R G
//! ```
//!
//! # Algorithms
//!
//! - **Bilinear**: Fast, simple interpolation. Good for previews.
//! - **VNG**: Variable Number of Gradients. Better edge handling.
//! - **AHD**: Adaptive Homogeneity-Directed. High quality, slower.
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::imagebufalgo::{demosaic, BayerPattern, DemosaicAlgorithm};
//!
//! let rgb = demosaic(&raw_bayer, BayerPattern::RGGB, DemosaicAlgorithm::VNG);
//! ```

use crate::imagebuf::{ImageBuf, InitializePixels, WrapMode};
use vfx_core::ImageSpec;

/// Bayer pattern arrangement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BayerPattern {
    /// Red-Green / Green-Blue (most common)
    #[default]
    RGGB,
    /// Blue-Green / Green-Red
    BGGR,
    /// Green-Red / Blue-Green
    GRBG,
    /// Green-Blue / Red-Green
    GBRG,
}

impl BayerPattern {
    /// Parse pattern from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "RGGB" => Some(BayerPattern::RGGB),
            "BGGR" => Some(BayerPattern::BGGR),
            "GRBG" => Some(BayerPattern::GRBG),
            "GBRG" => Some(BayerPattern::GBRG),
            _ => None,
        }
    }

    /// Get color at position (0=R, 1=G, 2=B).
    #[inline]
    fn color_at(&self, x: i32, y: i32) -> usize {
        let x_odd = (x & 1) as usize;
        let y_odd = (y & 1) as usize;

        match self {
            BayerPattern::RGGB => match (x_odd, y_odd) {
                (0, 0) => 0, // R
                (1, 0) => 1, // G
                (0, 1) => 1, // G
                (1, 1) => 2, // B
                _ => unreachable!(),
            },
            BayerPattern::BGGR => match (x_odd, y_odd) {
                (0, 0) => 2, // B
                (1, 0) => 1, // G
                (0, 1) => 1, // G
                (1, 1) => 0, // R
                _ => unreachable!(),
            },
            BayerPattern::GRBG => match (x_odd, y_odd) {
                (0, 0) => 1, // G
                (1, 0) => 0, // R
                (0, 1) => 2, // B
                (1, 1) => 1, // G
                _ => unreachable!(),
            },
            BayerPattern::GBRG => match (x_odd, y_odd) {
                (0, 0) => 1, // G
                (1, 0) => 2, // B
                (0, 1) => 0, // R
                (1, 1) => 1, // G
                _ => unreachable!(),
            },
        }
    }
}

/// Demosaicing algorithm.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DemosaicAlgorithm {
    /// Simple bilinear interpolation. Fast but lower quality.
    Bilinear,
    /// Variable Number of Gradients. Good balance of speed and quality.
    #[default]
    VNG,
    /// Adaptive Homogeneity-Directed. Highest quality, slower.
    AHD,
}

/// Demosaic a Bayer pattern image to RGB.
///
/// # Arguments
/// * `src` - Single-channel Bayer pattern image
/// * `pattern` - Bayer pattern arrangement
/// * `algorithm` - Demosaicing algorithm to use
///
/// # Returns
/// 3-channel RGB image.
pub fn demosaic(
    src: &ImageBuf,
    pattern: BayerPattern,
    algorithm: DemosaicAlgorithm,
) -> ImageBuf {
    match algorithm {
        DemosaicAlgorithm::Bilinear => demosaic_bilinear(src, pattern),
        DemosaicAlgorithm::VNG => demosaic_vng(src, pattern),
        DemosaicAlgorithm::AHD => demosaic_ahd(src, pattern),
    }
}

/// Get single channel value from source.
#[inline]
fn get_val(src: &ImageBuf, x: i32, y: i32) -> f32 {
    let mut p = [0.0f32];
    src.getpixel(x, y, 0, &mut p, WrapMode::Clamp);
    p[0]
}

/// Bilinear demosaicing - simple and fast.
fn demosaic_bilinear(src: &ImageBuf, pattern: BayerPattern) -> ImageBuf {
    let w = src.width() as i32;
    let h = src.height() as i32;

    let spec = ImageSpec::rgb(w as u32, h as u32);
    let mut dst = ImageBuf::new(spec, InitializePixels::No);

    for y in 0..h {
        for x in 0..w {
            let color = pattern.color_at(x, y);
            let src_val = get_val(src, x, y);

            let mut rgb = [0.0f32; 3];
            rgb[color] = src_val;

            // Interpolate missing colors
            match color {
                0 => {
                    // Red pixel - need G and B
                    rgb[1] = interpolate_green_at_rb(src, x, y, w, h);
                    rgb[2] = interpolate_blue_at_red(src, x, y, w, h);
                }
                1 => {
                    // Green pixel - need R and B
                    let (r, b) = interpolate_rb_at_green(src, x, y, w, h, pattern);
                    rgb[0] = r;
                    rgb[2] = b;
                }
                2 => {
                    // Blue pixel - need R and G
                    rgb[0] = interpolate_red_at_blue(src, x, y, w, h);
                    rgb[1] = interpolate_green_at_rb(src, x, y, w, h);
                }
                _ => unreachable!(),
            }

            dst.setpixel(x, y, 0, &rgb);
        }
    }

    dst
}

/// Interpolate green at red/blue position (4-neighbor average).
#[inline]
fn interpolate_green_at_rb(src: &ImageBuf, x: i32, y: i32, w: i32, h: i32) -> f32 {
    let mut sum = 0.0;
    let mut count = 0;

    if x > 0 {
        sum += get_val(src, x - 1, y);
        count += 1;
    }
    if x < w - 1 {
        sum += get_val(src, x + 1, y);
        count += 1;
    }
    if y > 0 {
        sum += get_val(src, x, y - 1);
        count += 1;
    }
    if y < h - 1 {
        sum += get_val(src, x, y + 1);
        count += 1;
    }

    if count > 0 { sum / count as f32 } else { 0.0 }
}

/// Interpolate blue at red position (diagonal average).
#[inline]
fn interpolate_blue_at_red(src: &ImageBuf, x: i32, y: i32, w: i32, h: i32) -> f32 {
    let mut sum = 0.0;
    let mut count = 0;

    if x > 0 && y > 0 {
        sum += get_val(src, x - 1, y - 1);
        count += 1;
    }
    if x < w - 1 && y > 0 {
        sum += get_val(src, x + 1, y - 1);
        count += 1;
    }
    if x > 0 && y < h - 1 {
        sum += get_val(src, x - 1, y + 1);
        count += 1;
    }
    if x < w - 1 && y < h - 1 {
        sum += get_val(src, x + 1, y + 1);
        count += 1;
    }

    if count > 0 { sum / count as f32 } else { 0.0 }
}

/// Interpolate red at blue position (diagonal average).
#[inline]
fn interpolate_red_at_blue(src: &ImageBuf, x: i32, y: i32, w: i32, h: i32) -> f32 {
    interpolate_blue_at_red(src, x, y, w, h)
}

/// Interpolate R and B at green position.
#[inline]
fn interpolate_rb_at_green(src: &ImageBuf, x: i32, y: i32, w: i32, h: i32, pattern: BayerPattern) -> (f32, f32) {
    let on_red_row = match pattern {
        BayerPattern::RGGB | BayerPattern::GRBG => (y & 1) == 0,
        BayerPattern::BGGR | BayerPattern::GBRG => (y & 1) == 1,
    };

    let mut r_sum = 0.0;
    let mut r_count = 0;
    let mut b_sum = 0.0;
    let mut b_count = 0;

    if on_red_row {
        if x > 0 {
            r_sum += get_val(src, x - 1, y);
            r_count += 1;
        }
        if x < w - 1 {
            r_sum += get_val(src, x + 1, y);
            r_count += 1;
        }
        if y > 0 {
            b_sum += get_val(src, x, y - 1);
            b_count += 1;
        }
        if y < h - 1 {
            b_sum += get_val(src, x, y + 1);
            b_count += 1;
        }
    } else {
        if x > 0 {
            b_sum += get_val(src, x - 1, y);
            b_count += 1;
        }
        if x < w - 1 {
            b_sum += get_val(src, x + 1, y);
            b_count += 1;
        }
        if y > 0 {
            r_sum += get_val(src, x, y - 1);
            r_count += 1;
        }
        if y < h - 1 {
            r_sum += get_val(src, x, y + 1);
            r_count += 1;
        }
    }

    let r = if r_count > 0 { r_sum / r_count as f32 } else { 0.0 };
    let b = if b_count > 0 { b_sum / b_count as f32 } else { 0.0 };

    (r, b)
}

/// VNG demosaicing - better edge handling than bilinear.
fn demosaic_vng(src: &ImageBuf, pattern: BayerPattern) -> ImageBuf {
    let w = src.width() as i32;
    let h = src.height() as i32;

    let spec = ImageSpec::rgb(w as u32, h as u32);
    let mut dst = ImageBuf::new(spec, InitializePixels::No);

    const GRAD_DIRS: [(i32, i32); 8] = [
        (-1, -1), (0, -1), (1, -1),
        (-1,  0),          (1,  0),
        (-1,  1), (0,  1), (1,  1),
    ];

    for y in 0..h {
        for x in 0..w {
            let color = pattern.color_at(x, y);
            let src_val = get_val(src, x, y);

            let mut rgb = [0.0f32; 3];
            rgb[color] = src_val;

            // For edge pixels, fall back to bilinear
            if x < 2 || x >= w - 2 || y < 2 || y >= h - 2 {
                match color {
                    0 => {
                        rgb[1] = interpolate_green_at_rb(src, x, y, w, h);
                        rgb[2] = interpolate_blue_at_red(src, x, y, w, h);
                    }
                    1 => {
                        let (r, b) = interpolate_rb_at_green(src, x, y, w, h, pattern);
                        rgb[0] = r;
                        rgb[2] = b;
                    }
                    2 => {
                        rgb[0] = interpolate_red_at_blue(src, x, y, w, h);
                        rgb[1] = interpolate_green_at_rb(src, x, y, w, h);
                    }
                    _ => {}
                }
                dst.setpixel(x, y, 0, &rgb);
                continue;
            }

            // Compute gradients in 8 directions
            let mut gradients = [0.0f32; 8];
            for (i, &(dx, dy)) in GRAD_DIRS.iter().enumerate() {
                let nx = x + dx * 2;
                let ny = y + dy * 2;
                gradients[i] = (src_val - get_val(src, nx, ny)).abs();
            }

            // Find threshold
            let min_grad = gradients.iter().cloned().fold(f32::INFINITY, f32::min);
            let threshold = min_grad * 1.5 + 0.001;

            // Average colors from low-gradient directions
            let mut rgb_sum = [0.0f32; 3];
            let mut count = 0.0;

            for (i, &(dx, dy)) in GRAD_DIRS.iter().enumerate() {
                if gradients[i] <= threshold {
                    let nx = x + dx;
                    let ny = y + dy;
                    let ncolor = pattern.color_at(nx, ny);
                    let nval = get_val(src, nx, ny);
                    rgb_sum[ncolor] += nval;
                    count += 1.0;
                }
            }

            if count > 0.0 {
                for c in 0..3 {
                    if c != color {
                        rgb[c] = rgb_sum[c] / count;
                    }
                }
            }

            dst.setpixel(x, y, 0, &rgb);
        }
    }

    dst
}

/// AHD demosaicing - highest quality, adaptive.
fn demosaic_ahd(src: &ImageBuf, pattern: BayerPattern) -> ImageBuf {
    let w = src.width() as i32;
    let h = src.height() as i32;

    let spec = ImageSpec::rgb(w as u32, h as u32);
    let mut dst_h = ImageBuf::new(spec.clone(), InitializePixels::No);
    let mut dst_v = ImageBuf::new(spec.clone(), InitializePixels::No);
    let mut dst = ImageBuf::new(spec, InitializePixels::No);

    // First pass: create H and V interpolations
    for y in 0..h {
        for x in 0..w {
            let color = pattern.color_at(x, y);
            let src_val = get_val(src, x, y);

            let mut rgb_h = [0.0f32; 3];
            let mut rgb_v = [0.0f32; 3];

            rgb_h[color] = src_val;
            rgb_v[color] = src_val;

            if color == 1 {
                // Green pixel
                rgb_h[1] = src_val;
                rgb_v[1] = src_val;
                let (r, b) = interpolate_rb_at_green(src, x, y, w, h, pattern);
                rgb_h[0] = r;
                rgb_h[2] = b;
                rgb_v[0] = r;
                rgb_v[2] = b;
            } else {
                // R or B pixel - interpolate green differently for H and V
                let g_h = if x > 0 && x < w - 1 {
                    (get_val(src, x - 1, y) + get_val(src, x + 1, y)) / 2.0
                } else if x > 0 {
                    get_val(src, x - 1, y)
                } else {
                    get_val(src, x + 1, y)
                };

                let g_v = if y > 0 && y < h - 1 {
                    (get_val(src, x, y - 1) + get_val(src, x, y + 1)) / 2.0
                } else if y > 0 {
                    get_val(src, x, y - 1)
                } else {
                    get_val(src, x, y + 1)
                };

                rgb_h[1] = g_h;
                rgb_v[1] = g_v;

                // Interpolate the other color
                if color == 0 {
                    let b = interpolate_blue_at_red(src, x, y, w, h);
                    rgb_h[2] = b;
                    rgb_v[2] = b;
                } else {
                    let r = interpolate_red_at_blue(src, x, y, w, h);
                    rgb_h[0] = r;
                    rgb_v[0] = r;
                }
            }

            dst_h.setpixel(x, y, 0, &rgb_h);
            dst_v.setpixel(x, y, 0, &rgb_v);
        }
    }

    // Second pass: choose between H and V based on homogeneity
    for y in 0..h {
        for x in 0..w {
            let homo_h = compute_homogeneity(&dst_h, x, y, w, h);
            let homo_v = compute_homogeneity(&dst_v, x, y, w, h);

            let mut rgb = [0.0f32; 3];
            if homo_h < homo_v {
                dst_h.getpixel(x, y, 0, &mut rgb, WrapMode::Clamp);
            } else {
                dst_v.getpixel(x, y, 0, &mut rgb, WrapMode::Clamp);
            }
            dst.setpixel(x, y, 0, &rgb);
        }
    }

    dst
}

/// Compute local homogeneity (lower = more homogeneous).
fn compute_homogeneity(img: &ImageBuf, x: i32, y: i32, w: i32, h: i32) -> f32 {
    let mut center = [0.0f32; 3];
    img.getpixel(x, y, 0, &mut center, WrapMode::Clamp);

    let mut diff_sum = 0.0;
    let mut count = 0;

    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x + dx;
            let ny = y + dy;
            if nx >= 0 && nx < w && ny >= 0 && ny < h {
                let mut neighbor = [0.0f32; 3];
                img.getpixel(nx, ny, 0, &mut neighbor, WrapMode::Clamp);
                for c in 0..3 {
                    diff_sum += (center[c] - neighbor[c]).abs();
                }
                count += 1;
            }
        }
    }

    if count > 0 { diff_sum / count as f32 } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bayer_pattern() {
        assert_eq!(BayerPattern::RGGB.color_at(0, 0), 0);
        assert_eq!(BayerPattern::RGGB.color_at(1, 0), 1);
        assert_eq!(BayerPattern::RGGB.color_at(0, 1), 1);
        assert_eq!(BayerPattern::RGGB.color_at(1, 1), 2);
        assert_eq!(BayerPattern::BGGR.color_at(0, 0), 2);
        assert_eq!(BayerPattern::BGGR.color_at(1, 1), 0);
    }

    #[test]
    fn test_pattern_from_str() {
        assert_eq!(BayerPattern::from_str("RGGB"), Some(BayerPattern::RGGB));
        assert_eq!(BayerPattern::from_str("rggb"), Some(BayerPattern::RGGB));
        assert_eq!(BayerPattern::from_str("invalid"), None);
    }

    #[test]
    fn test_demosaic_bilinear() {
        let spec = ImageSpec::gray(4, 4);
        let mut src = ImageBuf::new(spec, InitializePixels::No);
        
        // RGGB pattern
        for y in 0..4i32 {
            for x in 0..4i32 {
                let val = match ((x & 1), (y & 1)) {
                    (0, 0) => 1.0,  // R
                    (1, 0) => 0.5,  // G
                    (0, 1) => 0.5,  // G
                    (1, 1) => 0.0,  // B
                    _ => 0.0,
                };
                src.setpixel(x, y, 0, &[val]);
            }
        }

        let result = demosaic(&src, BayerPattern::RGGB, DemosaicAlgorithm::Bilinear);
        assert_eq!(result.width(), 4);
        assert_eq!(result.height(), 4);
        assert_eq!(result.nchannels(), 3);
    }

    #[test]
    fn test_demosaic_vng() {
        let spec = ImageSpec::gray(8, 8);
        let mut src = ImageBuf::new(spec, InitializePixels::No);
        for y in 0..8i32 {
            for x in 0..8i32 {
                src.setpixel(x, y, 0, &[(x + y) as f32 / 16.0]);
            }
        }

        let result = demosaic(&src, BayerPattern::RGGB, DemosaicAlgorithm::VNG);
        assert_eq!(result.nchannels(), 3);
    }

    #[test]
    fn test_demosaic_ahd() {
        let spec = ImageSpec::gray(8, 8);
        let mut src = ImageBuf::new(spec, InitializePixels::No);
        for y in 0..8i32 {
            for x in 0..8i32 {
                src.setpixel(x, y, 0, &[(x + y) as f32 / 16.0]);
            }
        }

        let result = demosaic(&src, BayerPattern::RGGB, DemosaicAlgorithm::AHD);
        assert_eq!(result.nchannels(), 3);
    }
}
