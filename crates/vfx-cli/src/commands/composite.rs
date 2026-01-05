//! Composite command

use crate::CompositeArgs;
use tracing::{debug, info, trace};
use anyhow::{Result, bail};
use vfx_io::ImageData;
use vfx_ops::composite::{over, blend, BlendMode};

pub fn run(args: CompositeArgs, verbose: u8, allow_non_color: bool) -> Result<()> {
    let fg = super::load_image(&args.fg)?;
    let bg = super::load_image(&args.bg)?;
    super::ensure_color_processing(&fg, "composite", allow_non_color)?;
    super::ensure_color_processing(&bg, "composite", allow_non_color)?;

    if fg.width != bg.width || fg.height != bg.height {
        bail!("Image dimensions don't match: {}x{} vs {}x{}",
            fg.width, fg.height, bg.width, bg.height);
    }

    if verbose > 0 {
        println!("Compositing {} over {} with mode '{}'",
            args.fg.display(), args.bg.display(), args.mode);
    }

    let fg_data = fg.to_f32();
    let bg_data = bg.to_f32();
    let w = fg.width as usize;
    let h = fg.height as usize;

    let result = match args.mode.to_lowercase().as_str() {
        "over" => over(&fg_data, &bg_data, w, h)?,
        "add" => blend(&fg_data, &bg_data, w, h, BlendMode::Add)?,
        "multiply" | "mult" => blend(&fg_data, &bg_data, w, h, BlendMode::Multiply)?,
        "screen" => blend(&fg_data, &bg_data, w, h, BlendMode::Screen)?,
        _ => bail!("Unknown blend mode: {}", args.mode),
    };

    let output = ImageData::from_f32(fg.width, fg.height, fg.channels, result);

    super::save_image(&args.output, &output)?;

    if verbose > 0 {
        println!("Done.");
    }

    Ok(())
}
