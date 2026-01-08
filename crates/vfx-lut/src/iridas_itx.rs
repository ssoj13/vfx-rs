//! Iridas ITX 3D LUT format parser.
//!
//! Simple 3D LUT format used by Iridas/Adobe SpeedGrade.
//!
//! # Format
//!
//! ```text
//! LUT_3D_SIZE 17
//! # optional comment
//! 0.0 0.0 0.0
//! 1.0 0.0 0.0
//! ...
//! ```
//!
//! - Red coordinate changes fastest, then green, then blue
//! - Values are floating point RGB triplets
//! - Lines starting with `#` are comments

use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;

use crate::error::{LutError, LutResult};
use crate::lut3d::Lut3D;

/// Parses an Iridas ITX file from a reader.
pub fn parse_itx<R: Read>(reader: R) -> LutResult<Lut3D> {
    let reader = BufReader::new(reader);
    let mut size = 0usize;
    let mut data: Vec<[f32; 3]> = Vec::new();
    let mut in_data = false;

    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let lower = trimmed.to_lowercase();

        // Parse LUT_3D_SIZE header
        if lower.starts_with("lut_3d_size") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() != 2 {
                return Err(LutError::ParseError(format!(
                    "malformed LUT_3D_SIZE at line {}",
                    line_num + 1
                )));
            }
            size = parts[1].parse().map_err(|_| {
                LutError::ParseError(format!("invalid size at line {}", line_num + 1))
            })?;
            data.reserve(size * size * size);
            in_data = true;
            continue;
        }

        // Parse data triplets
        if in_data {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() != 3 {
                return Err(LutError::ParseError(format!(
                    "expected 3 values at line {}, got {}",
                    line_num + 1,
                    parts.len()
                )));
            }

            let r: f32 = parts[0].parse().map_err(|_| {
                LutError::ParseError(format!("invalid float at line {}", line_num + 1))
            })?;
            let g: f32 = parts[1].parse().map_err(|_| {
                LutError::ParseError(format!("invalid float at line {}", line_num + 1))
            })?;
            let b: f32 = parts[2].parse().map_err(|_| {
                LutError::ParseError(format!("invalid float at line {}", line_num + 1))
            })?;

            data.push([r, g, b]);
        }
    }

    if size == 0 {
        return Err(LutError::ParseError("no LUT_3D_SIZE found".into()));
    }

    let expected = size * size * size;
    if data.len() != expected {
        return Err(LutError::ParseError(format!(
            "expected {} entries, got {}",
            expected,
            data.len()
        )));
    }

    // ITX uses red-fastest order, same as our internal format
    Lut3D::from_data(data, size)
}

/// Reads an Iridas ITX file from disk.
pub fn read_itx<P: AsRef<Path>>(path: P) -> LutResult<Lut3D> {
    let file = std::fs::File::open(path.as_ref())?;
    parse_itx(file)
}

/// Writes a 3D LUT to Iridas ITX format.
pub fn write_itx<W: Write>(writer: &mut W, lut: &Lut3D) -> LutResult<()> {
    writeln!(writer, "LUT_3D_SIZE {}", lut.size)?;

    // Write data in red-fastest order
    for entry in &lut.data {
        writeln!(writer, "{:.6} {:.6} {:.6}", entry[0], entry[1], entry[2])?;
    }

    Ok(())
}

/// Writes a 3D LUT to an Iridas ITX file.
pub fn write_itx_file<P: AsRef<Path>>(path: P, lut: &Lut3D) -> LutResult<()> {
    let mut file = std::fs::File::create(path.as_ref())?;
    write_itx(&mut file, lut)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_itx() {
        let data = r#"LUT_3D_SIZE 2
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
"#;
        let lut = parse_itx(data.as_bytes()).unwrap();
        assert_eq!(lut.size, 2);
        assert_eq!(lut.data.len(), 8);

        // Check corners
        assert_eq!(lut.data[0], [0.0, 0.0, 0.0]); // black
        assert_eq!(lut.data[7], [1.0, 1.0, 1.0]); // white
    }

    #[test]
    fn parse_with_comments() {
        let data = r#"# This is a comment
LUT_3D_SIZE 2
# Another comment
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
"#;
        let lut = parse_itx(data.as_bytes()).unwrap();
        assert_eq!(lut.size, 2);
    }

    #[test]
    fn roundtrip() {
        let original = Lut3D::identity(4);
        let mut buf = Vec::new();
        write_itx(&mut buf, &original).unwrap();

        let parsed = parse_itx(&buf[..]).unwrap();
        assert_eq!(parsed.size, 4);
        assert_eq!(parsed.data.len(), 64);

        // Check identity
        for (i, entry) in parsed.data.iter().enumerate() {
            let orig = original.data[i];
            assert!((entry[0] - orig[0]).abs() < 1e-5);
            assert!((entry[1] - orig[1]).abs() < 1e-5);
            assert!((entry[2] - orig[2]).abs() < 1e-5);
        }
    }

    #[test]
    fn error_missing_size() {
        let data = "0.0 0.0 0.0\n1.0 1.0 1.0\n";
        let result = parse_itx(data.as_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn error_wrong_count() {
        let data = r#"LUT_3D_SIZE 2
0.0 0.0 0.0
1.0 0.0 0.0
"#;
        let result = parse_itx(data.as_bytes());
        assert!(result.is_err());
    }
}
