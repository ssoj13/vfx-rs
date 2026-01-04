//! Image info command (like iinfo).
//!
//! Displays image metadata, dimensions, channels, and for EXR files - layer information.

use crate::InfoArgs;
use anyhow::Result;
use std::fs;
use vfx_io::exr::ExrReader;
use vfx_io::Format;

/// Runs the info command, displaying image metadata.
///
/// For EXR files with multiple layers, shows layer details when verbose or --all.
pub fn run(args: InfoArgs, verbose: bool) -> Result<()> {
    for path in &args.input {
        let metadata = fs::metadata(path)?;
        let file_size = metadata.len();
        let format = Format::detect(path).unwrap_or(Format::Unknown);

        // For EXR, try to get layer info
        let layer_info = if format == Format::Exr {
            ExrReader::default().read_layers(path).ok()
        } else {
            None
        };

        let image = super::load_image(path)?;

        if args.json {
            print_json(&args, path, &image, file_size, &layer_info);
        } else {
            print_text(&args, path, &image, file_size, format, verbose, &layer_info);
        }

        if args.input.len() > 1 {
            println!();
        }
    }

    Ok(())
}

/// Prints info in human-readable text format.
fn print_text(
    args: &InfoArgs,
    path: &std::path::Path,
    image: &vfx_io::ImageData,
    file_size: u64,
    format: Format,
    verbose: bool,
    layer_info: &Option<vfx_io::LayeredImage>,
) {
    println!("{}", path.display());
    println!("  Resolution: {}x{}", image.width, image.height);
    println!("  Channels:   {}", image.channels);
    println!("  Pixels:     {}", image.width as u64 * image.height as u64);
    println!("  File size:  {}", super::format_size(file_size));

    // Show layer count for multi-layer EXR
    if let Some(layered) = layer_info {
        if layered.layers.len() > 1 {
            println!("  Layers:     {}", layered.layers.len());
        }
    }

    if args.stats || args.all {
        let data = image.to_f32();
        let (min, max, avg) = compute_stats(&data);
        println!("  Min value:  {:.6}", min);
        println!("  Max value:  {:.6}", max);
        println!("  Avg value:  {:.6}", avg);
    }

    if verbose || args.all {
        println!("  Format:     {:?}", format);
    }

    // Show layer details when verbose or --all
    if (verbose || args.all) && layer_info.is_some() {
        if let Some(layered) = layer_info {
            if layered.layers.len() > 1 || args.all {
                println!("  Layer details:");
                for (idx, layer) in layered.layers.iter().enumerate() {
                    let ch_names: Vec<&str> =
                        layer.channels.iter().map(|c| c.name.as_str()).collect();
                    println!(
                        "    [{}] \"{}\" ({}x{}) {}",
                        idx,
                        layer.name,
                        layer.width,
                        layer.height,
                        ch_names.join(", ")
                    );
                }
            }
        }
    }

    if args.all {
        if let Some(colorspace) = &image.metadata.colorspace {
            println!("  Colorspace: {}", colorspace);
        }
        if let Some(gamma) = image.metadata.gamma {
            println!("  Gamma:      {}", gamma);
        }
        if let Some(dpi) = image.metadata.dpi {
            println!("  DPI:        {}", dpi);
        }

        let mut attrs: Vec<_> = image.metadata.attrs.iter().collect();
        attrs.sort_by(|a, b| a.0.cmp(b.0));
        if !attrs.is_empty() {
            println!("  Metadata:");
            for (key, value) in attrs {
                println!("    {}: {}", key, attr_to_string(value));
            }
        }
    }
}

/// Prints info in JSON format.
fn print_json(
    args: &InfoArgs,
    path: &std::path::Path,
    image: &vfx_io::ImageData,
    file_size: u64,
    layer_info: &Option<vfx_io::LayeredImage>,
) {
    println!("{{");
    println!("  \"file\": \"{}\",", path.display());
    println!("  \"width\": {},", image.width);
    println!("  \"height\": {},", image.height);
    println!("  \"channels\": {},", image.channels);
    println!("  \"size_bytes\": {},", file_size);

    // Add layers array for EXR
    if let Some(layered) = layer_info {
        println!("  \"layers\": [");
        for (idx, layer) in layered.layers.iter().enumerate() {
            let comma = if idx + 1 < layered.layers.len() { "," } else { "" };
            let ch_names: Vec<String> = layer
                .channels
                .iter()
                .map(|c| format!("\"{}\"", c.name))
                .collect();
            println!(
                "    {{\"name\": \"{}\", \"width\": {}, \"height\": {}, \"channels\": [{}]}}{}",
                layer.name,
                layer.width,
                layer.height,
                ch_names.join(", "),
                comma
            );
        }
        println!("  ],");
    }

    println!("  \"metadata\": {{");
    if let Some(colorspace) = &image.metadata.colorspace {
        println!("    \"colorspace\": \"{}\",", json_escape(colorspace));
    }
    if let Some(gamma) = image.metadata.gamma {
        println!("    \"gamma\": \"{}\",", gamma);
    }
    if let Some(dpi) = image.metadata.dpi {
        println!("    \"dpi\": \"{}\",", dpi);
    }
    println!("    \"attrs\": {{");
    let mut keys: Vec<_> = image.metadata.attrs.iter().collect();
    keys.sort_by(|a, b| a.0.cmp(b.0));
    for (idx, (key, value)) in keys.iter().enumerate() {
        let value = attr_to_string(value);
        let trailing = if idx + 1 == keys.len() { "" } else { "," };
        println!(
            "      \"{}\": \"{}\"{}",
            json_escape(key),
            json_escape(&value),
            trailing
        );
    }
    println!("    }}");
    println!("  }}");
    println!("}}");
    let _ = args; // suppress unused warning
}

fn attr_to_string(value: &vfx_io::AttrValue) -> String {
    match value {
        vfx_io::AttrValue::Bool(v) => v.to_string(),
        vfx_io::AttrValue::Str(v) => v.clone(),
        vfx_io::AttrValue::Int(v) => v.to_string(),
        vfx_io::AttrValue::UInt(v) => v.to_string(),
        vfx_io::AttrValue::Int64(v) => v.to_string(),
        vfx_io::AttrValue::UInt64(v) => v.to_string(),
        vfx_io::AttrValue::Float(v) => v.to_string(),
        vfx_io::AttrValue::Double(v) => v.to_string(),
        vfx_io::AttrValue::Bytes(v) => format!("{} bytes", v.len()),
        vfx_io::AttrValue::List(v) => format!("list({})", v.len()),
        vfx_io::AttrValue::Map(v) => format!("map({})", v.len()),
        vfx_io::AttrValue::Rational(n, d) => format!("{}/{}", n, d),
        vfx_io::AttrValue::URational(n, d) => format!("{}/{}", n, d),
        vfx_io::AttrValue::Group(g) => format!("group({})", g.len()),
    }
}

fn json_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

fn compute_stats(data: &[f32]) -> (f32, f32, f32) {
    if data.is_empty() {
        return (0.0, 0.0, 0.0);
    }

    let mut min = f32::MAX;
    let mut max = f32::MIN;
    let mut sum = 0.0f64;

    for &v in data {
        if v < min {
            min = v;
        }
        if v > max {
            max = v;
        }
        sum += v as f64;
    }

    (min, max, (sum / data.len() as f64) as f32)
}
