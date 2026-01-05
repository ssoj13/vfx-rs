//! Image diff command (like idiff)

use crate::DiffArgs;
use anyhow::{Result, bail};
use vfx_io::ImageData;

pub fn run(args: DiffArgs, verbose: u8, allow_non_color: bool) -> Result<()> {
    let img_a = super::load_image(&args.a)?;
    let img_b = super::load_image(&args.b)?;
    super::ensure_color_processing(&img_a, "diff", allow_non_color)?;
    super::ensure_color_processing(&img_b, "diff", allow_non_color)?;

    if img_a.width != img_b.width || img_a.height != img_b.height {
        bail!("Image dimensions don't match: {}x{} vs {}x{}",
            img_a.width, img_a.height, img_b.width, img_b.height);
    }

    // Compare common channels (RGB even if one is RGBA)
    let channels_a = img_a.channels as usize;
    let channels_b = img_b.channels as usize;
    let compare_channels = channels_a.min(channels_b);
    
    if channels_a != channels_b && verbose {
        println!("Note: Channel count differs ({} vs {}), comparing {} common channels",
            channels_a, channels_b, compare_channels);
    }

    let data_a = img_a.to_f32();
    let data_b = img_b.to_f32();
    let pixels = (img_a.width * img_a.height) as usize;
    
    let (max_diff, mean_diff, rms_diff, diff_pixels) = compute_diff(
        &data_a, &data_b, pixels, channels_a, channels_b, compare_channels
    );

    println!("Comparing {} vs {}", args.a.display(), args.b.display());
    println!("  Max difference:  {:.6}", max_diff);
    println!("  Mean difference: {:.6}", mean_diff);
    println!("  RMS difference:  {:.6}", rms_diff);
    println!("  Pixels differ:   {} ({:.2}%)",
        diff_pixels,
        100.0 * diff_pixels as f64 / (img_a.width as u64 * img_a.height as u64) as f64);

    // Save difference image if requested
    if let Some(ref output) = args.output {
        let diff_data = create_diff_image(&data_a, &data_b, pixels, channels_a, channels_b, compare_channels);
        let diff_image = ImageData::from_f32(img_a.width, img_a.height, compare_channels as u32, diff_data);
        super::save_image(output, &diff_image)?;
        if verbose {
            println!("Difference image saved to {}", output.display());
        }
    }

    // Check threshold
    if max_diff > args.threshold && args.threshold > 0.0 {
        bail!("FAIL: Max difference {} exceeds threshold {}", max_diff, args.threshold);
    }

    if let Some(warn) = args.warn {
        if max_diff > warn {
            println!("WARNING: Max difference {} exceeds warning threshold {}", max_diff, warn);
        }
    }

    println!("PASS");
    Ok(())
}

/// Compare images with potentially different channel counts.
/// Only compares the first `compare_channels` channels of each pixel.
fn compute_diff(
    a: &[f32], b: &[f32],
    pixels: usize,
    channels_a: usize, channels_b: usize,
    compare_channels: usize,
) -> (f32, f32, f32, usize) {
    let mut max_diff = 0.0f32;
    let mut sum_diff = 0.0f64;
    let mut sum_sq = 0.0f64;
    let mut diff_pixels = 0usize;
    let mut n = 0usize;

    for p in 0..pixels {
        let mut pixel_differs = false;
        for c in 0..compare_channels {
            let va = a[p * channels_a + c];
            let vb = b[p * channels_b + c];
            let d = (va - vb).abs();
            
            if d > max_diff { max_diff = d; }
            sum_diff += d as f64;
            sum_sq += (d * d) as f64;
            n += 1;
            
            if d > 1e-6 { pixel_differs = true; }
        }
        if pixel_differs { diff_pixels += 1; }
    }

    let n = n as f64;
    let mean = (sum_diff / n) as f32;
    let rms = (sum_sq / n).sqrt() as f32;

    (max_diff, mean, rms, diff_pixels)
}

/// Create difference image from two images with potentially different channel counts.
fn create_diff_image(
    a: &[f32], b: &[f32],
    pixels: usize,
    channels_a: usize, channels_b: usize,
    compare_channels: usize,
) -> Vec<f32> {
    let scale = 10.0; // Amplify differences for visibility
    let mut result = Vec::with_capacity(pixels * compare_channels);
    
    for p in 0..pixels {
        for c in 0..compare_channels {
            let va = a[p * channels_a + c];
            let vb = b[p * channels_b + c];
            let d = ((va - vb).abs() * scale).min(1.0);
            result.push(d);
        }
    }
    
    result
}
