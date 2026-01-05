//! LUT application command

use crate::LutArgs;
#[allow(unused_imports)]
use tracing::{debug, info, trace};
use anyhow::{Result, bail};
use vfx_io::ImageData;
use vfx_lut::cube;

pub fn run(args: LutArgs, verbose: u8, allow_non_color: bool) -> Result<()> {
    let image = super::load_image(&args.input)?;
    super::ensure_color_processing(&image, "lut", allow_non_color)?;

    if verbose > 0 {
        println!("Applying LUT {} to {}", args.lut.display(), args.input.display());
    }

    // Detect LUT type and load
    let ext = args.lut.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let result = match ext.as_str() {
        "cube" => apply_cube_lut(&image, &args.lut, args.invert)?,
        _ => bail!("Unsupported LUT format: .{}", ext),
    };

    super::save_image(&args.output, &result)?;

    if verbose > 0 {
        println!("Done.");
    }

    Ok(())
}

fn apply_cube_lut(image: &ImageData, lut_path: &std::path::Path, invert: bool) -> Result<ImageData> {
    let lut3d = cube::read_3d(lut_path).ok();
    let lut1d = if lut3d.is_none() {
        Some(cube::read_1d(lut_path)?)
    } else {
        None
    };

    // Invert LUTs if requested
    let lut3d = if invert {
        lut3d.map(|lut| lut.invert().expect("Failed to invert 3D LUT"))
    } else {
        lut3d
    };
    
    let lut1d = if invert {
        lut1d.map(|lut| lut.invert().expect("Failed to invert 1D LUT"))
    } else {
        lut1d
    };

    let mut data = image.to_f32();
    let w = image.width as usize;
    let h = image.height as usize;
    let c = image.channels as usize;

    // Apply LUT to each pixel
    let channels = c.min(3);
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) * c;
            let rgb = [
                data[idx],
                if channels > 1 { data[idx + 1] } else { data[idx] },
                if channels > 2 { data[idx + 2] } else { data[idx] },
            ];

            let result = if let Some(lut) = &lut3d {
                lut.apply(rgb)
            } else {
                lut1d.as_ref().unwrap().apply_rgb(rgb)
            };

            data[idx] = result[0];
            if channels > 1 { data[idx + 1] = result[1]; }
            if channels > 2 { data[idx + 2] = result[2]; }
        }
    }

    Ok(ImageData::from_f32(image.width, image.height, image.channels, data))
}
