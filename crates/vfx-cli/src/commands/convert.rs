//! Format conversion command (like iconvert).
//!
//! Converts between image formats with optional bit depth and compression settings.
//! For EXR-to-EXR conversions, preserves all layers and channels.

use crate::ConvertArgs;
use anyhow::{Context, Result};
use tracing::{debug, info, trace};
use vfx_io::exr::{Compression, ExrReader, ExrWriter, ExrWriterOptions};
use vfx_io::{Format, FormatWriter, PixelFormat};

/// Runs the convert command.
///
/// When both input and output are EXR, uses layered I/O to preserve all layers.
/// Otherwise falls back to single-layer ImageData conversion.
pub fn run(args: ConvertArgs, verbose: u8) -> Result<()> {
    trace!(input = %args.input.display(), output = %args.output.display(), "convert::run");
    
    let input_format = Format::detect(&args.input).unwrap_or(Format::Unknown);
    let output_format = Format::detect(&args.output).unwrap_or(Format::Unknown);

    info!(
        input = %args.input.display(),
        input_format = ?input_format,
        output = %args.output.display(),
        output_format = ?output_format,
        "Converting image"
    );
    
    if verbose > 0 {
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
    let mut image = super::load_image(&args.input)?;

    // Apply bit depth conversion if requested
    if let Some(ref depth) = args.depth {
        let target_format = parse_depth(depth)?;
        debug!(from = ?image.format, to = ?target_format, "Converting bit depth");
        if verbose > 0 {
            println!("  Converting depth: {:?} -> {:?}", image.format, target_format);
        }
        image = image.convert_to(target_format);
    }

    super::save_image(&args.output, &image)?;

    if verbose > 0 {
        println!("Done.");
    }

    Ok(())
}

/// Converts EXR to EXR preserving all layers and channels.
fn convert_exr_layered(args: &ConvertArgs, verbose: u8) -> Result<()> {
    trace!(input = %args.input.display(), "convert_exr_layered");
    
    let reader = ExrReader::default();
    let layered = reader
        .read_layers(&args.input)
        .with_context(|| format!("Failed to read layers from {}", args.input.display()))?;

    debug!(layers = layered.layers.len(), "Read EXR layers");
    
    if verbose > 0 {
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

    if verbose > 0 {
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

/// Parses bit depth string into PixelFormat.
fn parse_depth(s: &str) -> Result<PixelFormat> {
    match s.to_lowercase().as_str() {
        "8" | "u8" | "uint8" => Ok(PixelFormat::U8),
        "16" | "u16" | "uint16" => Ok(PixelFormat::U16),
        "32" | "f32" | "float" | "float32" => Ok(PixelFormat::F32),
        "half" | "f16" | "float16" => Ok(PixelFormat::F16),
        _ => anyhow::bail!(
            "Unknown bit depth '{}'. Options: 8, 16, 32, half (or u8, u16, f32, f16)",
            s
        ),
    }
}
