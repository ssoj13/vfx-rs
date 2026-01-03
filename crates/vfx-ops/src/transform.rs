//! Geometric transformation operations.
//!
//! Provides crop, flip, rotate, and other spatial transforms.
//!
//! # Operations
//!
//! - [`crop`] - Extract region of interest
//! - [`flip_h`] - Horizontal flip (mirror)
//! - [`flip_v`] - Vertical flip
//! - [`rotate_90`] - 90-degree rotations
//! - [`pad`] - Add border padding
//!
//! # Example
//!
//! ```rust
//! use vfx_ops::transform::{flip_h, crop};
//!
//! let src = vec![0.5f32; 64 * 64 * 4];
//!
//! // Flip horizontally
//! let flipped = flip_h(&src, 64, 64, 4);
//!
//! // Crop center region
//! let cropped = crop(&src, 64, 64, 4, 16, 16, 32, 32).unwrap();
//! ```

use crate::{OpsError, OpsResult};

/// Crops a region from the image.
///
/// # Arguments
///
/// * `src` - Source pixel data
/// * `src_w`, `src_h` - Source dimensions
/// * `channels` - Number of channels
/// * `x`, `y` - Crop origin (top-left)
/// * `w`, `h` - Crop dimensions
///
/// # Example
///
/// ```rust
/// use vfx_ops::transform::crop;
///
/// let src = vec![0.5f32; 64 * 64 * 3];
/// let cropped = crop(&src, 64, 64, 3, 10, 10, 20, 20).unwrap();
/// assert_eq!(cropped.len(), 20 * 20 * 3);
/// ```
pub fn crop(
    src: &[f32],
    src_w: usize,
    src_h: usize,
    channels: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
) -> OpsResult<Vec<f32>> {
    // Validate bounds
    if x + w > src_w || y + h > src_h {
        return Err(OpsError::InvalidDimensions(format!(
            "crop region {}x{} at ({},{}) exceeds {}x{}",
            w, h, x, y, src_w, src_h
        )));
    }

    let mut dst = Vec::with_capacity(w * h * channels);

    for row in y..(y + h) {
        let src_start = (row * src_w + x) * channels;
        let src_end = src_start + w * channels;
        dst.extend_from_slice(&src[src_start..src_end]);
    }

    Ok(dst)
}

/// Flips image horizontally (left-right mirror).
///
/// # Example
///
/// ```rust
/// use vfx_ops::transform::flip_h;
///
/// let src = vec![
///     1.0, 0.0, 0.0, // Left pixel (red)
///     0.0, 1.0, 0.0, // Right pixel (green)
/// ];
/// let flipped = flip_h(&src, 2, 1, 3);
/// assert_eq!(flipped[0], 0.0); // Was right, now left (green)
/// assert_eq!(flipped[3], 1.0); // Was left, now right (red)
/// ```
pub fn flip_h(src: &[f32], width: usize, height: usize, channels: usize) -> Vec<f32> {
    let mut dst = vec![0.0f32; src.len()];

    for y in 0..height {
        for x in 0..width {
            let src_idx = (y * width + x) * channels;
            let dst_idx = (y * width + (width - 1 - x)) * channels;

            for c in 0..channels {
                dst[dst_idx + c] = src[src_idx + c];
            }
        }
    }

    dst
}

/// Flips image vertically (top-bottom mirror).
///
/// # Example
///
/// ```rust
/// use vfx_ops::transform::flip_v;
///
/// let src = vec![
///     1.0, 0.0, 0.0, // Top pixel
///     0.0, 1.0, 0.0, // Bottom pixel
/// ];
/// let flipped = flip_v(&src, 1, 2, 3);
/// assert_eq!(flipped[0], 0.0); // Was bottom, now top
/// assert_eq!(flipped[3], 1.0); // Was top, now bottom
/// ```
pub fn flip_v(src: &[f32], width: usize, height: usize, channels: usize) -> Vec<f32> {
    let mut dst = vec![0.0f32; src.len()];
    let row_size = width * channels;

    for y in 0..height {
        let src_start = y * row_size;
        let dst_start = (height - 1 - y) * row_size;
        dst[dst_start..dst_start + row_size].copy_from_slice(&src[src_start..src_start + row_size]);
    }

    dst
}

/// Rotates image 90 degrees clockwise.
///
/// # Returns
///
/// Tuple of (new_data, new_width, new_height).
///
/// # Example
///
/// ```rust
/// use vfx_ops::transform::rotate_90_cw;
///
/// let src = vec![0.5f32; 4 * 2 * 3]; // 4x2 RGB
/// let (dst, w, h) = rotate_90_cw(&src, 4, 2, 3);
/// assert_eq!((w, h), (2, 4)); // Now 2x4
/// ```
pub fn rotate_90_cw(
    src: &[f32],
    width: usize,
    height: usize,
    channels: usize,
) -> (Vec<f32>, usize, usize) {
    let new_w = height;
    let new_h = width;
    let mut dst = vec![0.0f32; new_w * new_h * channels];

    for y in 0..height {
        for x in 0..width {
            let src_idx = (y * width + x) * channels;
            // New position: (y, width-1-x) -> (new_x, new_y)
            let new_x = height - 1 - y;
            let new_y = x;
            let dst_idx = (new_y * new_w + new_x) * channels;

            for c in 0..channels {
                dst[dst_idx + c] = src[src_idx + c];
            }
        }
    }

    (dst, new_w, new_h)
}

/// Rotates image 90 degrees counter-clockwise.
pub fn rotate_90_ccw(
    src: &[f32],
    width: usize,
    height: usize,
    channels: usize,
) -> (Vec<f32>, usize, usize) {
    let new_w = height;
    let new_h = width;
    let mut dst = vec![0.0f32; new_w * new_h * channels];

    for y in 0..height {
        for x in 0..width {
            let src_idx = (y * width + x) * channels;
            let new_x = y;
            let new_y = width - 1 - x;
            let dst_idx = (new_y * new_w + new_x) * channels;

            for c in 0..channels {
                dst[dst_idx + c] = src[src_idx + c];
            }
        }
    }

    (dst, new_w, new_h)
}

/// Rotates image 180 degrees.
pub fn rotate_180(
    src: &[f32],
    width: usize,
    height: usize,
    channels: usize,
) -> Vec<f32> {
    let mut dst = vec![0.0f32; src.len()];

    for y in 0..height {
        for x in 0..width {
            let src_idx = (y * width + x) * channels;
            let dst_idx = ((height - 1 - y) * width + (width - 1 - x)) * channels;

            for c in 0..channels {
                dst[dst_idx + c] = src[src_idx + c];
            }
        }
    }

    dst
}

/// Pads image with border.
///
/// # Arguments
///
/// * `src` - Source pixel data
/// * `width`, `height` - Source dimensions
/// * `channels` - Number of channels
/// * `top`, `right`, `bottom`, `left` - Padding amounts
/// * `fill` - Fill value for padding
///
/// # Returns
///
/// Tuple of (new_data, new_width, new_height).
///
/// # Example
///
/// ```rust
/// use vfx_ops::transform::pad;
///
/// let src = vec![1.0f32; 4 * 4 * 3];
/// let (dst, w, h) = pad(&src, 4, 4, 3, 2, 2, 2, 2, 0.0);
/// assert_eq!((w, h), (8, 8));
/// ```
pub fn pad(
    src: &[f32],
    width: usize,
    height: usize,
    channels: usize,
    top: usize,
    right: usize,
    bottom: usize,
    left: usize,
    fill: f32,
) -> (Vec<f32>, usize, usize) {
    let new_w = width + left + right;
    let new_h = height + top + bottom;
    let mut dst = vec![fill; new_w * new_h * channels];

    // Copy source into padded region
    for y in 0..height {
        let src_start = y * width * channels;
        let dst_start = ((y + top) * new_w + left) * channels;
        let row_bytes = width * channels;
        dst[dst_start..dst_start + row_bytes]
            .copy_from_slice(&src[src_start..src_start + row_bytes]);
    }

    (dst, new_w, new_h)
}

/// Tiles image to fill target dimensions.
///
/// # Example
///
/// ```rust
/// use vfx_ops::transform::tile;
///
/// let src = vec![0.5f32; 4 * 4 * 3]; // 4x4 tile
/// let dst = tile(&src, 4, 4, 3, 12, 12);
/// assert_eq!(dst.len(), 12 * 12 * 3);
/// ```
pub fn tile(
    src: &[f32],
    src_w: usize,
    src_h: usize,
    channels: usize,
    dst_w: usize,
    dst_h: usize,
) -> Vec<f32> {
    let mut dst = vec![0.0f32; dst_w * dst_h * channels];

    for y in 0..dst_h {
        for x in 0..dst_w {
            let sx = x % src_w;
            let sy = y % src_h;

            let src_idx = (sy * src_w + sx) * channels;
            let dst_idx = (y * dst_w + x) * channels;

            for c in 0..channels {
                dst[dst_idx + c] = src[src_idx + c];
            }
        }
    }

    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crop() {
        // 4x4 image with gradient
        let mut src = Vec::new();
        for y in 0..4 {
            for x in 0..4 {
                src.push(x as f32 / 3.0);
                src.push(y as f32 / 3.0);
                src.push(0.0);
            }
        }

        // Crop center 2x2
        let cropped = crop(&src, 4, 4, 3, 1, 1, 2, 2).unwrap();
        assert_eq!(cropped.len(), 2 * 2 * 3);

        // Check first pixel of cropped is (1,1) of original
        assert!((cropped[0] - 1.0 / 3.0).abs() < 0.01);
        assert!((cropped[1] - 1.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_crop_out_of_bounds() {
        let src = vec![0.0f32; 4 * 4 * 3];
        let result = crop(&src, 4, 4, 3, 3, 3, 2, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_flip_h() {
        // 2x1 image: [R, G]
        let src = vec![
            1.0, 0.0, 0.0, // Red
            0.0, 1.0, 0.0, // Green
        ];
        let dst = flip_h(&src, 2, 1, 3);

        // Should be [G, R]
        assert!((dst[0] - 0.0).abs() < 0.01); // Green.r
        assert!((dst[1] - 1.0).abs() < 0.01); // Green.g
        assert!((dst[3] - 1.0).abs() < 0.01); // Red.r
    }

    #[test]
    fn test_flip_v() {
        // 1x2 image: [R, G] (top to bottom)
        let src = vec![
            1.0, 0.0, 0.0, // Red (top)
            0.0, 1.0, 0.0, // Green (bottom)
        ];
        let dst = flip_v(&src, 1, 2, 3);

        // Should be [G, R]
        assert!((dst[0] - 0.0).abs() < 0.01); // Green.r
        assert!((dst[1] - 1.0).abs() < 0.01); // Green.g
        assert!((dst[3] - 1.0).abs() < 0.01); // Red.r
    }

    #[test]
    fn test_rotate_90_cw() {
        // 2x1 -> 1x2
        let src = vec![1.0, 2.0]; // 2 grayscale pixels
        let (dst, w, h) = rotate_90_cw(&src, 2, 1, 1);

        assert_eq!((w, h), (1, 2));
        assert_eq!(dst.len(), 2);
    }

    #[test]
    fn test_rotate_180() {
        let src = vec![
            1.0, 2.0,
            3.0, 4.0,
        ];
        let dst = rotate_180(&src, 2, 2, 1);

        assert!((dst[0] - 4.0).abs() < 0.01);
        assert!((dst[3] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_pad() {
        let src = vec![1.0f32; 2 * 2 * 1]; // 2x2 white
        let (dst, w, h) = pad(&src, 2, 2, 1, 1, 1, 1, 1, 0.0);

        assert_eq!((w, h), (4, 4));

        // Center should be white, border black
        assert!((dst[0] - 0.0).abs() < 0.01); // Top-left corner (padding)
        assert!((dst[5] - 1.0).abs() < 0.01); // Center area (original)
    }

    #[test]
    fn test_tile() {
        let src = vec![1.0, 0.5]; // 2x1 grayscale
        let dst = tile(&src, 2, 1, 1, 4, 2);

        assert_eq!(dst.len(), 4 * 2);
        // Pattern repeats
        assert!((dst[0] - 1.0).abs() < 0.01);
        assert!((dst[1] - 0.5).abs() < 0.01);
        assert!((dst[2] - 1.0).abs() < 0.01);
    }
}
