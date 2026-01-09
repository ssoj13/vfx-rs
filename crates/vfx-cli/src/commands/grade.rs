//! CDL grading command.
//!
//! Applies ASC CDL (Color Decision List) grading with slope, offset, power, and saturation.

use std::path::PathBuf;
use anyhow::Result;
use clap::Args;

use vfx_io::ImageData;

use super::{load_image_layer, save_image_layer, ensure_color_processing};
use crate::log_verbose;

/// Arguments for the `grade` command.
#[derive(Args)]
pub struct GradeArgs {
    /// Input image
    pub input: PathBuf,

    /// Output image
    #[arg(short, long)]
    pub output: PathBuf,

    /// Slope (R,G,B) - multiplier before offset. Default: 1,1,1
    #[arg(long, default_value = "1,1,1")]
    pub slope: String,

    /// Offset (R,G,B) - added after slope. Default: 0,0,0
    #[arg(long, default_value = "0,0,0")]
    pub offset: String,

    /// Power (R,G,B) - gamma/power applied last. Default: 1,1,1
    #[arg(long, default_value = "1,1,1")]
    pub power: String,

    /// Saturation multiplier (0=grayscale, 1=unchanged, >1=oversaturated)
    #[arg(long, default_value = "1.0")]
    pub saturation: f32,

    /// Process only this layer (for multi-layer EXR)
    #[arg(long)]
    pub layer: Option<String>,
}

/// Parse comma-separated RGB values.
fn parse_rgb(s: &str) -> Result<[f32; 3]> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 3 {
        anyhow::bail!("Expected 3 values (R,G,B), got {}", parts.len());
    }
    Ok([
        parts[0].trim().parse()?,
        parts[1].trim().parse()?,
        parts[2].trim().parse()?,
    ])
}

/// Run the grade command.
pub fn run(args: GradeArgs, verbose: u8, allow_non_color: bool) -> Result<()> {
    log_verbose(&format!("Grading: {}", args.input.display()), verbose);

    let image = load_image_layer(&args.input, args.layer.as_deref())?;
    ensure_color_processing(&image, "grade", allow_non_color)?;

    let slope = parse_rgb(&args.slope)?;
    let offset = parse_rgb(&args.offset)?;
    let power = parse_rgb(&args.power)?;
    let saturation = args.saturation;

    log_verbose(&format!("  Slope: {:?}", slope), verbose);
    log_verbose(&format!("  Offset: {:?}", offset), verbose);
    log_verbose(&format!("  Power: {:?}", power), verbose);
    log_verbose(&format!("  Saturation: {}", saturation), verbose);

    // Apply CDL formula: out = (in * slope + offset) ^ power
    let mut data = image.to_f32();
    let channels = image.channels as usize;

    for chunk in data.chunks_mut(channels) {
        // Apply slope and offset
        let r = chunk[0] * slope[0] + offset[0];
        let g = if channels > 1 { chunk[1] * slope[1] + offset[1] } else { r };
        let b = if channels > 2 { chunk[2] * slope[2] + offset[2] } else { r };

        // Apply power (handling negatives)
        let r = if r >= 0.0 { r.powf(power[0]) } else { -(-r).powf(power[0]) };
        let g = if g >= 0.0 { g.powf(power[1]) } else { -(-g).powf(power[1]) };
        let b = if b >= 0.0 { b.powf(power[2]) } else { -(-b).powf(power[2]) };

        // Apply saturation
        if (saturation - 1.0).abs() > 0.001 {
            let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            chunk[0] = luma + (r - luma) * saturation;
            if channels > 1 { chunk[1] = luma + (g - luma) * saturation; }
            if channels > 2 { chunk[2] = luma + (b - luma) * saturation; }
        } else {
            chunk[0] = r;
            if channels > 1 { chunk[1] = g; }
            if channels > 2 { chunk[2] = b; }
        }
    }

    let output = ImageData::from_f32(image.width, image.height, image.channels, data);
    save_image_layer(&args.output, &output, args.layer.as_deref())?;
    log_verbose(&format!("Saved: {}", args.output.display()), verbose);

    Ok(())
}
