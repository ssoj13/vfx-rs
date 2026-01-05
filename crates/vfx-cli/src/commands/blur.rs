//! Blur command
//!
//! Applies box or gaussian blur to images.
//! Supports `--layer` for processing specific layers in multi-layer EXR.

use crate::BlurArgs;
#[allow(unused_imports)]
use tracing::{debug, info, trace};
use anyhow::Result;
use vfx_io::ImageData;
use vfx_ops::filter::{box_blur, Kernel, convolve};

pub fn run(args: BlurArgs, verbose: u8, allow_non_color: bool) -> Result<()> {
    trace!(input = %args.input.display(), blur_type = %args.blur_type, radius = args.radius, "blur::run");
    
    let image = super::load_image_layer(&args.input, args.layer.as_deref())?;
    super::ensure_color_processing(&image, "blur", allow_non_color)?;
    let w = image.width as usize;
    let h = image.height as usize;
    let c = image.channels as usize;

    info!(blur_type = %args.blur_type, radius = args.radius, w = w, h = h, "Applying blur");
    
    if verbose > 0 {
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

    if verbose > 0 {
        println!("Done.");
    }

    Ok(())
}
