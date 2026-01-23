//! Blur command
//!
//! Applies box or gaussian blur to images.
//! Supports `--layer` for processing specific layers in multi-layer EXR.
//! Preserves alpha channel (only RGB is blurred).

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
    
    // Preserve alpha - only blur RGB channels
    let has_alpha = c == 4 || c == 2; // RGBA or grayscale+alpha
    let blur_channels = if has_alpha { c - 1 } else { c };
    
    // Extract alpha if present
    let alpha: Option<Vec<f32>> = if has_alpha {
        let mut a = Vec::with_capacity(w * h);
        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) * c + (c - 1);
                a.push(src_data[idx]);
            }
        }
        Some(a)
    } else {
        None
    };
    
    // Prepare RGB-only data for blur
    let rgb_data: Vec<f32> = if has_alpha {
        let mut rgb = Vec::with_capacity(w * h * blur_channels);
        for y in 0..h {
            for x in 0..w {
                let base = (y * w + x) * c;
                for ch in 0..blur_channels {
                    rgb.push(src_data[base + ch]);
                }
            }
        }
        rgb
    } else {
        src_data.clone()
    };

    // Apply blur to RGB only
    let blurred_rgb = match args.blur_type.to_lowercase().as_str() {
        "box" => box_blur(&rgb_data, w, h, blur_channels, args.radius)?,
        "gaussian" | "gauss" => {
            let kernel = Kernel::gaussian(args.radius * 2 + 1, args.radius as f32 / 2.0);
            convolve(&rgb_data, w, h, blur_channels, &kernel)?
        }
        _ => box_blur(&rgb_data, w, h, blur_channels, args.radius)?,
    };

    // Recombine with preserved alpha
    let blurred = if let Some(alpha) = alpha {
        let mut result = Vec::with_capacity(w * h * c);
        for y in 0..h {
            for x in 0..w {
                let rgb_base = (y * w + x) * blur_channels;
                for ch in 0..blur_channels {
                    result.push(blurred_rgb[rgb_base + ch]);
                }
                // Append preserved alpha
                result.push(alpha[y * w + x]);
            }
        }
        result
    } else {
        blurred_rgb
    };

    let output = ImageData::from_f32(image.width, image.height, image.channels, blurred);

    super::save_image_layer(&args.output, &output, args.layer.as_deref())?;

    if verbose > 0 {
        println!("Done.");
    }

    Ok(())
}
