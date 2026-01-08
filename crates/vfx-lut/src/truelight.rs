//! FilmLight Truelight .cub format support.
//!
//! The Truelight .cub format is used by FilmLight Truelight and other tools.
//! It supports 3D LUTs with optional 1D shaper (InputLUT).
//!
//! # Format
//!
//! ```text
//! # Truelight Cube v2.0
//! # lutLength 1024
//! # iDims     3
//! # oDims     3
//! # width     32 32 32
//!
//! # InputLUT
//! 0.000000 0.000000 0.000000
//! 0.030303 0.030303 0.030303
//! ...
//!
//! # Cube
//! 0.0 0.0 0.0
//! 1.0 0.0 0.0
//! ...
//! # end
//! ```
//!
//! # Notes
//!
//! - InputLUT values are scaled to [0, cube_size-1] range
//! - On read, values are descaled back to [0, 1]
//! - 3D LUT must be a cube (equal dimensions)

use crate::{Lut1D, Lut3D, LutError, LutResult};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;

/// Parsed Truelight file.
#[derive(Debug, Clone)]
pub struct TruelightFile {
    /// Optional 1D shaper LUT (InputLUT)
    pub shaper: Option<Lut1D>,
    /// 3D cube LUT
    pub cube: Lut3D,
}

impl TruelightFile {
    /// Create from 3D LUT only.
    pub fn new(cube: Lut3D) -> Self {
        Self { shaper: None, cube }
    }

    /// Create with shaper and 3D LUT.
    pub fn with_shaper(shaper: Lut1D, cube: Lut3D) -> Self {
        Self {
            shaper: Some(shaper),
            cube,
        }
    }
}

/// Read a Truelight .cub file.
pub fn read_cub<P: AsRef<Path>>(path: P) -> LutResult<TruelightFile> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    parse_cub(reader)
}

/// Parse a Truelight .cub file from reader.
pub fn parse_cub<R: Read>(reader: R) -> LutResult<TruelightFile> {
    let buf_reader = BufReader::new(reader);
    let mut lines = buf_reader.lines();

    // Validate first line
    let first_line = lines
        .next()
        .ok_or_else(|| LutError::ParseError("empty file".into()))??;

    if !first_line.to_lowercase().contains("truelight cube") {
        return Err(LutError::ParseError(
            "not a Truelight .cub file".into(),
        ));
    }

    // Parse headers and data
    let mut lut_length = 0;
    let mut width = [0, 0, 0];
    let mut in_1d = false;
    let mut in_3d = false;
    let mut raw_1d: Vec<f32> = Vec::new();
    let mut raw_3d: Vec<f32> = Vec::new();

    for line_result in lines {
        let line = line_result?;
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        // Parse header comments
        if trimmed.starts_with('#') {
            let lower = trimmed.to_lowercase();
            let parts: Vec<&str> = lower.split_whitespace().collect();

            if parts.len() < 2 {
                continue;
            }

            match parts[1] {
                "lutlength" => {
                    if parts.len() >= 3 {
                        lut_length = parts[2].parse().unwrap_or(0);
                    }
                }
                "width" => {
                    if parts.len() >= 5 {
                        width[0] = parts[2].parse().unwrap_or(0);
                        width[1] = parts[3].parse().unwrap_or(0);
                        width[2] = parts[4].parse().unwrap_or(0);
                    }
                }
                "inputlut" => {
                    in_1d = true;
                    in_3d = false;
                }
                "cube" => {
                    in_3d = true;
                    in_1d = false;
                }
                "end" => {
                    break;
                }
                _ => {}
            }
            continue;
        }

        // Parse data lines
        let values: Vec<f32> = trimmed
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        if values.len() == 3 {
            if in_1d {
                raw_1d.extend_from_slice(&values);
            } else if in_3d {
                raw_3d.extend_from_slice(&values);
            }
        }
    }

    // Validate sizes
    if width[0] != width[1] || width[0] != width[2] {
        return Err(LutError::ParseError(
            "non-cubic 3D LUT not supported".into(),
        ));
    }

    let cube_size = width[0];
    if cube_size == 0 {
        return Err(LutError::ParseError("missing width".into()));
    }

    let expected_3d = cube_size * cube_size * cube_size * 3;
    if raw_3d.len() < expected_3d {
        return Err(LutError::ParseError(format!(
            "3D LUT has {} values, expected {}",
            raw_3d.len(),
            expected_3d
        )));
    }

    // Build 3D LUT
    let cube_data: Vec<[f32; 3]> = raw_3d[..expected_3d]
        .chunks(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect();

    let cube = Lut3D::from_data(cube_data, cube_size)?;

    // Build optional 1D shaper
    let shaper = if lut_length > 0 && raw_1d.len() >= lut_length * 3 {
        // Descale from [0, cube_size-1] to [0, 1]
        let descale = 1.0 / (cube_size - 1) as f32;

        let r: Vec<f32> = raw_1d[..lut_length * 3]
            .iter()
            .step_by(3)
            .map(|&v| v * descale)
            .collect();
        let g: Vec<f32> = raw_1d[1..lut_length * 3]
            .iter()
            .step_by(3)
            .map(|&v| v * descale)
            .collect();
        let b: Vec<f32> = raw_1d[2..lut_length * 3]
            .iter()
            .step_by(3)
            .map(|&v| v * descale)
            .collect();

        Some(Lut1D::from_rgb(r, g, b, 0.0, 1.0)?)
    } else {
        None
    };

    Ok(TruelightFile { shaper, cube })
}

/// Write a Truelight .cub file.
pub fn write_cub<P: AsRef<Path>>(path: P, tl: &TruelightFile) -> LutResult<()> {
    let file = File::create(path.as_ref())?;
    let writer = BufWriter::new(file);
    write_cub_to(writer, tl)
}

/// Write a Truelight .cub file to any writer.
pub fn write_cub_to<W: Write>(mut writer: W, tl: &TruelightFile) -> LutResult<()> {
    let cube_size = tl.cube.size;
    let shaper_size = tl.shaper.as_ref().map(|s| s.size()).unwrap_or(1024);

    // Header
    writeln!(writer, "# Truelight Cube v2.0")?;
    writeln!(writer, "# lutLength {}", shaper_size)?;
    writeln!(writer, "# iDims     3")?;
    writeln!(writer, "# oDims     3")?;
    writeln!(
        writer,
        "# width     {} {} {}",
        cube_size, cube_size, cube_size
    )?;
    writeln!(writer)?;

    // InputLUT (shaper)
    writeln!(writer, "# InputLUT")?;

    if let Some(shaper) = &tl.shaper {
        // Scale to [0, cube_size-1]
        let scale = (cube_size - 1) as f32;
        let r = &shaper.r;
        let g = shaper.g.as_ref().unwrap_or(&shaper.r);
        let b = shaper.b.as_ref().unwrap_or(&shaper.r);

        for i in 0..shaper.size() {
            writeln!(
                writer,
                "{:.6} {:.6} {:.6}",
                r[i] * scale,
                g[i] * scale,
                b[i] * scale
            )?;
        }
    } else {
        // Generate identity shaper
        let scale = (cube_size - 1) as f32;
        for i in 0..shaper_size {
            let v = (i as f32 / (shaper_size - 1) as f32) * scale;
            writeln!(writer, "{:.6} {:.6} {:.6}", v, v, v)?;
        }
    }

    writeln!(writer)?;

    // Cube
    writeln!(writer, "# Cube")?;
    for rgb in &tl.cube.data {
        writeln!(writer, "{:.6} {:.6} {:.6}", rgb[0], rgb[1], rgb[2])?;
    }

    writeln!(writer, "# end")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parse_basic_cub() {
        let content = r#"# Truelight Cube v2.0
# lutLength 4
# iDims     3
# oDims     3
# width     2 2 2

# InputLUT
0.0 0.0 0.0
0.333 0.333 0.333
0.666 0.666 0.666
1.0 1.0 1.0

# Cube
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
# end
"#;
        let tl = parse_cub(Cursor::new(content)).expect("parse failed");

        assert_eq!(tl.cube.size, 2);
        assert!(tl.shaper.is_some());

        let shaper = tl.shaper.unwrap();
        assert_eq!(shaper.size(), 4);
    }

    #[test]
    fn parse_no_shaper() {
        let content = r#"# Truelight Cube v2.0
# width     2 2 2

# Cube
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
# end
"#;
        let tl = parse_cub(Cursor::new(content)).expect("parse failed");

        assert_eq!(tl.cube.size, 2);
        assert!(tl.shaper.is_none());
    }

    #[test]
    fn roundtrip_3d() {
        let cube = Lut3D::identity(4);
        let tl = TruelightFile::new(cube);

        let mut buf = Vec::new();
        write_cub_to(&mut buf, &tl).expect("write failed");

        let parsed = parse_cub(Cursor::new(buf)).expect("parse failed");

        assert_eq!(parsed.cube.size, 4);
    }

    #[test]
    fn roundtrip_with_shaper() {
        let shaper = Lut1D::gamma(64, 2.2);
        let cube = Lut3D::identity(8);
        let tl = TruelightFile::with_shaper(shaper, cube);

        let mut buf = Vec::new();
        write_cub_to(&mut buf, &tl).expect("write failed");

        let parsed = parse_cub(Cursor::new(buf)).expect("parse failed");

        assert_eq!(parsed.cube.size, 8);
        assert!(parsed.shaper.is_some());
        assert_eq!(parsed.shaper.unwrap().size(), 64);
    }

    #[test]
    fn shaper_descaling() {
        // Test that shaper values are properly descaled
        let content = r#"# Truelight Cube v2.0
# lutLength 2
# width     4 4 4

# InputLUT
0.0 0.0 0.0
3.0 3.0 3.0

# Cube
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
# end
"#;
        let tl = parse_cub(Cursor::new(content)).expect("parse failed");

        let shaper = tl.shaper.expect("should have shaper");
        // cube_size = 4, so descale = 1/3
        // 0.0 -> 0.0, 3.0 -> 1.0
        assert!((shaper.r[0] - 0.0).abs() < 0.01);
        assert!((shaper.r[1] - 1.0).abs() < 0.01);
    }
}
