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

/// Pastes foreground image onto background at specified position.
///
/// Supports alpha blending when fg has 4 channels (RGBA).
/// Out-of-bounds areas are clipped.
///
/// # Arguments
///
/// * `bg` - Background pixel data
/// * `bg_w`, `bg_h` - Background dimensions
/// * `fg` - Foreground pixel data
/// * `fg_w`, `fg_h` - Foreground dimensions
/// * `channels` - Number of channels (must match)
/// * `x`, `y` - Position to paste (can be negative for partial paste)
/// * `blend` - If true, use alpha blending for RGBA images
///
/// # Example
///
/// ```rust
/// use vfx_ops::transform::paste;
///
/// let bg = vec![0.0f32; 64 * 64 * 4]; // Black background
/// let fg = vec![1.0f32; 16 * 16 * 4]; // White foreground
/// let result = paste(&bg, 64, 64, &fg, 16, 16, 4, 10, 10, true);
/// ```
pub fn paste(
    bg: &[f32],
    bg_w: usize,
    bg_h: usize,
    fg: &[f32],
    fg_w: usize,
    fg_h: usize,
    channels: usize,
    x: i32,
    y: i32,
    blend: bool,
) -> Vec<f32> {
    let mut dst = bg.to_vec();
    
    // Calculate visible region
    let fg_start_x = (-x).max(0) as usize;
    let fg_start_y = (-y).max(0) as usize;
    let bg_start_x = x.max(0) as usize;
    let bg_start_y = y.max(0) as usize;
    
    let copy_w = (fg_w - fg_start_x).min(bg_w.saturating_sub(bg_start_x));
    let copy_h = (fg_h - fg_start_y).min(bg_h.saturating_sub(bg_start_y));
    
    if copy_w == 0 || copy_h == 0 {
        return dst;
    }
    
    let use_alpha = blend && channels == 4;
    
    for row in 0..copy_h {
        let fg_y = fg_start_y + row;
        let bg_y = bg_start_y + row;
        
        for col in 0..copy_w {
            let fg_x = fg_start_x + col;
            let bg_x = bg_start_x + col;
            
            let fg_idx = (fg_y * fg_w + fg_x) * channels;
            let bg_idx = (bg_y * bg_w + bg_x) * channels;
            
            if use_alpha {
                // Alpha blending: out = fg * fg_a + bg * (1 - fg_a)
                let fg_a = fg[fg_idx + 3];
                let inv_a = 1.0 - fg_a;
                
                for c in 0..3 {
                    dst[bg_idx + c] = fg[fg_idx + c] * fg_a + dst[bg_idx + c] * inv_a;
                }
                // Combine alpha: out_a = fg_a + bg_a * (1 - fg_a)
                dst[bg_idx + 3] = fg_a + dst[bg_idx + 3] * inv_a;
            } else {
                // Direct copy
                for c in 0..channels {
                    dst[bg_idx + c] = fg[fg_idx + c];
                }
            }
        }
    }
    
    dst
}

/// Rotates image by arbitrary angle (in degrees).
///
/// Uses bilinear interpolation. Output dimensions are adjusted to fit
/// the rotated image. Rotation is around image center.
///
/// # Arguments
///
/// * `src` - Source pixel data
/// * `width`, `height` - Source dimensions
/// * `channels` - Number of channels
/// * `angle_deg` - Rotation angle in degrees (positive = counter-clockwise)
/// * `bg` - Background color for empty areas (length = channels)
///
/// # Returns
///
/// Tuple of (new_data, new_width, new_height).
///
/// # Example
///
/// ```rust
/// use vfx_ops::transform::rotate;
///
/// let src = vec![1.0f32; 64 * 64 * 3];
/// let (dst, w, h) = rotate(&src, 64, 64, 3, 45.0, &[0.0, 0.0, 0.0]);
/// ```
pub fn rotate(
    src: &[f32],
    width: usize,
    height: usize,
    channels: usize,
    angle_deg: f32,
    bg: &[f32],
) -> (Vec<f32>, usize, usize) {
    let angle_rad = angle_deg.to_radians();
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();
    
    // Calculate new dimensions to fit rotated image
    let w = width as f32;
    let h = height as f32;
    
    // Corners after rotation
    let corners = [
        (0.0, 0.0),
        (w, 0.0),
        (0.0, h),
        (w, h),
    ];
    
    let cx = w / 2.0;
    let cy = h / 2.0;
    
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;
    
    for (px, py) in corners {
        let dx = px - cx;
        let dy = py - cy;
        let rx = dx * cos_a - dy * sin_a + cx;
        let ry = dx * sin_a + dy * cos_a + cy;
        min_x = min_x.min(rx);
        max_x = max_x.max(rx);
        min_y = min_y.min(ry);
        max_y = max_y.max(ry);
    }
    
    let new_w = (max_x - min_x).ceil() as usize;
    let new_h = (max_y - min_y).ceil() as usize;
    
    let offset_x = min_x;
    let offset_y = min_y;
    
    let mut dst = vec![0.0f32; new_w * new_h * channels];
    
    // Fill with background
    for y in 0..new_h {
        for x in 0..new_w {
            let idx = (y * new_w + x) * channels;
            for c in 0..channels {
                dst[idx + c] = bg.get(c).copied().unwrap_or(0.0);
            }
        }
    }
    
    // Inverse transform: for each dst pixel, find src pixel
    for dy in 0..new_h {
        for dx in 0..new_w {
            // Map dst coords to rotated space
            let px = dx as f32 + offset_x;
            let py = dy as f32 + offset_y;
            
            // Inverse rotation to find source coords
            let rx = px - cx;
            let ry = py - cy;
            let sx = rx * cos_a + ry * sin_a + cx;
            let sy = -rx * sin_a + ry * cos_a + cy;
            
            // Bilinear interpolation
            if sx >= 0.0 && sx < w - 1.0 && sy >= 0.0 && sy < h - 1.0 {
                let x0 = sx.floor() as usize;
                let y0 = sy.floor() as usize;
                let x1 = x0 + 1;
                let y1 = y0 + 1;
                
                let fx = sx - sx.floor();
                let fy = sy - sy.floor();
                
                let dst_idx = (dy * new_w + dx) * channels;
                
                for c in 0..channels {
                    let p00 = src[(y0 * width + x0) * channels + c];
                    let p10 = src[(y0 * width + x1) * channels + c];
                    let p01 = src[(y1 * width + x0) * channels + c];
                    let p11 = src[(y1 * width + x1) * channels + c];
                    
                    // Bilinear blend
                    let top = p00 * (1.0 - fx) + p10 * fx;
                    let bot = p01 * (1.0 - fx) + p11 * fx;
                    dst[dst_idx + c] = top * (1.0 - fy) + bot * fy;
                }
            }
        }
    }
    
    (dst, new_w, new_h)
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

    #[test]
    fn test_paste_simple() {
        // 4x4 black background, paste 2x2 white in center
        let bg = vec![0.0f32; 4 * 4]; // grayscale
        let fg = vec![1.0f32; 2 * 2];
        
        let result = paste(&bg, 4, 4, &fg, 2, 2, 1, 1, 1, false);
        
        assert_eq!(result.len(), 16);
        // Top-left stays black
        assert!((result[0] - 0.0).abs() < 0.01);
        // Center (1,1) should be white
        assert!((result[5] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_paste_negative_offset() {
        // Paste with negative offset (clipping)
        let bg = vec![0.0f32; 4 * 4];
        let fg = vec![1.0f32; 2 * 2];
        
        let result = paste(&bg, 4, 4, &fg, 2, 2, 1, -1, -1, false);
        
        // Only bottom-right of fg should appear at (0,0) of bg
        assert!((result[0] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_rotate_0_degrees() {
        // Larger image for better bilinear accuracy
        let src = vec![0.5f32; 4 * 4]; // 4x4 uniform gray
        let bg = vec![0.0];
        let (dst, w, h) = rotate(&src, 4, 4, 1, 0.0, &bg);
        
        assert_eq!(w, 4);
        assert_eq!(h, 4);
        // Center pixels should preserve value with bilinear interp
        assert!((dst[5] - 0.5).abs() < 0.2);
    }

    #[test]
    fn test_rotate_90_degrees() {
        // Simple 2x2 image
        let src = vec![1.0, 2.0, 3.0, 4.0];
        let bg = vec![0.0];
        let (dst, w, h) = rotate(&src, 2, 2, 1, 90.0, &bg);
        
        // Rotated 90 degrees
        assert_eq!(w, 2);
        assert_eq!(h, 2);
        assert_eq!(dst.len(), 4);
    }

    #[test]
    fn test_rotate_45_expands() {
        // 2x2 rotated 45 degrees should expand
        let src = vec![1.0f32; 4];
        let bg = vec![0.0];
        let (dst, w, h) = rotate(&src, 2, 2, 1, 45.0, &bg);
        
        // Rotated 45 should have larger bounding box
        assert!(w >= 2);
        assert!(h >= 2);
        assert!(dst.len() >= 4);
    }
}
