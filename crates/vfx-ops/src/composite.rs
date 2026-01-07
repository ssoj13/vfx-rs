//! Image compositing and blending operations.
//!
//! Provides Porter-Duff operations and Photoshop-style blend modes
//! commonly used in VFX compositing.
//!
//! # Porter-Duff Operations
//!
//! - [`over`] - Standard A over B composition
//! - [`under`] - A under B (B over A)
//! - [`atop`] - A atop B
//! - [`inside`] - A inside B (A * B.alpha)
//! - [`outside`] - A outside B (A * (1 - B.alpha))
//!
//! # Blend Modes
//!
//! - [`BlendMode::Normal`] - Standard over
//! - [`BlendMode::Multiply`] - Darken by multiplication
//! - [`BlendMode::Screen`] - Lighten (inverse multiply)
//! - [`BlendMode::Add`] - Linear dodge
//! - [`BlendMode::Overlay`] - Contrast enhancement
//!
//! # Example
//!
//! ```rust
//! use vfx_ops::composite::{over_pixel, blend_pixel, BlendMode};
//!
//! let fg = [1.0, 0.0, 0.0, 0.5]; // Semi-transparent red
//! let bg = [0.0, 0.0, 1.0, 1.0]; // Opaque blue
//!
//! let result = over_pixel(fg, bg);
//! ```

use crate::{OpsError, OpsResult};
#[allow(unused_imports)]
use tracing::{debug, trace};
use vfx_compute::{Processor, ComputeImage, BlendMode as ComputeBlendMode};

/// Blend mode for compositing operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendMode {
    /// Normal (over) blending.
    #[default]
    Normal,
    /// Multiply (darken).
    Multiply,
    /// Screen (lighten).
    Screen,
    /// Add (linear dodge).
    Add,
    /// Subtract.
    Subtract,
    /// Overlay (contrast).
    Overlay,
    /// Soft light.
    SoftLight,
    /// Hard light.
    HardLight,
    /// Difference.
    Difference,
    /// Exclusion.
    Exclusion,
}

impl BlendMode {
    /// Convert to vfx-compute BlendMode
    fn to_compute(self) -> ComputeBlendMode {
        match self {
            Self::Normal => ComputeBlendMode::Normal,
            Self::Multiply => ComputeBlendMode::Multiply,
            Self::Screen => ComputeBlendMode::Screen,
            Self::Add => ComputeBlendMode::Add,
            Self::Subtract => ComputeBlendMode::Subtract,
            Self::Overlay => ComputeBlendMode::Overlay,
            Self::SoftLight => ComputeBlendMode::SoftLight,
            Self::HardLight => ComputeBlendMode::HardLight,
            Self::Difference => ComputeBlendMode::Difference,
            // Exclusion not in compute, fallback to Difference
            Self::Exclusion => ComputeBlendMode::Difference,
        }
    }
}

/// Composites foreground over background (Porter-Duff Over).
///
/// Standard alpha compositing: `Fg + Bg * (1 - Fg.alpha)`
///
/// # Example
///
/// ```rust
/// use vfx_ops::composite::over_pixel;
///
/// let fg = [1.0, 0.0, 0.0, 0.5]; // Semi-transparent red
/// let bg = [0.0, 0.0, 1.0, 1.0]; // Opaque blue
/// let result = over_pixel(fg, bg);
///
/// // Result should be purple-ish (red + blue)
/// assert!(result[0] > 0.4); // Has red
/// assert!(result[2] > 0.4); // Has blue
/// ```
#[inline]
pub fn over_pixel(fg: [f32; 4], bg: [f32; 4]) -> [f32; 4] {
    let fg_a = fg[3];
    let bg_a = bg[3];
    let out_a = fg_a + bg_a * (1.0 - fg_a);

    if out_a < 1e-8 {
        return [0.0, 0.0, 0.0, 0.0];
    }

    let inv_out_a = 1.0 / out_a;
    [
        (fg[0] * fg_a + bg[0] * bg_a * (1.0 - fg_a)) * inv_out_a,
        (fg[1] * fg_a + bg[1] * bg_a * (1.0 - fg_a)) * inv_out_a,
        (fg[2] * fg_a + bg[2] * bg_a * (1.0 - fg_a)) * inv_out_a,
        out_a,
    ]
}

/// Composites foreground under background.
///
/// Equivalent to `over_pixel(bg, fg)`.
#[inline]
pub fn under_pixel(fg: [f32; 4], bg: [f32; 4]) -> [f32; 4] {
    over_pixel(bg, fg)
}

/// Composites foreground atop background.
///
/// Places Fg where Bg is visible: `Fg * Bg.alpha + Bg * (1 - Fg.alpha)`
#[inline]
pub fn atop_pixel(fg: [f32; 4], bg: [f32; 4]) -> [f32; 4] {
    let fg_a = fg[3];
    let bg_a = bg[3];

    [
        fg[0] * bg_a + bg[0] * (1.0 - fg_a),
        fg[1] * bg_a + bg[1] * (1.0 - fg_a),
        fg[2] * bg_a + bg[2] * (1.0 - fg_a),
        bg_a,
    ]
}

/// Composites foreground inside background.
///
/// Shows Fg only where Bg is visible: `Fg * Bg.alpha`
#[inline]
pub fn inside_pixel(fg: [f32; 4], bg: [f32; 4]) -> [f32; 4] {
    let bg_a = bg[3];
    [fg[0] * bg_a, fg[1] * bg_a, fg[2] * bg_a, fg[3] * bg_a]
}

/// Composites foreground outside background.
///
/// Shows Fg only where Bg is transparent: `Fg * (1 - Bg.alpha)`
#[inline]
pub fn outside_pixel(fg: [f32; 4], bg: [f32; 4]) -> [f32; 4] {
    let inv_bg_a = 1.0 - bg[3];
    [
        fg[0] * inv_bg_a,
        fg[1] * inv_bg_a,
        fg[2] * inv_bg_a,
        fg[3] * inv_bg_a,
    ]
}

/// Blends two pixels using the specified blend mode.
///
/// # Example
///
/// ```rust
/// use vfx_ops::composite::{blend_pixel, BlendMode};
///
/// let a = [0.8, 0.4, 0.2, 1.0];
/// let b = [0.2, 0.6, 0.8, 1.0];
///
/// let result = blend_pixel(a, b, BlendMode::Multiply);
/// assert!((result[0] - 0.16).abs() < 0.01); // 0.8 * 0.2
/// ```
#[inline]
pub fn blend_pixel(a: [f32; 4], b: [f32; 4], mode: BlendMode) -> [f32; 4] {
    let blend = |av: f32, bv: f32| -> f32 {
        match mode {
            BlendMode::Normal => av,
            BlendMode::Multiply => av * bv,
            BlendMode::Screen => 1.0 - (1.0 - av) * (1.0 - bv),
            BlendMode::Add => (av + bv).min(1.0),
            BlendMode::Subtract => (bv - av).max(0.0),
            BlendMode::Overlay => {
                if bv < 0.5 {
                    2.0 * av * bv
                } else {
                    1.0 - 2.0 * (1.0 - av) * (1.0 - bv)
                }
            }
            BlendMode::SoftLight => {
                if av < 0.5 {
                    bv - (1.0 - 2.0 * av) * bv * (1.0 - bv)
                } else {
                    let d = if bv < 0.25 {
                        ((16.0 * bv - 12.0) * bv + 4.0) * bv
                    } else {
                        bv.sqrt()
                    };
                    bv + (2.0 * av - 1.0) * (d - bv)
                }
            }
            BlendMode::HardLight => {
                if av < 0.5 {
                    2.0 * av * bv
                } else {
                    1.0 - 2.0 * (1.0 - av) * (1.0 - bv)
                }
            }
            BlendMode::Difference => (av - bv).abs(),
            BlendMode::Exclusion => av + bv - 2.0 * av * bv,
        }
    };

    // Apply blend to RGB, keep alpha from a
    [
        blend(a[0], b[0]),
        blend(a[1], b[1]),
        blend(a[2], b[2]),
        a[3],
    ]
}

/// Composites two RGBA f32 images using Porter-Duff Over.
///
/// # Arguments
///
/// * `fg` - Foreground pixel data (RGBA)
/// * `bg` - Background pixel data (RGBA)
/// * `width` - Image width
/// * `height` - Image height
///
/// # Returns
///
/// Composited image as Vec<f32>.
///
/// # Example
///
/// ```rust
/// use vfx_ops::composite::over;
///
/// let fg = vec![1.0, 0.0, 0.0, 0.5]; // 1x1 red
/// let bg = vec![0.0, 0.0, 1.0, 1.0]; // 1x1 blue
/// let result = over(&fg, &bg, 1, 1).unwrap();
/// ```
pub fn over(
    fg: &[f32],
    bg: &[f32],
    width: usize,
    height: usize,
) -> OpsResult<Vec<f32>> {
    trace!(width, height, "composite::over");
    debug!(width, height, "Compositing over");
    
    let size = width * height * 4;
    if fg.len() != size || bg.len() != size {
        return Err(OpsError::SizeMismatch(format!(
            "expected {} pixels, got fg={}, bg={}",
            size,
            fg.len(),
            bg.len()
        )));
    }

    // Try GPU-accelerated composite via vfx-compute
    if let Ok(proc) = Processor::auto() {
        if let Ok(fg_img) = ComputeImage::from_f32(fg.to_vec(), width as u32, height as u32, 4) {
            if let Ok(mut bg_img) = ComputeImage::from_f32(bg.to_vec(), width as u32, height as u32, 4) {
                if proc.composite_over(&fg_img, &mut bg_img).is_ok() {
                    return Ok(bg_img.data().to_vec());
                }
            }
        }
    }

    // Fallback: CPU per-pixel
    let mut result = vec![0.0f32; size];
    for i in 0..(width * height) {
        let idx = i * 4;
        let fg_px = [fg[idx], fg[idx + 1], fg[idx + 2], fg[idx + 3]];
        let bg_px = [bg[idx], bg[idx + 1], bg[idx + 2], bg[idx + 3]];
        let out = over_pixel(fg_px, bg_px);
        result[idx] = out[0];
        result[idx + 1] = out[1];
        result[idx + 2] = out[2];
        result[idx + 3] = out[3];
    }
    Ok(result)
}

/// Blends two RGBA f32 images using the specified blend mode.
pub fn blend(
    a: &[f32],
    b: &[f32],
    width: usize,
    height: usize,
    mode: BlendMode,
) -> OpsResult<Vec<f32>> {
    let size = width * height * 4;
    if a.len() != size || b.len() != size {
        return Err(OpsError::SizeMismatch(format!(
            "expected {} pixels, got a={}, b={}",
            size,
            a.len(),
            b.len()
        )));
    }

    // Try GPU-accelerated blend via vfx-compute (except Exclusion)
    if mode != BlendMode::Exclusion {
        if let Ok(proc) = Processor::auto() {
            if let Ok(fg_img) = ComputeImage::from_f32(a.to_vec(), width as u32, height as u32, 4) {
                if let Ok(mut bg_img) = ComputeImage::from_f32(b.to_vec(), width as u32, height as u32, 4) {
                    if proc.blend(&fg_img, &mut bg_img, mode.to_compute(), 1.0).is_ok() {
                        return Ok(bg_img.data().to_vec());
                    }
                }
            }
        }
    }

    // Fallback: CPU per-pixel
    let mut result = vec![0.0f32; size];
    for i in 0..(width * height) {
        let idx = i * 4;
        let a_px = [a[idx], a[idx + 1], a[idx + 2], a[idx + 3]];
        let b_px = [b[idx], b[idx + 1], b[idx + 2], b[idx + 3]];
        let out = blend_pixel(a_px, b_px, mode);
        result[idx] = out[0];
        result[idx + 1] = out[1];
        result[idx + 2] = out[2];
        result[idx + 3] = out[3];
    }
    Ok(result)
}

/// Premultiplies alpha for RGBA pixel.
///
/// Converts straight alpha to premultiplied: `RGB *= A`
#[inline]
pub fn premultiply(rgba: [f32; 4]) -> [f32; 4] {
    let a = rgba[3];
    [rgba[0] * a, rgba[1] * a, rgba[2] * a, a]
}

/// Unpremultiplies alpha for RGBA pixel.
///
/// Converts premultiplied to straight alpha: `RGB /= A`
#[inline]
pub fn unpremultiply(rgba: [f32; 4]) -> [f32; 4] {
    let a = rgba[3];
    if a < 1e-8 {
        [0.0, 0.0, 0.0, 0.0]
    } else {
        let inv_a = 1.0 / a;
        [rgba[0] * inv_a, rgba[1] * inv_a, rgba[2] * inv_a, a]
    }
}

/// Premultiplies alpha for entire RGBA image.
///
/// Converts straight alpha to premultiplied: `RGB *= A` for each pixel.
///
/// # Arguments
///
/// * `data` - RGBA pixel data (4 floats per pixel)
/// * `width` - Image width
/// * `height` - Image height
///
/// # Returns
///
/// New image with premultiplied alpha.
pub fn premultiply_image(
    data: &[f32],
    width: usize,
    height: usize,
) -> OpsResult<Vec<f32>> {
    let size = width * height * 4;
    if data.len() != size {
        return Err(OpsError::SizeMismatch(format!(
            "expected {} values, got {}",
            size,
            data.len()
        )));
    }

    let mut result = vec![0.0f32; size];

    for i in 0..(width * height) {
        let idx = i * 4;
        let px = [data[idx], data[idx + 1], data[idx + 2], data[idx + 3]];
        let out = premultiply(px);
        result[idx] = out[0];
        result[idx + 1] = out[1];
        result[idx + 2] = out[2];
        result[idx + 3] = out[3];
    }

    Ok(result)
}

/// Unpremultiplies alpha for entire RGBA image.
///
/// Converts premultiplied to straight alpha: `RGB /= A` for each pixel.
///
/// # Arguments
///
/// * `data` - RGBA pixel data (4 floats per pixel)
/// * `width` - Image width
/// * `height` - Image height
///
/// # Returns
///
/// New image with straight (unpremultiplied) alpha.
pub fn unpremultiply_image(
    data: &[f32],
    width: usize,
    height: usize,
) -> OpsResult<Vec<f32>> {
    let size = width * height * 4;
    if data.len() != size {
        return Err(OpsError::SizeMismatch(format!(
            "expected {} values, got {}",
            size,
            data.len()
        )));
    }

    let mut result = vec![0.0f32; size];

    for i in 0..(width * height) {
        let idx = i * 4;
        let px = [data[idx], data[idx + 1], data[idx + 2], data[idx + 3]];
        let out = unpremultiply(px);
        result[idx] = out[0];
        result[idx + 1] = out[1];
        result[idx + 2] = out[2];
        result[idx + 3] = out[3];
    }

    Ok(result)
}

/// Premultiplies alpha in-place for RGBA image.
///
/// More efficient than `premultiply_image` when original data isn't needed.
pub fn premultiply_inplace(data: &mut [f32]) {
    for chunk in data.chunks_exact_mut(4) {
        let a = chunk[3];
        chunk[0] *= a;
        chunk[1] *= a;
        chunk[2] *= a;
    }
}

/// Unpremultiplies alpha in-place for RGBA image.
///
/// More efficient than `unpremultiply_image` when original data isn't needed.
pub fn unpremultiply_inplace(data: &mut [f32]) {
    for chunk in data.chunks_exact_mut(4) {
        let a = chunk[3];
        if a > 1e-8 {
            let inv_a = 1.0 / a;
            chunk[0] *= inv_a;
            chunk[1] *= inv_a;
            chunk[2] *= inv_a;
        } else {
            chunk[0] = 0.0;
            chunk[1] = 0.0;
            chunk[2] = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_over_opaque() {
        let fg = [1.0, 0.0, 0.0, 1.0]; // Opaque red
        let bg = [0.0, 0.0, 1.0, 1.0]; // Opaque blue
        let result = over_pixel(fg, bg);

        // Opaque foreground completely covers
        assert!((result[0] - 1.0).abs() < 0.01);
        assert!((result[1] - 0.0).abs() < 0.01);
        assert!((result[2] - 0.0).abs() < 0.01);
        assert!((result[3] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_over_transparent() {
        let fg = [1.0, 0.0, 0.0, 0.0]; // Transparent red
        let bg = [0.0, 0.0, 1.0, 1.0]; // Opaque blue
        let result = over_pixel(fg, bg);

        // Transparent foreground shows background
        assert!((result[0] - 0.0).abs() < 0.01);
        assert!((result[2] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_over_semi_transparent() {
        let fg = [1.0, 0.0, 0.0, 0.5]; // 50% red
        let bg = [0.0, 0.0, 1.0, 1.0]; // Opaque blue
        let result = over_pixel(fg, bg);

        // Should be purple-ish
        assert!(result[0] > 0.4 && result[0] < 0.6);
        assert!(result[2] > 0.4 && result[2] < 0.6);
    }

    #[test]
    fn test_blend_multiply() {
        let a = [0.8, 0.5, 0.2, 1.0];
        let b = [0.5, 0.5, 0.5, 1.0];
        let result = blend_pixel(a, b, BlendMode::Multiply);

        assert!((result[0] - 0.4).abs() < 0.01); // 0.8 * 0.5
        assert!((result[1] - 0.25).abs() < 0.01); // 0.5 * 0.5
    }

    #[test]
    fn test_blend_screen() {
        let a = [0.5, 0.5, 0.5, 1.0];
        let b = [0.5, 0.5, 0.5, 1.0];
        let result = blend_pixel(a, b, BlendMode::Screen);

        // 1 - (1-0.5)*(1-0.5) = 1 - 0.25 = 0.75
        assert!((result[0] - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_blend_add() {
        let a = [0.6, 0.3, 0.1, 1.0];
        let b = [0.5, 0.8, 0.2, 1.0];
        let result = blend_pixel(a, b, BlendMode::Add);

        // Clamped add
        assert!((result[0] - 1.0).abs() < 0.01); // 0.6 + 0.5 > 1.0, clamped
        assert!((result[1] - 1.0).abs() < 0.01); // 0.3 + 0.8 > 1.0, clamped
        assert!((result[2] - 0.3).abs() < 0.01); // 0.1 + 0.2 = 0.3
    }

    #[test]
    fn test_premultiply() {
        let straight = [1.0, 0.5, 0.0, 0.5];
        let premult = premultiply(straight);

        assert!((premult[0] - 0.5).abs() < 0.01);
        assert!((premult[1] - 0.25).abs() < 0.01);
        assert!((premult[2] - 0.0).abs() < 0.01);
        assert!((premult[3] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_unpremultiply() {
        let premult = [0.5, 0.25, 0.0, 0.5];
        let straight = unpremultiply(premult);

        assert!((straight[0] - 1.0).abs() < 0.01);
        assert!((straight[1] - 0.5).abs() < 0.01);
        assert!((straight[2] - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_over_image() {
        let fg = vec![1.0, 0.0, 0.0, 0.5, 0.0, 1.0, 0.0, 0.5]; // 2 pixels
        let bg = vec![0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0];
        let result = over(&fg, &bg, 2, 1).unwrap();
        assert_eq!(result.len(), 8);
    }
}
