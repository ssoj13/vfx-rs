//! Sony Pictures Imageworks LUT formats (SPI1D, SPI3D).
//!
//! These formats are used by Sony Pictures Imageworks' OpenColorIO pipeline
//! and other VFX tools. They are simple, human-readable text formats.
//!
//! # SPI1D Format
//!
//! 1-dimensional lookup table with the following structure:
//!
//! ```text
//! Version 1
//! From 0.0 1.0
//! Length 1024
//! Components 3
//! {
//!   0.000000 0.000000 0.000000
//!   0.001000 0.001000 0.001000
//!   ...
//! }
//! ```
//!
//! # SPI3D Format
//!
//! 3-dimensional lookup table with the following structure:
//!
//! ```text
//! SPILUT 1.0
//! 3 3
//! 32 32 32
//! 0 0 0 0.000000 0.000000 0.000000
//! 1 0 0 0.033333 0.000000 0.000000
//! ...
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use vfx_lut::spi::{read_spi1d, read_spi3d};
//! use std::path::Path;
//!
//! // Read a 1D LUT
//! let lut1d = read_spi1d(Path::new("gamma.spi1d")).unwrap();
//!
//! // Read a 3D LUT
//! let lut3d = read_spi3d(Path::new("grade.spi3d")).unwrap();
//! ```
//!
//! # References
//!
//! - [OpenColorIO SPI1D](https://opencolorio.readthedocs.io/en/latest/guides/authoring/luts.html)
//! - [OpenColorIO SPI3D](https://opencolorio.readthedocs.io/en/latest/guides/authoring/luts.html)

use crate::{Lut1D, Lut3D, LutError, LutResult};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

/// Reads an SPI1D file from disk.
///
/// # Arguments
///
/// * `path` - Path to the .spi1d file
///
/// # Format
///
/// SPI1D files contain a header with version, domain range, length,
/// and number of components, followed by the LUT data in braces.
///
/// # Errors
///
/// Returns error if file cannot be read or has invalid format.
///
/// # Example
///
/// ```rust,no_run
/// use vfx_lut::spi::read_spi1d;
/// use std::path::Path;
///
/// let lut = read_spi1d(Path::new("gamma.spi1d")).unwrap();
/// let output = lut.apply(0.5);
/// ```
pub fn read_spi1d(path: &Path) -> LutResult<Lut1D> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    parse_spi1d(reader)
}

/// Parses SPI1D from a reader.
///
/// # Arguments
///
/// * `reader` - Any type implementing `BufRead`
pub fn parse_spi1d<R: BufRead>(reader: R) -> LutResult<Lut1D> {
    let mut version = 0;
    let mut from_min = 0.0f32;
    let mut from_max = 1.0f32;
    let mut length = 0usize;
    let mut components = 1usize;
    let mut in_data = false;
    let mut r_data: Vec<f32> = Vec::new();
    let mut g_data: Vec<f32> = Vec::new();
    let mut b_data: Vec<f32> = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Data section
        if line == "{" {
            in_data = true;
            continue;
        }
        if line == "}" {
            in_data = false;
            continue;
        }

        if in_data {
            let values: Vec<f32> = line
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();

            if components == 1 {
                // Mono LUT
                if let Some(&v) = values.first() {
                    r_data.push(v);
                }
            } else if components >= 3 && values.len() >= 3 {
                // RGB LUT
                r_data.push(values[0]);
                g_data.push(values[1]);
                b_data.push(values[2]);
            }
        } else {
            // Header parsing
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0].to_lowercase().as_str() {
                "version" => {
                    if parts.len() >= 2 {
                        version = parts[1].parse().unwrap_or(1);
                    }
                }
                "from" => {
                    if parts.len() >= 3 {
                        from_min = parts[1].parse().unwrap_or(0.0);
                        from_max = parts[2].parse().unwrap_or(1.0);
                    }
                }
                "length" => {
                    if parts.len() >= 2 {
                        length = parts[1].parse().unwrap_or(0);
                    }
                }
                "components" => {
                    if parts.len() >= 2 {
                        components = parts[1].parse().unwrap_or(1);
                    }
                }
                _ => {}
            }
        }
    }

    // Validate
    if r_data.is_empty() {
        return Err(LutError::ParseError("no LUT data found".into()));
    }

    // Use parsed length or actual data length
    let _actual_length = if length > 0 { length } else { r_data.len() };
    let _ = version; // Silence unused warning

    // Build LUT
    if components == 1 || g_data.is_empty() {
        Lut1D::from_data(r_data, from_min, from_max)
    } else {
        Lut1D::from_rgb(r_data, g_data, b_data, from_min, from_max)
    }
}

/// Writes an SPI1D file to disk.
///
/// # Arguments
///
/// * `path` - Output path for the .spi1d file
/// * `lut` - The 1D LUT to write
///
/// # Errors
///
/// Returns error if file cannot be written.
///
/// # Example
///
/// ```rust,no_run
/// use vfx_lut::{Lut1D, spi::write_spi1d};
/// use std::path::Path;
///
/// let lut = Lut1D::gamma(1024, 2.2);
/// write_spi1d(Path::new("gamma.spi1d"), &lut).unwrap();
/// ```
pub fn write_spi1d(path: &Path, lut: &Lut1D) -> LutResult<()> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    write_spi1d_to(writer, lut)
}

/// Writes SPI1D to any writer.
pub fn write_spi1d_to<W: Write>(mut writer: W, lut: &Lut1D) -> LutResult<()> {
    let is_rgb = lut.g.is_some();
    let components = if is_rgb { 3 } else { 1 };

    // Header
    writeln!(writer, "Version 1")?;
    // SPI1D uses scalar domain; use R channel (assume uniform)
    writeln!(writer, "From {} {}", lut.domain_min[0], lut.domain_max[0])?;
    writeln!(writer, "Length {}", lut.size())?;
    writeln!(writer, "Components {}", components)?;
    writeln!(writer, "{{")?;

    // Data
    if is_rgb {
        let g = lut.g.as_ref().unwrap();
        let b = lut.b.as_ref().unwrap();
        for i in 0..lut.size() {
            writeln!(writer, "  {:.6} {:.6} {:.6}", lut.r[i], g[i], b[i])?;
        }
    } else {
        for v in &lut.r {
            writeln!(writer, "  {:.6}", v)?;
        }
    }

    writeln!(writer, "}}")?;

    Ok(())
}

/// Reads an SPI3D file from disk.
///
/// # Arguments
///
/// * `path` - Path to the .spi3d file
///
/// # Format
///
/// SPI3D files contain a header with version and size info,
/// followed by grid index and RGB output values per line.
///
/// # Errors
///
/// Returns error if file cannot be read or has invalid format.
///
/// # Example
///
/// ```rust,no_run
/// use vfx_lut::spi::read_spi3d;
/// use std::path::Path;
///
/// let lut = read_spi3d(Path::new("grade.spi3d")).unwrap();
/// let output = lut.apply([0.5, 0.3, 0.2]);
/// ```
pub fn read_spi3d(path: &Path) -> LutResult<Lut3D> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    parse_spi3d(reader)
}

/// Parses SPI3D from a reader.
///
/// # Arguments
///
/// * `reader` - Any type implementing `BufRead`
pub fn parse_spi3d<R: BufRead>(reader: R) -> LutResult<Lut3D> {
    let mut size = 0usize;
    let mut data: Vec<[f32; 3]> = Vec::new();
    let mut header_lines = 0;

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();

        // Parse header
        if header_lines < 3 {
            header_lines += 1;

            // First line: "SPILUT 1.0" (optional magic)
            if line.to_uppercase().starts_with("SPILUT") {
                continue;
            }

            // Size line: "32 32 32" or just "32"
            if parts.len() >= 1 && parts.len() <= 3 {
                if let Ok(s) = parts[0].parse::<usize>() {
                    // Check if it's a size specification
                    if s > 0 && s <= 256 {
                        size = s;
                        continue;
                    }
                }
            }

            // Header line with "3 3" (components)
            if parts.len() == 2 {
                if let (Ok(a), Ok(b)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                    if a == 3 && b == 3 {
                        continue;
                    }
                }
            }
        }

        // Data lines: "r g b R G B" (indices + values) or just "R G B"
        if parts.len() >= 3 {
            // Try parsing last 3 values as RGB output
            let rgb_start = if parts.len() >= 6 { 3 } else { 0 };

            if rgb_start + 2 < parts.len() || parts.len() == 3 {
                let r: f32 = parts[rgb_start].parse().unwrap_or(0.0);
                let g: f32 = parts[rgb_start + 1].parse().unwrap_or(0.0);
                let b: f32 = parts[rgb_start + 2].parse().unwrap_or(0.0);
                data.push([r, g, b]);
            }
        }
    }

    // Infer size from data if not specified
    if size == 0 {
        let total = data.len();
        // Find cube root
        for s in 2..=128 {
            if s * s * s == total {
                size = s;
                break;
            }
        }
    }

    if size == 0 || data.is_empty() {
        return Err(LutError::ParseError("invalid SPI3D format".into()));
    }

    Lut3D::from_data(data, size)
}

/// Writes an SPI3D file to disk.
///
/// # Arguments
///
/// * `path` - Output path for the .spi3d file
/// * `lut` - The 3D LUT to write
///
/// # Errors
///
/// Returns error if file cannot be written.
///
/// # Example
///
/// ```rust,no_run
/// use vfx_lut::{Lut3D, spi::write_spi3d};
/// use std::path::Path;
///
/// let lut = Lut3D::identity(33);
/// write_spi3d(Path::new("identity.spi3d"), &lut).unwrap();
/// ```
pub fn write_spi3d(path: &Path, lut: &Lut3D) -> LutResult<()> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    write_spi3d_to(writer, lut)
}

/// Writes SPI3D to any writer.
pub fn write_spi3d_to<W: Write>(mut writer: W, lut: &Lut3D) -> LutResult<()> {
    let size = lut.size;

    // Header
    writeln!(writer, "SPILUT 1.0")?;
    writeln!(writer, "3 3")?;
    writeln!(writer, "{} {} {}", size, size, size)?;

    // Data with indices
    // Memory is Blue-major: idx = B + dim*G + dim²*R  
    for b in 0..size {
        for g in 0..size {
            for r in 0..size {
                // Blue-major index: B + size*G + size²*R
                let idx = b + size * (g + size * r);
                let rgb = lut.data[idx];
                writeln!(
                    writer,
                    "{} {} {} {:.6} {:.6} {:.6}",
                    r, g, b, rgb[0], rgb[1], rgb[2]
                )?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_spi1d_mono() {
        let data = r#"
Version 1
From 0.0 1.0
Length 4
Components 1
{
  0.0
  0.333333
  0.666666
  1.0
}
"#;
        let lut = parse_spi1d(Cursor::new(data)).unwrap();
        assert_eq!(lut.size(), 4);
        assert!(lut.is_mono());
        assert!((lut.apply(0.5) - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_parse_spi1d_rgb() {
        let data = r#"
Version 1
From 0.0 1.0
Length 3
Components 3
{
  0.0 0.0 0.0
  0.5 0.5 0.5
  1.0 1.0 1.0
}
"#;
        let lut = parse_spi1d(Cursor::new(data)).unwrap();
        assert_eq!(lut.size(), 3);
        assert!(!lut.is_mono());
    }

    #[test]
    fn test_spi1d_roundtrip() {
        let lut = Lut1D::gamma(64, 2.2);

        let mut buf = Vec::new();
        write_spi1d_to(&mut buf, &lut).unwrap();

        let parsed = parse_spi1d(Cursor::new(buf)).unwrap();

        assert_eq!(parsed.size(), 64);
        assert!((parsed.apply(0.5) - lut.apply(0.5)).abs() < 0.001);
    }

    #[test]
    fn test_parse_spi3d() {
        let data = r#"
SPILUT 1.0
3 3
2 2 2
0 0 0 0.0 0.0 0.0
1 0 0 1.0 0.0 0.0
0 1 0 0.0 1.0 0.0
1 1 0 1.0 1.0 0.0
0 0 1 0.0 0.0 1.0
1 0 1 1.0 0.0 1.0
0 1 1 0.0 1.0 1.0
1 1 1 1.0 1.0 1.0
"#;
        let lut = parse_spi3d(Cursor::new(data)).unwrap();
        assert_eq!(lut.size, 2);

        // Test black corner
        let black = lut.apply([0.0, 0.0, 0.0]);
        assert!((black[0]).abs() < 0.01);

        // Test white corner
        let white = lut.apply([1.0, 1.0, 1.0]);
        assert!((white[0] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_spi3d_roundtrip() {
        let lut = Lut3D::identity(8);

        let mut buf = Vec::new();
        write_spi3d_to(&mut buf, &lut).unwrap();

        let parsed = parse_spi3d(Cursor::new(buf)).unwrap();

        assert_eq!(parsed.size, 8);

        // Test mid-gray
        let result = parsed.apply([0.5, 0.5, 0.5]);
        assert!((result[0] - 0.5).abs() < 0.1);
        assert!((result[1] - 0.5).abs() < 0.1);
        assert!((result[2] - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_spi3d_no_indices() {
        // Some SPI3D files only have RGB values without indices
        let data = r#"
SPILUT 1.0
3 3
2 2 2
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
"#;
        let lut = parse_spi3d(Cursor::new(data)).unwrap();
        assert_eq!(lut.size, 2);
    }
}
