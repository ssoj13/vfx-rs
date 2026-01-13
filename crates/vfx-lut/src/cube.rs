//! Adobe/Resolve .cube LUT format support.
//!
//! The .cube format is a simple text-based LUT format widely supported
//! by DaVinci Resolve, Adobe applications, and many other tools.
//!
//! # Format
//!
//! ```text
//! # Comment
//! TITLE "LUT Name"
//! LUT_3D_SIZE 33
//! DOMAIN_MIN 0.0 0.0 0.0
//! DOMAIN_MAX 1.0 1.0 1.0
//! 0.0 0.0 0.0
//! ...
//! 1.0 1.0 1.0
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use vfx_lut::cube;
//!
//! let lut = cube::read_3d("grade.cube")?;
//! let rgb = lut.apply([0.5, 0.3, 0.2]);
//! ```

use crate::{Lut1D, Lut3D, LutError, LutResult};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

/// Reads a 1D LUT from a .cube file.
///
/// # Example
///
/// ```rust,ignore
/// let lut = cube::read_1d("curve.cube")?;
/// ```
pub fn read_1d<P: AsRef<Path>>(path: P) -> LutResult<Lut1D> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    parse_1d(reader)
}

/// Reads a 3D LUT from a .cube file.
///
/// # Example
///
/// ```rust,ignore
/// let lut = cube::read_3d("grade.cube")?;
/// ```
pub fn read_3d<P: AsRef<Path>>(path: P) -> LutResult<Lut3D> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    parse_3d(reader)
}

/// Parses a 1D LUT from a reader.
pub fn parse_1d<R: BufRead>(reader: R) -> LutResult<Lut1D> {
    let mut size: Option<usize> = None;
    let mut domain_min = [0.0_f32; 3];
    let mut domain_max = [1.0_f32; 3];
    let mut data: Vec<[f32; 3]> = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse keywords
        if line.starts_with("TITLE") {
            continue;
        } else if line.starts_with("LUT_1D_SIZE") {
            size = Some(parse_size(line)?);
        } else if line.starts_with("LUT_3D_SIZE") {
            return Err(LutError::ParseError("expected 1D LUT, found 3D".into()));
        } else if line.starts_with("DOMAIN_MIN") {
            domain_min = parse_domain(line)?;
        } else if line.starts_with("DOMAIN_MAX") {
            domain_max = parse_domain(line)?;
        } else {
            // Data line
            let rgb = parse_rgb(line)?;
            data.push(rgb);
        }
    }

    let size = size.ok_or_else(|| LutError::ParseError("missing LUT_1D_SIZE".into()))?;

    if data.len() != size {
        return Err(LutError::ParseError(format!(
            "expected {} entries, found {}",
            size,
            data.len()
        )));
    }

    // Convert to separate channels
    let r: Vec<f32> = data.iter().map(|rgb| rgb[0]).collect();
    let g: Vec<f32> = data.iter().map(|rgb| rgb[1]).collect();
    let b: Vec<f32> = data.iter().map(|rgb| rgb[2]).collect();

    Lut1D::from_rgb(r, g, b, domain_min[0], domain_max[0])
}

/// Parses a 3D LUT from a reader.
pub fn parse_3d<R: BufRead>(reader: R) -> LutResult<Lut3D> {
    let mut size: Option<usize> = None;
    let mut domain_min = [0.0_f32; 3];
    let mut domain_max = [1.0_f32; 3];
    let mut data: Vec<[f32; 3]> = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse keywords
        if line.starts_with("TITLE") {
            continue;
        } else if line.starts_with("LUT_3D_SIZE") {
            size = Some(parse_size(line)?);
        } else if line.starts_with("LUT_1D_SIZE") {
            return Err(LutError::ParseError("expected 3D LUT, found 1D".into()));
        } else if line.starts_with("DOMAIN_MIN") {
            domain_min = parse_domain(line)?;
        } else if line.starts_with("DOMAIN_MAX") {
            domain_max = parse_domain(line)?;
        } else {
            // Data line
            let rgb = parse_rgb(line)?;
            data.push(rgb);
        }
    }

    let size = size.ok_or_else(|| LutError::ParseError("missing LUT_3D_SIZE".into()))?;
    let expected = size * size * size;

    if data.len() != expected {
        return Err(LutError::ParseError(format!(
            "expected {} values, found {}",
            expected,
            data.len()
        )));
    }

    // Convert from file order (R-fastest) to memory order (Blue-major: B-fastest)
    // CUBE file: idx = r + g*size + b*size²
    // Memory: idx = b + g*size + r*size²
    let mut reordered = vec![[0.0f32; 3]; expected];
    for b in 0..size {
        for g in 0..size {
            for r in 0..size {
                let file_idx = r + g * size + b * size * size;  // R-fastest
                let mem_idx = b + g * size + r * size * size;   // B-fastest (Blue-major)
                reordered[mem_idx] = data[file_idx];
            }
        }
    }

    let lut = Lut3D::from_data(reordered, size)?
        .with_domain(domain_min, domain_max);

    Ok(lut)
}

/// Writes a 1D LUT to a .cube file.
///
/// # Example
///
/// ```rust,ignore
/// let lut = Lut1D::gamma(1024, 2.2);
/// cube::write_1d("curve.cube", &lut)?;
/// ```
pub fn write_1d<P: AsRef<Path>>(path: P, lut: &Lut1D) -> LutResult<()> {
    let file = File::create(path.as_ref())?;
    let mut writer = BufWriter::new(file);

    // Header
    writeln!(writer, "# Generated by vfx-lut")?;
    writeln!(writer, "LUT_1D_SIZE {}", lut.size())?;

    let min = lut.domain_min;
    let max = lut.domain_max;
    if min != 0.0 || max != 1.0 {
        writeln!(writer, "DOMAIN_MIN {} {} {}", min, min, min)?;
        writeln!(writer, "DOMAIN_MAX {} {} {}", max, max, max)?;
    }
    writeln!(writer)?;

    // Data
    let r = &lut.r;
    let g = lut.g.as_ref().unwrap_or(&lut.r);
    let b = lut.b.as_ref().unwrap_or(&lut.r);
    for i in 0..lut.size() {
        writeln!(writer, "{:.6} {:.6} {:.6}", r[i], g[i], b[i])?;
    }

    Ok(())
}

/// Writes a 3D LUT to a .cube file.
///
/// # Example
///
/// ```rust,ignore
/// let lut = Lut3D::identity(33);
/// cube::write_3d("identity.cube", &lut)?;
/// ```
pub fn write_3d<P: AsRef<Path>>(path: P, lut: &Lut3D) -> LutResult<()> {
    let file = File::create(path.as_ref())?;
    let mut writer = BufWriter::new(file);

    // Header
    writeln!(writer, "# Generated by vfx-lut")?;
    writeln!(writer, "LUT_3D_SIZE {}", lut.size)?;

    let min = lut.domain_min;
    let max = lut.domain_max;
    if min != [0.0, 0.0, 0.0] || max != [1.0, 1.0, 1.0] {
        writeln!(writer, "DOMAIN_MIN {} {} {}", min[0], min[1], min[2])?;
        writeln!(writer, "DOMAIN_MAX {} {} {}", max[0], max[1], max[2])?;
    }
    writeln!(writer)?;

    // Data - iterate R fastest, then G, then B (file format requirement)
    // Memory is Blue-major: idx = B + dim*G + dim²*R
    let data = &lut.data;
    let size = lut.size;
    for b_idx in 0..size {
        for g_idx in 0..size {
            for r_idx in 0..size {
                // Blue-major index: B + size*G + size²*R
                let i = b_idx + size * (g_idx + size * r_idx);
                let rgb = data[i];
                writeln!(writer, "{:.6} {:.6} {:.6}", rgb[0], rgb[1], rgb[2])?;
            }
        }
    }

    Ok(())
}

// Helper functions

fn parse_size(line: &str) -> LutResult<usize> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(LutError::ParseError("invalid size line".into()));
    }
    parts[1]
        .parse()
        .map_err(|_| LutError::ParseError("invalid size value".into()))
}

fn parse_domain(line: &str) -> LutResult<[f32; 3]> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 4 {
        return Err(LutError::ParseError("invalid domain line".into()));
    }
    Ok([
        parts[1].parse().map_err(|_| LutError::ParseError("invalid domain R".into()))?,
        parts[2].parse().map_err(|_| LutError::ParseError("invalid domain G".into()))?,
        parts[3].parse().map_err(|_| LutError::ParseError("invalid domain B".into()))?,
    ])
}

fn parse_rgb(line: &str) -> LutResult<[f32; 3]> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return Err(LutError::ParseError(format!("invalid RGB line: {}", line)));
    }
    Ok([
        parts[0].parse().map_err(|_| LutError::ParseError("invalid R value".into()))?,
        parts[1].parse().map_err(|_| LutError::ParseError("invalid G value".into()))?,
        parts[2].parse().map_err(|_| LutError::ParseError("invalid B value".into()))?,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parse_3d_cube() {
        let cube = r#"
# Test LUT
TITLE "Test Grade"
LUT_3D_SIZE 2
DOMAIN_MIN 0.0 0.0 0.0
DOMAIN_MAX 1.0 1.0 1.0

0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
"#;
        let reader = Cursor::new(cube);
        let lut = parse_3d(reader).expect("parse failed");

        assert_eq!(lut.size, 2);
    }

    #[test]
    fn parse_1d_cube() {
        let cube = r#"
TITLE "Gamma 2.2"
LUT_1D_SIZE 3

0.0 0.0 0.0
0.5 0.5 0.5
1.0 1.0 1.0
"#;
        let reader = Cursor::new(cube);
        let lut = parse_1d(reader).expect("parse failed");

        assert_eq!(lut.size(), 3);
    }

    #[test]
    fn roundtrip_3d() {
        let lut = Lut3D::identity(4);
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_roundtrip.cube");

        write_3d(&path, &lut).expect("write failed");
        let loaded = read_3d(&path).expect("read failed");

        assert_eq!(loaded.size, 4);
        let _ = std::fs::remove_file(&path);
    }
}
