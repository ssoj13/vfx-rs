//! Rotate command - arbitrary angle rotation

use anyhow::Result;
use crate::{RotateArgs, commands::{load_image, save_image}};
use vfx_io::ImageData;
use vfx_ops::transform::rotate;

pub fn run(args: RotateArgs, verbose: u8) -> Result<()> {
    if verbose > 0 {
        println!("Loading: {}", args.input.display());
    }
    
    let input = load_image(&args.input)?;
    
    if verbose > 0 {
        println!("Size: {}x{} ({} ch)", input.width, input.height, input.channels);
        println!("Rotation: {}°", args.angle);
    }
    
    let bg_color = parse_color(&args.bg_color, input.channels as usize)?;
    
    let result = rotate_image(&input, args.angle, &bg_color)?;
    
    if verbose > 0 {
        println!("New size: {}x{}", result.width, result.height);
    }
    
    save_image(&args.output, &result)?;
    
    if verbose > 0 {
        println!("Saved: {}", args.output.display());
    }
    
    Ok(())
}

fn rotate_image(input: &ImageData, angle: f32, bg_color: &[f32]) -> Result<ImageData> {
    let data = input.to_f32();
    let channels = input.channels as usize;
    
    let (result_data, new_w, new_h) = rotate(
        &data,
        input.width as usize,
        input.height as usize,
        channels,
        angle,
        bg_color,
    );
    
    Ok(ImageData::from_f32(new_w as u32, new_h as u32, channels as u32, result_data))
}

/// Parse color string like "0,0,0" or "0.5,0.5,0.5,1.0"
fn parse_color(s: &str, channels: usize) -> Result<Vec<f32>> {
    let parts: Result<Vec<f32>, _> = s.split(',')
        .map(|p| p.trim().parse::<f32>())
        .collect();
    
    let mut color = parts.map_err(|_| anyhow::anyhow!("Invalid color format: {}", s))?;
    
    // Extend to match channel count
    while color.len() < channels {
        if color.len() == 3 && channels == 4 {
            color.push(1.0); // Alpha
        } else {
            color.push(0.0);
        }
    }
    
    // Truncate if too many
    color.truncate(channels);
    
    Ok(color)
}
