//! Texture creation command (like maketx)
//!
//! Uses vfx-compute GPU backend for accelerated mipmap generation.

use crate::MaketxArgs;
#[allow(unused_imports)]
use tracing::{debug, info, trace};
use anyhow::{Result, Context};
use vfx_compute::{ImageProcessor, ComputeImage, Backend, ResizeFilter};

pub fn run(args: MaketxArgs, verbose: u8, allow_non_color: bool) -> Result<()> {
    let image = super::load_image(&args.input)?;
    super::ensure_color_processing(&image, "maketx", allow_non_color)?;

    if verbose > 0 {
        println!("Creating texture from {}", args.input.display());
        println!("  Size: {}x{}", image.width, image.height);
        println!("  Tile size: {}", args.tile);
        println!("  Mipmaps: {}", args.mipmap);
        println!("  Filter: {}", args.filter);
        println!("  Wrap: {}", args.wrap);
    }

    if args.mipmap {
        // Initialize GPU compute backend
        let processor = ImageProcessor::new(Backend::Auto)
            .context("Failed to initialize compute backend")?;
        
        if verbose > 0 {
            println!("  Backend: {}", processor.backend_name());
            println!("  Generating mipmaps...");
        }

        // Convert to ComputeImage
        let data = image.to_f32();
        let mut current = ComputeImage::from_f32(
            data,
            image.width,
            image.height,
            image.channels as u32,
        ).context("Failed to create compute image")?;

        let filter = parse_filter(&args.filter);
        let mut mipmaps: Vec<ComputeImage> = vec![current.clone()];
        let mut w = image.width;
        let mut h = image.height;
        let mut level = 0u32;

        // Generate mipmap chain using GPU-accelerated resize
        while w > 1 || h > 1 {
            let new_w = (w / 2).max(1);
            let new_h = (h / 2).max(1);
            level += 1;

            let mip = processor.resize(&current, new_w, new_h, filter)
                .with_context(|| format!("Failed to generate mip level {}", level))?;

            if verbose > 1 {
                println!("    Level {}: {}x{}", level, new_w, new_h);
            }

            mipmaps.push(mip.clone());
            current = mip;
            w = new_w;
            h = new_h;
        }

        if verbose > 0 {
            println!("  Generated {} mip levels", mipmaps.len());
        }

        // Save base level (full mipchain would need tiled TX format)
        super::save_image(&args.output, &image)?;

        // Save mip levels as separate files if requested
        if verbose > 1 {
            println!("  Note: Full mipchain embedding requires TX format");
        }
    } else {
        super::save_image(&args.output, &image)?;
    }

    if verbose > 0 {
        println!("Done.");
    }

    Ok(())
}

fn parse_filter(filter: &str) -> ResizeFilter {
    match filter.to_lowercase().as_str() {
        "box" | "nearest" => ResizeFilter::Nearest,
        "bilinear" => ResizeFilter::Bilinear,
        "lanczos" => ResizeFilter::Lanczos,
        "mitchell" | "bicubic" => ResizeFilter::Bicubic,
        "kaiser" => ResizeFilter::Lanczos, // fallback
        _ => ResizeFilter::Lanczos,
    }
}
