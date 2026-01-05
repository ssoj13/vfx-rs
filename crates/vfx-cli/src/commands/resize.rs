//! Resize command
//!
//! Scales images using various resampling filters.
//! Supports `--layer` for processing specific layers in multi-layer EXR.

use crate::ResizeArgs;
use tracing::{debug, info, trace};
use anyhow::{Result, bail};
use vfx_io::ImageData;
use vfx_ops::resize::{resize_f32, Filter};

pub fn run(args: ResizeArgs, verbose: u8, allow_non_color: bool) -> Result<()> {
    let image = super::load_image_layer(&args.input, args.layer.as_deref())?;
    super::ensure_color_processing(&image, "resize", allow_non_color)?;
    let src_w = image.width as usize;
    let src_h = image.height as usize;

    // Determine target dimensions
    let (dst_w, dst_h) = match (args.width, args.height, args.scale) {
        (Some(w), Some(h), _) => (w, h),
        (Some(w), None, _) => {
            let h = (src_h as f32 * w as f32 / src_w as f32).round() as usize;
            (w, h)
        }
        (None, Some(h), _) => {
            let w = (src_w as f32 * h as f32 / src_h as f32).round() as usize;
            (w, h)
        }
        (None, None, Some(s)) => {
            let w = (src_w as f32 * s).round() as usize;
            let h = (src_h as f32 * s).round() as usize;
            (w, h)
        }
        _ => bail!("Specify --width, --height, or --scale"),
    };

    if verbose > 0 {
        println!("Resizing {}x{} -> {}x{}", src_w, src_h, dst_w, dst_h);
    }

    let filter = match args.filter.to_lowercase().as_str() {
        "nearest" => Filter::Nearest,
        "bilinear" | "linear" => Filter::Bilinear,
        "bicubic" | "cubic" => Filter::Bicubic,
        "lanczos" | "lanczos3" => Filter::Lanczos3,
        _ => Filter::Lanczos3,
    };

    let src_data = image.to_f32();
    let channels = image.channels as usize;

    let resized = resize_f32(&src_data, src_w, src_h, channels, dst_w, dst_h, filter)?;

    let output = ImageData::from_f32(dst_w as u32, dst_h as u32, image.channels, resized);

    super::save_image_layer(&args.output, &output, args.layer.as_deref())?;

    if verbose > 0 {
        println!("Done.");
    }

    Ok(())
}
