//! Texture creation command (like maketx)

use crate::MaketxArgs;
use anyhow::Result;
use vfx_io::ImageData;
use vfx_ops::resize::{resize_f32, Filter};

pub fn run(args: MaketxArgs, verbose: bool) -> Result<()> {
    let image = super::load_image(&args.input)?;

    if verbose {
        println!("Creating texture from {}", args.input.display());
        println!("  Tile size: {}", args.tile);
        println!("  Mipmaps: {}", args.mipmap);
        println!("  Filter: {}", args.filter);
    }

    // For now, just copy the image with mipmap generation if requested
    if args.mipmap {
        if verbose {
            println!("  Generating mipmaps...");
        }
        let mipmaps = generate_mipmaps(&image, &args.filter)?;
        if verbose {
            println!("  Generated {} mip levels", mipmaps.len());
        }
        // Save base level (full mipmap chain would need tiled format)
        super::save_image(&args.output, &image)?;
    } else {
        super::save_image(&args.output, &image)?;
    }

    if verbose {
        println!("Done.");
    }

    Ok(())
}

fn generate_mipmaps(image: &ImageData, filter: &str) -> Result<Vec<ImageData>> {
    let mut mipmaps = Vec::new();
    let mut data = image.to_f32();
    let mut w = image.width as usize;
    let mut h = image.height as usize;
    let c = image.channels as usize;

    let filter_type = match filter.to_lowercase().as_str() {
        "nearest" => Filter::Nearest,
        "bilinear" => Filter::Bilinear,
        "lanczos" => Filter::Lanczos3,
        "mitchell" | "bicubic" => Filter::Bicubic,
        _ => Filter::Lanczos3,
    };

    // Generate mipmaps until 1x1
    while w > 1 || h > 1 {
        let new_w = (w / 2).max(1);
        let new_h = (h / 2).max(1);

        let resized = resize_f32(&data, w, h, c, new_w, new_h, filter_type)?;

        let mip = ImageData::from_f32(new_w as u32, new_h as u32, image.channels, resized.clone());
        mipmaps.push(mip);

        data = resized;
        w = new_w;
        h = new_h;
    }

    Ok(mipmaps)
}
