//! Batch processing command

use crate::BatchArgs;
#[allow(unused_imports)]
use tracing::{debug, info, trace};
use anyhow::{Result, bail};
use std::path::PathBuf;
use rayon::prelude::*;
use vfx_io::ImageData;

pub fn run(args: BatchArgs, verbose: u8, allow_non_color: bool) -> Result<()> {
    trace!(pattern = %args.input, op = %args.op, "batch::run");
    
    // Find matching files
    let files: Vec<PathBuf> = glob::glob(&args.input)?
        .filter_map(|r| r.ok())
        .collect();

    if files.is_empty() {
        bail!("No files match pattern: {}", args.input);
    }

    info!(files = files.len(), pattern = %args.input, op = %args.op, "Starting batch processing");
    
    if verbose > 0 {
        println!("Found {} files matching '{}'", files.len(), args.input);
    }

    // Create output directory
    std::fs::create_dir_all(&args.output_dir)?;

    // Parse operation args
    let op_args: std::collections::HashMap<String, String> = args.args
        .iter()
        .filter_map(|s| {
            let parts: Vec<&str> = s.splitn(2, '=').collect();
            if parts.len() == 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect();

    // Process files in parallel
    let results: Vec<Result<()>> = files.par_iter().map(|input| {
        process_file(
            input,
            &args.output_dir,
            &args.op,
            &op_args,
            args.format.as_deref(),
            allow_non_color,
            verbose,
        )
    }).collect();

    // Report results
    let mut success = 0;
    let mut failed = 0;
    for r in results {
        match r {
            Ok(_) => success += 1,
            Err(e) => {
                failed += 1;
                eprintln!("Error: {}", e);
            }
        }
    }

    info!(success = success, failed = failed, "Batch processing complete");
    println!("Processed: {} success, {} failed", success, failed);

    if failed > 0 {
        bail!("{} files failed", failed);
    }

    Ok(())
}

fn process_file(
    input: &PathBuf,
    output_dir: &PathBuf,
    op: &str,
    args: &std::collections::HashMap<String, String>,
    format: Option<&str>,
    allow_non_color: bool,
    verbose: u8,
) -> Result<()> {
    // Determine output path
    let stem = input.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let ext = format.unwrap_or_else(|| {
        input.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("exr")
    });

    let output = output_dir.join(format!("{}.{}", stem, ext));

    if verbose > 0 {
        println!("Processing {} -> {}", input.display(), output.display());
    }

    let image = super::load_image(input)?;
    if !op.eq_ignore_ascii_case("convert") {
        super::ensure_color_processing(&image, op, allow_non_color)?;
    }
    let data = image.to_f32();
    let w = image.width as usize;
    let h = image.height as usize;
    let c = image.channels as usize;

    let result = match op.to_lowercase().as_str() {
        "convert" => {
            // Just convert format
            image
        }
        "resize" => {
            let scale: f32 = args.get("scale")
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.0);

            let new_w = (w as f32 * scale) as usize;
            let new_h = (h as f32 * scale) as usize;

            let filter = vfx_ops::resize::Filter::Lanczos3;
            let resized = vfx_ops::resize::resize_f32(&data, w, h, c, new_w, new_h, filter)?;

            ImageData::from_f32(new_w as u32, new_h as u32, image.channels, resized)
        }
        "blur" => {
            let radius: usize = args.get("radius")
                .and_then(|s| s.parse().ok())
                .unwrap_or(3);

            let blurred = vfx_ops::filter::box_blur(&data, w, h, c, radius)?;

            ImageData::from_f32(image.width, image.height, image.channels, blurred)
        }
        "flip_h" => {
            let flipped = vfx_ops::transform::flip_h(&data, w, h, c);
            ImageData::from_f32(image.width, image.height, image.channels, flipped)
        }
        "flip_v" => {
            let flipped = vfx_ops::transform::flip_v(&data, w, h, c);
            ImageData::from_f32(image.width, image.height, image.channels, flipped)
        }
        _ => bail!("Unknown operation: {}", op),
    };

    super::save_image(&output, &result)?;

    Ok(())
}
