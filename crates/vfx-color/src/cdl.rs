//! ASC Color Decision List (CDL) support.
//!
//! The ASC CDL is an industry-standard format for exchanging basic color
//! correction information between systems. It defines a simple but powerful
//! mathematical model for primary color correction.
//!
//! # CDL Operations
//!
//! The CDL formula applies Slope, Offset, Power (SOP) followed by Saturation:
//!
//! ```text
//! // Per-channel SOP:
//! out = clamp((in * slope + offset) ^ power, 0, 1)
//!
//! // Global saturation:
//! luma = 0.2126 * R + 0.7152 * G + 0.0722 * B
//! out = luma + (out - luma) * saturation
//! ```
//!
//! # File Formats
//!
//! - `.cc` - Single ColorCorrection (XML)
//! - `.ccc` - ColorCorrectionCollection (XML, multiple corrections)
//! - `.cdl` - ColorDecisionList (XML, with media references)
//!
//! # Example
//!
//! ```rust
//! use vfx_color::cdl::Cdl;
//!
//! // Create a simple color correction
//! let cdl = Cdl::new()
//!     .with_slope([1.1, 1.0, 0.9])
//!     .with_offset([0.01, 0.0, -0.01])
//!     .with_power([1.0, 1.0, 1.0])
//!     .with_saturation(1.2);
//!
//! // Apply to RGB
//! let mut rgb = [0.5, 0.5, 0.5];
//! cdl.apply(&mut rgb);
//! ```
//!
//! # References
//!
//! - [ASC CDL Specification](https://theasc.com/asc/science-and-technology/asc-cdl)
//! - [ACESclip Container](https://docs.acescentral.com/)

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use vfx_core::{REC709_LUMA_B, REC709_LUMA_G, REC709_LUMA_R};

use crate::sse_math::fast_pow;
use crate::{ColorError, ColorResult};

/// ASC Color Decision List correction parameters.
///
/// Represents a single color correction with Slope, Offset, Power (SOP)
/// and Saturation values.
///
/// # Example
///
/// ```rust
/// use vfx_color::cdl::Cdl;
///
/// let cdl = Cdl::new()
///     .with_slope([1.2, 1.0, 0.8])
///     .with_offset([0.0, 0.01, 0.0])
///     .with_power([1.0, 1.0, 1.0])
///     .with_saturation(1.1);
///
/// // Apply to a pixel
/// let mut pixel = [0.5, 0.4, 0.3];
/// cdl.apply(&mut pixel);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Cdl {
    /// Unique identifier for this correction.
    pub id: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Slope (multiply) per channel [R, G, B].
    pub slope: [f32; 3],
    /// Offset (add) per channel [R, G, B].
    pub offset: [f32; 3],
    /// Power (gamma) per channel [R, G, B].
    pub power: [f32; 3],
    /// Saturation adjustment (1.0 = no change).
    pub saturation: f32,
}

impl Default for Cdl {
    fn default() -> Self {
        Self::new()
    }
}

impl Cdl {
    /// Creates a new identity CDL (no color change).
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_color::cdl::Cdl;
    ///
    /// let cdl = Cdl::new();
    /// assert_eq!(cdl.slope, [1.0, 1.0, 1.0]);
    /// assert_eq!(cdl.offset, [0.0, 0.0, 0.0]);
    /// assert_eq!(cdl.power, [1.0, 1.0, 1.0]);
    /// assert_eq!(cdl.saturation, 1.0);
    /// ```
    pub fn new() -> Self {
        Self {
            id: None,
            description: None,
            slope: [1.0, 1.0, 1.0],
            offset: [0.0, 0.0, 0.0],
            power: [1.0, 1.0, 1.0],
            saturation: 1.0,
        }
    }

    /// Sets the correction ID.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Sets the slope values.
    pub fn with_slope(mut self, slope: [f32; 3]) -> Self {
        self.slope = slope;
        self
    }

    /// Sets the offset values.
    pub fn with_offset(mut self, offset: [f32; 3]) -> Self {
        self.offset = offset;
        self
    }

    /// Sets the power values.
    pub fn with_power(mut self, power: [f32; 3]) -> Self {
        self.power = power;
        self
    }

    /// Sets the saturation value.
    pub fn with_saturation(mut self, saturation: f32) -> Self {
        self.saturation = saturation;
        self
    }

    /// Check if this CDL is identity (no-op).
    #[inline]
    pub fn is_identity(&self) -> bool {
        self.slope == [1.0, 1.0, 1.0]
            && self.offset == [0.0, 0.0, 0.0]
            && self.power == [1.0, 1.0, 1.0]
            && (self.saturation - 1.0).abs() < 1e-6
    }

    /// Applies the CDL correction to an RGB pixel in-place.
    ///
    /// Uses the standard ASC CDL formula with clamping.
    ///
    /// # Arguments
    ///
    /// * `rgb` - RGB values to transform
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_color::cdl::Cdl;
    ///
    /// let cdl = Cdl::new().with_slope([1.1, 1.0, 0.9]);
    /// let mut pixel = [0.5, 0.5, 0.5];
    /// cdl.apply(&mut pixel);
    /// ```
    #[inline]
    pub fn apply(&self, rgb: &mut [f32; 3]) {
        // ASC CDL v1.2 forward with clamp (matches OCIO CDL_V1_2_FWD)
        // Order: Slope -> Offset -> Clamp [0,1] -> Power -> Saturation -> Clamp [0,1]
        for i in 0..3 {
            let v = rgb[i] * self.slope[i] + self.offset[i];
            // Clamp to [0,1] BEFORE power (ASC CDL spec)
            let clamped = v.clamp(0.0, 1.0);
            // Skip pow when power == 1.0 to avoid numerical error from log2->mul->exp2 chain
            rgb[i] = if self.power[i] == 1.0 {
                clamped
            } else {
                fast_pow(clamped, self.power[i])
            };
        }

        // Saturation (Rec. 709 luma weights)
        // OCIO-compatible order: multiply then sum (matches CDLOpCPU.cpp ApplySaturation)
        if (self.saturation - 1.0).abs() > 1e-6 {
            let src = *rgb; // save original
            let wr = src[0] * REC709_LUMA_R;
            let wg = src[1] * REC709_LUMA_G;
            let wb = src[2] * REC709_LUMA_B;
            let luma = wr + wg + wb;
            rgb[0] = luma + self.saturation * (src[0] - luma);
            rgb[1] = luma + self.saturation * (src[1] - luma);
            rgb[2] = luma + self.saturation * (src[2] - luma);
        }

        // Final clamp [0,1] after saturation (ASC CDL spec)
        for v in rgb.iter_mut() {
            *v = v.clamp(0.0, 1.0);
        }
    }

    /// Applies the CDL correction without clamping.
    ///
    /// Useful for scene-referred workflows where values can exceed [0, 1].
    /// Matches OCIO CDL_NO_CLAMP_FWD behavior.
    #[inline]
    pub fn apply_unclamped(&self, rgb: &mut [f32; 3]) {
        // ASC CDL forward without clamp (matches OCIO CDL_NO_CLAMP_FWD)
        // Negative values pass through unchanged (not raised to power)
        for i in 0..3 {
            let v = rgb[i] * self.slope[i] + self.offset[i];
            // NaN -> 0, negative -> pass through, positive -> power
            rgb[i] = if v.is_nan() {
                0.0
            } else if v < 0.0 {
                v // Pass through negative values unchanged (OCIO behavior)
            } else if self.power[i] == 1.0 {
                v // Skip pow to avoid numerical error from log2->mul->exp2 chain
            } else {
                fast_pow(v, self.power[i])
            };
        }

        // Saturation (no clamp in this mode)
        // OCIO-compatible order: multiply then sum (matches CDLOpCPU.cpp ApplySaturation)
        if (self.saturation - 1.0).abs() > 1e-6 {
            let src = *rgb; // save original
            let wr = src[0] * REC709_LUMA_R;
            let wg = src[1] * REC709_LUMA_G;
            let wb = src[2] * REC709_LUMA_B;
            let luma = wr + wg + wb;
            rgb[0] = luma + self.saturation * (src[0] - luma);
            rgb[1] = luma + self.saturation * (src[1] - luma);
            rgb[2] = luma + self.saturation * (src[2] - luma);
        }
        // No final clamp in NO_CLAMP mode
    }

    /// Applies the CDL to a buffer of pixels.
    ///
    /// # Arguments
    ///
    /// * `pixels` - Array of RGB pixels
    pub fn apply_buffer(&self, pixels: &mut [[f32; 3]]) {
        for pixel in pixels {
            self.apply(pixel);
        }
    }

    /// Computes the inverse CDL parameters.
    ///
    /// Note: This is an approximation and may not be perfectly invertible.
    pub fn invert(&self) -> Self {
        // Inverse slope/offset/power
        let inv_slope = [
            1.0 / self.slope[0],
            1.0 / self.slope[1],
            1.0 / self.slope[2],
        ];
        let inv_power = [
            1.0 / self.power[0],
            1.0 / self.power[1],
            1.0 / self.power[2],
        ];
        let inv_offset = [
            -self.offset[0] / self.slope[0],
            -self.offset[1] / self.slope[1],
            -self.offset[2] / self.slope[2],
        ];

        Self {
            id: self.id.clone().map(|id| format!("{}_inv", id)),
            description: self.description.clone(),
            slope: inv_slope,
            offset: inv_offset,
            power: inv_power,
            saturation: 1.0 / self.saturation,
        }
    }

    /// Combines this CDL with another (this applied first, then other).
    pub fn chain(&self, other: &Cdl) -> Self {
        // Combined slope = slope1 * slope2
        // Combined offset = offset1 * slope2 + offset2
        // Combined power = power1 * power2 (approximate)
        Self {
            id: None,
            description: None,
            slope: [
                self.slope[0] * other.slope[0],
                self.slope[1] * other.slope[1],
                self.slope[2] * other.slope[2],
            ],
            offset: [
                self.offset[0] * other.slope[0] + other.offset[0],
                self.offset[1] * other.slope[1] + other.offset[1],
                self.offset[2] * other.slope[2] + other.offset[2],
            ],
            power: [
                self.power[0] * other.power[0],
                self.power[1] * other.power[1],
                self.power[2] * other.power[2],
            ],
            saturation: self.saturation * other.saturation,
        }
    }
}

/// A collection of CDL corrections.
///
/// Represents a .ccc (ColorCorrectionCollection) file.
#[derive(Debug, Clone, Default)]
pub struct CdlCollection {
    /// Collection ID.
    pub id: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// List of corrections.
    pub corrections: Vec<Cdl>,
}

impl CdlCollection {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a correction to the collection.
    pub fn add(&mut self, cdl: Cdl) {
        self.corrections.push(cdl);
    }

    /// Gets a correction by ID.
    pub fn get(&self, id: &str) -> Option<&Cdl> {
        self.corrections.iter().find(|c| c.id.as_deref() == Some(id))
    }

    /// Gets a correction by index.
    pub fn get_index(&self, index: usize) -> Option<&Cdl> {
        self.corrections.get(index)
    }

    /// Returns the number of corrections.
    pub fn len(&self) -> usize {
        self.corrections.len()
    }

    /// Returns true if empty.
    pub fn is_empty(&self) -> bool {
        self.corrections.is_empty()
    }
}

/// Reads a CDL from a .cc file.
///
/// # Arguments
///
/// * `path` - Path to the .cc file
///
/// # Example
///
/// ```rust,no_run
/// use vfx_color::cdl::read_cc;
/// use std::path::Path;
///
/// let cdl = read_cc(Path::new("correction.cc")).unwrap();
/// ```
pub fn read_cc(path: &Path) -> ColorResult<Cdl> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut content = String::new();
    reader.read_to_string(&mut content)?;
    parse_cc(&content)
}

/// Parses a CDL from CC XML string.
pub fn parse_cc(xml: &str) -> ColorResult<Cdl> {
    let mut cdl = Cdl::new();

    // Simple parsing - extract values from XML
    if let Some(id) = extract_attr(xml, "id") {
        cdl.id = Some(id);
    }

    if let Some(slope) = extract_element(xml, "Slope") {
        cdl.slope = parse_rgb(&slope)?;
    }

    if let Some(offset) = extract_element(xml, "Offset") {
        cdl.offset = parse_rgb(&offset)?;
    }

    if let Some(power) = extract_element(xml, "Power") {
        cdl.power = parse_rgb(&power)?;
    }

    if let Some(sat) = extract_element(xml, "Saturation") {
        cdl.saturation = sat.trim().parse().unwrap_or(1.0);
    }

    if let Some(desc) = extract_element(xml, "Description") {
        cdl.description = Some(desc);
    }

    Ok(cdl)
}

/// Reads a CDL collection from a .ccc file.
pub fn read_ccc(path: &Path) -> ColorResult<CdlCollection> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut content = String::new();
    reader.read_to_string(&mut content)?;
    parse_ccc(&content)
}

/// Parses a CDL collection from CCC XML string.
pub fn parse_ccc(xml: &str) -> ColorResult<CdlCollection> {
    let mut collection = CdlCollection::new();

    if let Some(id) = extract_attr(xml, "id") {
        collection.id = Some(id);
    }

    // Find all ColorCorrection elements
    let mut pos = 0;
    while let Some(start) = xml[pos..].find("<ColorCorrection") {
        let abs_start = pos + start;
        if let Some(end) = xml[abs_start..].find("</ColorCorrection>") {
            let cc_xml = &xml[abs_start..abs_start + end + 18];
            if let Ok(cdl) = parse_cc(cc_xml) {
                collection.add(cdl);
            }
            pos = abs_start + end + 18;
        } else {
            break;
        }
    }

    Ok(collection)
}

/// Writes a CDL to a .cc file.
///
/// # Example
///
/// ```rust,no_run
/// use vfx_color::cdl::{Cdl, write_cc};
/// use std::path::Path;
///
/// let cdl = Cdl::new().with_slope([1.1, 1.0, 0.9]);
/// write_cc(Path::new("output.cc"), &cdl).unwrap();
/// ```
pub fn write_cc(path: &Path, cdl: &Cdl) -> ColorResult<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    write_cc_to(&mut writer, cdl)
}

/// Writes a CDL to a writer.
pub fn write_cc_to<W: Write>(writer: &mut W, cdl: &Cdl) -> ColorResult<()> {
    writeln!(writer, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;

    let id = cdl.id.as_deref().unwrap_or("cc01");
    writeln!(writer, "<ColorCorrection id=\"{}\">", id)?;

    if let Some(ref desc) = cdl.description {
        writeln!(writer, "  <Description>{}</Description>", desc)?;
    }

    writeln!(writer, "  <SOPNode>")?;
    writeln!(
        writer,
        "    <Slope>{} {} {}</Slope>",
        cdl.slope[0], cdl.slope[1], cdl.slope[2]
    )?;
    writeln!(
        writer,
        "    <Offset>{} {} {}</Offset>",
        cdl.offset[0], cdl.offset[1], cdl.offset[2]
    )?;
    writeln!(
        writer,
        "    <Power>{} {} {}</Power>",
        cdl.power[0], cdl.power[1], cdl.power[2]
    )?;
    writeln!(writer, "  </SOPNode>")?;

    writeln!(writer, "  <SatNode>")?;
    writeln!(writer, "    <Saturation>{}</Saturation>", cdl.saturation)?;
    writeln!(writer, "  </SatNode>")?;

    writeln!(writer, "</ColorCorrection>")?;

    Ok(())
}

/// Writes a CDL collection to a .ccc file.
pub fn write_ccc(path: &Path, collection: &CdlCollection) -> ColorResult<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    write_ccc_to(&mut writer, collection)
}

/// Writes a CDL collection to a writer.
pub fn write_ccc_to<W: Write>(writer: &mut W, collection: &CdlCollection) -> ColorResult<()> {
    writeln!(writer, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;

    let id = collection.id.as_deref().unwrap_or("ccc01");
    writeln!(writer, "<ColorCorrectionCollection id=\"{}\">", id)?;

    if let Some(ref desc) = collection.description {
        writeln!(writer, "  <Description>{}</Description>", desc)?;
    }

    for cdl in &collection.corrections {
        let id = cdl.id.as_deref().unwrap_or("cc");
        writeln!(writer, "  <ColorCorrection id=\"{}\">", id)?;

        if let Some(ref desc) = cdl.description {
            writeln!(writer, "    <Description>{}</Description>", desc)?;
        }

        writeln!(writer, "    <SOPNode>")?;
        writeln!(
            writer,
            "      <Slope>{} {} {}</Slope>",
            cdl.slope[0], cdl.slope[1], cdl.slope[2]
        )?;
        writeln!(
            writer,
            "      <Offset>{} {} {}</Offset>",
            cdl.offset[0], cdl.offset[1], cdl.offset[2]
        )?;
        writeln!(
            writer,
            "      <Power>{} {} {}</Power>",
            cdl.power[0], cdl.power[1], cdl.power[2]
        )?;
        writeln!(writer, "    </SOPNode>")?;

        writeln!(writer, "    <SatNode>")?;
        writeln!(writer, "      <Saturation>{}</Saturation>", cdl.saturation)?;
        writeln!(writer, "    </SatNode>")?;

        writeln!(writer, "  </ColorCorrection>")?;
    }

    writeln!(writer, "</ColorCorrectionCollection>")?;

    Ok(())
}

// Helper functions for simple XML parsing

fn extract_element(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);

    let start = xml.find(&open)?;
    let end = xml.find(&close)?;

    Some(xml[start + open.len()..end].trim().to_string())
}

fn extract_attr(xml: &str, name: &str) -> Option<String> {
    let pattern = format!("{}=\"", name);
    let start = xml.find(&pattern)?;
    let value_start = start + pattern.len();
    let end = xml[value_start..].find('"')?;
    Some(xml[value_start..value_start + end].to_string())
}

fn parse_rgb(s: &str) -> ColorResult<[f32; 3]> {
    let values: Vec<f32> = s
        .split_whitespace()
        .filter_map(|v| v.parse().ok())
        .collect();

    if values.len() >= 3 {
        Ok([values[0], values[1], values[2]])
    } else {
        Err(ColorError::ParseError("expected 3 RGB values".into()))
    }
}

// ============================================================================
// Conversions from vfx-lut types
// ============================================================================

impl From<vfx_lut::cdl::ColorCorrection> for Cdl {
    fn from(cc: vfx_lut::cdl::ColorCorrection) -> Self {
        Self {
            id: cc.id,
            description: cc.descriptions.first().cloned(),
            slope: cc.slope,
            offset: cc.offset,
            power: cc.power,
            saturation: cc.saturation,
        }
    }
}

impl From<&vfx_lut::cdl::ColorCorrection> for Cdl {
    fn from(cc: &vfx_lut::cdl::ColorCorrection) -> Self {
        Self {
            id: cc.id.clone(),
            description: cc.descriptions.first().cloned(),
            slope: cc.slope,
            offset: cc.offset,
            power: cc.power,
            saturation: cc.saturation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity() {
        let cdl = Cdl::new();
        let mut pixel = [0.5, 0.3, 0.2];
        let original = pixel;
        cdl.apply(&mut pixel);

        assert!((pixel[0] - original[0]).abs() < 0.001);
        assert!((pixel[1] - original[1]).abs() < 0.001);
        assert!((pixel[2] - original[2]).abs() < 0.001);
    }

    #[test]
    fn test_slope() {
        let cdl = Cdl::new().with_slope([2.0, 1.0, 0.5]);
        let mut pixel = [0.25, 0.5, 0.5];
        cdl.apply(&mut pixel);

        assert!((pixel[0] - 0.5).abs() < 0.001);
        assert!((pixel[1] - 0.5).abs() < 0.001);
        assert!((pixel[2] - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_offset() {
        let cdl = Cdl::new().with_offset([0.1, 0.0, -0.1]);
        let mut pixel = [0.5, 0.5, 0.5];
        cdl.apply(&mut pixel);

        assert!((pixel[0] - 0.6).abs() < 0.001);
        assert!((pixel[1] - 0.5).abs() < 0.001);
        assert!((pixel[2] - 0.4).abs() < 0.001);
    }

    #[test]
    fn test_power() {
        let cdl = Cdl::new().with_power([2.0, 1.0, 0.5]);
        let mut pixel = [0.5, 0.5, 0.5];
        cdl.apply(&mut pixel);

        assert!((pixel[0] - 0.25).abs() < 0.001); // 0.5^2
        assert!((pixel[1] - 0.5).abs() < 0.001);  // 0.5^1
        assert!((pixel[2] - 0.707).abs() < 0.01); // 0.5^0.5
    }

    #[test]
    fn test_saturation() {
        let cdl = Cdl::new().with_saturation(0.0);
        let mut pixel = [1.0, 0.0, 0.0]; // Pure red
        cdl.apply(&mut pixel);

        // Should be grayscale (luma)
        let luma = 0.2126;
        assert!((pixel[0] - luma).abs() < 0.001);
        assert!((pixel[1] - luma).abs() < 0.001);
        assert!((pixel[2] - luma).abs() < 0.001);
    }

    #[test]
    fn test_chain() {
        let cdl1 = Cdl::new().with_slope([2.0, 2.0, 2.0]);
        let cdl2 = Cdl::new().with_offset([0.1, 0.1, 0.1]);
        let combined = cdl1.chain(&cdl2);

        let mut pixel1 = [0.25, 0.25, 0.25];
        let mut pixel2 = [0.25, 0.25, 0.25];

        // Apply separately
        cdl1.apply(&mut pixel1);
        cdl2.apply(&mut pixel1);

        // Apply combined
        combined.apply(&mut pixel2);

        assert!((pixel1[0] - pixel2[0]).abs() < 0.01);
    }

    #[test]
    fn test_parse_cc() {
        let xml = r#"
            <ColorCorrection id="test01">
                <Description>Test correction</Description>
                <SOPNode>
                    <Slope>1.1 1.0 0.9</Slope>
                    <Offset>0.01 0.0 -0.01</Offset>
                    <Power>1.0 1.0 1.0</Power>
                </SOPNode>
                <SatNode>
                    <Saturation>1.2</Saturation>
                </SatNode>
            </ColorCorrection>
        "#;

        let cdl = parse_cc(xml).unwrap();
        assert_eq!(cdl.id, Some("test01".into()));
        assert!((cdl.slope[0] - 1.1).abs() < 0.001);
        assert!((cdl.offset[0] - 0.01).abs() < 0.001);
        assert!((cdl.saturation - 1.2).abs() < 0.001);
    }

    #[test]
    fn test_roundtrip() {
        let cdl = Cdl::new()
            .with_id("roundtrip")
            .with_slope([1.1, 1.0, 0.9])
            .with_offset([0.01, 0.0, -0.01])
            .with_power([1.0, 1.05, 1.0])
            .with_saturation(1.1);

        let mut buf = Vec::new();
        write_cc_to(&mut buf, &cdl).unwrap();
        let xml = String::from_utf8(buf).unwrap();

        let parsed = parse_cc(&xml).unwrap();

        assert_eq!(parsed.id, cdl.id);
        assert!((parsed.slope[0] - cdl.slope[0]).abs() < 0.001);
        assert!((parsed.offset[0] - cdl.offset[0]).abs() < 0.001);
        assert!((parsed.saturation - cdl.saturation).abs() < 0.001);
    }
}
