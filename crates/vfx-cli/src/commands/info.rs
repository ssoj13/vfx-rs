//! Image info command (like iinfo)

use crate::InfoArgs;
use anyhow::Result;
use std::fs;
use vfx_io::Format;

pub fn run(args: InfoArgs, verbose: bool) -> Result<()> {
    for path in &args.input {
        let metadata = fs::metadata(path)?;
        let file_size = metadata.len();

        let image = super::load_image(path)?;

        if args.json {
            println!("{{");
            println!("  \"file\": \"{}\",", path.display());
            println!("  \"width\": {},", image.width);
            println!("  \"height\": {},", image.height);
            println!("  \"channels\": {},", image.channels);
            println!("  \"size_bytes\": {},", file_size);
            println!("  \"metadata\": {{");
            if let Some(colorspace) = &image.metadata.colorspace {
                println!(
                    "    \"colorspace\": \"{}\",",
                    json_escape(colorspace)
                );
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
        } else {
            println!("{}", path.display());
            println!("  Resolution: {}x{}", image.width, image.height);
            println!("  Channels:   {}", image.channels);
            println!("  Pixels:     {}", image.width as u64 * image.height as u64);
            println!("  File size:  {}", super::format_size(file_size));

            if args.stats || args.all {
                let data = image.to_f32();
                let (min, max, avg) = compute_stats(&data);
                println!("  Min value:  {:.6}", min);
                println!("  Max value:  {:.6}", max);
                println!("  Avg value:  {:.6}", avg);
            }

            if verbose || args.all {
                let format = Format::detect(path).unwrap_or(Format::Unknown);
                println!("  Format:     {:?}", format);
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

        if args.input.len() > 1 {
            println!();
        }
    }

    Ok(())
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
        if v < min { min = v; }
        if v > max { max = v; }
        sum += v as f64;
    }

    (min, max, (sum / data.len() as f64) as f32)
}
