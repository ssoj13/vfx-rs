//! Pandora MGA/M3D 3D LUT format parser.
//!
//! Format used by Pandora color grading systems.
//!
//! # Format
//!
//! ```text
//! channel: 3d
//! in: 4096
//! out: 4096
//! format: lut
//! values: red green blue
//! 0 0 0 0
//! 1 0 0 4095
//! ...
//! ```
//!
//! - First column is index (ignored)
//! - Blue coordinate changes fastest (blue-fastest order)
//! - Values are integers scaled by `out` max value

use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use crate::error::{LutError, LutResult};
use crate::lut3d::Lut3D;

/// Parses a Pandora MGA/M3D file from a reader.
#[allow(unused_assignments)]
pub fn parse_mga<R: Read>(reader: R) -> LutResult<Lut3D> {
    let reader = BufReader::new(reader);
    
    let mut in_count = 0usize;
    let mut out_max = 0i32;
    let mut in_lut = false;
    let mut raw_data: Vec<[i32; 3]> = Vec::new();

    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let lower = trimmed.to_lowercase();
        let parts: Vec<&str> = trimmed.split_whitespace().collect();

        // Parse headers
        if lower.starts_with("channel:") {
            if !lower.contains("3d") {
                return Err(LutError::ParseError("only 3D LUTs supported".into()));
            }
            continue;
        }

        if lower.starts_with("in:") {
            if parts.len() >= 2 {
                in_count = parts[1].parse().map_err(|_| {
                    LutError::ParseError(format!("invalid 'in' value at line {}", line_num + 1))
                })?;
                raw_data.reserve(in_count);
            }
            continue;
        }

        if lower.starts_with("out:") {
            if parts.len() >= 2 {
                out_max = parts[1].parse().map_err(|_| {
                    LutError::ParseError(format!("invalid 'out' value at line {}", line_num + 1))
                })?;
            }
            continue;
        }

        if lower.starts_with("format:") {
            if !lower.contains("lut") {
                return Err(LutError::ParseError("only LUT format supported".into()));
            }
            continue;
        }

        if lower.starts_with("values:") {
            in_lut = true;
            continue;
        }

        // Parse LUT data
        if in_lut {
            if parts.len() != 4 {
                return Err(LutError::ParseError(format!(
                    "expected 4 values at line {}, got {}",
                    line_num + 1,
                    parts.len()
                )));
            }

            // First value is index (ignored), then R, G, B
            let r: i32 = parts[1].parse().map_err(|_| {
                LutError::ParseError(format!("invalid R value at line {}", line_num + 1))
            })?;
            let g: i32 = parts[2].parse().map_err(|_| {
                LutError::ParseError(format!("invalid G value at line {}", line_num + 1))
            })?;
            let b: i32 = parts[3].parse().map_err(|_| {
                LutError::ParseError(format!("invalid B value at line {}", line_num + 1))
            })?;

            raw_data.push([r, g, b]);
        }
    }

    if out_max <= 0 {
        return Err(LutError::ParseError("missing or invalid 'out' value".into()));
    }

    if raw_data.is_empty() {
        return Err(LutError::ParseError("no LUT data found".into()));
    }

    // Calculate LUT size (cube root of count)
    let count = raw_data.len();
    let size = (count as f64).cbrt().round() as usize;
    if size * size * size != count {
        return Err(LutError::ParseError(format!(
            "{} entries is not a perfect cube",
            count
        )));
    }

    // Scale and convert to float
    // Pandora uses blue-fastest, same as our internal Blue-major format
    let scale = 1.0 / (out_max - 1) as f32;
    let mut data = vec![[0.0f32; 3]; count];

    // Both file and memory use Blue-major: idx = B + G*size + R*size²
    // No conversion needed, just scale
    for (i, rgb) in raw_data.iter().enumerate() {
        data[i] = [
            rgb[0] as f32 * scale,
            rgb[1] as f32 * scale,
            rgb[2] as f32 * scale,
        ];
    }

    Lut3D::from_data(data, size)
}

/// Reads a Pandora MGA/M3D file from disk.
pub fn read_mga<P: AsRef<Path>>(path: P) -> LutResult<Lut3D> {
    let file = std::fs::File::open(path.as_ref())?;
    parse_mga(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_mga() {
        let data = r#"channel: 3d
in: 8
out: 256
format: lut
values: red green blue
0 0 0 0
1 0 0 255
2 0 255 0
3 0 255 255
4 255 0 0
5 255 0 255
6 255 255 0
7 255 255 255
"#;
        let lut = parse_mga(data.as_bytes()).unwrap();
        assert_eq!(lut.size, 2);
        assert_eq!(lut.data.len(), 8);

        // Check corners (after blue->red fastest conversion)
        // data[0] = black (0,0,0)
        assert!((lut.data[0][0] - 0.0).abs() < 0.01);
        // data[7] = white (1,1,1)
        assert!((lut.data[7][0] - 1.0).abs() < 0.01);
    }

    #[test]
    fn error_missing_out() {
        let data = r#"channel: 3d
in: 8
format: lut
values: red green blue
0 0 0 0
"#;
        assert!(parse_mga(data.as_bytes()).is_err());
    }

    #[test]
    fn error_non_cube() {
        let data = r#"channel: 3d
in: 7
out: 256
format: lut
values: red green blue
0 0 0 0
1 0 0 255
2 0 255 0
3 0 255 255
4 255 0 0
5 255 0 255
6 255 255 0
"#;
        assert!(parse_mga(data.as_bytes()).is_err());
    }
}
