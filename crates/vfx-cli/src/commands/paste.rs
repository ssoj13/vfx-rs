//! Paste command - overlay one image onto another

use tracing::{debug, info, trace};
use anyhow::Result;
use crate::{PasteArgs, commands::{load_image, save_image}};
use vfx_io::ImageData;
use vfx_ops::transform::paste;

pub fn run(args: PasteArgs, verbose: u8) -> Result<()> {
    if verbose > 0 {
        println!("Loading background: {}", args.background.display());
        println!("Loading foreground: {}", args.foreground.display());
    }
    
    let bg = load_image(&args.background)?;
    let fg = load_image(&args.foreground)?;
    
    if verbose > 0 {
        println!("Background: {}x{} ({} ch)", bg.width, bg.height, bg.channels);
        println!("Foreground: {}x{} ({} ch)", fg.width, fg.height, fg.channels);
        println!("Offset: ({}, {}), blend: {}", args.x, args.y, args.blend);
    }
    
    let result = paste_images(&bg, &fg, args.x, args.y, args.blend)?;
    
    save_image(&args.output, &result)?;
    
    if verbose > 0 {
        println!("Saved: {}", args.output.display());
    }
    
    Ok(())
}

fn paste_images(bg: &ImageData, fg: &ImageData, x: i32, y: i32, blend: bool) -> Result<ImageData> {
    let bg_data = bg.to_f32();
    let fg_data = fg.to_f32();
    
    // Determine channels to use (minimum of both, but use bg channels for output)
    let channels = bg.channels as usize;
    let fg_channels = fg.channels as usize;
    
    // If channel counts differ, expand/truncate fg to match bg
    let fg_adjusted = if fg_channels != channels {
        adjust_channels(&fg_data, fg_channels, channels, fg.width as usize * fg.height as usize)
    } else {
        fg_data
    };
    
    let result_data = paste(
        &bg_data,
        bg.width as usize,
        bg.height as usize,
        &fg_adjusted,
        fg.width as usize,
        fg.height as usize,
        channels,
        x,
        y,
        blend,
    );
    
    Ok(ImageData::from_f32(bg.width, bg.height, bg.channels, result_data))
}

/// Adjust channel count by expanding or truncating
fn adjust_channels(data: &[f32], src_ch: usize, dst_ch: usize, pixels: usize) -> Vec<f32> {
    let mut result = vec![0.0f32; pixels * dst_ch];
    
    for i in 0..pixels {
        for c in 0..dst_ch {
            if c < src_ch {
                result[i * dst_ch + c] = data[i * src_ch + c];
            } else if c == 3 {
                // Alpha defaults to 1.0
                result[i * dst_ch + c] = 1.0;
            }
        }
    }
    
    result
}
