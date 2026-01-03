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
            println!("  \"size_bytes\": {}", file_size);
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
        }

        if args.input.len() > 1 {
            println!();
        }
    }

    Ok(())
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
