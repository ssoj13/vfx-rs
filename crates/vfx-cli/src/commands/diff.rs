//! Image diff command (like idiff)

use crate::DiffArgs;
use anyhow::{Result, bail};
use vfx_io::ImageData;

pub fn run(args: DiffArgs, verbose: bool) -> Result<()> {
    let img_a = super::load_image(&args.a)?;
    let img_b = super::load_image(&args.b)?;

    if img_a.width != img_b.width || img_a.height != img_b.height {
        bail!("Image dimensions don't match: {}x{} vs {}x{}",
            img_a.width, img_a.height, img_b.width, img_b.height);
    }

    if img_a.channels != img_b.channels {
        bail!("Channel count doesn't match: {} vs {}", img_a.channels, img_b.channels);
    }

    let data_a = img_a.to_f32();
    let data_b = img_b.to_f32();
    
    let (max_diff, mean_diff, rms_diff, diff_pixels) = compute_diff(&data_a, &data_b, img_a.channels as usize);

    println!("Comparing {} vs {}", args.a.display(), args.b.display());
    println!("  Max difference:  {:.6}", max_diff);
    println!("  Mean difference: {:.6}", mean_diff);
    println!("  RMS difference:  {:.6}", rms_diff);
    println!("  Pixels differ:   {} ({:.2}%)",
        diff_pixels,
        100.0 * diff_pixels as f64 / (img_a.width as u64 * img_a.height as u64) as f64);

    // Save difference image if requested
    if let Some(ref output) = args.output {
        let diff_data = create_diff_image(&data_a, &data_b);
        let diff_image = ImageData::from_f32(img_a.width, img_a.height, img_a.channels, diff_data);
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

fn compute_diff(a: &[f32], b: &[f32], channels: usize) -> (f32, f32, f32, usize) {
    let mut max_diff = 0.0f32;
    let mut sum_diff = 0.0f64;
    let mut sum_sq = 0.0f64;
    let mut diff_pixels = 0usize;

    let pixels = a.len() / channels;

    for i in 0..a.len() {
        let d = (a[i] - b[i]).abs();
        if d > max_diff { max_diff = d; }
        sum_diff += d as f64;
        sum_sq += (d * d) as f64;
    }

    // Count pixels that differ
    for p in 0..pixels {
        let mut differs = false;
        for c in 0..channels {
            let idx = p * channels + c;
            if (a[idx] - b[idx]).abs() > 1e-6 {
                differs = true;
                break;
            }
        }
        if differs { diff_pixels += 1; }
    }

    let n = a.len() as f64;
    let mean = (sum_diff / n) as f32;
    let rms = (sum_sq / n).sqrt() as f32;

    (max_diff, mean, rms, diff_pixels)
}

fn create_diff_image(a: &[f32], b: &[f32]) -> Vec<f32> {
    let scale = 10.0; // Amplify differences for visibility
    a.iter()
        .zip(b.iter())
        .map(|(&va, &vb)| ((va - vb).abs() * scale).min(1.0))
        .collect()
}
