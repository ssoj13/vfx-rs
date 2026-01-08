//! Cinespace (CSP) LUT file format parser and writer.
//!
//! CSP is a text-based LUT format used by Rising Sun Research Cinespace.
//! It supports:
//! - 1D LUTs (per-channel curves)
//! - 3D LUTs (color cubes)
//! - Pre-LUT/Shaper (input transform before the main LUT)
//! - Metadata
//!
//! # Format Structure
//!
//! ```text
//! CSPLUTV100
//! 1D or 3D
//!
//! BEGIN METADATA
//! <metadata>
//! END METADATA
//!
//! <prelut_r_count>
//! <input_samples_r>
//! <output_samples_r>
//! <prelut_g_count>
//! <input_samples_g>
//! <output_samples_g>
//! <prelut_b_count>
//! <input_samples_b>
//! <output_samples_b>
//!
//! <lut_size> (for 1D) or <size_r> <size_g> <size_b> (for 3D)
//! <r g b>
//! ...
//! ```
//!
//! # References
//!
//! - OpenColorIO FileFormatCSP.cpp

use crate::{Lut1D, Lut3D, LutError, LutResult};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

/// Pre-LUT channel data (shaper curve).
#[derive(Debug, Clone, PartialEq)]
pub struct PreLutChannel {
    /// Input sample positions.
    pub inputs: Vec<f32>,
    /// Output sample values.
    pub outputs: Vec<f32>,
}

impl PreLutChannel {
    /// Creates an identity pre-LUT (pass-through).
    pub fn identity() -> Self {
        Self {
            inputs: vec![0.0, 1.0],
            outputs: vec![0.0, 1.0],
        }
    }

    /// Interpolates a value through this pre-LUT channel.
    pub fn apply(&self, x: f32) -> f32 {
        if self.inputs.len() < 2 {
            return x;
        }

        // Clamp to range
        if x <= self.inputs[0] {
            return self.outputs[0];
        }
        if x >= *self.inputs.last().unwrap() {
            return *self.outputs.last().unwrap();
        }

        // Find segment
        let mut i = 0;
        while i < self.inputs.len() - 1 && x > self.inputs[i + 1] {
            i += 1;
        }

        // Linear interpolation
        let t = (x - self.inputs[i]) / (self.inputs[i + 1] - self.inputs[i]);
        self.outputs[i] + t * (self.outputs[i + 1] - self.outputs[i])
    }
}

/// Pre-LUT (shaper) for all three channels.
#[derive(Debug, Clone, PartialEq)]
pub struct PreLut {
    /// Red channel pre-LUT.
    pub red: PreLutChannel,
    /// Green channel pre-LUT.
    pub green: PreLutChannel,
    /// Blue channel pre-LUT.
    pub blue: PreLutChannel,
}

impl PreLut {
    /// Creates an identity pre-LUT.
    pub fn identity() -> Self {
        Self {
            red: PreLutChannel::identity(),
            green: PreLutChannel::identity(),
            blue: PreLutChannel::identity(),
        }
    }

    /// Checks if this is an identity pre-LUT.
    pub fn is_identity(&self) -> bool {
        self.red.inputs == vec![0.0, 1.0] && self.red.outputs == vec![0.0, 1.0] &&
        self.green.inputs == vec![0.0, 1.0] && self.green.outputs == vec![0.0, 1.0] &&
        self.blue.inputs == vec![0.0, 1.0] && self.blue.outputs == vec![0.0, 1.0]
    }

    /// Applies the pre-LUT to RGB values.
    pub fn apply(&self, rgb: &mut [f32; 3]) {
        rgb[0] = self.red.apply(rgb[0]);
        rgb[1] = self.green.apply(rgb[1]);
        rgb[2] = self.blue.apply(rgb[2]);
    }
}

/// CSP file contents (can be 1D or 3D).
#[derive(Debug, Clone)]
pub struct CspFile {
    /// Metadata string (if present).
    pub metadata: Option<String>,
    /// Pre-LUT/shaper.
    pub prelut: PreLut,
    /// 1D LUT (if present).
    pub lut1d: Option<Lut1D>,
    /// 3D LUT (if present).
    pub lut3d: Option<Lut3D>,
}

impl CspFile {
    /// Creates an empty CSP file.
    pub fn new() -> Self {
        Self {
            metadata: None,
            prelut: PreLut::identity(),
            lut1d: None,
            lut3d: None,
        }
    }

    /// Creates a CSP file with a 3D LUT.
    pub fn with_3d(lut: Lut3D) -> Self {
        Self {
            metadata: None,
            prelut: PreLut::identity(),
            lut1d: None,
            lut3d: Some(lut),
        }
    }

    /// Creates a CSP file with a 1D LUT.
    pub fn with_1d(lut: Lut1D) -> Self {
        Self {
            metadata: None,
            prelut: PreLut::identity(),
            lut1d: Some(lut),
            lut3d: None,
        }
    }
}

impl Default for CspFile {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Parsing
// ============================================================================

/// Reads a CSP file.
pub fn read_csp(path: &Path) -> LutResult<CspFile> {
    let file = File::open(path)?;
    parse_csp(BufReader::new(file))
}

/// Parses a CSP file from a reader.
pub fn parse_csp<R: BufRead>(reader: R) -> LutResult<CspFile> {
    let mut lines: Vec<String> = reader.lines()
        .filter_map(|l| l.ok())
        .collect();
    
    if lines.is_empty() {
        return Err(LutError::ParseError("Empty CSP file".to_string()));
    }

    // Check header
    let header = lines.remove(0).trim().to_string();
    if header != "CSPLUTV100" {
        return Err(LutError::ParseError(format!("Invalid CSP header: {}", header)));
    }

    // Get LUT type
    let lut_type = skip_empty_get_next(&mut lines)?;
    let is_3d = match lut_type.as_str() {
        "3D" => true,
        "1D" => false,
        _ => return Err(LutError::ParseError(format!("Invalid LUT type: {}", lut_type))),
    };

    // Parse metadata (optional)
    let metadata = parse_metadata(&mut lines);

    // Parse pre-LUT channels
    let prelut_r = parse_prelut_channel(&mut lines)?;
    let prelut_g = parse_prelut_channel(&mut lines)?;
    let prelut_b = parse_prelut_channel(&mut lines)?;

    let prelut = PreLut {
        red: prelut_r,
        green: prelut_g,
        blue: prelut_b,
    };

    // Parse main LUT
    let (lut1d, lut3d) = if is_3d {
        let lut = parse_3d_lut(&mut lines)?;
        (None, Some(lut))
    } else {
        let lut = parse_1d_lut(&mut lines)?;
        (Some(lut), None)
    };

    Ok(CspFile {
        metadata,
        prelut,
        lut1d,
        lut3d,
    })
}

fn skip_empty_get_next(lines: &mut Vec<String>) -> LutResult<String> {
    while !lines.is_empty() {
        let line = lines.remove(0).trim().to_string();
        if !line.is_empty() {
            return Ok(line);
        }
    }
    Err(LutError::ParseError("Unexpected end of file".to_string()))
}

fn parse_metadata(lines: &mut Vec<String>) -> Option<String> {
    // Look for "BEGIN METADATA"
    while !lines.is_empty() {
        let line = lines[0].trim();
        if line.is_empty() {
            lines.remove(0);
            continue;
        }
        if line == "BEGIN METADATA" {
            lines.remove(0);
            break;
        }
        return None; // No metadata
    }

    // Collect until "END METADATA"
    let mut metadata = String::new();
    while !lines.is_empty() {
        let line = lines.remove(0);
        if line.trim() == "END METADATA" {
            break;
        }
        if !metadata.is_empty() {
            metadata.push('\n');
        }
        metadata.push_str(&line);
    }

    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

fn parse_prelut_channel(lines: &mut Vec<String>) -> LutResult<PreLutChannel> {
    let count_str = skip_empty_get_next(lines)?;
    let count: usize = count_str.parse()
        .map_err(|e| LutError::ParseError(format!("Invalid prelut count: {}", e)))?;

    let inputs_line = skip_empty_get_next(lines)?;
    let inputs: Vec<f32> = inputs_line.split_whitespace()
        .map(|s| s.parse::<f32>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| LutError::ParseError(format!("Invalid prelut inputs: {}", e)))?;

    let outputs_line = skip_empty_get_next(lines)?;
    let outputs: Vec<f32> = outputs_line.split_whitespace()
        .map(|s| s.parse::<f32>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| LutError::ParseError(format!("Invalid prelut outputs: {}", e)))?;

    if inputs.len() != count || outputs.len() != count {
        return Err(LutError::ParseError(format!(
            "Prelut count mismatch: expected {}, got inputs={}, outputs={}",
            count, inputs.len(), outputs.len()
        )));
    }

    Ok(PreLutChannel { inputs, outputs })
}

fn parse_3d_lut(lines: &mut Vec<String>) -> LutResult<Lut3D> {
    let dims_line = skip_empty_get_next(lines)?;
    let dims: Vec<usize> = dims_line.split_whitespace()
        .map(|s| s.parse::<usize>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| LutError::ParseError(format!("Invalid 3D LUT dimensions: {}", e)))?;

    if dims.len() != 3 {
        return Err(LutError::ParseError(format!("Expected 3 dimensions, got {}", dims.len())));
    }

    let size_r = dims[0];
    let size_g = dims[1];
    let size_b = dims[2];
    let total = size_r * size_g * size_b;

    // Parse LUT data
    let mut data: Vec<[f32; 3]> = Vec::with_capacity(total);
    for _ in 0..total {
        let line = skip_empty_get_next(lines)?;
        let values: Vec<f32> = line.split_whitespace()
            .map(|s| s.parse::<f32>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| LutError::ParseError(format!("Invalid LUT value: {}", e)))?;
        
        if values.len() != 3 {
            return Err(LutError::ParseError(format!("Expected 3 values per entry, got {}", values.len())));
        }
        data.push([values[0], values[1], values[2]]);
    }

    // CSP uses B-fastest order, but we need to reorder
    // File order: for r in 0..R { for g in 0..G { for b in 0..B { ... } } }
    // Our order should be the same (standard OCIO order)
    
    // For now, assume cube LUT (same size in all dimensions)
    if size_r != size_g || size_g != size_b {
        return Err(LutError::ParseError("Non-cubic CSP LUTs not yet supported".to_string()));
    }

    Lut3D::from_data(data, size_r)
}

fn parse_1d_lut(lines: &mut Vec<String>) -> LutResult<Lut1D> {
    let size_line = skip_empty_get_next(lines)?;
    let size: usize = size_line.parse()
        .map_err(|e| LutError::ParseError(format!("Invalid 1D LUT size: {}", e)))?;

    let mut red = Vec::with_capacity(size);
    let mut green = Vec::with_capacity(size);
    let mut blue = Vec::with_capacity(size);

    for _ in 0..size {
        let line = skip_empty_get_next(lines)?;
        let values: Vec<f32> = line.split_whitespace()
            .map(|s| s.parse::<f32>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| LutError::ParseError(format!("Invalid LUT value: {}", e)))?;
        
        if values.len() != 3 {
            return Err(LutError::ParseError(format!("Expected 3 values per entry, got {}", values.len())));
        }
        red.push(values[0]);
        green.push(values[1]);
        blue.push(values[2]);
    }

    Lut1D::from_rgb(red, green, blue, 0.0, 1.0)
}

// ============================================================================
// Writing
// ============================================================================

/// Writes a CSP 3D LUT file.
pub fn write_csp_3d(path: &Path, lut: &Lut3D) -> LutResult<()> {
    let mut file = File::create(path)?;
    write_csp_3d_to(&mut file, lut, None)
}

/// Writes a CSP 3D LUT to a writer.
pub fn write_csp_3d_to<W: Write>(w: &mut W, lut: &Lut3D, metadata: Option<&str>) -> LutResult<()> {
    writeln!(w, "CSPLUTV100")?;
    writeln!(w, "3D")?;
    writeln!(w)?;

    if let Some(meta) = metadata {
        writeln!(w, "BEGIN METADATA")?;
        writeln!(w, "{}", meta)?;
        writeln!(w, "END METADATA")?;
        writeln!(w)?;
    }

    // Identity pre-LUT
    for _ in 0..3 {
        writeln!(w, "2")?;
        writeln!(w, "0.0 1.0")?;
        writeln!(w, "0.0 1.0")?;
    }
    writeln!(w)?;

    let size = lut.size;
    writeln!(w, "{} {} {}", size, size, size)?;

    for r in 0..size {
        for g in 0..size {
            for b in 0..size {
                let idx = b * size * size + g * size + r;
                let rgb = lut.data[idx];
                writeln!(w, "{} {} {}", rgb[0], rgb[1], rgb[2])?;
            }
        }
    }

    Ok(())
}

/// Writes a CSP 1D LUT file.
pub fn write_csp_1d(path: &Path, lut: &Lut1D) -> LutResult<()> {
    let mut file = File::create(path)?;
    write_csp_1d_to(&mut file, lut, None)
}

/// Writes a CSP 1D LUT to a writer.
pub fn write_csp_1d_to<W: Write>(w: &mut W, lut: &Lut1D, metadata: Option<&str>) -> LutResult<()> {
    writeln!(w, "CSPLUTV100")?;
    writeln!(w, "1D")?;
    writeln!(w)?;

    if let Some(meta) = metadata {
        writeln!(w, "BEGIN METADATA")?;
        writeln!(w, "{}", meta)?;
        writeln!(w, "END METADATA")?;
        writeln!(w)?;
    }

    // Identity pre-LUT
    for _ in 0..3 {
        writeln!(w, "2")?;
        writeln!(w, "0.0 1.0")?;
        writeln!(w, "0.0 1.0")?;
    }
    writeln!(w)?;

    let size = lut.size();
    writeln!(w, "{}", size)?;

    for i in 0..size {
        let r = lut.r[i];
        let g = lut.g.as_ref().map(|v| v[i]).unwrap_or(r);
        let b = lut.b.as_ref().map(|v| v[i]).unwrap_or(r);
        writeln!(w, "{} {} {}", r, g, b)?;
    }

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const CSP_3D_SAMPLE: &str = r#"CSPLUTV100
3D

2
0.0 1.0
0.0 1.0
2
0.0 1.0
0.0 1.0
2
0.0 1.0
0.0 1.0

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

    const CSP_1D_SAMPLE: &str = r#"CSPLUTV100
1D

2
0.0 1.0
0.0 1.0
2
0.0 1.0
0.0 1.0
2
0.0 1.0
0.0 1.0

4
0.0 0.0 0.0
0.333 0.333 0.333
0.666 0.666 0.666
1.0 1.0 1.0
"#;

    #[test]
    fn parse_csp_3d() {
        let csp = parse_csp(Cursor::new(CSP_3D_SAMPLE)).unwrap();
        
        assert!(csp.lut3d.is_some());
        assert!(csp.lut1d.is_none());
        assert!(csp.prelut.is_identity());
        
        let lut = csp.lut3d.unwrap();
        assert_eq!(lut.size, 2);
        
        // Check corners (B-fastest order: idx = b*4 + g*2 + r)
        assert_eq!(lut.data[0], [0.0, 0.0, 0.0]);  // r=0, g=0, b=0
        assert_eq!(lut.data[1], [1.0, 0.0, 0.0]);  // r=1, g=0, b=0
        assert_eq!(lut.data[2], [0.0, 1.0, 0.0]);  // r=0, g=1, b=0
        assert_eq!(lut.data[7], [1.0, 1.0, 1.0]);  // r=1, g=1, b=1
    }

    #[test]
    fn parse_csp_1d() {
        let csp = parse_csp(Cursor::new(CSP_1D_SAMPLE)).unwrap();
        
        assert!(csp.lut1d.is_some());
        assert!(csp.lut3d.is_none());
        assert!(csp.prelut.is_identity());
        
        let lut = csp.lut1d.unwrap();
        assert_eq!(lut.size(), 4);
    }

    #[test]
    fn prelut_apply() {
        let prelut = PreLutChannel {
            inputs: vec![0.0, 0.5, 1.0],
            outputs: vec![0.0, 0.25, 1.0],
        };
        
        assert!((prelut.apply(0.0) - 0.0).abs() < 1e-6);
        assert!((prelut.apply(0.5) - 0.25).abs() < 1e-6);
        assert!((prelut.apply(1.0) - 1.0).abs() < 1e-6);
        assert!((prelut.apply(0.25) - 0.125).abs() < 1e-6);
    }

    #[test]
    fn csp_3d_roundtrip() {
        let original = Lut3D::identity(4);
        
        let mut buf = Vec::new();
        write_csp_3d_to(&mut buf, &original, Some("test")).unwrap();
        
        let csp = parse_csp(Cursor::new(buf)).unwrap();
        assert!(csp.lut3d.is_some());
        assert_eq!(csp.metadata, Some("test".to_string()));
        
        let parsed = csp.lut3d.unwrap();
        assert_eq!(parsed.size, original.size);
    }

    #[test]
    fn parse_with_metadata() {
        let csp_with_meta = r#"CSPLUTV100
3D

BEGIN METADATA
test metadata
END METADATA

2
0.0 1.0
0.0 1.0
2
0.0 1.0
0.0 1.0
2
0.0 1.0
0.0 1.0

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
        let csp = parse_csp(Cursor::new(csp_with_meta)).unwrap();
        assert_eq!(csp.metadata, Some("test metadata".to_string()));
    }
}
