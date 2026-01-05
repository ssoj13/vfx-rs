//! Transform definitions for color space conversions.
//!
//! Transforms define operations applied to pixel values:
//! - Matrix transforms (primaries conversion)
//! - Transfer function (OETF/EOTF)
//! - LUT application (1D, 3D)
//! - CDL (Color Decision List)
//! - And more...
//!
//! Transforms can be chained via `GroupTransform`.

use std::path::PathBuf;

/// Transform application direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransformDirection {
    /// Forward transform.
    #[default]
    Forward,
    /// Inverse transform.
    Inverse,
}

impl TransformDirection {
    /// Returns the opposite direction.
    #[inline]
    pub fn inverse(self) -> Self {
        match self {
            Self::Forward => Self::Inverse,
            Self::Inverse => Self::Forward,
        }
    }
}

/// Interpolation method for LUTs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Interpolation {
    /// Nearest neighbor.
    Nearest,
    /// Linear interpolation (default for 1D).
    #[default]
    Linear,
    /// Tetrahedral interpolation (default for 3D).
    Tetrahedral,
    /// Best available (context-dependent).
    Best,
}

/// Color transform definition.
///
/// This enum represents all transform types supported by OCIO.
#[derive(Debug, Clone)]
pub enum Transform {
    /// 4x4 matrix transform.
    Matrix(MatrixTransform),

    /// Transfer function (builtin).
    BuiltinTransfer(BuiltinTransferTransform),

    /// Exponent/gamma.
    Exponent(ExponentTransform),

    /// Log transform.
    Log(LogTransform),

    /// 1D LUT from file.
    FileTransform(FileTransform),

    /// CDL (slope/offset/power/sat).
    Cdl(CdlTransform),

    /// Range remapping.
    Range(RangeTransform),

    /// Fixed function (ACES specific).
    FixedFunction(FixedFunctionTransform),

    /// Exposure/contrast adjustment.
    ExposureContrast(ExposureContrastTransform),

    /// Reference to named color space.
    ColorSpace(ColorSpaceTransform),

    /// Reference to named look.
    Look(LookTransform),

    /// Reference to display/view.
    DisplayView(DisplayViewTransform),

    /// Group of chained transforms.
    Group(GroupTransform),

    /// Builtin transform by name (OCIO v2).
    Builtin(BuiltinTransform),

    /// Allocation transform (for GPU optimization).
    Allocation(AllocationTransform),

    /// Grading primary transform (lift/gamma/gain).
    GradingPrimary(GradingPrimaryTransform),

    /// Grading RGB curves.
    GradingRgbCurve(GradingRgbCurveTransform),

    /// Grading tone transform (shadows/midtones/highlights).
    GradingTone(GradingToneTransform),

    /// Inline 1D LUT transform.
    Lut1D(Lut1DTransform),

    /// Inline 3D LUT transform.
    Lut3D(Lut3DTransform),
}

impl Transform {
    /// Creates a matrix transform from a 4x4 array.
    pub fn matrix(m: [f64; 16]) -> Self {
        Self::Matrix(MatrixTransform {
            matrix: m,
            offset: [0.0; 4],
            direction: TransformDirection::Forward,
        })
    }

    /// Creates a group transform.
    pub fn group(transforms: Vec<Transform>) -> Self {
        Self::Group(GroupTransform {
            transforms,
            direction: TransformDirection::Forward,
        })
    }

    /// Creates a file transform (LUT reference).
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self::FileTransform(FileTransform {
            src: path.into(),
            ccc_id: None,
            interpolation: Interpolation::default(),
            direction: TransformDirection::Forward,
        })
    }

    /// Returns the inverse of this transform.
    pub fn inverse(self) -> Self {
        match self {
            Self::Matrix(mut t) => {
                t.direction = t.direction.inverse();
                Self::Matrix(t)
            }
            Self::BuiltinTransfer(mut t) => {
                t.direction = t.direction.inverse();
                Self::BuiltinTransfer(t)
            }
            Self::Exponent(mut t) => {
                t.direction = t.direction.inverse();
                Self::Exponent(t)
            }
            Self::Log(mut t) => {
                t.direction = t.direction.inverse();
                Self::Log(t)
            }
            Self::FileTransform(mut t) => {
                t.direction = t.direction.inverse();
                Self::FileTransform(t)
            }
            Self::Cdl(mut t) => {
                t.direction = t.direction.inverse();
                Self::Cdl(t)
            }
            Self::Range(mut t) => {
                t.direction = t.direction.inverse();
                Self::Range(t)
            }
            Self::FixedFunction(mut t) => {
                t.direction = t.direction.inverse();
                Self::FixedFunction(t)
            }
            Self::ExposureContrast(mut t) => {
                t.direction = t.direction.inverse();
                Self::ExposureContrast(t)
            }
            Self::ColorSpace(mut t) => {
                t.direction = t.direction.inverse();
                Self::ColorSpace(t)
            }
            Self::Look(mut t) => {
                t.direction = t.direction.inverse();
                Self::Look(t)
            }
            Self::DisplayView(mut t) => {
                t.direction = t.direction.inverse();
                Self::DisplayView(t)
            }
            Self::Group(mut t) => {
                t.direction = t.direction.inverse();
                t.transforms.reverse();
                Self::Group(t)
            }
            Self::Builtin(mut t) => {
                t.direction = t.direction.inverse();
                Self::Builtin(t)
            }
            Self::Allocation(mut t) => {
                t.direction = t.direction.inverse();
                Self::Allocation(t)
            }
            Self::GradingPrimary(mut t) => {
                t.direction = t.direction.inverse();
                Self::GradingPrimary(t)
            }
            Self::GradingRgbCurve(mut t) => {
                t.direction = t.direction.inverse();
                Self::GradingRgbCurve(t)
            }
            Self::GradingTone(mut t) => {
                t.direction = t.direction.inverse();
                Self::GradingTone(t)
            }
            Self::Lut1D(mut t) => {
                t.direction = t.direction.inverse();
                Self::Lut1D(t)
            }
            Self::Lut3D(mut t) => {
                t.direction = t.direction.inverse();
                Self::Lut3D(t)
            }
        }
    }
}

/// 4x4 matrix + offset transform.
#[derive(Debug, Clone)]
pub struct MatrixTransform {
    /// 4x4 matrix in row-major order.
    pub matrix: [f64; 16],
    /// RGBA offset.
    pub offset: [f64; 4],
    /// Direction.
    pub direction: TransformDirection,
}

impl MatrixTransform {
    /// Identity matrix.
    pub const IDENTITY: [f64; 16] = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
}

/// Builtin transfer function transform.
#[derive(Debug, Clone)]
pub struct BuiltinTransferTransform {
    /// Transfer function name (sRGB, Rec709, PQ, HLG, etc.).
    pub style: String,
    /// Direction.
    pub direction: TransformDirection,
}

/// Exponent/gamma transform.
#[derive(Debug, Clone)]
pub struct ExponentTransform {
    /// Per-channel exponents [R, G, B, A].
    pub value: [f64; 4],
    /// Negative handling style.
    pub negative_style: NegativeStyle,
    /// Direction.
    pub direction: TransformDirection,
}

/// Negative value handling for exponent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NegativeStyle {
    /// Clamp negatives to zero.
    #[default]
    Clamp,
    /// Mirror: sign * pow(abs(x), exp).
    Mirror,
    /// Pass through unchanged.
    PassThru,
}

/// Log transform (lin-to-log or log-to-lin).
#[derive(Debug, Clone)]
pub struct LogTransform {
    /// Base of logarithm (10 or 2).
    pub base: f64,
    /// Direction.
    pub direction: TransformDirection,
}

/// File-based transform (LUT, etc.).
#[derive(Debug, Clone)]
pub struct FileTransform {
    /// Source file path.
    pub src: PathBuf,
    /// CDL correction ID (for .ccc/.cdl files).
    pub ccc_id: Option<String>,
    /// Interpolation method.
    pub interpolation: Interpolation,
    /// Direction.
    pub direction: TransformDirection,
}

/// CDL (ASC Color Decision List) transform.
#[derive(Debug, Clone)]
pub struct CdlTransform {
    /// Per-channel slope [R, G, B].
    pub slope: [f64; 3],
    /// Per-channel offset [R, G, B].
    pub offset: [f64; 3],
    /// Per-channel power [R, G, B].
    pub power: [f64; 3],
    /// Saturation (1.0 = no change).
    pub saturation: f64,
    /// CDL style.
    pub style: CdlStyle,
    /// Direction.
    pub direction: TransformDirection,
}

/// CDL style (order of operations).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CdlStyle {
    /// ASC CDL v1.2 standard (default).
    #[default]
    AscCdl,
    /// No clamping.
    NoClamp,
}

impl Default for CdlTransform {
    fn default() -> Self {
        Self {
            slope: [1.0, 1.0, 1.0],
            offset: [0.0, 0.0, 0.0],
            power: [1.0, 1.0, 1.0],
            saturation: 1.0,
            style: CdlStyle::default(),
            direction: TransformDirection::Forward,
        }
    }
}

/// Range remapping transform.
#[derive(Debug, Clone)]
pub struct RangeTransform {
    /// Input min (None = no clamping).
    pub min_in: Option<f64>,
    /// Input max.
    pub max_in: Option<f64>,
    /// Output min.
    pub min_out: Option<f64>,
    /// Output max.
    pub max_out: Option<f64>,
    /// Style.
    pub style: RangeStyle,
    /// Direction.
    pub direction: TransformDirection,
}

/// Range transform style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RangeStyle {
    /// Clamp values to range.
    #[default]
    Clamp,
    /// No clamping, just scale.
    NoClamp,
}

/// Fixed function transform (ACES-specific operations).
#[derive(Debug, Clone)]
pub struct FixedFunctionTransform {
    /// Function style.
    pub style: FixedFunctionStyle,
    /// Optional parameters.
    pub params: Vec<f64>,
    /// Direction.
    pub direction: TransformDirection,
}

/// Fixed function styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedFunctionStyle {
    /// ACES red modifier.
    AcesRedMod03,
    /// ACES red modifier (improved).
    AcesRedMod10,
    /// ACES glow.
    AcesGlow03,
    /// ACES glow (improved).
    AcesGlow10,
    /// ACES gamut compress.
    AcesGamutComp13,
    /// RGB to HSV.
    RgbToHsv,
    /// HSV to RGB.
    HsvToRgb,
    /// XYZ to xyY.
    XyzToXyy,
    /// xyY to XYZ.
    XyyToXyz,
    /// XYZ to uvY.
    XyzToUvy,
    /// uvY to XYZ.
    UvyToXyz,
    /// XYZ to Luv.
    XyzToLuv,
    /// Luv to XYZ.
    LuvToXyz,
}

/// Exposure/contrast adjustment.
#[derive(Debug, Clone)]
pub struct ExposureContrastTransform {
    /// Exposure in stops.
    pub exposure: f64,
    /// Contrast (1.0 = no change).
    pub contrast: f64,
    /// Gamma (1.0 = no change).
    pub gamma: f64,
    /// Pivot point for contrast.
    pub pivot: f64,
    /// Style (linear or video).
    pub style: ExposureContrastStyle,
    /// Direction.
    pub direction: TransformDirection,
}

/// Exposure/contrast style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExposureContrastStyle {
    /// Linear domain.
    #[default]
    Linear,
    /// Video/log domain.
    Video,
    /// Logarithmic.
    Logarithmic,
}

/// Reference to a named color space.
#[derive(Debug, Clone)]
pub struct ColorSpaceTransform {
    /// Source color space name.
    pub src: String,
    /// Destination color space name.
    pub dst: String,
    /// Direction.
    pub direction: TransformDirection,
}

/// Reference to a named look.
#[derive(Debug, Clone)]
pub struct LookTransform {
    /// Source color space.
    pub src: String,
    /// Destination color space.
    pub dst: String,
    /// Look names (comma-separated for multiple).
    pub looks: String,
    /// Direction.
    pub direction: TransformDirection,
}

/// Display/view transform reference.
#[derive(Debug, Clone)]
pub struct DisplayViewTransform {
    /// Source color space.
    pub src: String,
    /// Display name.
    pub display: String,
    /// View name.
    pub view: String,
    /// Direction.
    pub direction: TransformDirection,
}

/// Group of chained transforms.
#[derive(Debug, Clone)]
pub struct GroupTransform {
    /// Ordered list of transforms.
    pub transforms: Vec<Transform>,
    /// Direction (affects iteration order).
    pub direction: TransformDirection,
}

/// Builtin transform by name (OCIO v2).
#[derive(Debug, Clone)]
pub struct BuiltinTransform {
    /// Builtin name (e.g., "ACEScct_to_ACES2065-1").
    pub style: String,
    /// Direction.
    pub direction: TransformDirection,
}

/// Allocation transform for GPU optimization.
#[derive(Debug, Clone)]
pub struct AllocationTransform {
    /// Allocation type.
    pub allocation: AllocationType,
    /// Allocation variables (min, max, etc.).
    pub vars: Vec<f64>,
    /// Direction.
    pub direction: TransformDirection,
}

/// Allocation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AllocationType {
    /// Uniform allocation.
    #[default]
    Uniform,
    /// Logarithmic allocation.
    Log2,
}

/// Grading primary transform (lift/gamma/gain).
///
/// Applies color correction using the standard lift/gamma/gain model:
/// - Lift: Adjusts shadows (added to dark values)
/// - Gamma: Adjusts midtones (power curve)
/// - Gain: Adjusts highlights (multiplier)
#[derive(Debug, Clone)]
pub struct GradingPrimaryTransform {
    /// Per-channel lift [R, G, B].
    pub lift: [f64; 3],
    /// Per-channel gamma [R, G, B].
    pub gamma: [f64; 3],
    /// Per-channel gain [R, G, B].
    pub gain: [f64; 3],
    /// Master offset.
    pub offset: f64,
    /// Master exposure (stops).
    pub exposure: f64,
    /// Master contrast.
    pub contrast: f64,
    /// Master saturation.
    pub saturation: f64,
    /// Pivot point for contrast.
    pub pivot: f64,
    /// Clamp black level.
    pub clamp_black: Option<f64>,
    /// Clamp white level.
    pub clamp_white: Option<f64>,
    /// Direction.
    pub direction: TransformDirection,
}

impl Default for GradingPrimaryTransform {
    fn default() -> Self {
        Self {
            lift: [0.0, 0.0, 0.0],
            gamma: [1.0, 1.0, 1.0],
            gain: [1.0, 1.0, 1.0],
            offset: 0.0,
            exposure: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            pivot: 0.18,
            clamp_black: None,
            clamp_white: None,
            direction: TransformDirection::Forward,
        }
    }
}

/// Grading RGB curve transform.
///
/// Applies per-channel curves for color correction.
#[derive(Debug, Clone)]
pub struct GradingRgbCurveTransform {
    /// Red channel control points [(x, y), ...].
    pub red: Vec<[f64; 2]>,
    /// Green channel control points.
    pub green: Vec<[f64; 2]>,
    /// Blue channel control points.
    pub blue: Vec<[f64; 2]>,
    /// Master (luminance) control points.
    pub master: Vec<[f64; 2]>,
    /// Direction.
    pub direction: TransformDirection,
}

impl Default for GradingRgbCurveTransform {
    fn default() -> Self {
        // Identity curve: (0,0) -> (1,1)
        let identity = vec![[0.0, 0.0], [1.0, 1.0]];
        Self {
            red: identity.clone(),
            green: identity.clone(),
            blue: identity.clone(),
            master: identity,
            direction: TransformDirection::Forward,
        }
    }
}

/// Grading tone transform.
///
/// Applies tonal adjustments (shadows, midtones, highlights).
#[derive(Debug, Clone)]
pub struct GradingToneTransform {
    /// Shadows RGB + Master.
    pub shadows: [f64; 4],
    /// Midtones RGB + Master.
    pub midtones: [f64; 4],
    /// Highlights RGB + Master.
    pub highlights: [f64; 4],
    /// White point RGB + Master.
    pub whites: [f64; 4],
    /// Black point RGB + Master.
    pub blacks: [f64; 4],
    /// Shadow start.
    pub shadow_start: f64,
    /// Shadow pivot.
    pub shadow_pivot: f64,
    /// Highlight start.
    pub highlight_start: f64,
    /// Highlight pivot.
    pub highlight_pivot: f64,
    /// Direction.
    pub direction: TransformDirection,
}

impl Default for GradingToneTransform {
    fn default() -> Self {
        Self {
            shadows: [1.0, 1.0, 1.0, 1.0],
            midtones: [1.0, 1.0, 1.0, 1.0],
            highlights: [1.0, 1.0, 1.0, 1.0],
            whites: [1.0, 1.0, 1.0, 1.0],
            blacks: [0.0, 0.0, 0.0, 0.0],
            shadow_start: 0.0,
            shadow_pivot: 0.09,
            highlight_start: 0.5,
            highlight_pivot: 0.89,
            direction: TransformDirection::Forward,
        }
    }
}

/// Inline 1D LUT transform.
///
/// Contains embedded 1D LUT data rather than a file reference.
/// This is useful for procedurally generated LUTs or when the
/// LUT data needs to be stored directly in the configuration.
///
/// # Structure
///
/// - Data stored per channel (R, G, B) or as single shared curve
/// - Linear interpolation between entries
/// - Configurable input/output ranges
#[derive(Debug, Clone)]
pub struct Lut1DTransform {
    /// Red channel LUT data.
    pub red: Vec<f32>,
    /// Green channel LUT data (if None, uses red).
    pub green: Option<Vec<f32>>,
    /// Blue channel LUT data (if None, uses red).
    pub blue: Option<Vec<f32>>,
    /// Input domain minimum.
    pub input_min: f32,
    /// Input domain maximum.
    pub input_max: f32,
    /// Output range minimum (for half-domain LUTs).
    pub output_min: f32,
    /// Output range maximum.
    pub output_max: f32,
    /// Whether input values use half-domain (for HDR).
    pub half_domain: bool,
    /// Whether output uses raw half encoding.
    pub raw_halfs: bool,
    /// Interpolation method.
    pub interpolation: Interpolation,
    /// Direction.
    pub direction: TransformDirection,
}

impl Lut1DTransform {
    /// Creates a new 1D LUT transform with identity values.
    pub fn identity(size: usize) -> Self {
        let data: Vec<f32> = (0..size)
            .map(|i| i as f32 / (size - 1) as f32)
            .collect();
        Self {
            red: data,
            green: None,
            blue: None,
            input_min: 0.0,
            input_max: 1.0,
            output_min: 0.0,
            output_max: 1.0,
            half_domain: false,
            raw_halfs: false,
            interpolation: Interpolation::Linear,
            direction: TransformDirection::Forward,
        }
    }

    /// Creates a 1D LUT from raw data.
    pub fn from_data(data: Vec<f32>) -> Self {
        Self {
            red: data,
            green: None,
            blue: None,
            input_min: 0.0,
            input_max: 1.0,
            output_min: 0.0,
            output_max: 1.0,
            half_domain: false,
            raw_halfs: false,
            interpolation: Interpolation::Linear,
            direction: TransformDirection::Forward,
        }
    }

    /// Creates a 1D LUT with separate RGB channels.
    pub fn from_rgb(red: Vec<f32>, green: Vec<f32>, blue: Vec<f32>) -> Self {
        Self {
            red,
            green: Some(green),
            blue: Some(blue),
            input_min: 0.0,
            input_max: 1.0,
            output_min: 0.0,
            output_max: 1.0,
            half_domain: false,
            raw_halfs: false,
            interpolation: Interpolation::Linear,
            direction: TransformDirection::Forward,
        }
    }

    /// Returns the LUT size.
    pub fn size(&self) -> usize {
        self.red.len()
    }

    /// Returns whether this is a single-channel (mono) LUT.
    pub fn is_mono(&self) -> bool {
        self.green.is_none()
    }

    /// Applies the 1D LUT to a single value.
    pub fn apply(&self, value: f32) -> f32 {
        self.interpolate_channel(&self.red, value)
    }

    /// Applies the 1D LUT to RGB values.
    pub fn apply_rgb(&self, rgb: [f32; 3]) -> [f32; 3] {
        let r = self.interpolate_channel(&self.red, rgb[0]);
        let g = self.interpolate_channel(
            self.green.as_ref().unwrap_or(&self.red),
            rgb[1],
        );
        let b = self.interpolate_channel(
            self.blue.as_ref().unwrap_or(&self.red),
            rgb[2],
        );
        [r, g, b]
    }

    fn interpolate_channel(&self, data: &[f32], value: f32) -> f32 {
        if data.is_empty() {
            return value;
        }

        let size = data.len();
        let range = self.input_max - self.input_min;
        let t = if range.abs() < 1e-10 {
            0.0
        } else {
            ((value - self.input_min) / range).clamp(0.0, 1.0)
        };

        let idx_f = t * (size - 1) as f32;
        let idx0 = (idx_f.floor() as usize).min(size - 1);
        let idx1 = (idx0 + 1).min(size - 1);
        let frac = idx_f - idx0 as f32;

        match self.interpolation {
            Interpolation::Nearest => data[idx0],
            _ => data[idx0] * (1.0 - frac) + data[idx1] * frac,
        }
    }
}

/// Inline 3D LUT transform.
///
/// Contains embedded 3D LUT data rather than a file reference.
/// 3D LUTs map RGB input to RGB output through a cube of color values.
///
/// # Structure
///
/// - Data stored as cube of RGB triplets
/// - Size^3 entries in R-major order
/// - Supports trilinear and tetrahedral interpolation
#[derive(Debug, Clone)]
pub struct Lut3DTransform {
    /// LUT data: RGB triplets in R-major order.
    pub data: Vec<[f32; 3]>,
    /// Cube size (e.g., 17, 33, 65).
    pub size: usize,
    /// Input domain minimum per channel.
    pub domain_min: [f32; 3],
    /// Input domain maximum per channel.
    pub domain_max: [f32; 3],
    /// Interpolation method.
    pub interpolation: Interpolation,
    /// Direction.
    pub direction: TransformDirection,
}

impl Lut3DTransform {
    /// Creates a new 3D LUT transform with identity values.
    pub fn identity(size: usize) -> Self {
        let total = size * size * size;
        let mut data = Vec::with_capacity(total);

        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    let rf = r as f32 / (size - 1) as f32;
                    let gf = g as f32 / (size - 1) as f32;
                    let bf = b as f32 / (size - 1) as f32;
                    data.push([rf, gf, bf]);
                }
            }
        }

        Self {
            data,
            size,
            domain_min: [0.0, 0.0, 0.0],
            domain_max: [1.0, 1.0, 1.0],
            interpolation: Interpolation::Tetrahedral,
            direction: TransformDirection::Forward,
        }
    }

    /// Creates a 3D LUT from raw data.
    ///
    /// Data must be in R-major order with exactly size^3 entries.
    pub fn from_data(data: Vec<[f32; 3]>, size: usize) -> Option<Self> {
        let expected = size * size * size;
        if data.len() != expected {
            return None;
        }
        Some(Self {
            data,
            size,
            domain_min: [0.0, 0.0, 0.0],
            domain_max: [1.0, 1.0, 1.0],
            interpolation: Interpolation::Tetrahedral,
            direction: TransformDirection::Forward,
        })
    }

    /// Sets the input domain.
    pub fn with_domain(mut self, min: [f32; 3], max: [f32; 3]) -> Self {
        self.domain_min = min;
        self.domain_max = max;
        self
    }

    /// Sets the interpolation method.
    pub fn with_interpolation(mut self, interp: Interpolation) -> Self {
        self.interpolation = interp;
        self
    }

    /// Returns the total number of entries.
    pub fn entry_count(&self) -> usize {
        self.size * self.size * self.size
    }

    /// Gets value at grid position (r, g, b).
    fn get(&self, r: usize, g: usize, b: usize) -> [f32; 3] {
        let idx = b * self.size * self.size + g * self.size + r;
        self.data[idx]
    }

    /// Applies the 3D LUT to RGB values.
    pub fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        match self.interpolation {
            Interpolation::Nearest => self.apply_nearest(rgb),
            Interpolation::Linear => self.apply_trilinear(rgb),
            Interpolation::Tetrahedral | Interpolation::Best => self.apply_tetrahedral(rgb),
        }
    }

    fn normalize(&self, rgb: [f32; 3]) -> (f32, f32, f32) {
        let r = ((rgb[0] - self.domain_min[0]) / (self.domain_max[0] - self.domain_min[0])).clamp(0.0, 1.0);
        let g = ((rgb[1] - self.domain_min[1]) / (self.domain_max[1] - self.domain_min[1])).clamp(0.0, 1.0);
        let b = ((rgb[2] - self.domain_min[2]) / (self.domain_max[2] - self.domain_min[2])).clamp(0.0, 1.0);
        (r, g, b)
    }

    fn apply_nearest(&self, rgb: [f32; 3]) -> [f32; 3] {
        let (r, g, b) = self.normalize(rgb);
        let n = (self.size - 1) as f32;
        let ri = (r * n).round() as usize;
        let gi = (g * n).round() as usize;
        let bi = (b * n).round() as usize;
        self.get(
            ri.min(self.size - 1),
            gi.min(self.size - 1),
            bi.min(self.size - 1),
        )
    }

    fn apply_trilinear(&self, rgb: [f32; 3]) -> [f32; 3] {
        let (r, g, b) = self.normalize(rgb);
        let n = (self.size - 1) as f32;

        let ri = ((r * n).floor() as usize).min(self.size - 2);
        let gi = ((g * n).floor() as usize).min(self.size - 2);
        let bi = ((b * n).floor() as usize).min(self.size - 2);

        let rf = r * n - ri as f32;
        let gf = g * n - gi as f32;
        let bf = b * n - bi as f32;

        let c000 = self.get(ri, gi, bi);
        let c100 = self.get(ri + 1, gi, bi);
        let c010 = self.get(ri, gi + 1, bi);
        let c110 = self.get(ri + 1, gi + 1, bi);
        let c001 = self.get(ri, gi, bi + 1);
        let c101 = self.get(ri + 1, gi, bi + 1);
        let c011 = self.get(ri, gi + 1, bi + 1);
        let c111 = self.get(ri + 1, gi + 1, bi + 1);

        let mut result = [0.0f32; 3];
        for i in 0..3 {
            let c00 = c000[i] * (1.0 - rf) + c100[i] * rf;
            let c01 = c001[i] * (1.0 - rf) + c101[i] * rf;
            let c10 = c010[i] * (1.0 - rf) + c110[i] * rf;
            let c11 = c011[i] * (1.0 - rf) + c111[i] * rf;
            let c0 = c00 * (1.0 - gf) + c10 * gf;
            let c1 = c01 * (1.0 - gf) + c11 * gf;
            result[i] = c0 * (1.0 - bf) + c1 * bf;
        }
        result
    }

    fn apply_tetrahedral(&self, rgb: [f32; 3]) -> [f32; 3] {
        let (r, g, b) = self.normalize(rgb);
        let n = (self.size - 1) as f32;

        let ri = ((r * n).floor() as usize).min(self.size - 2);
        let gi = ((g * n).floor() as usize).min(self.size - 2);
        let bi = ((b * n).floor() as usize).min(self.size - 2);

        let rf = r * n - ri as f32;
        let gf = g * n - gi as f32;
        let bf = b * n - bi as f32;

        let c000 = self.get(ri, gi, bi);
        let c100 = self.get(ri + 1, gi, bi);
        let c010 = self.get(ri, gi + 1, bi);
        let c110 = self.get(ri + 1, gi + 1, bi);
        let c001 = self.get(ri, gi, bi + 1);
        let c101 = self.get(ri + 1, gi, bi + 1);
        let c011 = self.get(ri, gi + 1, bi + 1);
        let c111 = self.get(ri + 1, gi + 1, bi + 1);

        let mut result = [0.0f32; 3];
        for i in 0..3 {
            result[i] = if rf > gf {
                if gf > bf {
                    c000[i] + rf * (c100[i] - c000[i]) + gf * (c110[i] - c100[i]) + bf * (c111[i] - c110[i])
                } else if rf > bf {
                    c000[i] + rf * (c100[i] - c000[i]) + bf * (c101[i] - c100[i]) + gf * (c111[i] - c101[i])
                } else {
                    c000[i] + bf * (c001[i] - c000[i]) + rf * (c101[i] - c001[i]) + gf * (c111[i] - c101[i])
                }
            } else if gf > bf {
                if rf > bf {
                    c000[i] + gf * (c010[i] - c000[i]) + rf * (c110[i] - c010[i]) + bf * (c111[i] - c110[i])
                } else {
                    c000[i] + gf * (c010[i] - c000[i]) + bf * (c011[i] - c010[i]) + rf * (c111[i] - c011[i])
                }
            } else {
                c000[i] + bf * (c001[i] - c000[i]) + gf * (c011[i] - c001[i]) + rf * (c111[i] - c011[i])
            };
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_inverse() {
        assert_eq!(TransformDirection::Forward.inverse(), TransformDirection::Inverse);
        assert_eq!(TransformDirection::Inverse.inverse(), TransformDirection::Forward);
    }

    #[test]
    fn matrix_transform() {
        let t = Transform::matrix(MatrixTransform::IDENTITY);
        if let Transform::Matrix(m) = t {
            assert_eq!(m.matrix[0], 1.0);
            assert_eq!(m.direction, TransformDirection::Forward);
        }
    }

    #[test]
    fn group_inverse() {
        let g = Transform::group(vec![
            Transform::matrix(MatrixTransform::IDENTITY),
            Transform::file("test.spi1d"),
        ]);
        let inv = g.inverse();
        if let Transform::Group(g) = inv {
            assert_eq!(g.direction, TransformDirection::Inverse);
            assert_eq!(g.transforms.len(), 2);
        }
    }

    #[test]
    fn lut1d_identity() {
        let lut = Lut1DTransform::identity(256);
        assert_eq!(lut.size(), 256);
        assert!(lut.is_mono());

        // Identity should pass through values
        assert!((lut.apply(0.0) - 0.0).abs() < 0.01);
        assert!((lut.apply(0.5) - 0.5).abs() < 0.01);
        assert!((lut.apply(1.0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn lut1d_rgb() {
        let lut = Lut1DTransform::identity(256);
        let result = lut.apply_rgb([0.5, 0.3, 0.8]);
        assert!((result[0] - 0.5).abs() < 0.01);
        assert!((result[1] - 0.3).abs() < 0.01);
        assert!((result[2] - 0.8).abs() < 0.01);
    }

    #[test]
    fn lut1d_inverse() {
        let lut = Transform::Lut1D(Lut1DTransform::identity(256));
        let inv = lut.inverse();
        if let Transform::Lut1D(l) = inv {
            assert_eq!(l.direction, TransformDirection::Inverse);
        }
    }

    #[test]
    fn lut3d_identity() {
        let lut = Lut3DTransform::identity(17);
        assert_eq!(lut.size, 17);
        assert_eq!(lut.entry_count(), 17 * 17 * 17);

        // Identity should pass through values
        let result = lut.apply([0.5, 0.3, 0.8]);
        assert!((result[0] - 0.5).abs() < 0.1);
        assert!((result[1] - 0.3).abs() < 0.1);
        assert!((result[2] - 0.8).abs() < 0.1);
    }

    #[test]
    fn lut3d_corners() {
        let lut = Lut3DTransform::identity(33);

        // Black
        let black = lut.apply([0.0, 0.0, 0.0]);
        assert!(black[0].abs() < 0.01);

        // White
        let white = lut.apply([1.0, 1.0, 1.0]);
        assert!((white[0] - 1.0).abs() < 0.01);

        // Red
        let red = lut.apply([1.0, 0.0, 0.0]);
        assert!((red[0] - 1.0).abs() < 0.01);
        assert!(red[1].abs() < 0.01);
    }

    #[test]
    fn lut3d_from_data() {
        let data: Vec<[f32; 3]> = (0..8).map(|_| [0.5, 0.5, 0.5]).collect();
        let lut = Lut3DTransform::from_data(data, 2).unwrap();
        let result = lut.apply([0.5, 0.5, 0.5]);
        assert_eq!(result, [0.5, 0.5, 0.5]);
    }

    #[test]
    fn lut3d_inverse() {
        let lut = Transform::Lut3D(Lut3DTransform::identity(17));
        let inv = lut.inverse();
        if let Transform::Lut3D(l) = inv {
            assert_eq!(l.direction, TransformDirection::Inverse);
        }
    }
}
