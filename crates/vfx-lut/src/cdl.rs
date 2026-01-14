//! ASC CDL file format parser and writer.
//!
//! Supports three ASC CDL file formats:
//! - `.cc` - Single ColorCorrection
//! - `.ccc` - ColorCorrectionCollection (multiple CCs)
//! - `.cdl` - ColorDecisionList (CCs wrapped in ColorDecisions)
//!
//! # Example
//!
//! ```rust,no_run
//! use vfx_lut::cdl::{read_cc, read_ccc, ColorCorrection};
//! use std::path::Path;
//!
//! // Read a single CC
//! let cc = read_cc(Path::new("grade.cc")).unwrap();
//! println!("Slope: {:?}", cc.slope);
//!
//! // Read a CCC collection
//! let ccc = read_ccc(Path::new("grades.ccc")).unwrap();
//! for cc in &ccc.corrections {
//!     println!("ID: {:?}", cc.id);
//! }
//! ```
//!
//! # References
//!
//! - ASC CDL v1.01 Specification
//! - OpenColorIO CDLParser

use vfx_core::luminance_rec709;

use crate::{LutError, LutResult};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

// ============================================================================
// Data structures
// ============================================================================

/// A single ASC Color Correction (CC).
#[derive(Debug, Clone, PartialEq)]
pub struct ColorCorrection {
    /// Unique identifier for this correction.
    pub id: Option<String>,
    /// Description text (can have multiple).
    pub descriptions: Vec<String>,
    /// Input description.
    pub input_description: Option<String>,
    /// Viewing description.
    pub viewing_description: Option<String>,
    /// Slope (multiply) per channel [R, G, B].
    pub slope: [f32; 3],
    /// Offset (add) per channel [R, G, B].
    pub offset: [f32; 3],
    /// Power (gamma) per channel [R, G, B].
    pub power: [f32; 3],
    /// Saturation adjustment (1.0 = no change).
    pub saturation: f32,
}

impl Default for ColorCorrection {
    fn default() -> Self {
        Self {
            id: None,
            descriptions: Vec::new(),
            input_description: None,
            viewing_description: None,
            slope: [1.0, 1.0, 1.0],
            offset: [0.0, 0.0, 0.0],
            power: [1.0, 1.0, 1.0],
            saturation: 1.0,
        }
    }
}

impl ColorCorrection {
    /// Creates a new ColorCorrection with given SOP values.
    pub fn new(slope: [f32; 3], offset: [f32; 3], power: [f32; 3]) -> Self {
        Self { slope, offset, power, ..Default::default() }
    }

    /// Creates a new ColorCorrection with id and SOP values.
    pub fn with_id(id: &str, slope: [f32; 3], offset: [f32; 3], power: [f32; 3]) -> Self {
        Self { id: Some(id.to_string()), slope, offset, power, ..Default::default() }
    }

    /// Applies CDL to RGB values in-place.
    pub fn apply(&self, rgb: &mut [f32; 3]) {
        for i in 0..3 {
            let v = rgb[i] * self.slope[i] + self.offset[i];
            rgb[i] = v.max(0.0).powf(self.power[i]);
        }
        if (self.saturation - 1.0).abs() > 1e-6 {
            let luma = luminance_rec709(*rgb);
            for v in rgb.iter_mut() {
                *v = luma + (*v - luma) * self.saturation;
            }
        }
    }

    /// Applies CDL with clamping to [0, 1].
    pub fn apply_clamped(&self, rgb: &mut [f32; 3]) {
        self.apply(rgb);
        for v in rgb.iter_mut() {
            *v = v.clamp(0.0, 1.0);
        }
    }
}

/// A ColorDecision containing a ColorCorrection and optional media reference.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColorDecision {
    /// Description text.
    pub descriptions: Vec<String>,
    /// Input description.
    pub input_description: Option<String>,
    /// Viewing description.
    pub viewing_description: Option<String>,
    /// Media reference path.
    pub media_ref: Option<String>,
    /// The color correction.
    pub correction: ColorCorrection,
}

/// A ColorCorrectionCollection (CCC file).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColorCorrectionCollection {
    /// Description text.
    pub descriptions: Vec<String>,
    /// Input description.
    pub input_description: Option<String>,
    /// Viewing description.
    pub viewing_description: Option<String>,
    /// List of color corrections.
    pub corrections: Vec<ColorCorrection>,
}

impl ColorCorrectionCollection {
    /// Finds a ColorCorrection by id.
    pub fn find(&self, id: &str) -> Option<&ColorCorrection> {
        self.corrections.iter().find(|cc| cc.id.as_deref() == Some(id))
    }

    /// Gets the first ColorCorrection (if any).
    pub fn first(&self) -> Option<&ColorCorrection> {
        self.corrections.first()
    }
}

/// A ColorDecisionList (CDL file).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColorDecisionList {
    /// Description text.
    pub descriptions: Vec<String>,
    /// Input description.
    pub input_description: Option<String>,
    /// Viewing description.
    pub viewing_description: Option<String>,
    /// List of color decisions.
    pub decisions: Vec<ColorDecision>,
}

impl ColorDecisionList {
    /// Finds a ColorCorrection by id.
    pub fn find(&self, id: &str) -> Option<&ColorCorrection> {
        self.decisions.iter()
            .find(|cd| cd.correction.id.as_deref() == Some(id))
            .map(|cd| &cd.correction)
    }

    /// Converts to a ColorCorrectionCollection.
    pub fn to_collection(&self) -> ColorCorrectionCollection {
        ColorCorrectionCollection {
            descriptions: self.descriptions.clone(),
            input_description: self.input_description.clone(),
            viewing_description: self.viewing_description.clone(),
            corrections: self.decisions.iter().map(|cd| cd.correction.clone()).collect(),
        }
    }
}

// ============================================================================
// Parsing helpers
// ============================================================================

fn parse_rgb(s: &str) -> LutResult<[f32; 3]> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() != 3 {
        return Err(LutError::ParseError(format!("Expected 3 values, got {}", parts.len())));
    }
    Ok([
        parts[0].parse().map_err(|e| LutError::ParseError(format!("Invalid R: {}", e)))?,
        parts[1].parse().map_err(|e| LutError::ParseError(format!("Invalid G: {}", e)))?,
        parts[2].parse().map_err(|e| LutError::ParseError(format!("Invalid B: {}", e)))?,
    ])
}

fn get_attr(e: &quick_xml::events::BytesStart, key: &[u8]) -> Option<String> {
    e.attributes().flatten().find(|a| a.key.as_ref() == key)
        .map(|a| String::from_utf8_lossy(&a.value).to_string())
        .filter(|s| !s.is_empty())
}

// ============================================================================
// CC Parser
// ============================================================================

/// Reads a CC file (single ColorCorrection).
pub fn read_cc(path: &Path) -> LutResult<ColorCorrection> {
    let file = File::open(path)?;
    parse_cc(BufReader::new(file))
}

/// Parses a CC from a reader.
pub fn parse_cc<R: BufRead>(reader: R) -> LutResult<ColorCorrection> {
    let mut xml = Reader::from_reader(reader);
    xml.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut cc = ColorCorrection::default();
    let mut text = String::new();
    
    // Track hierarchy with a stack
    let mut stack: Vec<String> = Vec::new();

    loop {
        match xml.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "ColorCorrection" {
                    cc.id = get_attr(&e, b"id");
                }
                stack.push(name);
                text.clear();
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let parent = stack.get(stack.len().saturating_sub(2)).map(|s| s.as_str());
                
                match name.as_str() {
                    "Slope" => cc.slope = parse_rgb(&text)?,
                    "Offset" => cc.offset = parse_rgb(&text)?,
                    "Power" => cc.power = parse_rgb(&text)?,
                    "Saturation" => {
                        cc.saturation = text.trim().parse()
                            .map_err(|e| LutError::ParseError(format!("Invalid saturation: {}", e)))?;
                    }
                    "Description" if parent == Some("ColorCorrection") => {
                        cc.descriptions.push(text.trim().to_string());
                    }
                    "InputDescription" => cc.input_description = Some(text.trim().to_string()),
                    "ViewingDescription" => cc.viewing_description = Some(text.trim().to_string()),
                    _ => {}
                }
                stack.pop();
            }
            Ok(Event::Text(e)) => {
                text.push_str(&e.decode().unwrap_or_default());
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(LutError::ParseError(format!("XML error: {}", e))),
            _ => {}
        }
        buf.clear();
    }

    Ok(cc)
}

// ============================================================================
// CCC Parser
// ============================================================================

/// Reads a CCC file (ColorCorrectionCollection).
pub fn read_ccc(path: &Path) -> LutResult<ColorCorrectionCollection> {
    let file = File::open(path)?;
    parse_ccc(BufReader::new(file))
}

/// Parses a CCC from a reader.
pub fn parse_ccc<R: BufRead>(reader: R) -> LutResult<ColorCorrectionCollection> {
    let mut xml = Reader::from_reader(reader);
    xml.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut ccc = ColorCorrectionCollection::default();
    let mut current_cc = ColorCorrection::default();
    let mut text = String::new();
    let mut stack: Vec<String> = Vec::new();

    loop {
        match xml.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "ColorCorrection" {
                    current_cc = ColorCorrection::default();
                    current_cc.id = get_attr(&e, b"id");
                }
                stack.push(name);
                text.clear();
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let parent = stack.get(stack.len().saturating_sub(2)).map(|s| s.as_str());
                let in_cc = stack.iter().any(|s| s == "ColorCorrection");
                
                match name.as_str() {
                    "Slope" => current_cc.slope = parse_rgb(&text)?,
                    "Offset" => current_cc.offset = parse_rgb(&text)?,
                    "Power" => current_cc.power = parse_rgb(&text)?,
                    "Saturation" => {
                        current_cc.saturation = text.trim().parse()
                            .map_err(|e| LutError::ParseError(format!("Invalid saturation: {}", e)))?;
                    }
                    "Description" => {
                        if in_cc && parent == Some("ColorCorrection") {
                            current_cc.descriptions.push(text.trim().to_string());
                        } else if parent == Some("ColorCorrectionCollection") {
                            ccc.descriptions.push(text.trim().to_string());
                        }
                    }
                    "ColorCorrection" => {
                        ccc.corrections.push(current_cc.clone());
                    }
                    _ => {}
                }
                stack.pop();
            }
            Ok(Event::Text(e)) => {
                text.push_str(&e.decode().unwrap_or_default());
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(LutError::ParseError(format!("XML error: {}", e))),
            _ => {}
        }
        buf.clear();
    }

    Ok(ccc)
}

// ============================================================================
// CDL Parser
// ============================================================================

/// Reads a CDL file (ColorDecisionList).
pub fn read_cdl(path: &Path) -> LutResult<ColorDecisionList> {
    let file = File::open(path)?;
    parse_cdl(BufReader::new(file))
}

/// Parses a CDL from a reader.
pub fn parse_cdl<R: BufRead>(reader: R) -> LutResult<ColorDecisionList> {
    let mut xml = Reader::from_reader(reader);
    xml.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut cdl = ColorDecisionList::default();
    let mut current_decision = ColorDecision::default();
    let mut current_cc = ColorCorrection::default();
    let mut text = String::new();
    let mut stack: Vec<String> = Vec::new();

    loop {
        match xml.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "ColorDecision" => current_decision = ColorDecision::default(),
                    "ColorCorrection" => {
                        current_cc = ColorCorrection::default();
                        current_cc.id = get_attr(&e, b"id");
                    }
                    _ => {}
                }
                stack.push(name);
                text.clear();
            }
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "MediaRef" {
                    current_decision.media_ref = get_attr(&e, b"ref");
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                
                match name.as_str() {
                    "Slope" => current_cc.slope = parse_rgb(&text)?,
                    "Offset" => current_cc.offset = parse_rgb(&text)?,
                    "Power" => current_cc.power = parse_rgb(&text)?,
                    "Saturation" => {
                        current_cc.saturation = text.trim().parse()
                            .map_err(|e| LutError::ParseError(format!("Invalid saturation: {}", e)))?;
                    }
                    "ColorCorrection" => {
                        current_decision.correction = current_cc.clone();
                    }
                    "ColorDecision" => {
                        cdl.decisions.push(current_decision.clone());
                    }
                    _ => {}
                }
                stack.pop();
            }
            Ok(Event::Text(e)) => {
                text.push_str(&e.decode().unwrap_or_default());
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(LutError::ParseError(format!("XML error: {}", e))),
            _ => {}
        }
        buf.clear();
    }

    Ok(cdl)
}

/// Auto-detect format and read any CDL file (.cc, .ccc, .cdl).
pub fn read_any(path: &Path) -> LutResult<ColorCorrectionCollection> {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "cc" => {
            let cc = read_cc(path)?;
            Ok(ColorCorrectionCollection { corrections: vec![cc], ..Default::default() })
        }
        "ccc" => read_ccc(path),
        "cdl" => Ok(read_cdl(path)?.to_collection()),
        _ => Err(LutError::ParseError(format!("Unknown CDL extension: {}", ext))),
    }
}

// ============================================================================
// Writing
// ============================================================================

/// Writes a ColorCorrection to a CC file.
pub fn write_cc(path: &Path, cc: &ColorCorrection) -> LutResult<()> {
    let mut file = File::create(path)?;
    write_cc_to(&mut file, cc)
}

/// Writes a ColorCorrection to a writer.
pub fn write_cc_to<W: Write>(w: &mut W, cc: &ColorCorrection) -> LutResult<()> {
    let id = cc.id.as_deref().unwrap_or("");
    writeln!(w, r#"<ColorCorrection id="{}">"#, id)?;
    writeln!(w, "  <SOPNode>")?;
    writeln!(w, "    <Slope>{} {} {}</Slope>", cc.slope[0], cc.slope[1], cc.slope[2])?;
    writeln!(w, "    <Offset>{} {} {}</Offset>", cc.offset[0], cc.offset[1], cc.offset[2])?;
    writeln!(w, "    <Power>{} {} {}</Power>", cc.power[0], cc.power[1], cc.power[2])?;
    writeln!(w, "  </SOPNode>")?;
    writeln!(w, "  <SatNode>")?;
    writeln!(w, "    <Saturation>{}</Saturation>", cc.saturation)?;
    writeln!(w, "  </SatNode>")?;
    writeln!(w, "</ColorCorrection>")?;
    Ok(())
}

/// Writes a ColorCorrectionCollection to a CCC file.
pub fn write_ccc(path: &Path, ccc: &ColorCorrectionCollection) -> LutResult<()> {
    let mut file = File::create(path)?;
    write_ccc_to(&mut file, ccc)
}

/// Writes a ColorCorrectionCollection to a writer.
pub fn write_ccc_to<W: Write>(w: &mut W, ccc: &ColorCorrectionCollection) -> LutResult<()> {
    writeln!(w, r#"<ColorCorrectionCollection xmlns="urn:ASC:CDL:v1.01">"#)?;
    for cc in &ccc.corrections {
        let id = cc.id.as_deref().unwrap_or("");
        writeln!(w)?;
        writeln!(w, r#"  <ColorCorrection id="{}">"#, id)?;
        writeln!(w, "    <SOPNode>")?;
        writeln!(w, "      <Slope>{} {} {}</Slope>", cc.slope[0], cc.slope[1], cc.slope[2])?;
        writeln!(w, "      <Offset>{} {} {}</Offset>", cc.offset[0], cc.offset[1], cc.offset[2])?;
        writeln!(w, "      <Power>{} {} {}</Power>", cc.power[0], cc.power[1], cc.power[2])?;
        writeln!(w, "    </SOPNode>")?;
        writeln!(w, "    <SatNode>")?;
        writeln!(w, "      <Saturation>{}</Saturation>", cc.saturation)?;
        writeln!(w, "    </SatNode>")?;
        writeln!(w, "  </ColorCorrection>")?;
    }
    writeln!(w)?;
    writeln!(w, "</ColorCorrectionCollection>")?;
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const CC_SAMPLE: &str = r#"<ColorCorrection id="foo">
  <SOPNode>
    <Description>this is a description</Description>
    <Slope>1.1 1.2 1.3</Slope>
    <Offset>2.1 2.2 2.3</Offset>
    <Power>3.1 3.2 3.3</Power>
  </SOPNode>
  <SatNode>
    <Saturation>0.7</Saturation>
  </SatNode>
</ColorCorrection>"#;

    const CCC_SAMPLE: &str = r#"<ColorCorrectionCollection xmlns="urn:ASC:CDL:v1.01">
  <ColorCorrection id="cc0001">
    <SOPNode>
      <Slope>1.0 1.0 0.9</Slope>
      <Offset>-0.03 -0.02 0</Offset>
      <Power>1.25 1 1</Power>
    </SOPNode>
    <SatNode>
      <Saturation>1.7</Saturation>
    </SatNode>
  </ColorCorrection>
  <ColorCorrection id="cc0002">
    <SOPNode>
      <Slope>0.9 0.7 0.6</Slope>
      <Offset>0.1 0.1 0.1</Offset>
      <Power>0.9 0.9 0.9</Power>
    </SOPNode>
    <SatNode>
      <Saturation>0.7</Saturation>
    </SatNode>
  </ColorCorrection>
</ColorCorrectionCollection>"#;

    const CDL_SAMPLE: &str = r#"<ColorDecisionList xmlns="urn:ASC:CDL:v1.01">
  <ColorDecision>
    <MediaRef ref="some/path.dpx"/>
    <ColorCorrection id="shot001">
      <SOPNode>
        <Slope>1.1 1.0 0.9</Slope>
        <Offset>0.0 0.01 0.0</Offset>
        <Power>1.0 1.0 1.0</Power>
      </SOPNode>
      <SatNode>
        <Saturation>1.2</Saturation>
      </SatNode>
    </ColorCorrection>
  </ColorDecision>
</ColorDecisionList>"#;

    #[test]
    fn parse_cc_sample() {
        let cc = parse_cc(Cursor::new(CC_SAMPLE)).unwrap();
        assert_eq!(cc.id, Some("foo".to_string()));
        assert_eq!(cc.slope, [1.1, 1.2, 1.3]);
        assert_eq!(cc.offset, [2.1, 2.2, 2.3]);
        assert_eq!(cc.power, [3.1, 3.2, 3.3]);
        assert!((cc.saturation - 0.7).abs() < 1e-6);
    }

    #[test]
    fn parse_ccc_sample() {
        let ccc = parse_ccc(Cursor::new(CCC_SAMPLE)).unwrap();
        assert_eq!(ccc.corrections.len(), 2);
        
        let cc1 = &ccc.corrections[0];
        assert_eq!(cc1.id, Some("cc0001".to_string()));
        assert_eq!(cc1.slope, [1.0, 1.0, 0.9]);
        assert!((cc1.saturation - 1.7).abs() < 1e-6);
        
        let cc2 = &ccc.corrections[1];
        assert_eq!(cc2.id, Some("cc0002".to_string()));
        assert_eq!(cc2.slope, [0.9, 0.7, 0.6]);
    }

    #[test]
    fn parse_cdl_sample() {
        let cdl = parse_cdl(Cursor::new(CDL_SAMPLE)).unwrap();
        assert_eq!(cdl.decisions.len(), 1);
        
        let decision = &cdl.decisions[0];
        assert_eq!(decision.media_ref, Some("some/path.dpx".to_string()));
        
        let cc = &decision.correction;
        assert_eq!(cc.id, Some("shot001".to_string()));
        assert_eq!(cc.slope, [1.1, 1.0, 0.9]);
        assert!((cc.saturation - 1.2).abs() < 1e-6);
    }

    #[test]
    fn cc_apply() {
        let cc = ColorCorrection {
            slope: [1.1, 1.0, 0.9],
            offset: [0.0, 0.01, 0.0],
            power: [1.0, 1.0, 1.0],
            saturation: 1.0,
            ..Default::default()
        };
        let mut rgb = [0.5, 0.5, 0.5];
        cc.apply(&mut rgb);
        assert!((rgb[0] - 0.55).abs() < 1e-6);
        assert!((rgb[1] - 0.51).abs() < 1e-6);
        assert!((rgb[2] - 0.45).abs() < 1e-6);
    }

    #[test]
    fn cc_apply_with_power() {
        let cc = ColorCorrection {
            power: [2.0, 2.0, 2.0],
            ..Default::default()
        };
        let mut rgb = [0.5, 0.5, 0.5];
        cc.apply(&mut rgb);
        assert!((rgb[0] - 0.25).abs() < 1e-6);
    }

    #[test]
    fn cc_apply_with_saturation() {
        let cc = ColorCorrection {
            saturation: 0.0,
            ..Default::default()
        };
        let mut rgb = [1.0, 0.5, 0.0];
        cc.apply(&mut rgb);
        let luma = luminance_rec709([1.0, 0.5, 0.0]);
        assert!((rgb[0] - luma).abs() < 1e-6);
        assert!((rgb[1] - luma).abs() < 1e-6);
        assert!((rgb[2] - luma).abs() < 1e-6);
    }

    #[test]
    fn cc_roundtrip() {
        let original = ColorCorrection {
            id: Some("test".to_string()),
            slope: [1.1, 1.2, 1.3],
            offset: [0.01, 0.02, 0.03],
            power: [0.9, 1.0, 1.1],
            saturation: 0.8,
            ..Default::default()
        };
        let mut buf = Vec::new();
        write_cc_to(&mut buf, &original).unwrap();
        let parsed = parse_cc(Cursor::new(buf)).unwrap();
        assert_eq!(parsed.id, original.id);
        assert_eq!(parsed.slope, original.slope);
        assert_eq!(parsed.offset, original.offset);
        assert_eq!(parsed.power, original.power);
        assert!((parsed.saturation - original.saturation).abs() < 1e-6);
    }

    #[test]
    fn ccc_find() {
        let ccc = parse_ccc(Cursor::new(CCC_SAMPLE)).unwrap();
        let cc1 = ccc.find("cc0001").unwrap();
        assert_eq!(cc1.slope, [1.0, 1.0, 0.9]);
        let cc2 = ccc.find("cc0002").unwrap();
        assert_eq!(cc2.slope, [0.9, 0.7, 0.6]);
        assert!(ccc.find("nonexistent").is_none());
    }

    #[test]
    fn cdl_to_collection() {
        let cdl = parse_cdl(Cursor::new(CDL_SAMPLE)).unwrap();
        let ccc = cdl.to_collection();
        assert_eq!(ccc.corrections.len(), 1);
        assert_eq!(ccc.corrections[0].id, Some("shot001".to_string()));
    }

    #[test]
    fn cc_identity() {
        let cc = ColorCorrection::default();
        let mut rgb = [0.5, 0.3, 0.7];
        let original = rgb;
        cc.apply(&mut rgb);
        assert!((rgb[0] - original[0]).abs() < 1e-6);
        assert!((rgb[1] - original[1]).abs() < 1e-6);
        assert!((rgb[2] - original[2]).abs() < 1e-6);
    }

    #[test]
    fn parse_negative_offset() {
        let xml = r#"<ColorCorrection id="test">
  <SOPNode>
    <Slope>1.0 1.0 0.9</Slope>
    <Offset>-.03 -2e-2 0</Offset>
    <Power>1.25 1 1e0</Power>
  </SOPNode>
  <SatNode>
    <Saturation>1.0</Saturation>
  </SatNode>
</ColorCorrection>"#;
        let cc = parse_cc(Cursor::new(xml)).unwrap();
        assert!((cc.offset[0] - (-0.03)).abs() < 1e-6);
        assert!((cc.offset[1] - (-0.02)).abs() < 1e-6);
        assert!((cc.offset[2] - 0.0).abs() < 1e-6);
        assert!((cc.power[0] - 1.25).abs() < 1e-6);
    }
}
