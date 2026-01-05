//! Layer inspection and extraction commands for multi-layer EXR files.
//!
//! This module provides CLI commands to:
//! - List all layers and their channels in a file
//! - Extract a single layer to a new file
//! - Merge layers from multiple files into one
//!
//! These operations are essential for compositing workflows where EXR files
//! contain multiple render passes (beauty, specular, diffuse, depth, etc.).

use crate::{ExtractLayerArgs, LayersArgs, MergeLayersArgs};
use anyhow::{Context, Result};
use std::path::Path;
use vfx_io::exr::{ExrReader, ExrWriter};
use vfx_io::{Format, LayeredImage};

/// Loads a layered image, supporting both EXR multi-layer and single-layer formats.
///
/// For EXR files, uses `read_layers` to get all layers.
/// For other formats, reads as ImageData and converts to single-layer LayeredImage.
fn load_layered(path: &Path) -> Result<LayeredImage> {
    let format = Format::detect(path).unwrap_or(Format::Unknown);

    match format {
        Format::Exr => {
            let reader = ExrReader::default();
            reader
                .read_layers(path)
                .with_context(|| format!("Failed to read EXR layers: {}", path.display()))
        }
        _ => {
            // Non-EXR: load as ImageData and convert to single layer
            let image = vfx_io::read(path)
                .with_context(|| format!("Failed to load: {}", path.display()))?;
            Ok(image.to_layered("default"))
        }
    }
}

/// Lists all layers and channels in the input file(s).
///
/// For each file, prints layer names, dimensions, and channel details
/// including name, type, and semantic kind (Color/Alpha/Depth/Id/etc).
pub fn run_layers(args: LayersArgs, verbose: u8) -> Result<()> {
    for path in &args.input {
        let layered = load_layered(path)?;

        if args.json {
            print_layers_json(path, &layered);
        } else {
            print_layers_text(path, &layered, verbose);
        }

        if args.input.len() > 1 {
            println!();
        }
    }

    Ok(())
}

/// Prints layer information in human-readable text format.
fn print_layers_text(path: &Path, layered: &LayeredImage, verbose: u8) {
    println!("{}", path.display());
    println!("  Layers: {}", layered.layers.len());

    for (idx, layer) in layered.layers.iter().enumerate() {
        println!();
        println!("  [{}] \"{}\"", idx, layer.name);
        println!("      Size: {}x{}", layer.width, layer.height);
        println!("      Channels: {}", layer.channels.len());

        if verbose {
            for ch in &layer.channels {
                println!(
                    "        {} ({:?}, {:?})",
                    ch.name, ch.sample_type, ch.kind
                );
            }
        } else {
            // Compact channel list
            let names: Vec<&str> = layer.channels.iter().map(|c| c.name.as_str()).collect();
            println!("        {}", names.join(", "));
        }
    }
}

/// Prints layer information in JSON format for scripting/automation.
fn print_layers_json(path: &Path, layered: &LayeredImage) {
    println!("{{");
    println!("  \"file\": \"{}\",", path.display());
    println!("  \"layers\": [");

    for (idx, layer) in layered.layers.iter().enumerate() {
        let comma = if idx + 1 < layered.layers.len() { "," } else { "" };
        println!("    {{");
        println!("      \"name\": \"{}\",", layer.name);
        println!("      \"width\": {},", layer.width);
        println!("      \"height\": {},", layer.height);
        println!("      \"channels\": [");

        for (ch_idx, ch) in layer.channels.iter().enumerate() {
            let ch_comma = if ch_idx + 1 < layer.channels.len() { "," } else { "" };
            println!(
                "        {{\"name\": \"{}\", \"type\": \"{:?}\", \"kind\": \"{:?}\"}}{}",
                ch.name, ch.sample_type, ch.kind, ch_comma
            );
        }

        println!("      ]");
        println!("    }}{}", comma);
    }

    println!("  ]");
    println!("}}");
}

/// Extracts a single layer from a multi-layer file and saves it.
///
/// The layer can be specified by name or index. If no layer is specified,
/// lists available layers and exits.
pub fn run_extract_layer(args: ExtractLayerArgs, verbose: u8) -> Result<()> {
    let layered = load_layered(&args.input)?;

    // Find the requested layer
    let layer_idx = if let Some(ref name) = args.layer {
        // Try as index first
        if let Ok(idx) = name.parse::<usize>() {
            if idx >= layered.layers.len() {
                anyhow::bail!(
                    "Layer index {} out of range (file has {} layers)",
                    idx,
                    layered.layers.len()
                );
            }
            idx
        } else {
            // Search by name
            layered
                .layers
                .iter()
                .position(|l| l.name == *name)
                .ok_or_else(|| {
                    let names: Vec<&str> = layered.layers.iter().map(|l| l.name.as_str()).collect();
                    anyhow::anyhow!(
                        "Layer '{}' not found. Available: {}",
                        name,
                        names.join(", ")
                    )
                })?
        }
    } else {
        // No layer specified - list available and exit
        println!("No layer specified. Available layers:");
        for (idx, layer) in layered.layers.iter().enumerate() {
            println!("  [{}] {}", idx, layer.name);
        }
        return Ok(());
    };

    let layer = &layered.layers[layer_idx];

    if verbose {
        println!(
            "Extracting layer '{}' ({}x{}, {} channels)",
            layer.name,
            layer.width,
            layer.height,
            layer.channels.len()
        );
    }

    // Convert layer to ImageData for output
    let image = layer
        .to_image_data()
        .with_context(|| format!("Failed to convert layer '{}' to image", layer.name))?;

    vfx_io::write(&args.output, &image)
        .with_context(|| format!("Failed to write: {}", args.output.display()))?;

    if verbose {
        println!("Saved to {}", args.output.display());
    }

    Ok(())
}

/// Merges layers from multiple input files into a single multi-layer EXR.
///
/// Each input file contributes one or more layers to the output.
/// Layer names can be customized with --names flag.
pub fn run_merge_layers(args: MergeLayersArgs, verbose: u8) -> Result<()> {
    if args.input.is_empty() {
        anyhow::bail!("No input files specified");
    }

    let mut output = LayeredImage::default();
    let custom_names: Vec<&str> = args.names.iter().map(|s| s.as_str()).collect();

    for (idx, path) in args.input.iter().enumerate() {
        let layered = load_layered(path)?;
        let is_single_layer = layered.layers.len() == 1;

        for mut layer in layered.layers {
            // Apply custom name if provided
            if is_single_layer {
                // Single-layer input: use custom name or filename stem
                if let Some(&name) = custom_names.get(idx) {
                    layer.name = name.to_string();
                } else if layer.name == "default" || layer.name.is_empty() {
                    layer.name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("layer")
                        .to_string();
                }
            }
            // For multi-layer inputs, preserve original names

            if verbose {
                println!(
                    "Adding layer '{}' from {} ({}x{}, {} ch)",
                    layer.name,
                    path.display(),
                    layer.width,
                    layer.height,
                    layer.channels.len()
                );
            }

            output.layers.push(layer);
        }
    }

    // Validate all layers have same dimensions
    if let Some(first) = output.layers.first() {
        let (w, h) = (first.width, first.height);
        for layer in &output.layers {
            if layer.width != w || layer.height != h {
                anyhow::bail!(
                    "Layer '{}' has different dimensions ({}x{}) than first layer ({}x{})",
                    layer.name,
                    layer.width,
                    layer.height,
                    w,
                    h
                );
            }
        }
    }

    if verbose {
        println!(
            "Writing {} layers to {}",
            output.layers.len(),
            args.output.display()
        );
    }

    // Write as multi-layer EXR
    let writer = ExrWriter::default();
    writer
        .write_layers(&args.output, &output)
        .with_context(|| format!("Failed to write: {}", args.output.display()))?;

    Ok(())
}
