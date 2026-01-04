//! Sharpen command

use crate::SharpenArgs;
use anyhow::Result;
use vfx_io::ImageData;
use vfx_ops::filter::{Kernel, convolve};

pub fn run(args: SharpenArgs, verbose: bool, allow_non_color: bool) -> Result<()> {
    let image = super::load_image(&args.input)?;
    super::ensure_color_processing(&image, "sharpen", allow_non_color)?;
    let w = image.width as usize;
    let h = image.height as usize;
    let c = image.channels as usize;

    if verbose {
        println!("Sharpening {} (amount={:.2})", args.input.display(), args.amount);
    }

    let src_data = image.to_f32();
    let kernel = Kernel::sharpen(args.amount);

    let sharpened = convolve(&src_data, w, h, c, &kernel)?;

    let output = ImageData::from_f32(image.width, image.height, image.channels, sharpened);

    super::save_image(&args.output, &output)?;

    if verbose {
        println!("Done.");
    }

    Ok(())
}
