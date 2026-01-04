//! Blur command
//!
//! Applies box or gaussian blur to images.
//! Supports `--layer` for processing specific layers in multi-layer EXR.

use crate::BlurArgs;
use anyhow::Result;
use vfx_io::ImageData;
use vfx_ops::filter::{box_blur, Kernel, convolve};

pub fn run(args: BlurArgs, verbose: bool, allow_non_color: bool) -> Result<()> {
    let image = super::load_image_layer(&args.input, args.layer.as_deref())?;
    super::ensure_color_processing(&image, "blur", allow_non_color)?;
    let w = image.width as usize;
    let h = image.height as usize;
    let c = image.channels as usize;

    if verbose {
        println!("Applying {} blur (radius={}) to {}",
            args.blur_type, args.radius, args.input.display());
    }

    let src_data = image.to_f32();

    let blurred = match args.blur_type.to_lowercase().as_str() {
        "box" => box_blur(&src_data, w, h, c, args.radius)?,
        "gaussian" | "gauss" => {
            let kernel = Kernel::gaussian(args.radius * 2 + 1, args.radius as f32 / 2.0);
            convolve(&src_data, w, h, c, &kernel)?
        }
        _ => box_blur(&src_data, w, h, c, args.radius)?,
    };

    let output = ImageData::from_f32(image.width, image.height, image.channels, blurred);

    super::save_image_layer(&args.output, &output, args.layer.as_deref())?;

    if verbose {
        println!("Done.");
    }

    Ok(())
}
