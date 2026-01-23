//! Texture creation command (like maketx)
//!
//! Uses vfx-compute GPU backend for accelerated mipmap generation.
//! Writes mipmapped tiled EXR using vfx-exr.

use crate::MaketxArgs;
#[allow(unused_imports)]
use tracing::{debug, info, trace};
use anyhow::{Result, Context};
use vfx_compute::{ImageProcessor, ComputeImage, Backend, ResizeFilter};
use vfx_exr::prelude::*;
use vfx_exr::math::RoundingMode;
use smallvec::smallvec;

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
            data.clone(),
            image.width as u32,
            image.height as u32,
            image.channels as u32,
        ).context("Failed to create compute image")?;

        let filter = parse_filter(&args.filter);
        
        // Store mipmap data: Vec of (width, height, flat_data)
        let mut mip_data: Vec<(usize, usize, Vec<f32>)> = vec![
            (image.width as usize, image.height as usize, data)
        ];
        
        let mut w = image.width as u32;
        let mut h = image.height as u32;
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

            // Extract data from ComputeImage
            let mip_pixels = mip.data().to_vec();
            mip_data.push((new_w as usize, new_h as usize, mip_pixels));
            
            current = mip;
            w = new_w;
            h = new_h;
        }

        if verbose > 0 {
            println!("  Generated {} mip levels", mip_data.len());
        }

        // Check output format
        let output_ext = args.output.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        if output_ext == "exr" {
            // Write mipmapped EXR using vfx-exr
            write_mipmapped_exr(&args, &mip_data, image.channels as usize, verbose)?;
        } else {
            // Non-EXR: save base level only
            if verbose > 0 {
                println!("  Note: Mipmap embedding only supported for EXR format");
            }
            super::save_image(&args.output, &image)?;
        }
    } else {
        super::save_image(&args.output, &image)?;
    }

    if verbose > 0 {
        println!("Done.");
    }

    Ok(())
}

/// Write mipmapped tiled EXR file
fn write_mipmapped_exr(
    args: &MaketxArgs,
    mip_data: &[(usize, usize, Vec<f32>)],
    channels: usize,
    verbose: u8,
) -> Result<()> {
    let (full_w, full_h, _) = mip_data[0];
    let full_size = Vec2(full_w, full_h);
    let rounding = RoundingMode::Down;
    
    // Build channel data with mipmaps
    let channel_names = match channels {
        1 => vec!["Y"],
        2 => vec!["Y", "A"],
        3 => vec!["R", "G", "B"],
        4 => vec!["R", "G", "B", "A"],
        _ => (0..channels).map(|i| match i {
            0 => "R", 1 => "G", 2 => "B", 3 => "A",
            _ => "X"
        }).collect(),
    };
    
    let mut any_channels = smallvec![];
    
    for (ch_idx, ch_name) in channel_names.iter().enumerate() {
        // Extract this channel from each mip level
        let level_data: Vec<FlatSamples> = mip_data.iter().map(|(w, h, pixels)| {
            let mut ch_data = Vec::with_capacity(w * h);
            for y in 0..*h {
                for x in 0..*w {
                    let idx = (y * w + x) * channels + ch_idx;
                    ch_data.push(pixels.get(idx).copied().unwrap_or(0.0));
                }
            }
            FlatSamples::F32(ch_data)
        }).collect();
        
        any_channels.push(AnyChannel::new(
            *ch_name,
            Levels::Mip {
                level_data,
                rounding_mode: rounding,
            }
        ));
    }
    
    // Tiled encoding - level_mode is inferred from Levels::Mip
    let tile_size = args.tile as usize;
    let encoding = Encoding {
        compression: Compression::ZIP16,
        blocks: Blocks::Tiles(Vec2(tile_size, tile_size)),
        line_order: LineOrder::Increasing,
    };
    
    let layer = Layer::new(
        full_size,
        LayerAttributes::named("rgba"),
        encoding,
        AnyChannels::sort(any_channels),
    );
    
    let image_attrs = ImageAttributes::new(IntegerBounds::from_dimensions(full_size));
    let exr_image = Image::empty(image_attrs).with_layer(layer);
    
    if verbose > 0 {
        println!("  Writing mipmapped tiled EXR...");
    }
    
    exr_image.write()
        .to_file(&args.output)
        .context("Failed to write mipmapped EXR")?;
    
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
