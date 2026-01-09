//! Alpha premultiplication command.
//!
//! Controls alpha premultiplication state of images.

use std::path::PathBuf;
use anyhow::{Result, bail};
use clap::Args;

use vfx_io::ImageData;

use super::{load_image_layer, save_image_layer};
use crate::log_verbose;

/// Arguments for the `premult` command.
#[derive(Args)]
pub struct PremultArgs {
    /// Input image
    pub input: PathBuf,

    /// Output image
    #[arg(short, long)]
    pub output: PathBuf,

    /// Premultiply RGB by alpha
    #[arg(long, group = "mode")]
    pub premultiply: bool,

    /// Unpremultiply RGB by alpha (divide)
    #[arg(long, group = "mode")]
    pub unpremultiply: bool,

    /// Process only this layer (for multi-layer EXR)
    #[arg(long)]
    pub layer: Option<String>,
}

/// Run the premult command.
pub fn run(args: PremultArgs, verbose: u8) -> Result<()> {
    if !args.premultiply && !args.unpremultiply {
        bail!("Must specify --premultiply or --unpremultiply");
    }

    log_verbose(&format!("Processing: {}", args.input.display()), verbose);

    let image = load_image_layer(&args.input, args.layer.as_deref())?;

    if image.channels < 4 {
        bail!("Image must have alpha channel (4 channels), found {}", image.channels);
    }

    let mut data = image.to_f32();
    let channels = image.channels as usize;

    if args.premultiply {
        log_verbose("  Mode: premultiply", verbose);
        for chunk in data.chunks_mut(channels) {
            let alpha = chunk[3];
            chunk[0] *= alpha;
            chunk[1] *= alpha;
            chunk[2] *= alpha;
        }
    } else {
        log_verbose("  Mode: unpremultiply", verbose);
        for chunk in data.chunks_mut(channels) {
            let alpha = chunk[3];
            if alpha > 1e-6 {
                chunk[0] /= alpha;
                chunk[1] /= alpha;
                chunk[2] /= alpha;
            }
        }
    }

    let output = ImageData::from_f32(image.width, image.height, image.channels, data);
    save_image_layer(&args.output, &output, args.layer.as_deref())?;
    log_verbose(&format!("Saved: {}", args.output.display()), verbose);

    Ok(())
}
