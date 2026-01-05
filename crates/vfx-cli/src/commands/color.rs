//! Color transform command
//!
//! Applies color adjustments: exposure, gamma, saturation, transfer functions.
//! Supports `--layer` for processing specific layers in multi-layer EXR.

use crate::ColorArgs;
use anyhow::Result;
use vfx_io::ImageData;

pub fn run(args: ColorArgs, verbose: u8, allow_non_color: bool) -> Result<()> {
    let image = super::load_image_layer(&args.input, args.layer.as_deref())?;
    super::ensure_color_processing(&image, "color", allow_non_color)?;
    let mut data = image.to_f32();
    let w = image.width as usize;
    let h = image.height as usize;
    let c = image.channels as usize;

    if verbose {
        println!("Applying color transforms to {}", args.input.display());
    }

    // Apply exposure adjustment
    if let Some(stops) = args.exposure {
        if verbose { println!("  Exposure: {:+.2} stops", stops); }
        let factor = 2.0f32.powf(stops);
        for v in &mut data {
            *v *= factor;
        }
    }

    // Apply gamma
    if let Some(gamma) = args.gamma {
        if verbose { println!("  Gamma: {:.2}", gamma); }
        for v in &mut data {
            if *v > 0.0 {
                *v = v.powf(gamma);
            }
        }
    }

    // Apply saturation
    if let Some(sat) = args.saturation {
        if verbose { println!("  Saturation: {:.2}", sat); }
        apply_saturation(&mut data, w, h, c, sat);
    }

    // Apply transfer function
    if let Some(ref tf) = args.transfer {
        if verbose { println!("  Transfer: {}", tf); }
        apply_transfer(&mut data, tf);
    }

    let output = ImageData::from_f32(image.width, image.height, image.channels, data);
    super::save_image_layer(&args.output, &output, args.layer.as_deref())?;

    if verbose {
        println!("Done.");
    }

    Ok(())
}

fn apply_saturation(data: &mut [f32], width: usize, height: usize, channels: usize, sat: f32) {
    if channels < 3 { return; }

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * channels;
            let r = data[idx];
            let g = data[idx + 1];
            let b = data[idx + 2];

            // Luminance (Rec.709)
            let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;

            // Interpolate between grayscale and color
            data[idx] = lum + (r - lum) * sat;
            data[idx + 1] = lum + (g - lum) * sat;
            data[idx + 2] = lum + (b - lum) * sat;
        }
    }
}

fn apply_transfer(data: &mut [f32], tf: &str) {
    match tf.to_lowercase().as_str() {
        "srgb" | "srgb_to_linear" => {
            for v in data.iter_mut() {
                *v = srgb_to_linear(*v);
            }
        }
        "linear_to_srgb" => {
            for v in data.iter_mut() {
                *v = linear_to_srgb(*v);
            }
        }
        "rec709" => {
            for v in data.iter_mut() {
                *v = rec709_to_linear(*v);
            }
        }
        _ => {}
    }
}

fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(v: f32) -> f32 {
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

fn rec709_to_linear(v: f32) -> f32 {
    if v < 0.081 {
        v / 4.5
    } else {
        ((v + 0.099) / 1.099).powf(1.0 / 0.45)
    }
}
