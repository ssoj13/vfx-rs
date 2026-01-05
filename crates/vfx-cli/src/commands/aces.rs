//! ACES color transform command.
//!
//! Applies ACES IDT, RRT, and ODT transforms.

use tracing::{debug, info, trace};
use anyhow::{Result, bail};
use crate::AcesArgs;
use super::{load_image, save_image};
use vfx_color::aces::{
    RrtParams, apply_rrt_odt_srgb, apply_inverse_odt_srgb,
    rrt, acescg_to_srgb,
};
use vfx_color::prelude::srgb;

pub fn run(args: AcesArgs, verbose: u8) -> Result<()> {
    trace!(input = %args.input.display(), transform = %args.transform, "aces::run");
    info!(transform = %args.transform, rrt = %args.rrt_variant, "Applying ACES transform");
    
    if verbose > 0 {
        println!("Loading: {}", args.input.display());
    }

    let input = load_image(&args.input)?;
    let channels = input.channels as usize;

    if channels < 3 {
        bail!("ACES transforms require at least 3 channels, got {}", channels);
    }

    let data = input.to_f32();
    
    let result_data = match args.transform.to_lowercase().as_str() {
        // IDT: sRGB gamma -> ACEScg linear
        "idt" | "input" | "srgb-to-acescg" => {
            if verbose > 0 {
                println!("Applying IDT: sRGB -> ACEScg");
            }
            apply_inverse_odt_srgb(&data, channels)
        }
        
        // RRT only: tonemap in ACEScg
        "rrt" | "tonemap" => {
            if verbose > 0 {
                println!("Applying RRT tonemap");
            }
            let params = get_rrt_params(&args.rrt_variant);
            apply_rrt_only(&data, channels, &params)
        }
        
        // ODT only: ACEScg -> sRGB (no tonemap)
        "odt" | "output" | "acescg-to-srgb" => {
            if verbose > 0 {
                println!("Applying ODT: ACEScg -> sRGB");
            }
            apply_odt_only(&data, channels)
        }
        
        // Full RRT+ODT: ACEScg -> display sRGB
        "rrt-odt" | "rrtodt" | "display" | "full" => {
            if verbose > 0 {
                println!("Applying RRT+ODT: ACEScg -> sRGB display");
            }
            apply_rrt_odt_srgb(&data, channels)
        }
        
        other => {
            bail!("Unknown ACES transform: '{}'. Use: idt, rrt, odt, rrt-odt", other);
        }
    };

    let result = vfx_io::ImageData::from_f32(input.width, input.height, channels as u32, result_data);

    if verbose > 0 {
        println!("Saving: {}", args.output.display());
    }

    save_image(&args.output, &result)?;

    println!(
        "ACES {} applied: {} -> {}",
        args.transform,
        args.input.display(),
        args.output.display()
    );

    Ok(())
}

fn get_rrt_params(variant: &str) -> RrtParams {
    match variant.to_lowercase().as_str() {
        "high-contrast" | "highcontrast" | "high" => RrtParams::aces_high_contrast(),
        _ => RrtParams::default(),
    }
}

/// Apply RRT tonemap only (no colorspace conversion).
fn apply_rrt_only(data: &[f32], channels: usize, params: &RrtParams) -> Vec<f32> {
    let mut result = data.to_vec();
    let pixels = data.len() / channels;

    for i in 0..pixels {
        let idx = i * channels;
        let (r, g, b) = rrt(data[idx], data[idx + 1], data[idx + 2], params);
        result[idx] = r;
        result[idx + 1] = g;
        result[idx + 2] = b;
    }

    result
}

/// Apply ODT only: ACEScg -> sRGB with gamma (no tonemap).
fn apply_odt_only(data: &[f32], channels: usize) -> Vec<f32> {
    let mut result = data.to_vec();
    let pixels = data.len() / channels;

    for i in 0..pixels {
        let idx = i * channels;
        let (r, g, b) = acescg_to_srgb(data[idx], data[idx + 1], data[idx + 2]);
        
        // Apply sRGB gamma
        result[idx] = srgb::oetf(r.clamp(0.0, 1.0));
        result[idx + 1] = srgb::oetf(g.clamp(0.0, 1.0));
        result[idx + 2] = srgb::oetf(b.clamp(0.0, 1.0));
    }

    result
}
