//! Deep image operations.
//!
//! Operations for working with deep images (images with multiple samples per pixel
//! at different Z depths). These are commonly used in VFX for deep compositing.
//!
//! # Operations
//!
//! - [`flatten`] - Flatten a deep image to a regular (flat) image
//! - [`deepen`] - Convert a flat image to a deep image
//! - [`deep_merge`] - Merge two deep images together
//! - [`deep_holdout`] - Apply holdout to a deep image
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::imagebufalgo::deep::{flatten, deepen, deep_merge};
//! use vfx_io::deepdata::DeepData;
//!
//! // Flatten a deep image to regular image
//! let flat = flatten(&deep_data, width, height);
//!
//! // Deepen a flat image
//! let deep = deepen(&image_buf, z_value);
//!
//! // Merge two deep images
//! let merged = deep_merge(&deep1, &deep2);
//! ```

use crate::deepdata::DeepData;
use crate::imagebuf::{ImageBuf, InitializePixels};
use vfx_core::{ImageSpec, TypeDesc};

/// Flattens a deep image to a regular (flat) image.
///
/// Composites all samples at each pixel using over blending, ordered by Z depth.
/// The result is a standard RGBA image.
///
/// # Arguments
///
/// * `deep` - The deep image data
/// * `width` - Width of the output image
/// * `height` - Height of the output image
///
/// # Returns
///
/// A flat RGBA image with composited pixel values.
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::deep::flatten_deep;
///
/// let flat_image = flatten_deep(&deep_data, 1920, 1080);
/// ```
pub fn flatten_deep(deep: &DeepData, width: u32, height: u32) -> ImageBuf {
    let spec = ImageSpec::rgba(width, height);
    let mut dst = ImageBuf::new(spec, InitializePixels::Yes);

    flatten_deep_into(&mut dst, deep);
    dst
}

/// Flattens a deep image into an existing ImageBuf.
pub fn flatten_deep_into(dst: &mut ImageBuf, deep: &DeepData) {
    let width = dst.width() as i64;
    let height = dst.height() as i64;
    let npixels = width * height;

    // Find color and alpha channels in deep data
    let nch = deep.channels();
    let a_ch = deep.a_channel();
    let z_ch = deep.z_channel();

    // Build channel mapping (R, G, B, A indices in deep data)
    let mut r_ch = -1i32;
    let mut g_ch = -1i32;
    let mut b_ch = -1i32;

    for c in 0..nch {
        let name = deep.channelname(c).to_lowercase();
        match name.as_str() {
            "r" | "red" => r_ch = c as i32,
            "g" | "green" => g_ch = c as i32,
            "b" | "blue" => b_ch = c as i32,
            _ => {}
        }
    }

    for pixel in 0..npixels.min(deep.pixels()) {
        let nsamps = deep.samples(pixel) as usize;
        if nsamps == 0 {
            continue;
        }

        // Sort samples by Z (deep data should already be sorted)
        // Composite using over blending
        let mut out_r = 0.0f32;
        let mut out_g = 0.0f32;
        let mut out_b = 0.0f32;
        let mut out_a = 0.0f32;

        for s in 0..nsamps {
            let a = if a_ch >= 0 {
                deep.deep_value(pixel, a_ch as usize, s)
            } else {
                1.0
            };

            let r = if r_ch >= 0 {
                deep.deep_value(pixel, r_ch as usize, s)
            } else {
                0.0
            };
            let g = if g_ch >= 0 {
                deep.deep_value(pixel, g_ch as usize, s)
            } else {
                0.0
            };
            let b = if b_ch >= 0 {
                deep.deep_value(pixel, b_ch as usize, s)
            } else {
                0.0
            };

            // Over blending: out = fg + bg * (1 - fg_alpha)
            let inv_a = 1.0 - a;
            out_r = r + out_r * inv_a;
            out_g = g + out_g * inv_a;
            out_b = b + out_b * inv_a;
            out_a = a + out_a * inv_a;

            // Early out if fully opaque
            if out_a >= 0.9999 {
                break;
            }
        }

        // Write to destination
        let x = (pixel % width) as i32;
        let y = (pixel / width) as i32;
        let rgba = [out_r, out_g, out_b, out_a];
        dst.setpixel(x, y, 0, &rgba);
    }
}

/// Converts a flat image to a deep image with one sample per pixel.
///
/// Each pixel becomes a single sample at the specified Z depth.
/// Useful for integrating flat elements into deep compositing pipelines.
///
/// # Arguments
///
/// * `src` - Source flat image
/// * `z_value` - Z depth to assign to all pixels
///
/// # Returns
///
/// A DeepData with one sample per pixel.
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::deep::deepen;
///
/// let deep = deepen(&flat_image, 100.0);
/// ```
pub fn deepen(src: &ImageBuf, z_value: f32) -> DeepData {
    let width = src.width() as i64;
    let height = src.height() as i64;
    let npixels = width * height;
    let src_nch = src.nchannels() as usize;

    // Build channel types and names for deep data (add Z channel)
    let mut channeltypes = vec![TypeDesc::FLOAT; src_nch + 1];
    let mut channelnames: Vec<String> = src.spec()
        .channel_names
        .iter()
        .cloned()
        .collect();

    // If no channel names, generate defaults
    if channelnames.is_empty() {
        channelnames = match src_nch {
            1 => vec!["Y".to_string()],
            2 => vec!["Y".to_string(), "A".to_string()],
            3 => vec!["R".to_string(), "G".to_string(), "B".to_string()],
            4 => vec!["R".to_string(), "G".to_string(), "B".to_string(), "A".to_string()],
            _ => (0..src_nch).map(|i| format!("channel{}", i)).collect(),
        };
    }
    channelnames.push("Z".to_string());

    let deep = DeepData::new(npixels, &channeltypes, &channelnames.iter().map(|s| s.as_str()).collect::<Vec<_>>());

    // Set all capacities first
    for pixel in 0..npixels {
        deep.set_capacity(pixel, 1);
        deep.set_samples(pixel, 1);
    }

    // Copy pixel data
    let mut src_pixel = vec![0.0f32; src_nch];

    for pixel in 0..npixels {
        let x = (pixel % width) as i32;
        let y = (pixel / width) as i32;

        src.getpixel(x, y, 0, &mut src_pixel, crate::imagebuf::WrapMode::Black);

        // Set color/alpha channels
        for c in 0..src_nch {
            deep.set_deep_value_f32(pixel, c, 0, src_pixel[c]);
        }

        // Set Z channel
        deep.set_deep_value_f32(pixel, src_nch, 0, z_value);
    }

    deep
}

/// Converts a flat image to a deep image with Z values from a separate depth image.
///
/// # Arguments
///
/// * `src` - Source flat image (color/alpha)
/// * `z_src` - Source Z depth image (single channel)
///
/// # Returns
///
/// A DeepData with one sample per pixel, Z from z_src.
pub fn deepen_with_z(src: &ImageBuf, z_src: &ImageBuf) -> DeepData {
    let width = src.width() as i64;
    let height = src.height() as i64;
    let npixels = width * height;
    let src_nch = src.nchannels() as usize;

    // Build channel types and names
    let channeltypes = vec![TypeDesc::FLOAT; src_nch + 1];
    let mut channelnames: Vec<String> = src.spec()
        .channel_names
        .iter()
        .cloned()
        .collect();

    if channelnames.is_empty() {
        channelnames = match src_nch {
            1 => vec!["Y".to_string()],
            2 => vec!["Y".to_string(), "A".to_string()],
            3 => vec!["R".to_string(), "G".to_string(), "B".to_string()],
            4 => vec!["R".to_string(), "G".to_string(), "B".to_string(), "A".to_string()],
            _ => (0..src_nch).map(|i| format!("channel{}", i)).collect(),
        };
    }
    channelnames.push("Z".to_string());

    let deep = DeepData::new(npixels, &channeltypes, &channelnames.iter().map(|s| s.as_str()).collect::<Vec<_>>());

    for pixel in 0..npixels {
        deep.set_capacity(pixel, 1);
        deep.set_samples(pixel, 1);
    }

    let mut src_pixel = vec![0.0f32; src_nch];
    let mut z_pixel = vec![0.0f32; 1];

    for pixel in 0..npixels {
        let x = (pixel % width) as i32;
        let y = (pixel / width) as i32;

        src.getpixel(x, y, 0, &mut src_pixel, crate::imagebuf::WrapMode::Black);
        z_src.getpixel(x, y, 0, &mut z_pixel, crate::imagebuf::WrapMode::Black);

        for c in 0..src_nch {
            deep.set_deep_value_f32(pixel, c, 0, src_pixel[c]);
        }
        deep.set_deep_value_f32(pixel, src_nch, 0, z_pixel[0]);
    }

    deep
}

/// Merges two deep images together.
///
/// Combines samples from both images at each pixel, sorting by Z depth
/// and merging overlapping samples.
///
/// # Arguments
///
/// * `a` - First deep image
/// * `b` - Second deep image
///
/// # Returns
///
/// A merged DeepData containing samples from both inputs.
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::deep::deep_merge;
///
/// let merged = deep_merge(&foreground_deep, &background_deep);
/// ```
pub fn deep_merge(a: &DeepData, b: &DeepData) -> DeepData {
    let npixels = a.pixels().max(b.pixels());

    // Use channel types from first image
    let channeltypes = a.all_channeltypes();
    let channelnames: Vec<String> = (0..a.channels())
        .map(|c| a.channelname(c))
        .collect();

    let result = DeepData::new(
        npixels,
        &channeltypes,
        &channelnames.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
    );

    deep_merge_into(&result, a, b);
    result
}

/// Merges two deep images into an existing DeepData.
pub fn deep_merge_into(dst: &DeepData, a: &DeepData, b: &DeepData) {
    let npixels = dst.pixels().min(a.pixels()).min(b.pixels());

    for pixel in 0..npixels {
        let a_samples = a.samples(pixel);
        let b_samples = b.samples(pixel);
        let total = a_samples + b_samples;

        if total == 0 {
            continue;
        }

        // Set capacity and copy samples
        dst.set_capacity(pixel, total);
        dst.set_samples(pixel, total);

        // Copy samples from a
        for s in 0..a_samples as usize {
            dst.copy_deep_sample(pixel, s, a, pixel, s);
        }

        // Copy samples from b
        for s in 0..b_samples as usize {
            dst.copy_deep_sample(pixel, (a_samples as usize) + s, b, pixel, s);
        }

        // Sort by Z and merge overlaps
        dst.sort(pixel);
        dst.merge_overlaps(pixel);
    }
}

/// Applies holdout to a deep image.
///
/// Removes samples that are behind (greater Z) the holdout depth.
/// This is used to cut holes in deep images for inserting other elements.
///
/// # Arguments
///
/// * `deep` - Deep image to modify
/// * `holdout_z` - Z depth at which to cut
pub fn deep_holdout(deep: &DeepData, holdout_z: f32) {
    let npixels = deep.pixels();
    let z_ch = deep.z_channel();

    if z_ch < 0 {
        return;
    }

    for pixel in 0..npixels {
        let nsamps = deep.samples(pixel) as usize;

        // Find samples to remove (from back to front for stable indices)
        let mut to_remove = Vec::new();
        for s in (0..nsamps).rev() {
            let z = deep.deep_value(pixel, z_ch as usize, s);
            if z > holdout_z {
                to_remove.push(s);
            }
        }

        // Remove samples
        for s in to_remove {
            deep.erase_samples(pixel, s, 1);
        }
    }
}

/// Applies holdout using a holdout matte (per-pixel cutout).
///
/// # Arguments
///
/// * `deep` - Deep image to modify
/// * `holdout` - Holdout deep image (samples define cutout regions)
pub fn deep_holdout_matte(deep: &DeepData, holdout: &DeepData) {
    let npixels = deep.pixels().min(holdout.pixels());
    let z_ch = deep.z_channel();

    if z_ch < 0 {
        return;
    }

    for pixel in 0..npixels {
        // Get minimum Z from holdout (closest holdout surface)
        let holdout_samples = holdout.samples(pixel) as usize;
        if holdout_samples == 0 {
            continue;
        }

        let mut min_holdout_z = f32::INFINITY;
        for s in 0..holdout_samples {
            let z = holdout.deep_value(pixel, z_ch as usize, s);
            min_holdout_z = min_holdout_z.min(z);
        }

        if min_holdout_z == f32::INFINITY {
            continue;
        }

        // Remove samples behind holdout
        let nsamps = deep.samples(pixel) as usize;
        let mut to_remove = Vec::new();
        for s in (0..nsamps).rev() {
            let z = deep.deep_value(pixel, z_ch as usize, s);
            if z > min_holdout_z {
                to_remove.push(s);
            }
        }

        for s in to_remove {
            deep.erase_samples(pixel, s, 1);
        }
    }
}

/// Tidies a deep image by sorting, merging overlaps, and culling occluded samples.
///
/// This is a cleanup operation that should be called after manipulating deep data.
///
/// # Arguments
///
/// * `deep` - Deep image to tidy
pub fn deep_tidy(deep: &DeepData) {
    let npixels = deep.pixels();

    for pixel in 0..npixels {
        deep.sort(pixel);
        deep.merge_overlaps(pixel);
        deep.occlusion_cull(pixel);
    }
}

/// Returns statistics about a deep image.
#[derive(Debug, Clone, Default)]
pub struct DeepStats {
    /// Total number of samples across all pixels
    pub total_samples: u64,
    /// Maximum samples in any single pixel
    pub max_samples_per_pixel: u32,
    /// Average samples per pixel
    pub avg_samples_per_pixel: f64,
    /// Number of pixels with zero samples
    pub empty_pixels: u64,
    /// Number of pixels with multiple samples
    pub multi_sample_pixels: u64,
    /// Minimum Z depth
    pub min_z: f32,
    /// Maximum Z depth
    pub max_z: f32,
}

/// Computes statistics about a deep image.
pub fn deep_stats(deep: &DeepData) -> DeepStats {
    let npixels = deep.pixels();
    let z_ch = deep.z_channel();

    let mut stats = DeepStats {
        min_z: f32::INFINITY,
        max_z: f32::NEG_INFINITY,
        ..Default::default()
    };

    for pixel in 0..npixels {
        let nsamps = deep.samples(pixel);
        stats.total_samples += nsamps as u64;
        stats.max_samples_per_pixel = stats.max_samples_per_pixel.max(nsamps);

        if nsamps == 0 {
            stats.empty_pixels += 1;
        } else if nsamps > 1 {
            stats.multi_sample_pixels += 1;
        }

        if z_ch >= 0 {
            for s in 0..nsamps as usize {
                let z = deep.deep_value(pixel, z_ch as usize, s);
                stats.min_z = stats.min_z.min(z);
                stats.max_z = stats.max_z.max(z);
            }
        }
    }

    if npixels > 0 {
        stats.avg_samples_per_pixel = stats.total_samples as f64 / npixels as f64;
    }

    if stats.min_z == f32::INFINITY {
        stats.min_z = 0.0;
    }
    if stats.max_z == f32::NEG_INFINITY {
        stats.max_z = 0.0;
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flatten_empty() {
        let types = vec![TypeDesc::FLOAT; 5];
        let names = vec!["R", "G", "B", "A", "Z"];
        let deep = DeepData::new(100, &types, &names);

        let flat = flatten(&deep, 10, 10);
        assert_eq!(flat.width(), 10);
        assert_eq!(flat.height(), 10);
    }

    #[test]
    fn test_flatten_single_sample() {
        let types = vec![TypeDesc::FLOAT; 5];
        let names = vec!["R", "G", "B", "A", "Z"];
        let deep = DeepData::new(4, &types, &names);

        // Set one sample for pixel 0
        deep.set_capacity(0, 1);
        deep.set_samples(0, 1);
        deep.set_deep_value_f32(0, 0, 0, 1.0); // R
        deep.set_deep_value_f32(0, 1, 0, 0.0); // G
        deep.set_deep_value_f32(0, 2, 0, 0.0); // B
        deep.set_deep_value_f32(0, 3, 0, 1.0); // A
        deep.set_deep_value_f32(0, 4, 0, 1.0); // Z

        let flat = flatten(&deep, 2, 2);
        let mut pixel = [0.0f32; 4];
        flat.getpixel(0, 0, 0, &mut pixel, crate::imagebuf::WrapMode::Black);

        assert!((pixel[0] - 1.0).abs() < 0.01); // R
        assert!(pixel[1].abs() < 0.01); // G
        assert!(pixel[2].abs() < 0.01); // B
        assert!((pixel[3] - 1.0).abs() < 0.01); // A
    }

    #[test]
    fn test_deepen() {
        let spec = ImageSpec::rgba(4, 4);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        // Set pixel 0 to red
        src.setpixel(0, 0, 0, &[1.0f32, 0.0, 0.0, 1.0]);

        let deep = deepen(&src, 5.0);

        assert_eq!(deep.pixels(), 16);
        assert_eq!(deep.samples(0), 1);

        // Check R channel
        assert!((deep.deep_value(0, 0, 0) - 1.0).abs() < 0.01);
        // Check Z channel (last channel)
        let z_ch = deep.z_channel();
        assert!((deep.deep_value(0, z_ch as usize, 0) - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_deep_merge() {
        let types = vec![TypeDesc::FLOAT; 5];
        let names = vec!["R", "G", "B", "A", "Z"];

        let a = DeepData::new(4, &types, &names);
        let b = DeepData::new(4, &types, &names);

        // Set sample in a
        a.set_capacity(0, 1);
        a.set_samples(0, 1);
        a.set_deep_value_f32(0, 4, 0, 1.0); // Z = 1

        // Set sample in b
        b.set_capacity(0, 1);
        b.set_samples(0, 1);
        b.set_deep_value_f32(0, 4, 0, 2.0); // Z = 2

        let merged = deep_merge(&a, &b);

        // Should have 2 samples merged
        assert!(merged.samples(0) >= 1);
    }

    #[test]
    fn test_deep_stats() {
        let types = vec![TypeDesc::FLOAT; 5];
        let names = vec!["R", "G", "B", "A", "Z"];
        let deep = DeepData::new(10, &types, &names);

        // Set up all capacities BEFORE setting any values
        // (this is required because ensure_allocated() computes cumcapacity once)
        deep.set_capacity(0, 2);
        deep.set_capacity(1, 1);

        // Now set samples and values
        deep.set_samples(0, 2);
        deep.set_samples(1, 1);

        deep.set_deep_value_f32(0, 4, 0, 1.0);
        deep.set_deep_value_f32(0, 4, 1, 5.0);
        deep.set_deep_value_f32(1, 4, 0, 3.0);

        let stats = deep_stats(&deep);

        assert_eq!(stats.total_samples, 3);
        assert_eq!(stats.max_samples_per_pixel, 2);
        assert_eq!(stats.multi_sample_pixels, 1);
        assert!((stats.min_z - 1.0).abs() < 0.01);
        assert!((stats.max_z - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_deep_holdout() {
        let types = vec![TypeDesc::FLOAT; 5];
        let names = vec!["R", "G", "B", "A", "Z"];
        let deep = DeepData::new(1, &types, &names);

        deep.set_capacity(0, 3);
        deep.set_samples(0, 3);
        deep.set_deep_value_f32(0, 4, 0, 1.0); // Z = 1
        deep.set_deep_value_f32(0, 4, 1, 5.0); // Z = 5
        deep.set_deep_value_f32(0, 4, 2, 10.0); // Z = 10

        // Holdout at Z = 6 should remove Z = 10 sample
        deep_holdout(&deep, 6.0);

        assert_eq!(deep.samples(0), 2);
    }
}
