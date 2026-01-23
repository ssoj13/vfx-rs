//! Warp/distortion operations.
//!
//! Provides lens distortion and artistic warp effects.
//! Based on ST map approach - each effect computes source coordinates for each destination pixel.
//!
//! When the `parallel` feature is enabled, uses rayon for multi-threaded processing.

use std::f32::consts::PI;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Normalize pixel coordinate to [0, 1] range.
#[inline]
fn normalize(coord: usize, size: usize) -> f32 {
    if size <= 1 { 0.5 } else { coord as f32 / (size - 1) as f32 }
}

/// Normalize to [-1, 1] centered.
#[inline]
fn normalize_centered(coord: usize, size: usize) -> f32 {
    normalize(coord, size).mul_add(2.0, -1.0)
}

/// Bilinear sample from source image.
fn sample_bilinear(src: &[f32], w: usize, h: usize, ch: usize, x: f32, y: f32) -> Vec<f32> {
    let mut result = vec![0.0; ch];
    
    if x < 0.0 || x >= (w - 1) as f32 || y < 0.0 || y >= (h - 1) as f32 {
        return result; // Out of bounds - return black
    }
    
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);
    
    let fx = x - x.floor();
    let fy = y - y.floor();
    
    for c in 0..ch {
        let p00 = src[(y0 * w + x0) * ch + c];
        let p10 = src[(y0 * w + x1) * ch + c];
        let p01 = src[(y1 * w + x0) * ch + c];
        let p11 = src[(y1 * w + x1) * ch + c];
        
        let top = p00 * (1.0 - fx) + p10 * fx;
        let bot = p01 * (1.0 - fx) + p11 * fx;
        result[c] = top * (1.0 - fy) + bot * fy;
    }
    
    result
}

/// Apply a generic warp using a coordinate generator function.
/// 
/// Uses rayon for parallel processing when the `parallel` feature is enabled.
#[cfg(feature = "parallel")]
fn apply_warp<F>(src: &[f32], w: usize, h: usize, ch: usize, coord_fn: F) -> Vec<f32>
where
    F: Fn(usize, usize, usize, usize) -> (f32, f32) + Sync, // (x, y, w, h) -> (src_x, src_y)
{
    let mut dst = vec![0.0; w * h * ch];
    
    // Process rows in parallel
    dst.par_chunks_mut(w * ch)
        .enumerate()
        .for_each(|(y, row)| {
            for x in 0..w {
                let (sx, sy) = coord_fn(x, y, w, h);
                let sample = sample_bilinear(src, w, h, ch, sx, sy);
                let idx = x * ch;
                row[idx..idx + ch].copy_from_slice(&sample);
            }
        });
    
    dst
}

/// Apply a generic warp using a coordinate generator function (single-threaded fallback).
#[cfg(not(feature = "parallel"))]
fn apply_warp<F>(src: &[f32], w: usize, h: usize, ch: usize, coord_fn: F) -> Vec<f32>
where
    F: Fn(usize, usize, usize, usize) -> (f32, f32), // (x, y, w, h) -> (src_x, src_y)
{
    let mut dst = vec![0.0; w * h * ch];
    
    for y in 0..h {
        for x in 0..w {
            let (sx, sy) = coord_fn(x, y, w, h);
            let sample = sample_bilinear(src, w, h, ch, sx, sy);
            let idx = (y * w + x) * ch;
            dst[idx..idx + ch].copy_from_slice(&sample);
        }
    }
    
    dst
}

// === Lens Distortion Effects ===

/// Apply barrel distortion (wide-angle lens effect).
/// 
/// Uses radial model: r' = r * (1 + k1*r² + k2*r⁴)
/// 
/// # Arguments
/// * `k1` - Primary distortion coefficient (positive for barrel)
/// * `k2` - Secondary distortion coefficient
pub fn barrel(src: &[f32], w: usize, h: usize, ch: usize, k1: f32, k2: f32) -> Vec<f32> {
    let k1 = k1.abs();
    let k2 = k2.abs();
    
    apply_warp(src, w, h, ch, |x, y, w, h| {
        let nx = normalize_centered(x, w);
        let ny = normalize_centered(y, h);
        
        let r2 = nx * nx + ny * ny;
        let r4 = r2 * r2;
        let factor = 1.0 + k1 * r2 + k2 * r4;
        
        let dx = nx * factor;
        let dy = ny * factor;
        
        let sx = (dx + 1.0) / 2.0 * (w - 1) as f32;
        let sy = (dy + 1.0) / 2.0 * (h - 1) as f32;
        
        (sx, sy)
    })
}

/// Apply pincushion distortion (telephoto lens effect).
/// 
/// Uses radial model with negative coefficients.
pub fn pincushion(src: &[f32], w: usize, h: usize, ch: usize, k1: f32, k2: f32) -> Vec<f32> {
    barrel(src, w, h, ch, -k1.abs(), -k2.abs())
}

/// Apply fisheye distortion.
/// 
/// # Arguments
/// * `strength` - Distortion strength (1.0 = standard fisheye)
pub fn fisheye(src: &[f32], w: usize, h: usize, ch: usize, strength: f32) -> Vec<f32> {
    apply_warp(src, w, h, ch, |x, y, w, h| {
        let nx = normalize_centered(x, w);
        let ny = normalize_centered(y, h);
        
        let r = nx.hypot(ny);
        
        if r < 0.001 {
            return ((w - 1) as f32 / 2.0, (h - 1) as f32 / 2.0);
        }
        
        let theta = r * strength;
        let new_r = if strength.abs() > 0.001 {
            theta.sin() / strength.sin().max(0.001)
        } else {
            r
        };
        
        let factor = new_r / r;
        let dx = nx * factor;
        let dy = ny * factor;
        
        let sx = (dx + 1.0) / 2.0 * (w - 1) as f32;
        let sy = (dy + 1.0) / 2.0 * (h - 1) as f32;
        
        (sx, sy)
    })
}

// === Artistic Effects ===

/// Apply twist/swirl effect.
/// 
/// Rotates pixels based on distance from center.
/// 
/// # Arguments
/// * `angle_deg` - Maximum twist angle at center
/// * `radius` - Effect radius (0.5 = half image, normalized)
pub fn twist(src: &[f32], w: usize, h: usize, ch: usize, angle_deg: f32, radius: f32) -> Vec<f32> {
    let angle_rad = angle_deg.to_radians();
    
    apply_warp(src, w, h, ch, |x, y, w, h| {
        let nx = normalize(x, w);
        let ny = normalize(y, h);
        
        let dx = nx - 0.5;
        let dy = ny - 0.5;
        let dist = dx.hypot(dy);
        
        let twist_amount = if dist < radius {
            angle_rad * (1.0 - dist / radius)
        } else {
            0.0
        };
        
        let cos_a = twist_amount.cos();
        let sin_a = twist_amount.sin();
        let rx = dx * cos_a - dy * sin_a;
        let ry = dx * sin_a + dy * cos_a;
        
        let sx = (0.5 + rx) * (w - 1) as f32;
        let sy = (0.5 + ry) * (h - 1) as f32;
        
        (sx, sy)
    })
}

/// Apply wave/sine distortion.
/// 
/// # Arguments
/// * `amplitude` - Wave amplitude (normalized, e.g., 0.02)
/// * `frequency` - Number of waves across image
pub fn wave(src: &[f32], w: usize, h: usize, ch: usize, amplitude: f32, frequency: f32) -> Vec<f32> {
    apply_warp(src, w, h, ch, |x, y, w, h| {
        let nx = normalize(x, w);
        let ny = normalize(y, h);
        
        // X displacement based on Y
        let wave_x = amplitude * (ny * frequency * 2.0 * PI).sin();
        
        let sx = (nx + wave_x).clamp(0.0, 1.0) * (w - 1) as f32;
        let sy = ny * (h - 1) as f32;
        
        (sx, sy)
    })
}

/// Apply spherize/bulge effect.
/// 
/// # Arguments
/// * `strength` - Effect strength (-1 to 1, positive = bulge out)
/// * `radius` - Effect radius (normalized)
pub fn spherize(src: &[f32], w: usize, h: usize, ch: usize, strength: f32, radius: f32) -> Vec<f32> {
    apply_warp(src, w, h, ch, |x, y, w, h| {
        let nx = normalize(x, w);
        let ny = normalize(y, h);
        
        let dx = nx - 0.5;
        let dy = ny - 0.5;
        let dist = dx.hypot(dy);
        let norm_dist = dist / radius;
        
        if norm_dist >= 1.0 {
            return (x as f32, y as f32);
        }
        
        let factor = if strength >= 0.0 {
            let power = 1.0 - strength * 0.9;
            norm_dist.powf(power) / norm_dist.max(1e-6)
        } else {
            let power = 1.0 + (-strength) * 0.9;
            norm_dist.powf(power) / norm_dist.max(1e-6)
        };
        
        let sx = (0.5 + dx * factor) * (w - 1) as f32;
        let sy = (0.5 + dy * factor) * (h - 1) as f32;
        
        (sx.clamp(0.0, (w - 1) as f32), sy.clamp(0.0, (h - 1) as f32))
    })
}

/// Apply concentric ripple effect.
/// 
/// # Arguments
/// * `amplitude` - Ripple amplitude (normalized)
/// * `frequency` - Number of ripples
/// * `decay` - Decay factor (0 = no decay)
pub fn ripple(src: &[f32], w: usize, h: usize, ch: usize, amplitude: f32, frequency: f32, decay: f32) -> Vec<f32> {
    apply_warp(src, w, h, ch, |x, y, w, h| {
        let nx = normalize(x, w);
        let ny = normalize(y, h);
        
        let dx = nx - 0.5;
        let dy = ny - 0.5;
        let dist = dx.hypot(dy);
        
        if dist < 1e-6 {
            return (x as f32, y as f32);
        }
        
        let wave_val = (dist * frequency * 2.0 * PI).sin();
        let decay_factor = if decay > 0.0 { (-dist * decay).exp() } else { 1.0 };
        let displacement = amplitude * wave_val * decay_factor;
        
        let factor = 1.0 + displacement / dist;
        let sx = (0.5 + dx * factor).clamp(0.0, 1.0) * (w - 1) as f32;
        let sy = (0.5 + dy * factor).clamp(0.0, 1.0) * (h - 1) as f32;
        
        (sx, sy)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_image(w: usize, h: usize, ch: usize) -> Vec<f32> {
        vec![0.5f32; w * h * ch]
    }

    #[test]
    fn test_barrel_preserves_center() {
        let src = make_test_image(64, 64, 3);
        let dst = barrel(&src, 64, 64, 3, 0.2, 0.0);
        
        // Center pixel should be approximately unchanged
        let center = 32 * 64 + 32;
        assert!((dst[center * 3] - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_pincushion_preserves_center() {
        let src = make_test_image(64, 64, 3);
        let dst = pincushion(&src, 64, 64, 3, 0.2, 0.0);
        
        let center = 32 * 64 + 32;
        assert!((dst[center * 3] - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_fisheye_preserves_center() {
        let src = make_test_image(64, 64, 3);
        let dst = fisheye(&src, 64, 64, 3, 1.0);
        
        let center = 32 * 64 + 32;
        assert!((dst[center * 3] - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_twist_output_size() {
        let src = make_test_image(64, 64, 3);
        let dst = twist(&src, 64, 64, 3, 90.0, 0.5);
        
        assert_eq!(dst.len(), src.len());
    }

    #[test]
    fn test_wave_output_size() {
        let src = make_test_image(64, 64, 3);
        let dst = wave(&src, 64, 64, 3, 0.02, 5.0);
        
        assert_eq!(dst.len(), src.len());
    }

    #[test]
    fn test_spherize_outside_radius() {
        let src = make_test_image(64, 64, 3);
        let dst = spherize(&src, 64, 64, 3, 0.5, 0.1);
        
        // Corners should be unchanged
        assert!((dst[0] - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_ripple_output_size() {
        let src = make_test_image(64, 64, 3);
        let dst = ripple(&src, 64, 64, 3, 0.03, 8.0, 0.0);
        
        assert_eq!(dst.len(), src.len());
    }
}
