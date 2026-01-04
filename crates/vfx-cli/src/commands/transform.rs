//! Transform command (flip, rotate, transpose)

use crate::TransformArgs;
use anyhow::{Result, bail};
use vfx_io::ImageData;
use vfx_ops::transform::{flip_h, flip_v, rotate_90_cw};

pub fn run(args: TransformArgs, verbose: bool) -> Result<()> {
    let image = super::load_image(&args.input)?;
    super::ensure_color_processing(&image, "transform")?;
    let mut data = image.to_f32();
    let mut width = image.width as usize;
    let mut height = image.height as usize;
    let channels = image.channels as usize;

    if verbose {
        println!("Transforming {}", args.input.display());
    }

    // Apply transformations in order
    if args.flip_h {
        if verbose { println!("  Flip horizontal"); }
        data = flip_h(&data, width, height, channels);
    }

    if args.flip_v {
        if verbose { println!("  Flip vertical"); }
        data = flip_v(&data, width, height, channels);
    }

    if let Some(angle) = args.rotate {
        if verbose { println!("  Rotate {}deg", angle); }
        match angle {
            90 => {
                let (new_data, new_w, new_h) = rotate_90_cw(&data, width, height, channels);
                data = new_data;
                width = new_w;
                height = new_h;
            }
            180 => {
                data = flip_h(&data, width, height, channels);
                data = flip_v(&data, width, height, channels);
            }
            270 | -90 => {
                // 270 = 3x 90
                for _ in 0..3 {
                    let (new_data, new_w, new_h) = rotate_90_cw(&data, width, height, channels);
                    data = new_data;
                    width = new_w;
                    height = new_h;
                }
            }
            _ => bail!("Unsupported rotation angle: {}. Use 90, 180, or 270.", angle),
        }
    }

    if args.transpose {
        if verbose { println!("  Transpose"); }
        data = transpose(&data, width, height, channels);
        std::mem::swap(&mut width, &mut height);
    }

    let output = ImageData::from_f32(width as u32, height as u32, image.channels, data);

    super::save_image(&args.output, &output)?;

    if verbose {
        println!("Done.");
    }

    Ok(())
}

fn transpose(data: &[f32], width: usize, height: usize, channels: usize) -> Vec<f32> {
    let mut result = vec![0.0f32; data.len()];

    for y in 0..height {
        for x in 0..width {
            let src = (y * width + x) * channels;
            let dst = (x * height + y) * channels;
            for c in 0..channels {
                result[dst + c] = data[src + c];
            }
        }
    }

    result
}
