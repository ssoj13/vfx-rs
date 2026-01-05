//! Warp command - apply lens distortion and artistic effects

#[allow(unused_imports)]
use tracing::{debug, info, trace};
use anyhow::{Result, bail};
use crate::{WarpArgs, commands::{load_image, save_image}};
use vfx_io::ImageData;
use vfx_ops::warp;

pub fn run(args: WarpArgs, verbose: u8) -> Result<()> {
    if verbose > 0 {
        println!("Loading: {}", args.input.display());
    }
    
    let input = load_image(&args.input)?;
    
    if verbose > 0 {
        println!("Size: {}x{} ({} ch)", input.width, input.height, input.channels);
        println!("Warp: {} (k1={}, k2={}, radius={})", 
                 args.warp_type, args.k1, args.k2, args.radius);
    }
    
    let result = apply_warp(&input, &args)?;
    
    save_image(&args.output, &result)?;
    
    if verbose > 0 {
        println!("Saved: {}", args.output.display());
    }
    
    Ok(())
}

fn apply_warp(input: &ImageData, args: &WarpArgs) -> Result<ImageData> {
    let data = input.to_f32();
    let w = input.width as usize;
    let h = input.height as usize;
    let ch = input.channels as usize;
    
    let result_data = match args.warp_type.to_lowercase().as_str() {
        "barrel" => warp::barrel(&data, w, h, ch, args.k1, args.k2),
        "pincushion" => warp::pincushion(&data, w, h, ch, args.k1, args.k2),
        "fisheye" => warp::fisheye(&data, w, h, ch, args.k1),
        "twist" | "swirl" => warp::twist(&data, w, h, ch, args.k1, args.radius),
        "wave" | "sine" => warp::wave(&data, w, h, ch, args.k1, args.k2.max(1.0)),
        "spherize" | "bulge" => warp::spherize(&data, w, h, ch, args.k1, args.radius),
        "ripple" => warp::ripple(&data, w, h, ch, args.k1, args.k2.max(1.0), args.radius),
        _ => bail!(
            "Unknown warp type: '{}'. Valid: barrel, pincushion, fisheye, twist, wave, spherize, ripple",
            args.warp_type
        ),
    };
    
    Ok(ImageData::from_f32(input.width, input.height, input.channels, result_data))
}
