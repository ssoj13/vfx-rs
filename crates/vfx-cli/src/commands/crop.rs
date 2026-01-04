//! Crop command

use crate::CropArgs;
use anyhow::Result;
use vfx_io::ImageData;
use vfx_ops::transform::crop;

pub fn run(args: CropArgs, verbose: bool, allow_non_color: bool) -> Result<()> {
    let image = super::load_image(&args.input)?;
    super::ensure_color_processing(&image, "crop", allow_non_color)?;
    let w = image.width as usize;
    let h = image.height as usize;
    let c = image.channels as usize;

    if verbose {
        println!("Cropping {}x{} @ ({},{}) from {}x{}", args.w, args.h, args.x, args.y, w, h);
    }

    let src_data = image.to_f32();
    let cropped = crop(&src_data, w, h, c, args.x, args.y, args.w, args.h)?;

    let output = ImageData::from_f32(args.w as u32, args.h as u32, image.channels, cropped);

    super::save_image(&args.output, &output)?;

    if verbose {
        println!("Done.");
    }

    Ok(())
}
