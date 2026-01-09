//! Clamp command.
//!
//! Clamps pixel values to a specified range.

use std::path::PathBuf;
use anyhow::Result;
use clap::Args;

use vfx_io::ImageData;

use super::{load_image_layer, save_image_layer};
use crate::log_verbose;

/// Arguments for the `clamp` command.
#[derive(Args)]
pub struct ClampArgs {
    /// Input image
    pub input: PathBuf,

    /// Output image
    #[arg(short, long)]
    pub output: PathBuf,

    /// Minimum value (default: 0.0)
    #[arg(long, default_value = "0.0")]
    pub min: f32,

    /// Maximum value (default: 1.0)
    #[arg(long, default_value = "1.0")]
    pub max: f32,

    /// Clamp only negative values (ignore --min/--max)
    #[arg(long)]
    pub negatives: bool,

    /// Clamp only values > 1.0 (ignore --min/--max)  
    #[arg(long)]
    pub fireflies: bool,

    /// Process only this layer (for multi-layer EXR)
    #[arg(long)]
    pub layer: Option<String>,
}

/// Run the clamp command.
pub fn run(args: ClampArgs, verbose: u8) -> Result<()> {
    log_verbose(&format!("Clamping: {}", args.input.display()), verbose);

    let image = load_image_layer(&args.input, args.layer.as_deref())?;

    let (min, max) = if args.negatives {
        log_verbose("  Mode: clamp negatives to 0", verbose);
        (0.0, f32::MAX)
    } else if args.fireflies {
        log_verbose("  Mode: clamp fireflies to 1.0", verbose);
        (f32::MIN, 1.0)
    } else {
        log_verbose(&format!("  Range: [{}, {}]", args.min, args.max), verbose);
        (args.min, args.max)
    };

    let mut data = image.to_f32();
    for p in data.iter_mut() {
        *p = p.clamp(min, max);
    }

    let output = ImageData::from_f32(image.width, image.height, image.channels, data);
    save_image_layer(&args.output, &output, args.layer.as_deref())?;
    log_verbose(&format!("Saved: {}", args.output.display()), verbose);

    Ok(())
}
