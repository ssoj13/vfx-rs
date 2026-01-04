//! Format conversion command (like iconvert).
//!
//! Converts between image formats with optional bit depth and compression settings.
//! For EXR-to-EXR conversions, preserves all layers and channels.

use crate::ConvertArgs;
use anyhow::{Context, Result};
use vfx_io::exr::{Compression, ExrReader, ExrWriter, ExrWriterOptions};
use vfx_io::{Format, FormatWriter};

/// Runs the convert command.
///
/// When both input and output are EXR, uses layered I/O to preserve all layers.
/// Otherwise falls back to single-layer ImageData conversion.
pub fn run(args: ConvertArgs, verbose: bool) -> Result<()> {
    let input_format = Format::detect(&args.input).unwrap_or(Format::Unknown);
    let output_format = Format::detect(&args.output).unwrap_or(Format::Unknown);

    if verbose {
        println!(
            "Converting {} ({:?}) -> {} ({:?})",
            args.input.display(),
            input_format,
            args.output.display(),
            output_format
        );
    }

    // EXR-to-EXR: preserve layers
    if input_format == Format::Exr && output_format == Format::Exr {
        return convert_exr_layered(&args, verbose);
    }

    // Standard single-layer conversion
    let image = super::load_image(&args.input)?;

    if let Some(ref depth) = args.depth {
        if verbose {
            println!("  Target depth: {}", depth);
        }
        // TODO: Apply bit depth conversion
    }

    super::save_image(&args.output, &image)?;

    if verbose {
        println!("Done.");
    }

    Ok(())
}

/// Converts EXR to EXR preserving all layers and channels.
fn convert_exr_layered(args: &ConvertArgs, verbose: bool) -> Result<()> {
    let reader = ExrReader::default();
    let layered = reader
        .read_layers(&args.input)
        .with_context(|| format!("Failed to read layers from {}", args.input.display()))?;

    if verbose {
        println!("  Layers: {}", layered.layers.len());
        for layer in &layered.layers {
            println!(
                "    {} ({}x{}, {} ch)",
                layer.name,
                layer.width,
                layer.height,
                layer.channels.len()
            );
        }
    }

    // Parse compression option
    let compression = args
        .compression
        .as_ref()
        .map(|c| parse_compression(c))
        .transpose()?
        .unwrap_or(Compression::Zip);

    let options = ExrWriterOptions {
        compression,
        layer_name: None, // Not used for layered writes
        use_half: args.depth.as_ref().map(|d| d == "half").unwrap_or(false),
    };

    let writer = ExrWriter::with_options(options);
    writer
        .write_layers(&args.output, &layered)
        .with_context(|| format!("Failed to write {}", args.output.display()))?;

    if verbose {
        println!("Done.");
    }

    Ok(())
}

/// Parses compression string into Compression enum.
fn parse_compression(s: &str) -> Result<Compression> {
    match s.to_lowercase().as_str() {
        "none" => Ok(Compression::None),
        "rle" => Ok(Compression::Rle),
        "zip" | "zips" => Ok(Compression::Zip),
        "piz" => Ok(Compression::Piz),
        "dwaa" => Ok(Compression::Dwaa),
        "dwab" => Ok(Compression::Dwab),
        _ => anyhow::bail!(
            "Unknown EXR compression '{}'. Options: none, rle, zip, piz, dwaa, dwab",
            s
        ),
    }
}
