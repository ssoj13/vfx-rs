//! LUT application command

use crate::LutArgs;
use anyhow::{Result, bail};
use vfx_io::ImageData;
use std::fs;

pub fn run(args: LutArgs, verbose: bool) -> Result<()> {
    let image = super::load_image(&args.input)?;

    if verbose {
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

    if verbose {
        println!("Done.");
    }

    Ok(())
}

fn apply_cube_lut(image: &ImageData, lut_path: &std::path::Path, invert: bool) -> Result<ImageData> {
    let content = fs::read_to_string(lut_path)?;
    let lut = parse_cube(&content)?;

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

            let result = lut.apply(rgb);

            data[idx] = result[0];
            if channels > 1 { data[idx + 1] = result[1]; }
            if channels > 2 { data[idx + 2] = result[2]; }
        }
    }

    let _ = invert; // TODO: implement LUT inversion

    Ok(ImageData::from_f32(image.width, image.height, image.channels, data))
}

/// Simple 3D LUT for CUBE format
struct CubeLut {
    data: Vec<[f32; 3]>,
    size: usize,
    domain_min: [f32; 3],
    domain_max: [f32; 3],
}

impl CubeLut {
    fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        // Normalize to LUT domain
        let r = (rgb[0] - self.domain_min[0]) / (self.domain_max[0] - self.domain_min[0]);
        let g = (rgb[1] - self.domain_min[1]) / (self.domain_max[1] - self.domain_min[1]);
        let b = (rgb[2] - self.domain_min[2]) / (self.domain_max[2] - self.domain_min[2]);

        // Clamp and convert to indices
        let r = r.clamp(0.0, 1.0) * (self.size - 1) as f32;
        let g = g.clamp(0.0, 1.0) * (self.size - 1) as f32;
        let b = b.clamp(0.0, 1.0) * (self.size - 1) as f32;

        // Trilinear interpolation
        let ri = (r.floor() as usize).min(self.size - 2);
        let gi = (g.floor() as usize).min(self.size - 2);
        let bi = (b.floor() as usize).min(self.size - 2);

        let rf = r - ri as f32;
        let gf = g - gi as f32;
        let bf = b - bi as f32;

        let idx = |r: usize, g: usize, b: usize| -> usize {
            b * self.size * self.size + g * self.size + r
        };

        let c000 = self.data[idx(ri, gi, bi)];
        let c100 = self.data[idx(ri + 1, gi, bi)];
        let c010 = self.data[idx(ri, gi + 1, bi)];
        let c110 = self.data[idx(ri + 1, gi + 1, bi)];
        let c001 = self.data[idx(ri, gi, bi + 1)];
        let c101 = self.data[idx(ri + 1, gi, bi + 1)];
        let c011 = self.data[idx(ri, gi + 1, bi + 1)];
        let c111 = self.data[idx(ri + 1, gi + 1, bi + 1)];

        let mut result = [0.0f32; 3];
        for i in 0..3 {
            let c00 = c000[i] * (1.0 - rf) + c100[i] * rf;
            let c01 = c001[i] * (1.0 - rf) + c101[i] * rf;
            let c10 = c010[i] * (1.0 - rf) + c110[i] * rf;
            let c11 = c011[i] * (1.0 - rf) + c111[i] * rf;
            let c0 = c00 * (1.0 - gf) + c10 * gf;
            let c1 = c01 * (1.0 - gf) + c11 * gf;
            result[i] = c0 * (1.0 - bf) + c1 * bf;
        }

        result
    }
}

fn parse_cube(content: &str) -> Result<CubeLut> {
    let mut size = 0usize;
    let mut domain_min = [0.0f32; 3];
    let mut domain_max = [1.0f32; 3];
    let mut data = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with("LUT_3D_SIZE") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                size = parts[1].parse().unwrap_or(0);
            }
        } else if line.starts_with("DOMAIN_MIN") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                domain_min[0] = parts[1].parse().unwrap_or(0.0);
                domain_min[1] = parts[2].parse().unwrap_or(0.0);
                domain_min[2] = parts[3].parse().unwrap_or(0.0);
            }
        } else if line.starts_with("DOMAIN_MAX") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                domain_max[0] = parts[1].parse().unwrap_or(1.0);
                domain_max[1] = parts[2].parse().unwrap_or(1.0);
                domain_max[2] = parts[3].parse().unwrap_or(1.0);
            }
        } else if !line.starts_with("TITLE") && !line.starts_with("LUT_") {
            // Parse RGB values
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let r: f32 = parts[0].parse().unwrap_or(0.0);
                let g: f32 = parts[1].parse().unwrap_or(0.0);
                let b: f32 = parts[2].parse().unwrap_or(0.0);
                data.push([r, g, b]);
            }
        }
    }

    if size == 0 || data.len() != size * size * size {
        bail!("Invalid CUBE LUT: size={}, data={}", size, data.len());
    }

    Ok(CubeLut { data, size, domain_min, domain_max })
}
