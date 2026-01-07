//! Texture processing utilities including mipmap generation.
//!
//! This module provides functions for generating texture mipmaps
//! with various filter options for high-quality downsampling.

use crate::imagebuf::{ImageBuf, InitializePixels, WrapMode};

/// Mipmap filter options for downsampling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MipmapFilter {
    /// Box filter (simple averaging) - fastest but lower quality.
    Box,
    /// Bilinear filter - good balance of speed and quality.
    #[default]
    Bilinear,
    /// Lanczos filter - highest quality, slower.
    Lanczos,
    /// Kaiser filter - sharp results, good for textures with fine detail.
    Kaiser,
}

/// Options for mipmap generation.
#[derive(Debug, Clone)]
pub struct MipmapOptions {
    /// Filter to use for downsampling.
    pub filter: MipmapFilter,
    /// Whether to use sRGB-aware filtering.
    pub srgb: bool,
    /// Whether to premultiply alpha before filtering.
    pub premultiply_alpha: bool,
    /// Wrap mode for edge pixels.
    pub wrap: WrapMode,
}

impl Default for MipmapOptions {
    fn default() -> Self {
        Self {
            filter: MipmapFilter::Bilinear,
            srgb: false,
            premultiply_alpha: true,
            wrap: WrapMode::Clamp,
        }
    }
}

/// Generate a complete mipmap chain from the source image.
///
/// Returns a vector of images, starting with level 0 (copy of source)
/// down to the smallest level (typically 1x1).
pub fn make_texture(src: &ImageBuf, options: &MipmapOptions) -> Vec<ImageBuf> {
    let width = src.width();
    let height = src.height();
    
    let max_dim = width.max(height);
    let num_levels = (max_dim as f32).log2().ceil() as usize + 1;
    
    let mut mipmaps = Vec::with_capacity(num_levels);
    mipmaps.push(src.clone());
    
    let mut current = src.clone();
    let mut w = width;
    let mut h = height;
    
    while w > 1 || h > 1 {
        let new_w = (w / 2).max(1);
        let new_h = (h / 2).max(1);
        
        let mip = downsample_2x(&current, new_w, new_h, options);
        mipmaps.push(mip.clone());
        
        current = mip;
        w = new_w;
        h = new_h;
    }
    
    mipmaps
}

/// Generate a single mip level by downsampling the source.
pub fn make_mip_level(src: &ImageBuf, level: u32, options: &MipmapOptions) -> ImageBuf {
    if level == 0 {
        return src.clone();
    }
    
    let mut w = src.width();
    let mut h = src.height();
    let mut current = src.clone();
    
    for _ in 0..level {
        if w <= 1 && h <= 1 {
            break;
        }
        let new_w = (w / 2).max(1);
        let new_h = (h / 2).max(1);
        current = downsample_2x(&current, new_w, new_h, options);
        w = new_w;
        h = new_h;
    }
    
    current
}

/// Calculate the number of mip levels for given dimensions.
pub fn mip_level_count(width: u32, height: u32) -> u32 {
    let max_dim = width.max(height);
    (max_dim as f32).log2().ceil() as u32 + 1
}

/// Calculate dimensions at a specific mip level.
pub fn mip_dimensions(width: u32, height: u32, level: u32) -> (u32, u32) {
    let w = (width >> level).max(1);
    let h = (height >> level).max(1);
    (w, h)
}

/// Downsample image by 2x.
fn downsample_2x(src: &ImageBuf, new_w: u32, new_h: u32, options: &MipmapOptions) -> ImageBuf {
    let src_w = src.width();
    let src_h = src.height();
    let nch = src.nchannels() as usize;
    
    let mut spec = src.spec().clone();
    spec.width = new_w;
    spec.height = new_h;
    
    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    
    let mut src_pixel = vec![0.0f32; nch];
    let mut dst_pixel = vec![0.0f32; nch];
    
    match options.filter {
        MipmapFilter::Box | MipmapFilter::Bilinear => {
            // Simple 2x2 box filter
            for dy in 0..new_h as i32 {
                for dx in 0..new_w as i32 {
                    let sx = dx * 2;
                    let sy = dy * 2;
                    
                    let mut sum = vec![0.0f32; nch];
                    let mut count = 0.0;
                    
                    for oy in 0..2 {
                        for ox in 0..2 {
                            let px = (sx + ox).min(src_w as i32 - 1);
                            let py = (sy + oy).min(src_h as i32 - 1);
                            src.getpixel(px, py, 0, &mut src_pixel, options.wrap);
                            
                            for c in 0..nch {
                                let val = if options.srgb && c < 3 {
                                    srgb_to_linear(src_pixel[c])
                                } else {
                                    src_pixel[c]
                                };
                                sum[c] += val;
                            }
                            count += 1.0;
                        }
                    }
                    
                    for c in 0..nch {
                        let mut val = sum[c] / count;
                        if options.srgb && c < 3 {
                            val = linear_to_srgb(val);
                        }
                        dst_pixel[c] = val;
                    }
                    
                    dst.setpixel(dx, dy, 0, &dst_pixel);
                }
            }
        }
        MipmapFilter::Lanczos | MipmapFilter::Kaiser => {
            // Higher quality filters with larger kernel
            let radius = 3i32;
            let scale_x = src_w as f32 / new_w as f32;
            let scale_y = src_h as f32 / new_h as f32;
            
            for dy in 0..new_h as i32 {
                for dx in 0..new_w as i32 {
                    let sx = (dx as f32 + 0.5) * scale_x - 0.5;
                    let sy = (dy as f32 + 0.5) * scale_y - 0.5;
                    let x0 = sx.floor() as i32;
                    let y0 = sy.floor() as i32;
                    
                    let mut sum = vec![0.0f32; nch];
                    let mut weight_sum = 0.0;
                    
                    for ky in -radius..=radius {
                        for kx in -radius..=radius {
                            let px = (x0 + kx).clamp(0, src_w as i32 - 1);
                            let py = (y0 + ky).clamp(0, src_h as i32 - 1);
                            
                            let dx_s = sx - (x0 + kx) as f32;
                            let dy_s = sy - (y0 + ky) as f32;
                            
                            let w = match options.filter {
                                MipmapFilter::Lanczos => {
                                    lanczos_weight(dx_s, radius as f32) * lanczos_weight(dy_s, radius as f32)
                                }
                                _ => {
                                    kaiser_weight(dx_s, radius as f32) * kaiser_weight(dy_s, radius as f32)
                                }
                            };
                            
                            if w.abs() > 1e-6 {
                                src.getpixel(px, py, 0, &mut src_pixel, options.wrap);
                                for c in 0..nch {
                                    let val = if options.srgb && c < 3 {
                                        srgb_to_linear(src_pixel[c])
                                    } else {
                                        src_pixel[c]
                                    };
                                    sum[c] += val * w;
                                }
                                weight_sum += w;
                            }
                        }
                    }
                    
                    if weight_sum.abs() > 1e-6 {
                        for c in 0..nch {
                            let mut val = sum[c] / weight_sum;
                            if options.srgb && c < 3 {
                                val = linear_to_srgb(val);
                            }
                            dst_pixel[c] = val;
                        }
                    }
                    
                    dst.setpixel(dx, dy, 0, &dst_pixel);
                }
            }
        }
    }
    
    dst
}

fn lanczos_weight(x: f32, a: f32) -> f32 {
    if x.abs() < 1e-6 { return 1.0; }
    if x.abs() >= a { return 0.0; }
    let pi_x = std::f32::consts::PI * x;
    let pi_x_a = pi_x / a;
    (pi_x.sin() / pi_x) * (pi_x_a.sin() / pi_x_a)
}

fn kaiser_weight(x: f32, radius: f32) -> f32 {
    if x.abs() >= radius { return 0.0; }
    let r = x / radius;
    let beta = 4.0;
    let arg = beta * (1.0 - r * r).max(0.0).sqrt();
    bessel_i0(arg) / bessel_i0(beta)
}

fn bessel_i0(x: f32) -> f32 {
    let ax = x.abs();
    if ax < 3.75 {
        let y = (x / 3.75).powi(2);
        1.0 + y * (3.5156229 + y * (3.0899424 + y * (1.2067492 
            + y * (0.2659732 + y * (0.0360768 + y * 0.0045813)))))
    } else {
        let y = 3.75 / ax;
        (ax.exp() / ax.sqrt()) * (0.39894228 + y * (0.01328592 
            + y * (0.00225319 + y * (-0.00157565 + y * (0.00916281 
            + y * (-0.02057706 + y * (0.02635537 + y * (-0.01647633 
            + y * 0.00392377))))))))
    }
}

fn srgb_to_linear(x: f32) -> f32 {
    if x <= 0.04045 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4) }
}

fn linear_to_srgb(x: f32) -> f32 {
    if x <= 0.0031308 { x * 12.92 } else { 1.055 * x.powf(1.0 / 2.4) - 0.055 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mip_level_count() {
        assert_eq!(mip_level_count(1024, 1024), 11);
        assert_eq!(mip_level_count(512, 512), 10);
        assert_eq!(mip_level_count(1, 1), 1);
    }

    #[test]
    fn test_mip_dimensions() {
        assert_eq!(mip_dimensions(1024, 1024, 0), (1024, 1024));
        assert_eq!(mip_dimensions(1024, 1024, 1), (512, 512));
        assert_eq!(mip_dimensions(1024, 1024, 10), (1, 1));
    }

    #[test]
    fn test_make_texture() {
        let spec = ImageSpec::rgb(4, 4);
        let mut img = ImageBuf::new(spec, InitializePixels::No);
        for y in 0..4i32 {
            for x in 0..4i32 {
                img.setpixel(x, y, 0, &[x as f32 / 3.0, y as f32 / 3.0, 0.5]);
            }
        }
        
        let mipmaps = make_texture(&img, &MipmapOptions::default());
        
        assert_eq!(mipmaps.len(), 3); // 4x4, 2x2, 1x1
        assert_eq!(mipmaps[0].width(), 4);
        assert_eq!(mipmaps[1].width(), 2);
        assert_eq!(mipmaps[2].width(), 1);
    }

    #[test]
    fn test_lanczos_kernel() {
        assert!((lanczos_weight(0.0, 3.0) - 1.0).abs() < 1e-5);
        assert!(lanczos_weight(3.0, 3.0).abs() < 1e-5);
    }

    #[test]
    fn test_srgb_roundtrip() {
        for v in [0.0, 0.1, 0.5, 0.9, 1.0] {
            let linear = srgb_to_linear(v);
            let back = linear_to_srgb(linear);
            assert!((v - back).abs() < 1e-5);
        }
    }
}
