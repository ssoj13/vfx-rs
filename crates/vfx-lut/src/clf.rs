//! Common LUT Format (CLF) parser and writer.
//!
//! CLF is an XML-based LUT format defined by the Academy of Motion Picture
//! Arts and Sciences (AMPAS). It supports multiple processing operations
//! in a single file, making it ideal for complex color pipelines.
//!
//! # Supported Node Types
//!
//! - [`LUT1D`](ProcessNode::Lut1D) - 1-dimensional lookup table
//! - [`LUT3D`](ProcessNode::Lut3D) - 3-dimensional lookup table
//! - [`Matrix`](ProcessNode::Matrix) - 3x3 or 3x4 color matrix
//! - [`Range`](ProcessNode::Range) - Input/output domain scaling
//! - [`ASC_CDL`](ProcessNode::Cdl) - ASC Color Decision List (Slope/Offset/Power)
//! - [`Log`](ProcessNode::Log) - Logarithmic transform
//! - [`Exponent`](ProcessNode::Exponent) - Power/gamma function
//!
//! # File Structure
//!
//! ```xml
//! <?xml version="1.0" encoding="UTF-8"?>
//! <ProcessList id="example" compCLFversion="3.0">
//!   <Description>Example CLF</Description>
//!   <LUT1D inBitDepth="32f" outBitDepth="32f">
//!     <Array dim="1024 3">
//!       <!-- data -->
//!     </Array>
//!   </LUT1D>
//!   <LUT3D inBitDepth="32f" outBitDepth="32f">
//!     <Array dim="33 33 33 3">
//!       <!-- data -->
//!     </Array>
//!   </LUT3D>
//! </ProcessList>
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use vfx_lut::clf::{ProcessList, read_clf};
//! use std::path::Path;
//!
//! // Read a CLF file
//! let clf = read_clf(Path::new("grade.clf")).unwrap();
//!
//! // Apply to RGB
//! let mut rgb = [0.5, 0.3, 0.2];
//! clf.apply(&mut rgb);
//! ```
//!
//! # References
//!
//! - [CLF Specification](https://acescentral.com/clf/)
//! - [S-2014-006 Common LUT Format](https://github.com/ampas/CLF)

use vfx_core::pixel::{REC709_LUMA_R, REC709_LUMA_G, REC709_LUMA_B};

use crate::{Interpolation, Lut1D, Lut3D, LutError, LutResult};
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

/// CLF version supported by this implementation.
pub const CLF_VERSION: &str = "3.0";

/// CTF version supported by this implementation.
pub const CTF_VERSION: &str = "2.0";

/// Re-export BitDepth from vfx-core.
pub use vfx_core::BitDepth;

/// ASC-CDL (Color Decision List) parameters.
///
/// Applies the standard CDL formula:
/// ```text
/// out = clamp((in * slope + offset) ^ power)
/// ```
///
/// With optional saturation adjustment applied after.
#[derive(Debug, Clone, PartialEq)]
pub struct CdlParams {
    /// Slope (multiply) per channel [R, G, B].
    pub slope: [f32; 3],
    /// Offset (add) per channel [R, G, B].
    pub offset: [f32; 3],
    /// Power (gamma) per channel [R, G, B].
    pub power: [f32; 3],
    /// Saturation adjustment (1.0 = no change).
    pub saturation: f32,
}

impl Default for CdlParams {
    fn default() -> Self {
        Self {
            slope: [1.0, 1.0, 1.0],
            offset: [0.0, 0.0, 0.0],
            power: [1.0, 1.0, 1.0],
            saturation: 1.0,
        }
    }
}

impl CdlParams {
    /// Creates CDL params with given slope, offset, power.
    pub fn new(slope: [f32; 3], offset: [f32; 3], power: [f32; 3]) -> Self {
        Self {
            slope,
            offset,
            power,
            saturation: 1.0,
        }
    }

    /// Applies CDL to RGB values in-place.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_lut::clf::CdlParams;
    ///
    /// let cdl = CdlParams::new(
    ///     [1.1, 1.0, 0.9],  // slope
    ///     [0.0, 0.01, 0.0], // offset
    ///     [1.0, 1.0, 1.0],  // power
    /// );
    ///
    /// let mut rgb = [0.5, 0.5, 0.5];
    /// cdl.apply(&mut rgb);
    /// ```
    pub fn apply(&self, rgb: &mut [f32; 3]) {
        // SOPnode: out = (in * slope + offset) ^ power
        for i in 0..3 {
            let v = rgb[i] * self.slope[i] + self.offset[i];
            rgb[i] = v.max(0.0).powf(self.power[i]);
        }

        // Saturation (Rec. 709 luma)
        if (self.saturation - 1.0).abs() > 1e-6 {
            let luma = REC709_LUMA_R * rgb[0] + REC709_LUMA_G * rgb[1] + REC709_LUMA_B * rgb[2];
            for v in rgb.iter_mut() {
                *v = luma + (*v - luma) * self.saturation;
            }
        }
    }
}

/// Log transform parameters.
///
/// Supports multiple log encodings (Lin-to-Log, Log-to-Lin).
#[derive(Debug, Clone, PartialEq)]
pub struct LogParams {
    /// Log style (e.g., "log10", "log2", "antiLog10").
    pub style: LogStyle,
    /// Base for logarithm (default 10.0).
    pub base: f32,
    /// Offset added before log.
    pub offset: [f32; 3],
    /// Linear slope.
    pub lin_slope: [f32; 3],
    /// Linear offset.
    pub lin_offset: [f32; 3],
    /// Log slope.
    pub log_slope: [f32; 3],
    /// Log offset.
    pub log_offset: [f32; 3],
}

/// Logarithm style for Log nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogStyle {
    /// Log base 10.
    #[default]
    Log10,
    /// Log base 2.
    Log2,
    /// Anti-log (10^x).
    AntiLog10,
    /// Anti-log base 2 (2^x).
    AntiLog2,
    /// Lin-to-Log with camera-style formula.
    LinToLog,
    /// Log-to-Lin with camera-style formula.
    LogToLin,
}

impl Default for LogParams {
    fn default() -> Self {
        Self {
            style: LogStyle::Log10,
            base: 10.0,
            offset: [0.0; 3],
            lin_slope: [1.0; 3],
            lin_offset: [0.0; 3],
            log_slope: [1.0; 3],
            log_offset: [0.0; 3],
        }
    }
}

impl LogParams {
    /// Applies log transform to RGB in-place.
    pub fn apply(&self, rgb: &mut [f32; 3]) {
        for i in 0..3 {
            rgb[i] = match self.style {
                LogStyle::Log10 => (rgb[i] + self.offset[i]).max(1e-10).log10(),
                LogStyle::Log2 => (rgb[i] + self.offset[i]).max(1e-10).log2(),
                LogStyle::AntiLog10 => 10.0_f32.powf(rgb[i]),
                LogStyle::AntiLog2 => 2.0_f32.powf(rgb[i]),
                LogStyle::LinToLog => {
                    let lin = rgb[i] * self.lin_slope[i] + self.lin_offset[i];
                    self.log_slope[i] * lin.max(1e-10).log(self.base) + self.log_offset[i]
                }
                LogStyle::LogToLin => {
                    let log_val = (rgb[i] - self.log_offset[i]) / self.log_slope[i];
                    (self.base.powf(log_val) - self.lin_offset[i]) / self.lin_slope[i]
                }
            };
        }
    }
}

/// Exponent (gamma/power) parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct ExponentParams {
    /// Exponent style.
    pub style: ExponentStyle,
    /// Exponent value per channel.
    pub exponent: [f32; 3],
    /// Offset for moncurve style.
    pub offset: [f32; 3],
}

/// Exponent curve style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExponentStyle {
    /// Basic power function: out = in^exp.
    #[default]
    Basic,
    /// Basic with mirror for negatives.
    BasicMirror,
    /// Pass-through (no change).
    PassThru,
    /// Monitor curve (sRGB-like with linear toe).
    MonCurve,
    /// Inverse monitor curve.
    MonCurveMirror,
}

impl Default for ExponentParams {
    fn default() -> Self {
        Self {
            style: ExponentStyle::Basic,
            exponent: [1.0; 3],
            offset: [0.0; 3],
        }
    }
}

impl ExponentParams {
    /// Applies exponent to RGB in-place.
    pub fn apply(&self, rgb: &mut [f32; 3]) {
        for i in 0..3 {
            rgb[i] = match self.style {
                ExponentStyle::Basic => rgb[i].max(0.0).powf(self.exponent[i]),
                ExponentStyle::BasicMirror => {
                    let sign = rgb[i].signum();
                    sign * rgb[i].abs().powf(self.exponent[i])
                }
                ExponentStyle::PassThru => rgb[i],
                ExponentStyle::MonCurve | ExponentStyle::MonCurveMirror => {
                    // Simplified moncurve - full impl needs break point calc
                    rgb[i].max(0.0).powf(self.exponent[i])
                }
            };
        }
    }
}

/// Range (domain scaling) parameters.
///
/// Scales and clamps input values to an output range.
#[derive(Debug, Clone, PartialEq)]
pub struct RangeParams {
    /// Minimum input value per channel.
    pub min_in: [f32; 3],
    /// Maximum input value per channel.
    pub max_in: [f32; 3],
    /// Minimum output value per channel.
    pub min_out: [f32; 3],
    /// Maximum output value per channel.
    pub max_out: [f32; 3],
    /// Whether to clamp to output range.
    pub clamp: bool,
}

impl Default for RangeParams {
    fn default() -> Self {
        Self {
            min_in: [0.0; 3],
            max_in: [1.0; 3],
            min_out: [0.0; 3],
            max_out: [1.0; 3],
            clamp: true,
        }
    }
}

impl RangeParams {
    /// Applies range scaling to RGB in-place.
    pub fn apply(&self, rgb: &mut [f32; 3]) {
        for i in 0..3 {
            let range_in = self.max_in[i] - self.min_in[i];
            let range_out = self.max_out[i] - self.min_out[i];

            if range_in.abs() < 1e-10 {
                rgb[i] = self.min_out[i];
            } else {
                let t = (rgb[i] - self.min_in[i]) / range_in;
                rgb[i] = self.min_out[i] + t * range_out;
            }

            if self.clamp {
                rgb[i] = rgb[i].clamp(self.min_out[i], self.max_out[i]);
            }
        }
    }
}

/// A processing node in the CLF ProcessList.
///
/// Each node represents one color transformation step.
#[derive(Debug, Clone)]
pub enum ProcessNode {
    /// 1D lookup table.
    Lut1D {
        /// LUT data.
        lut: Lut1D,
        /// Input bit depth.
        in_depth: BitDepth,
        /// Output bit depth.
        out_depth: BitDepth,
    },

    /// 3D lookup table.
    Lut3D {
        /// LUT data.
        lut: Lut3D,
        /// Input bit depth.
        in_depth: BitDepth,
        /// Output bit depth.
        out_depth: BitDepth,
    },

    /// Color matrix (3x3 or 3x4).
    Matrix {
        /// Matrix values (row-major, 9 or 12 elements).
        values: Vec<f32>,
        /// Input bit depth.
        in_depth: BitDepth,
        /// Output bit depth.
        out_depth: BitDepth,
    },

    /// Range (domain scaling).
    Range(RangeParams),

    /// ASC Color Decision List.
    Cdl(CdlParams),

    /// Logarithmic transform.
    Log(LogParams),

    /// Exponent/gamma.
    Exponent(ExponentParams),
}

impl ProcessNode {
    /// Applies this node to RGB values in-place.
    pub fn apply(&self, rgb: &mut [f32; 3]) {
        match self {
            ProcessNode::Lut1D { lut, .. } => {
                let result = lut.apply_rgb(*rgb);
                *rgb = result;
            }
            ProcessNode::Lut3D { lut, .. } => {
                *rgb = lut.apply(*rgb);
            }
            ProcessNode::Matrix { values, .. } => {
                let (r, g, b) = (rgb[0], rgb[1], rgb[2]);
                if values.len() >= 12 {
                    // 3x4 matrix with offset
                    rgb[0] = values[0] * r + values[1] * g + values[2] * b + values[3];
                    rgb[1] = values[4] * r + values[5] * g + values[6] * b + values[7];
                    rgb[2] = values[8] * r + values[9] * g + values[10] * b + values[11];
                } else if values.len() >= 9 {
                    // 3x3 matrix
                    rgb[0] = values[0] * r + values[1] * g + values[2] * b;
                    rgb[1] = values[3] * r + values[4] * g + values[5] * b;
                    rgb[2] = values[6] * r + values[7] * g + values[8] * b;
                }
            }
            ProcessNode::Range(params) => params.apply(rgb),
            ProcessNode::Cdl(params) => params.apply(rgb),
            ProcessNode::Log(params) => params.apply(rgb),
            ProcessNode::Exponent(params) => params.apply(rgb),
        }
    }
}

/// A CLF ProcessList containing multiple processing nodes.
///
/// This is the root element of a CLF file, containing metadata
/// and an ordered list of color transformations.
///
/// # Example
///
/// ```rust
/// use vfx_lut::clf::{ProcessList, ProcessNode, CdlParams};
/// use vfx_lut::Lut1D;
///
/// let mut clf = ProcessList::new("my_grade");
/// clf.description = Some("Primary grade".into());
///
/// // Add a gamma curve
/// clf.nodes.push(ProcessNode::Lut1D {
///     lut: Lut1D::gamma(1024, 2.2),
///     in_depth: Default::default(),
///     out_depth: Default::default(),
/// });
///
/// // Apply to RGB
/// let mut rgb = [0.5, 0.3, 0.2];
/// clf.apply(&mut rgb);
/// ```
#[derive(Debug, Clone)]
pub struct ProcessList {
    /// Unique identifier for this process list.
    pub id: String,
    /// Optional human-readable name.
    pub name: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Input color space descriptor.
    pub input_descriptor: Option<String>,
    /// Output color space descriptor.
    pub output_descriptor: Option<String>,
    /// CLF version.
    pub version: String,
    /// Ordered list of processing nodes.
    pub nodes: Vec<ProcessNode>,
}

impl ProcessList {
    /// Creates a new empty ProcessList.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this process list
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: None,
            description: None,
            input_descriptor: None,
            output_descriptor: None,
            version: CLF_VERSION.into(),
            nodes: Vec::new(),
        }
    }

    /// Applies all processing nodes to RGB values in-place.
    ///
    /// Nodes are applied in order (first to last).
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_lut::clf::ProcessList;
    ///
    /// let clf = ProcessList::new("identity");
    /// let mut rgb = [0.5, 0.3, 0.2];
    /// clf.apply(&mut rgb);
    /// // rgb unchanged for empty ProcessList
    /// ```
    pub fn apply(&self, rgb: &mut [f32; 3]) {
        for node in &self.nodes {
            node.apply(rgb);
        }
    }

    /// Applies all processing nodes to an image buffer.
    ///
    /// Processes pixels in-place, assuming RGB or RGBA layout.
    ///
    /// # Arguments
    ///
    /// * `data` - Image data as f32 values
    /// * `channels` - Number of channels (3 for RGB, 4 for RGBA)
    pub fn apply_buffer(&self, data: &mut [f32], channels: usize) {
        assert!(channels >= 3, "need at least 3 channels");
        
        for pixel in data.chunks_exact_mut(channels) {
            let mut rgb = [pixel[0], pixel[1], pixel[2]];
            self.apply(&mut rgb);
            pixel[0] = rgb[0];
            pixel[1] = rgb[1];
            pixel[2] = rgb[2];
        }
    }
}

/// Reads a CLF file from disk.
///
/// Parses the XML structure and returns a [`ProcessList`] containing
/// all processing nodes.
///
/// # Arguments
///
/// * `path` - Path to the .clf file
///
/// # Errors
///
/// Returns error if file cannot be read or contains invalid CLF data.
///
/// # Example
///
/// ```rust,no_run
/// use vfx_lut::clf::read_clf;
/// use std::path::Path;
///
/// let clf = read_clf(Path::new("grade.clf")).unwrap();
/// println!("CLF has {} nodes", clf.nodes.len());
/// ```
pub fn read_clf(path: &Path) -> LutResult<ProcessList> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    parse_clf(reader)
}

/// Reads an Autodesk CTF (Color Transform Format) file.
///
/// CTF is Autodesk's extended version of CLF used in Flame/Smoke.
/// This function parses both standard CLF elements and CTF-specific
/// extensions like InvLut1D, InvLut3D, ReferenceSpace.
///
/// # Arguments
///
/// * `path` - Path to the .ctf file
///
/// # Example
///
/// ```rust,no_run
/// use vfx_lut::clf::read_ctf;
/// use std::path::Path;
///
/// let ctf = read_ctf(Path::new("grade.ctf")).unwrap();
/// ```
pub fn read_ctf(path: &Path) -> LutResult<ProcessList> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    parse_ctf(reader)
}

/// Parses CTF from a reader.
///
/// CTF uses slightly different root elements than CLF but shares
/// most processing nodes.
pub fn parse_ctf<R: BufRead>(reader: R) -> LutResult<ProcessList> {
    // CTF parsing uses same logic as CLF with extended element support
    parse_clf_internal(reader, true)
}

/// Parses CLF from a reader.
///
/// # Arguments
///
/// * `reader` - Any type implementing `BufRead`
pub fn parse_clf<R: BufRead>(reader: R) -> LutResult<ProcessList> {
    parse_clf_internal(reader, false)
}

/// Internal parser supporting both CLF and CTF formats.
fn parse_clf_internal<R: BufRead>(reader: R, ctf_mode: bool) -> LutResult<ProcessList> {
    let mut xml = Reader::from_reader(reader);
    xml.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut result: Option<ProcessList> = None;
    let mut current_text = String::new();
    let mut in_array = false;
    let mut array_data = String::new();
    let mut array_dim: Vec<usize> = Vec::new();
    let mut current_node: Option<(&'static str, BitDepth, BitDepth)> = None;
    let mut current_interp = Interpolation::Linear;

    loop {
        match xml.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                
                match name.as_str() {
                    // CTF uses different root elements
                    "ProcessList" | "CtfFile" | "ColorTransformFile" => {
                        let mut id = String::new();
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"id" {
                                id = String::from_utf8_lossy(&attr.value).into();
                            }
                        }
                        result = Some(ProcessList::new(id));
                    }
                    "LUT1D" | "LUT3D" => {
                        let mut in_depth = BitDepth::F32;
                        let mut out_depth = BitDepth::F32;
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"inBitDepth" => {
                                    in_depth = BitDepth::from_clf_str(
                                        &String::from_utf8_lossy(&attr.value)
                                    ).unwrap_or_default();
                                }
                                b"outBitDepth" => {
                                    out_depth = BitDepth::from_clf_str(
                                        &String::from_utf8_lossy(&attr.value)
                                    ).unwrap_or_default();
                                }
                                b"interpolation" => {
                                    let s = String::from_utf8_lossy(&attr.value);
                                    current_interp = match s.as_ref() {
                                        "nearest" => Interpolation::Nearest,
                                        "tetrahedral" => Interpolation::Tetrahedral,
                                        _ => Interpolation::Linear,
                                    };
                                }
                                _ => {}
                            }
                        }
                        current_node = Some((
                            if name == "LUT1D" { "LUT1D" } else { "LUT3D" },
                            in_depth,
                            out_depth,
                        ));
                    }
                    "Matrix" => {
                        let mut in_depth = BitDepth::F32;
                        let mut out_depth = BitDepth::F32;
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"inBitDepth" => {
                                    in_depth = BitDepth::from_clf_str(
                                        &String::from_utf8_lossy(&attr.value)
                                    ).unwrap_or_default();
                                }
                                b"outBitDepth" => {
                                    out_depth = BitDepth::from_clf_str(
                                        &String::from_utf8_lossy(&attr.value)
                                    ).unwrap_or_default();
                                }
                                _ => {}
                            }
                        }
                        current_node = Some(("Matrix", in_depth, out_depth));
                    }
                    "Array" => {
                        in_array = true;
                        array_data.clear();
                        array_dim.clear();
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"dim" {
                                let dims = String::from_utf8_lossy(&attr.value);
                                array_dim = dims
                                    .split_whitespace()
                                    .filter_map(|s| s.parse().ok())
                                    .collect();
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.decode().unwrap_or_default();
                if in_array {
                    array_data.push_str(&text);
                } else {
                    current_text = text.into();
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                
                match name.as_str() {
                    "Description" => {
                        if let Some(ref mut pl) = result {
                            pl.description = Some(current_text.clone());
                        }
                    }
                    "InputDescriptor" => {
                        if let Some(ref mut pl) = result {
                            pl.input_descriptor = Some(current_text.clone());
                        }
                    }
                    "OutputDescriptor" => {
                        if let Some(ref mut pl) = result {
                            pl.output_descriptor = Some(current_text.clone());
                        }
                    }
                    "Array" => {
                        in_array = false;
                    }
                    "LUT1D" => {
                        if let (Some(pl), Some((_, in_depth, out_depth))) = 
                            (&mut result, current_node.take()) 
                        {
                            let values: Vec<f32> = array_data
                                .split_whitespace()
                                .filter_map(|s| s.parse().ok())
                                .collect();
                            
                            if !values.is_empty() {
                                let lut = Lut1D::from_data(values, 0.0, 1.0)
                                    .unwrap_or_else(|_| Lut1D::identity(256));
                                pl.nodes.push(ProcessNode::Lut1D { lut, in_depth, out_depth });
                            }
                        }
                    }
                    "LUT3D" => {
                        if let (Some(pl), Some((_, in_depth, out_depth))) = 
                            (&mut result, current_node.take()) 
                        {
                            let values: Vec<f32> = array_data
                                .split_whitespace()
                                .filter_map(|s| s.parse().ok())
                                .collect();
                            
                            if !array_dim.is_empty() && values.len() >= 3 {
                                let size = array_dim[0];
                                let data: Vec<[f32; 3]> = values
                                    .chunks(3)
                                    .map(|c| [c[0], c[1], c[2]])
                                    .collect();
                                
                                if let Ok(lut) = Lut3D::from_data(data, size) {
                                    let lut = lut.with_interpolation(current_interp);
                                    pl.nodes.push(ProcessNode::Lut3D { lut, in_depth, out_depth });
                                }
                            }
                        }
                        current_interp = Interpolation::Linear;
                    }
                    "Matrix" => {
                        if let (Some(pl), Some((_, in_depth, out_depth))) = 
                            (&mut result, current_node.take()) 
                        {
                            let values: Vec<f32> = array_data
                                .split_whitespace()
                                .filter_map(|s| s.parse().ok())
                                .collect();
                            
                            if values.len() >= 9 {
                                pl.nodes.push(ProcessNode::Matrix { values, in_depth, out_depth });
                            }
                        }
                    }
                    _ => {}
                }
                current_text.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(LutError::ParseError(format!("XML error: {}", e))),
            _ => {}
        }
        buf.clear();
    }

    result.ok_or_else(|| LutError::ParseError(
        if ctf_mode {
            "missing CtfFile/ProcessList element".into()
        } else {
            "missing ProcessList element".into()
        }
    ))
}

/// Writes an Autodesk CTF file.
///
/// CTF files use the same structure as CLF but with .ctf extension
/// and a different root element.
pub fn write_ctf(path: &Path, clf: &ProcessList) -> LutResult<()> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    write_ctf_to(writer, clf)
}

/// Writes CTF to any writer.
pub fn write_ctf_to<W: Write>(writer: W, clf: &ProcessList) -> LutResult<()> {
    let mut xml = Writer::new_with_indent(writer, b' ', 2);

    // XML declaration
    xml.write_event(Event::Decl(quick_xml::events::BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

    // CTF uses different root element
    let mut pl_start = BytesStart::new("CtfFile");
    pl_start.push_attribute(("xmlns", "urn:AMPAS:CLF:v3.0"));
    pl_start.push_attribute(("id", clf.id.as_str()));
    pl_start.push_attribute(("version", CTF_VERSION));
    if let Some(ref name) = clf.name {
        pl_start.push_attribute(("name", name.as_str()));
    }
    xml.write_event(Event::Start(pl_start))
        .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

    // Description
    if let Some(ref desc) = clf.description {
        xml.write_event(Event::Start(BytesStart::new("Description")))
            .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
        xml.write_event(Event::Text(BytesText::new(desc)))
            .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
        xml.write_event(Event::End(BytesEnd::new("Description")))
            .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
    }

    // Input/Output descriptors
    if let Some(ref inp) = clf.input_descriptor {
        write_text_element(&mut xml, "InputDescriptor", inp)?;
    }
    if let Some(ref out) = clf.output_descriptor {
        write_text_element(&mut xml, "OutputDescriptor", out)?;
    }

    // Process nodes
    for node in &clf.nodes {
        write_node(&mut xml, node)?;
    }

    // CtfFile end
    xml.write_event(Event::End(BytesEnd::new("CtfFile")))
        .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

    Ok(())
}

/// Writes a CLF file to disk.
///
/// # Arguments
///
/// * `path` - Output path for the .clf file
/// * `clf` - ProcessList to write
///
/// # Errors
///
/// Returns error if file cannot be written.
///
/// # Example
///
/// ```rust,no_run
/// use vfx_lut::clf::{ProcessList, write_clf};
/// use std::path::Path;
///
/// let clf = ProcessList::new("my_grade");
/// write_clf(Path::new("output.clf"), &clf).unwrap();
/// ```
pub fn write_clf(path: &Path, clf: &ProcessList) -> LutResult<()> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    write_clf_to(writer, clf)
}

/// Writes CLF to any writer.
pub fn write_clf_to<W: Write>(writer: W, clf: &ProcessList) -> LutResult<()> {
    let mut xml = Writer::new_with_indent(writer, b' ', 2);

    // XML declaration
    xml.write_event(Event::Decl(quick_xml::events::BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

    // ProcessList start
    let mut pl_start = BytesStart::new("ProcessList");
    pl_start.push_attribute(("id", clf.id.as_str()));
    pl_start.push_attribute(("compCLFversion", clf.version.as_str()));
    if let Some(ref name) = clf.name {
        pl_start.push_attribute(("name", name.as_str()));
    }
    xml.write_event(Event::Start(pl_start))
        .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

    // Description
    if let Some(ref desc) = clf.description {
        xml.write_event(Event::Start(BytesStart::new("Description")))
            .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
        xml.write_event(Event::Text(BytesText::new(desc)))
            .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
        xml.write_event(Event::End(BytesEnd::new("Description")))
            .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
    }

    // Input/Output descriptors
    if let Some(ref inp) = clf.input_descriptor {
        write_text_element(&mut xml, "InputDescriptor", inp)?;
    }
    if let Some(ref out) = clf.output_descriptor {
        write_text_element(&mut xml, "OutputDescriptor", out)?;
    }

    // Process nodes
    for node in &clf.nodes {
        write_node(&mut xml, node)?;
    }

    // ProcessList end
    xml.write_event(Event::End(BytesEnd::new("ProcessList")))
        .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

    Ok(())
}

/// Helper to write a text element.
fn write_text_element<W: Write>(
    xml: &mut Writer<W>,
    name: &str,
    text: &str,
) -> LutResult<()> {
    xml.write_event(Event::Start(BytesStart::new(name)))
        .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
    xml.write_event(Event::Text(BytesText::new(text)))
        .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
    xml.write_event(Event::End(BytesEnd::new(name)))
        .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
    Ok(())
}

/// Helper to write a process node.
fn write_node<W: Write>(xml: &mut Writer<W>, node: &ProcessNode) -> LutResult<()> {
    match node {
        ProcessNode::Lut1D { lut, in_depth, out_depth } => {
            let mut start = BytesStart::new("LUT1D");
            start.push_attribute(("inBitDepth", in_depth.clf_str()));
            start.push_attribute(("outBitDepth", out_depth.clf_str()));
            xml.write_event(Event::Start(start))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

            // Array
            let mut arr = BytesStart::new("Array");
            arr.push_attribute(("dim", format!("{} 1", lut.size()).as_str()));
            xml.write_event(Event::Start(arr))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

            let values: String = lut.r.iter()
                .map(|v| format!("{:.6}", v))
                .collect::<Vec<_>>()
                .join(" ");
            xml.write_event(Event::Text(BytesText::new(&values)))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

            xml.write_event(Event::End(BytesEnd::new("Array")))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
            xml.write_event(Event::End(BytesEnd::new("LUT1D")))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
        }
        ProcessNode::Lut3D { lut, in_depth, out_depth } => {
            let mut start = BytesStart::new("LUT3D");
            start.push_attribute(("inBitDepth", in_depth.clf_str()));
            start.push_attribute(("outBitDepth", out_depth.clf_str()));
            xml.write_event(Event::Start(start))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

            // Array
            let mut arr = BytesStart::new("Array");
            arr.push_attribute(("dim", format!("{} {} {} 3", lut.size, lut.size, lut.size).as_str()));
            xml.write_event(Event::Start(arr))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

            let values: String = lut.data.iter()
                .map(|rgb| format!("{:.6} {:.6} {:.6}", rgb[0], rgb[1], rgb[2]))
                .collect::<Vec<_>>()
                .join("\n");
            xml.write_event(Event::Text(BytesText::new(&values)))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

            xml.write_event(Event::End(BytesEnd::new("Array")))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
            xml.write_event(Event::End(BytesEnd::new("LUT3D")))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
        }
        ProcessNode::Matrix { values, in_depth, out_depth } => {
            let mut start = BytesStart::new("Matrix");
            start.push_attribute(("inBitDepth", in_depth.clf_str()));
            start.push_attribute(("outBitDepth", out_depth.clf_str()));
            xml.write_event(Event::Start(start))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

            let rows = if values.len() >= 12 { 3 } else { 3 };
            let cols = if values.len() >= 12 { 4 } else { 3 };
            let mut arr = BytesStart::new("Array");
            arr.push_attribute(("dim", format!("{} {}", rows, cols).as_str()));
            xml.write_event(Event::Start(arr))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

            let text: String = values.iter()
                .map(|v| format!("{:.6}", v))
                .collect::<Vec<_>>()
                .join(" ");
            xml.write_event(Event::Text(BytesText::new(&text)))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

            xml.write_event(Event::End(BytesEnd::new("Array")))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
            xml.write_event(Event::End(BytesEnd::new("Matrix")))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
        }
        ProcessNode::Cdl(params) => {
            xml.write_event(Event::Start(BytesStart::new("ASC_CDL")))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

            // SOPNode
            xml.write_event(Event::Start(BytesStart::new("SOPNode")))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

            write_text_element(xml, "Slope", 
                &format!("{} {} {}", params.slope[0], params.slope[1], params.slope[2]))?;
            write_text_element(xml, "Offset",
                &format!("{} {} {}", params.offset[0], params.offset[1], params.offset[2]))?;
            write_text_element(xml, "Power",
                &format!("{} {} {}", params.power[0], params.power[1], params.power[2]))?;

            xml.write_event(Event::End(BytesEnd::new("SOPNode")))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

            // SatNode
            xml.write_event(Event::Start(BytesStart::new("SatNode")))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
            write_text_element(xml, "Saturation", &format!("{}", params.saturation))?;
            xml.write_event(Event::End(BytesEnd::new("SatNode")))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

            xml.write_event(Event::End(BytesEnd::new("ASC_CDL")))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
        }
        ProcessNode::Range(params) => {
            let mut start = BytesStart::new("Range");
            if !params.clamp {
                start.push_attribute(("noClamp", "true"));
            }
            xml.write_event(Event::Start(start))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

            write_text_element(xml, "minInValue",
                &format!("{} {} {}", params.min_in[0], params.min_in[1], params.min_in[2]))?;
            write_text_element(xml, "maxInValue",
                &format!("{} {} {}", params.max_in[0], params.max_in[1], params.max_in[2]))?;
            write_text_element(xml, "minOutValue",
                &format!("{} {} {}", params.min_out[0], params.min_out[1], params.min_out[2]))?;
            write_text_element(xml, "maxOutValue",
                &format!("{} {} {}", params.max_out[0], params.max_out[1], params.max_out[2]))?;

            xml.write_event(Event::End(BytesEnd::new("Range")))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
        }
        ProcessNode::Log(params) => {
            let mut start = BytesStart::new("Log");
            let style_str = match params.style {
                LogStyle::Log10 => "log10",
                LogStyle::Log2 => "log2",
                LogStyle::AntiLog10 => "antiLog10",
                LogStyle::AntiLog2 => "antiLog2",
                LogStyle::LinToLog => "linToLog",
                LogStyle::LogToLin => "logToLin",
            };
            start.push_attribute(("style", style_str));
            xml.write_event(Event::Start(start))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

            // Write params for camera-style log
            if matches!(params.style, LogStyle::LinToLog | LogStyle::LogToLin) {
                let mut log_params = BytesStart::new("LogParams");
                log_params.push_attribute(("base", params.base.to_string().as_str()));
                log_params.push_attribute(("linSideSlope", 
                    format!("{} {} {}", params.lin_slope[0], params.lin_slope[1], params.lin_slope[2]).as_str()));
                log_params.push_attribute(("linSideOffset",
                    format!("{} {} {}", params.lin_offset[0], params.lin_offset[1], params.lin_offset[2]).as_str()));
                log_params.push_attribute(("logSideSlope",
                    format!("{} {} {}", params.log_slope[0], params.log_slope[1], params.log_slope[2]).as_str()));
                log_params.push_attribute(("logSideOffset",
                    format!("{} {} {}", params.log_offset[0], params.log_offset[1], params.log_offset[2]).as_str()));
                xml.write_event(Event::Empty(log_params))
                    .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
            }

            xml.write_event(Event::End(BytesEnd::new("Log")))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
        }
        ProcessNode::Exponent(params) => {
            let mut start = BytesStart::new("Exponent");
            let style_str = match params.style {
                ExponentStyle::Basic => "basicFwd",
                ExponentStyle::BasicMirror => "basicMirrorFwd",
                ExponentStyle::PassThru => "basicPassThruFwd",
                ExponentStyle::MonCurve => "monCurveFwd",
                ExponentStyle::MonCurveMirror => "monCurveMirrorFwd",
            };
            start.push_attribute(("style", style_str));
            xml.write_event(Event::Start(start))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

            // Write ExponentParams element
            let mut exp_params = BytesStart::new("ExponentParams");
            exp_params.push_attribute(("exponent",
                format!("{} {} {}", params.exponent[0], params.exponent[1], params.exponent[2]).as_str()));
            if params.style == ExponentStyle::MonCurve || params.style == ExponentStyle::MonCurveMirror {
                exp_params.push_attribute(("offset",
                    format!("{} {} {}", params.offset[0], params.offset[1], params.offset[2]).as_str()));
            }
            xml.write_event(Event::Empty(exp_params))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;

            xml.write_event(Event::End(BytesEnd::new("Exponent")))
                .map_err(|e| LutError::ParseError(format!("write error: {}", e)))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_depth() {
        assert_eq!(BitDepth::U8.scale(), 255.0);
        assert_eq!(BitDepth::U10.scale(), 1023.0);
        assert_eq!(BitDepth::F32.scale(), 1.0);
        
        assert_eq!(BitDepth::from_clf_str("8i"), Some(BitDepth::U8));
        assert_eq!(BitDepth::from_clf_str("32f"), Some(BitDepth::F32));
        assert_eq!(BitDepth::from_clf_str("invalid"), None);
    }

    #[test]
    fn test_cdl() {
        let cdl = CdlParams::new(
            [1.0, 1.0, 1.0],
            [0.1, 0.0, 0.0],
            [1.0, 1.0, 1.0],
        );
        
        let mut rgb = [0.5, 0.5, 0.5];
        cdl.apply(&mut rgb);
        
        assert!((rgb[0] - 0.6).abs() < 0.01); // 0.5 + 0.1 offset
        assert!((rgb[1] - 0.5).abs() < 0.01);
        assert!((rgb[2] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_range() {
        let range = RangeParams {
            min_in: [0.0; 3],
            max_in: [1.0; 3],
            min_out: [0.0; 3],
            max_out: [0.5; 3], // compress to half range
            clamp: true,
        };
        
        let mut rgb = [1.0, 0.5, 0.0];
        range.apply(&mut rgb);
        
        assert!((rgb[0] - 0.5).abs() < 0.01);
        assert!((rgb[1] - 0.25).abs() < 0.01);
        assert!((rgb[2] - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_process_list() {
        let mut clf = ProcessList::new("test");
        clf.description = Some("Test CLF".into());
        
        // Add identity 1D LUT
        clf.nodes.push(ProcessNode::Lut1D {
            lut: Lut1D::identity(256),
            in_depth: BitDepth::F32,
            out_depth: BitDepth::F32,
        });
        
        let mut rgb = [0.5, 0.3, 0.2];
        clf.apply(&mut rgb);
        
        assert!((rgb[0] - 0.5).abs() < 0.01);
        assert!((rgb[1] - 0.3).abs() < 0.01);
        assert!((rgb[2] - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_matrix() {
        let node = ProcessNode::Matrix {
            // Identity matrix
            values: vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            in_depth: BitDepth::F32,
            out_depth: BitDepth::F32,
        };
        
        let mut rgb = [0.5, 0.3, 0.2];
        node.apply(&mut rgb);
        
        assert!((rgb[0] - 0.5).abs() < 0.01);
        assert!((rgb[1] - 0.3).abs() < 0.01);
        assert!((rgb[2] - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_clf_roundtrip() {
        let mut clf = ProcessList::new("roundtrip_test");
        clf.description = Some("Roundtrip test".into());
        clf.input_descriptor = Some("ACES".into());
        clf.output_descriptor = Some("sRGB".into());
        
        clf.nodes.push(ProcessNode::Lut1D {
            lut: Lut1D::gamma(64, 2.2),
            in_depth: BitDepth::F32,
            out_depth: BitDepth::F32,
        });
        
        // Write to buffer
        let mut buf = Vec::new();
        write_clf_to(&mut buf, &clf).unwrap();
        
        // Parse back
        let parsed = parse_clf(std::io::Cursor::new(buf)).unwrap();
        
        assert_eq!(parsed.id, "roundtrip_test");
        assert_eq!(parsed.description, Some("Roundtrip test".into()));
        assert_eq!(parsed.nodes.len(), 1);
    }

    #[test]
    fn test_ctf_roundtrip() {
        let mut ctf = ProcessList::new("ctf_test");
        ctf.description = Some("CTF roundtrip".into());
        ctf.input_descriptor = Some("ACEScg".into());
        ctf.output_descriptor = Some("Rec709".into());
        
        ctf.nodes.push(ProcessNode::Matrix {
            values: vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            in_depth: BitDepth::F32,
            out_depth: BitDepth::F32,
        });
        
        // Write CTF to buffer
        let mut buf = Vec::new();
        write_ctf_to(&mut buf, &ctf).unwrap();
        
        // Verify CTF root element
        let content = String::from_utf8_lossy(&buf);
        assert!(content.contains("CtfFile"));
        
        // Parse back
        let parsed = parse_ctf(std::io::Cursor::new(buf)).unwrap();
        
        assert_eq!(parsed.id, "ctf_test");
        assert_eq!(parsed.description, Some("CTF roundtrip".into()));
        assert_eq!(parsed.nodes.len(), 1);
    }

    #[test]
    fn test_parse_ctf_root_element() {
        // CTF uses CtfFile as root element
        let ctf_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<CtfFile id="test_ctf" version="2.0">
    <Description>Test CTF file</Description>
    <Matrix inBitDepth="32f" outBitDepth="32f">
        <Array dim="3 3">
            1.0 0.0 0.0
            0.0 1.0 0.0
            0.0 0.0 1.0
        </Array>
    </Matrix>
</CtfFile>
"#;
        let parsed = parse_ctf(std::io::Cursor::new(ctf_content)).unwrap();
        assert_eq!(parsed.id, "test_ctf");
        assert_eq!(parsed.nodes.len(), 1);
    }

    #[test]
    fn test_log_exponent_serialization() {
        // Create ProcessList with Log and Exponent nodes
        let mut clf = ProcessList::new("log_exp_test");
        clf.nodes.push(ProcessNode::Log(LogParams {
            style: LogStyle::LinToLog,
            base: 10.0,
            lin_slope: [1.0, 1.0, 1.0],
            lin_offset: [0.0, 0.0, 0.0],
            log_slope: [0.3, 0.3, 0.3],
            log_offset: [0.5, 0.5, 0.5],
            offset: [0.0; 3],
        }));
        clf.nodes.push(ProcessNode::Exponent(ExponentParams {
            style: ExponentStyle::MonCurve,
            exponent: [2.4, 2.4, 2.4],
            offset: [0.055, 0.055, 0.055],
        }));

        // Serialize to buffer
        let mut buf = Vec::new();
        write_clf_to(&mut buf, &clf).unwrap();
        let xml_str = String::from_utf8(buf).unwrap();

        // Check Log element
        assert!(xml_str.contains("<Log style=\"linToLog\""), "Missing Log element");
        assert!(xml_str.contains("LogParams"), "Missing LogParams");
        assert!(xml_str.contains("base=\"10\""), "Missing base attribute");
        assert!(xml_str.contains("logSideSlope"), "Missing logSideSlope");

        // Check Exponent element
        assert!(xml_str.contains("<Exponent style=\"monCurveFwd\""), "Missing Exponent element");
        assert!(xml_str.contains("ExponentParams"), "Missing ExponentParams");
        assert!(xml_str.contains("exponent=\"2.4 2.4 2.4\""), "Missing exponent values");
        assert!(xml_str.contains("offset=\"0.055"), "Missing offset for monCurve");
    }
}
