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

use crate::{Interpolation, Lut1D, Lut3D, LutError, LutResult};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

/// Combined .cube file that may contain 1D and/or 3D LUTs.
///
/// Some applications (e.g., DaVinci Resolve) generate .cube files
/// containing both 1D shaper and 3D LUT in a single file.
#[derive(Debug, Clone)]
pub struct CubeFile {
    /// Optional title from TITLE keyword.
    pub title: Option<String>,
    /// Optional 1D LUT (shaper).
    pub lut1d: Option<Lut1D>,
    /// Optional 3D LUT.
    pub lut3d: Option<Lut3D>,
}

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

/// Reads a combined .cube file that may contain both 1D and 3D LUTs.
///
/// This supports Resolve-style .cube files with `LUT_1D_SIZE` + `LUT_3D_SIZE`.
///
/// # Example
///
/// ```rust,ignore
/// let cube = cube::read_cube("resolve_1d3d.cube")?;
/// if let Some(lut1d) = &cube.lut1d {
///     // Apply shaper...
/// }
/// if let Some(lut3d) = &cube.lut3d {
///     // Apply 3D LUT...
/// }
/// ```
pub fn read_cube<P: AsRef<Path>>(path: P) -> LutResult<CubeFile> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    parse_cube(reader)
}

/// Parses a combined .cube file from a reader.
///
/// Handles files with 1D only, 3D only, or both 1D and 3D LUTs.
pub fn parse_cube<R: BufRead>(reader: R) -> LutResult<CubeFile> {
    let mut title: Option<String> = None;
    let mut size_1d: Option<usize> = None;
    let mut size_3d: Option<usize> = None;
    let mut domain_min_1d = [0.0_f32; 3];
    let mut domain_max_1d = [1.0_f32; 3];
    let mut domain_min_3d = [0.0_f32; 3];
    let mut domain_max_3d = [1.0_f32; 3];
    let mut data: Vec<[f32; 3]> = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with("TITLE") {
            // Extract title in quotes
            if let Some(start) = line.find('"') {
                if let Some(end) = line.rfind('"') {
                    if end > start {
                        title = Some(line[start + 1..end].to_string());
                    }
                }
            }
        } else if line.starts_with("LUT_1D_SIZE") {
            size_1d = Some(parse_size(line)?);
        } else if line.starts_with("LUT_3D_SIZE") {
            size_3d = Some(parse_size(line)?);
        } else if line.starts_with("LUT_1D_INPUT_RANGE") {
            let (min, max) = parse_input_range(line)?;
            domain_min_1d = min;
            domain_max_1d = max;
        } else if line.starts_with("LUT_3D_INPUT_RANGE") {
            let (min, max) = parse_input_range(line)?;
            domain_min_3d = min;
            domain_max_3d = max;
        } else if line.starts_with("DOMAIN_MIN") {
            // Generic DOMAIN_MIN applies to both (use for whichever is present)
            let d = parse_domain(line)?;
            domain_min_1d = d;
            domain_min_3d = d;
        } else if line.starts_with("DOMAIN_MAX") {
            let d = parse_domain(line)?;
            domain_max_1d = d;
            domain_max_3d = d;
        } else {
            // Data line
            let rgb = parse_rgb(line)?;
            data.push(rgb);
        }
    }

    // Split data between 1D and 3D based on sizes
    let data_1d_count = size_1d.unwrap_or(0);
    let data_3d_count = size_3d.map(|s| s * s * s).unwrap_or(0);
    let expected = data_1d_count + data_3d_count;

    if data.len() != expected {
        return Err(LutError::ParseError(format!(
            "expected {} data lines (1D: {}, 3D: {}), found {}",
            expected, data_1d_count, data_3d_count, data.len()
        )));
    }

    // Build 1D LUT if present
    let lut1d = if let Some(size) = size_1d {
        let data_1d: Vec<[f32; 3]> = data[..size].to_vec();
        let r: Vec<f32> = data_1d.iter().map(|rgb| rgb[0]).collect();
        let g: Vec<f32> = data_1d.iter().map(|rgb| rgb[1]).collect();
        let b: Vec<f32> = data_1d.iter().map(|rgb| rgb[2]).collect();
        Some(Lut1D::from_rgb_per_channel(r, g, b, domain_min_1d, domain_max_1d)?)
    } else {
        None
    };

    // Build 3D LUT if present
    let lut3d = if let Some(size) = size_3d {
        let data_3d: Vec<[f32; 3]> = data[data_1d_count..].to_vec();
        
        // Reorder from file order (R-fastest) to memory order (B-fastest)
        let total = size * size * size;
        let mut reordered = vec![[0.0f32; 3]; total];
        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    let file_idx = r + g * size + b * size * size;
                    let mem_idx = b + g * size + r * size * size;
                    reordered[mem_idx] = data_3d[file_idx];
                }
            }
        }
        
        Some(Lut3D {
            data: reordered,
            size,
            domain_min: domain_min_3d,
            domain_max: domain_max_3d,
            interpolation: Interpolation::default(),
        })
    } else {
        None
    };

    if lut1d.is_none() && lut3d.is_none() {
        return Err(LutError::ParseError("no LUT_1D_SIZE or LUT_3D_SIZE found".into()));
    }

    Ok(CubeFile { title, lut1d, lut3d })
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
            return Err(LutError::ParseError("expected 1D LUT, found LUT_3D_SIZE. Use parse_cube() for combined 1D+3D files.".into()));
        } else if line.starts_with("DOMAIN_MIN") {
            domain_min = parse_domain(line)?;
        } else if line.starts_with("DOMAIN_MAX") {
            domain_max = parse_domain(line)?;
        } else if line.starts_with("LUT_1D_INPUT_RANGE") {
            // Resolve-style INPUT_RANGE: two scalar values (uniform domain)
            let (min, max) = parse_input_range(line)?;
            domain_min = min;
            domain_max = max;
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

    // Use per-channel domain from .cube file (supports different ranges per channel)
    Lut1D::from_rgb_per_channel(r, g, b, domain_min, domain_max)
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
            return Err(LutError::ParseError("expected 3D LUT, found LUT_1D_SIZE. Use parse_cube() for combined 1D+3D files.".into()));
        } else if line.starts_with("DOMAIN_MIN") {
            domain_min = parse_domain(line)?;
        } else if line.starts_with("DOMAIN_MAX") {
            domain_max = parse_domain(line)?;
        } else if line.starts_with("LUT_3D_INPUT_RANGE") {
            // Resolve-style INPUT_RANGE: two scalar values (uniform domain)
            let (min, max) = parse_input_range(line)?;
            domain_min = min;
            domain_max = max;
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
    // Write DOMAIN_MIN/MAX only if non-default (not all 0.0/1.0)
    let is_default = min == [0.0, 0.0, 0.0] && max == [1.0, 1.0, 1.0];
    if !is_default {
        writeln!(writer, "DOMAIN_MIN {} {} {}", min[0], min[1], min[2])?;
        writeln!(writer, "DOMAIN_MAX {} {} {}", max[0], max[1], max[2])?;
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

/// Parses INPUT_RANGE line (Resolve format): two scalar values (min, max).
/// Returns uniform per-channel domain arrays.
fn parse_input_range(line: &str) -> LutResult<([f32; 3], [f32; 3])> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return Err(LutError::ParseError("invalid INPUT_RANGE line".into()));
    }
    let min: f32 = parts[1].parse().map_err(|_| LutError::ParseError("invalid INPUT_RANGE min".into()))?;
    let max: f32 = parts[2].parse().map_err(|_| LutError::ParseError("invalid INPUT_RANGE max".into()))?;
    Ok(([min, min, min], [max, max, max]))
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
