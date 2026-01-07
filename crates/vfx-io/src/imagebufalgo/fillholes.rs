//! Hole filling algorithms for images with missing data.
//!
//! This module implements the push-pull algorithm for filling holes
//! (invalid/missing pixels) in images.

use crate::imagebuf::{ImageBuf, InitializePixels, WrapMode};

/// Options for the push-pull hole filling algorithm.
#[derive(Debug, Clone)]
pub struct FillHolesOptions {
    /// Alpha channel index (if -1, uses last channel or detects automatically).
    pub alpha_channel: i32,
    /// Threshold below which a pixel is considered a hole.
    pub alpha_threshold: f32,
    /// Whether to dilate the result by one pixel to ensure coverage.
    pub dilate: bool,
    /// Maximum pyramid levels (0 = automatic).
    pub max_levels: u32,
}

impl Default for FillHolesOptions {
    fn default() -> Self {
        Self {
            alpha_channel: -1,
            alpha_threshold: 0.001,
            dilate: true,
            max_levels: 0,
        }
    }
}

/// Fill holes in an image using the push-pull algorithm.
///
/// The algorithm identifies holes using the alpha channel and fills them
/// by propagating valid pixel values through a mipmap pyramid.
pub fn fillholes_pushpull(src: &ImageBuf, options: &FillHolesOptions) -> ImageBuf {
    let w = src.width() as i32;
    let h = src.height() as i32;
    let nch = src.nchannels() as usize;
    
    assert!(w > 0 && h > 0, "Empty image");
    
    // Determine alpha channel
    let alpha_ch = if options.alpha_channel >= 0 {
        options.alpha_channel as usize
    } else if nch == 4 || nch == 2 {
        nch - 1
    } else {
        return src.clone(); // No alpha, nothing to fill
    };
    
    assert!(alpha_ch < nch, "Alpha channel out of range");
    
    // Calculate pyramid levels
    let max_dim = w.max(h) as u32;
    let auto_levels = (max_dim as f32).log2().ceil() as u32;
    let num_levels = if options.max_levels > 0 {
        options.max_levels.min(auto_levels)
    } else {
        auto_levels
    };
    
    // Build pyramid (push phase)
    let pyramid = build_pyramid(src, alpha_ch, options.alpha_threshold, num_levels);
    
    // Reconstruct (pull phase)
    let mut result = pull_phase(&pyramid, alpha_ch, options.alpha_threshold);
    
    // Optional dilation
    if options.dilate {
        dilate_edges(&mut result, alpha_ch, options.alpha_threshold);
    }
    
    result
}

struct PyramidLevel {
    buf: ImageBuf,
}

fn build_pyramid(src: &ImageBuf, alpha_ch: usize, threshold: f32, num_levels: u32) -> Vec<PyramidLevel> {
    let mut pyramid = Vec::with_capacity(num_levels as usize);
    pyramid.push(PyramidLevel { buf: src.clone() });
    
    let mut w = src.width();
    let mut h = src.height();
    
    for _ in 1..num_levels {
        if w <= 1 && h <= 1 { break; }
        
        let new_w = (w / 2).max(1);
        let new_h = (h / 2).max(1);
        
        let prev = &pyramid.last().unwrap().buf;
        let next = downsample_with_alpha(prev, new_w, new_h, alpha_ch, threshold);
        
        pyramid.push(PyramidLevel { buf: next });
        w = new_w;
        h = new_h;
    }
    
    pyramid
}

fn downsample_with_alpha(src: &ImageBuf, dst_w: u32, dst_h: u32, alpha_ch: usize, threshold: f32) -> ImageBuf {
    let src_w = src.width() as i32;
    let src_h = src.height() as i32;
    let nch = src.nchannels() as usize;
    
    let mut spec = src.spec().clone();
    spec.width = dst_w;
    spec.height = dst_h;
    
    let mut dst = ImageBuf::new(spec, InitializePixels::Yes);
    let mut src_pixel = vec![0.0f32; nch];
    let mut dst_pixel = vec![0.0f32; nch];
    
    for dy in 0..dst_h as i32 {
        for dx in 0..dst_w as i32 {
            let sx = dx * 2;
            let sy = dy * 2;
            
            let mut sum = vec![0.0f32; nch];
            let mut weight_sum = 0.0;
            
            for oy in 0..2 {
                for ox in 0..2 {
                    let px = (sx + ox).min(src_w - 1);
                    let py = (sy + oy).min(src_h - 1);
                    src.getpixel(px, py, 0, &mut src_pixel, WrapMode::Clamp);
                    
                    let alpha = src_pixel[alpha_ch];
                    if alpha > threshold {
                        for c in 0..nch {
                            sum[c] += src_pixel[c] * alpha;
                        }
                        weight_sum += alpha;
                    }
                }
            }
            
            if weight_sum > 0.0 {
                for c in 0..nch {
                    dst_pixel[c] = sum[c] / weight_sum;
                }
                dst.setpixel(dx, dy, 0, &dst_pixel);
            }
        }
    }
    
    dst
}

fn pull_phase(pyramid: &[PyramidLevel], alpha_ch: usize, threshold: f32) -> ImageBuf {
    if pyramid.is_empty() {
        return ImageBuf::new_uninit();
    }
    
    let mut current = pyramid.last().unwrap().buf.clone();
    
    for level_idx in (0..pyramid.len() - 1).rev() {
        let finer = &pyramid[level_idx].buf;
        current = upsample_and_blend(&current, finer, alpha_ch, threshold);
    }
    
    current
}

fn upsample_and_blend(coarse: &ImageBuf, fine: &ImageBuf, alpha_ch: usize, threshold: f32) -> ImageBuf {
    let fine_w = fine.width() as i32;
    let fine_h = fine.height() as i32;
    let coarse_w = coarse.width() as i32;
    let coarse_h = coarse.height() as i32;
    let nch = fine.nchannels() as usize;
    
    let mut result = ImageBuf::new(fine.spec().clone(), InitializePixels::No);
    let mut fine_pixel = vec![0.0f32; nch];
    let mut coarse_pixel = vec![0.0f32; nch];
    
    for fy in 0..fine_h {
        for fx in 0..fine_w {
            fine.getpixel(fx, fy, 0, &mut fine_pixel, WrapMode::Clamp);
            
            if fine_pixel[alpha_ch] > threshold {
                // Fine pixel is valid, use it
                result.setpixel(fx, fy, 0, &fine_pixel);
            } else {
                // Interpolate from coarse
                let cx = (fx as f32 / fine_w as f32 * coarse_w as f32).floor() as i32;
                let cy = (fy as f32 / fine_h as f32 * coarse_h as f32).floor() as i32;
                
                coarse.getpixel(cx.min(coarse_w - 1), cy.min(coarse_h - 1), 0, &mut coarse_pixel, WrapMode::Clamp);
                result.setpixel(fx, fy, 0, &coarse_pixel);
            }
        }
    }
    
    result
}

fn dilate_edges(img: &mut ImageBuf, alpha_ch: usize, threshold: f32) {
    let w = img.width() as i32;
    let h = img.height() as i32;
    let nch = img.nchannels() as usize;
    
    let mut temp = img.clone();
    let mut pixel = vec![0.0f32; nch];
    let mut neighbor = vec![0.0f32; nch];
    
    for y in 0..h {
        for x in 0..w {
            img.getpixel(x, y, 0, &mut pixel, WrapMode::Clamp);
            
            if pixel[alpha_ch] <= threshold {
                let mut sum = vec![0.0f32; nch];
                let mut count = 0;
                
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let nx = x + dx;
                        let ny = y + dy;
                        if nx >= 0 && nx < w && ny >= 0 && ny < h {
                            img.getpixel(nx, ny, 0, &mut neighbor, WrapMode::Clamp);
                            if neighbor[alpha_ch] > threshold {
                                for c in 0..nch {
                                    sum[c] += neighbor[c];
                                }
                                count += 1;
                            }
                        }
                    }
                }
                
                if count > 0 {
                    for c in 0..nch {
                        pixel[c] = sum[c] / count as f32;
                    }
                    temp.setpixel(x, y, 0, &pixel);
                }
            }
        }
    }
    
    *img = temp;
}

/// Check if an image has any holes.
pub fn has_holes(src: &ImageBuf, options: &FillHolesOptions) -> bool {
    let nch = src.nchannels() as usize;
    
    let alpha_ch = if options.alpha_channel >= 0 {
        options.alpha_channel as usize
    } else if nch == 4 || nch == 2 {
        nch - 1
    } else {
        return false;
    };
    
    if alpha_ch >= nch { return false; }
    
    let mut pixel = vec![0.0f32; nch];
    for y in 0..src.height() as i32 {
        for x in 0..src.width() as i32 {
            src.getpixel(x, y, 0, &mut pixel, WrapMode::Clamp);
            if pixel[alpha_ch] <= options.alpha_threshold {
                return true;
            }
        }
    }
    
    false
}

/// Count the number of hole pixels.
pub fn count_holes(src: &ImageBuf, options: &FillHolesOptions) -> usize {
    let nch = src.nchannels() as usize;
    
    let alpha_ch = if options.alpha_channel >= 0 {
        options.alpha_channel as usize
    } else if nch == 4 || nch == 2 {
        nch - 1
    } else {
        return 0;
    };
    
    if alpha_ch >= nch { return 0; }
    
    let mut count = 0;
    let mut pixel = vec![0.0f32; nch];
    for y in 0..src.height() as i32 {
        for x in 0..src.width() as i32 {
            src.getpixel(x, y, 0, &mut pixel, WrapMode::Clamp);
            if pixel[alpha_ch] <= options.alpha_threshold {
                count += 1;
            }
        }
    }
    
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use vfx_core::ImageSpec;

    #[test]
    fn test_no_holes() {
        let spec = ImageSpec::rgba(4, 4);
        let mut img = ImageBuf::new(spec, InitializePixels::No);
        for y in 0..4i32 {
            for x in 0..4i32 {
                img.setpixel(x, y, 0, &[0.5, 0.5, 0.5, 1.0]);
            }
        }
        
        let opts = FillHolesOptions::default();
        assert!(!has_holes(&img, &opts));
        assert_eq!(count_holes(&img, &opts), 0);
    }

    #[test]
    fn test_with_holes() {
        let spec = ImageSpec::rgba(4, 4);
        let mut img = ImageBuf::new(spec, InitializePixels::No);
        for y in 0..4i32 {
            for x in 0..4i32 {
                img.setpixel(x, y, 0, &[0.5, 0.5, 0.5, 1.0]);
            }
        }
        // Make some holes
        img.setpixel(1, 1, 0, &[0.0, 0.0, 0.0, 0.0]);
        img.setpixel(2, 2, 0, &[0.0, 0.0, 0.0, 0.0]);
        
        let opts = FillHolesOptions::default();
        assert!(has_holes(&img, &opts));
        assert_eq!(count_holes(&img, &opts), 2);
    }

    #[test]
    fn test_fillholes() {
        let spec = ImageSpec::rgba(4, 4);
        let mut img = ImageBuf::new(spec, InitializePixels::No);
        
        // Fill with red
        for y in 0..4i32 {
            for x in 0..4i32 {
                img.setpixel(x, y, 0, &[1.0, 0.0, 0.0, 1.0]);
            }
        }
        // Make center a hole
        img.setpixel(1, 1, 0, &[0.0, 0.0, 0.0, 0.0]);
        
        let opts = FillHolesOptions::default();
        let filled = fillholes_pushpull(&img, &opts);
        
        let mut pixel = [0.0f32; 4];
        filled.getpixel(1, 1, 0, &mut pixel, WrapMode::Clamp);
        
        // Should be filled with red
        assert!(pixel[0] > 0.5, "Red channel should be filled");
    }
}
