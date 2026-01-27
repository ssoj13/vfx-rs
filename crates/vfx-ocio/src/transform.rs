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

    /// Exponent with linear segment (sRGB, Rec.709 style).
    ExponentWithLinear(ExponentWithLinearTransform),

    /// Log transform.
    Log(LogTransform),

    /// Log affine transform (OCIO v2).
    LogAffine(LogAffineTransform),

    /// Log camera transform (ACEScct, LogC, S-Log3).
    LogCamera(LogCameraTransform),

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

    /// Grading hue curve transform (8 hue-based curves).
    GradingHueCurve(GradingHueCurveTransform),

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
            Self::ExponentWithLinear(mut t) => {
                t.direction = t.direction.inverse();
                Self::ExponentWithLinear(t)
            }
            Self::Log(mut t) => {
                t.direction = t.direction.inverse();
                Self::Log(t)
            }
            Self::LogAffine(mut t) => {
                t.direction = t.direction.inverse();
                Self::LogAffine(t)
            }
            Self::LogCamera(mut t) => {
                t.direction = t.direction.inverse();
                Self::LogCamera(t)
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
            Self::GradingHueCurve(mut t) => {
                t.direction = t.direction.inverse();
                Self::GradingHueCurve(t)
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
    /// Clamp negatives to zero (default for ExponentTransform).
    #[default]
    Clamp,
    /// Mirror: sign * pow(abs(x), exp).
    Mirror,
    /// Pass through unchanged.
    PassThru,
    /// Linearly extrapolate for negatives (default for ExponentWithLinear).
    Linear,
}

/// Log transform (lin-to-log or log-to-lin).
#[derive(Debug, Clone)]
pub struct LogTransform {
    /// Base of logarithm (10 or 2).
    pub base: f64,
    /// Direction.
    pub direction: TransformDirection,
}

/// Log affine transform (OCIO v2).
///
/// Applies a logarithmic curve with affine parameters per channel.
/// Formula: out = logSideSlope * log(linSideSlope * in + linSideOffset, base) + logSideOffset
#[derive(Debug, Clone)]
pub struct LogAffineTransform {
    /// Logarithm base (typically 2 or 10).
    pub base: f64,
    /// Log side slope per channel [R, G, B].
    pub log_side_slope: [f64; 3],
    /// Log side offset per channel [R, G, B].
    pub log_side_offset: [f64; 3],
    /// Linear side slope per channel [R, G, B].
    pub lin_side_slope: [f64; 3],
    /// Linear side offset per channel [R, G, B].
    pub lin_side_offset: [f64; 3],
    /// Direction.
    pub direction: TransformDirection,
}

impl Default for LogAffineTransform {
    fn default() -> Self {
        Self {
            base: 2.0,
            log_side_slope: [1.0, 1.0, 1.0],
            log_side_offset: [0.0, 0.0, 0.0],
            lin_side_slope: [1.0, 1.0, 1.0],
            lin_side_offset: [0.0, 0.0, 0.0],
            direction: TransformDirection::Forward,
        }
    }
}

impl LogAffineTransform {
    /// Creates a new LogAffineTransform with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the base.
    pub fn with_base(mut self, base: f64) -> Self {
        self.base = base;
        self
    }

    /// Sets log side parameters.
    pub fn with_log_side(mut self, slope: [f64; 3], offset: [f64; 3]) -> Self {
        self.log_side_slope = slope;
        self.log_side_offset = offset;
        self
    }

    /// Sets linear side parameters.
    pub fn with_lin_side(mut self, slope: [f64; 3], offset: [f64; 3]) -> Self {
        self.lin_side_slope = slope;
        self.lin_side_offset = offset;
        self
    }

    /// Applies the transform to RGB values.
    pub fn apply(&self, rgb: [f64; 3]) -> [f64; 3] {
        let mut out = [0.0; 3];
        let ln_base = self.base.ln();

        for i in 0..3 {
            let lin = self.lin_side_slope[i] * rgb[i] + self.lin_side_offset[i];
            if lin > 0.0 {
                out[i] = self.log_side_slope[i] * lin.ln() / ln_base + self.log_side_offset[i];
            } else {
                out[i] = self.log_side_offset[i];
            }
        }
        out
    }

    /// Applies the inverse transform.
    pub fn apply_inverse(&self, rgb: [f64; 3]) -> [f64; 3] {
        let mut out = [0.0; 3];
        let ln_base = self.base.ln();

        for i in 0..3 {
            let log_val = (rgb[i] - self.log_side_offset[i]) / self.log_side_slope[i];
            let lin = (log_val * ln_base).exp();
            out[i] = (lin - self.lin_side_offset[i]) / self.lin_side_slope[i];
        }
        out
    }
}

/// Log camera transform (OCIO v2).
///
/// Camera-specific log encoding with linear segment below break point.
/// Used for ACEScct, S-Log3, LogC, etc.
#[derive(Debug, Clone)]
pub struct LogCameraTransform {
    /// Logarithm base (typically 2 or 10).
    pub base: f64,
    /// Log side slope per channel [R, G, B].
    pub log_side_slope: [f64; 3],
    /// Log side offset per channel [R, G, B].
    pub log_side_offset: [f64; 3],
    /// Linear side slope per channel [R, G, B].
    pub lin_side_slope: [f64; 3],
    /// Linear side offset per channel [R, G, B].
    pub lin_side_offset: [f64; 3],
    /// Linear side break point per channel.
    /// Below this value, a linear segment is used.
    pub lin_side_break: [f64; 3],
    /// Linear slope below break point (calculated from continuity).
    pub linear_slope: Option<[f64; 3]>,
    /// Direction.
    pub direction: TransformDirection,
}

impl Default for LogCameraTransform {
    fn default() -> Self {
        Self {
            base: 2.0,
            log_side_slope: [1.0, 1.0, 1.0],
            log_side_offset: [0.0, 0.0, 0.0],
            lin_side_slope: [1.0, 1.0, 1.0],
            lin_side_offset: [0.0, 0.0, 0.0],
            lin_side_break: [0.0, 0.0, 0.0],
            linear_slope: None,
            direction: TransformDirection::Forward,
        }
    }
}

impl LogCameraTransform {
    /// Creates a new LogCameraTransform with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the base.
    pub fn with_base(mut self, base: f64) -> Self {
        self.base = base;
        self
    }

    /// Sets the linear side break point.
    pub fn with_lin_side_break(mut self, brk: [f64; 3]) -> Self {
        self.lin_side_break = brk;
        self
    }

    /// Sets the linear slope for the segment below break.
    pub fn with_linear_slope(mut self, slope: [f64; 3]) -> Self {
        self.linear_slope = Some(slope);
        self
    }

    /// Creates ACEScct-like transform.
    pub fn acescct() -> Self {
        Self {
            base: 2.0,
            log_side_slope: [17.52 / 9.72; 3],
            log_side_offset: [0.0729; 3],
            lin_side_slope: [1.0; 3],
            lin_side_offset: [9.72; 3],
            lin_side_break: [0.0078125; 3], // 2^-7
            linear_slope: Some([10.5; 3]),
            direction: TransformDirection::Forward,
        }
    }

    /// Creates ARRI LogC3-like transform.
    pub fn logc3() -> Self {
        Self {
            base: 10.0,
            log_side_slope: [0.247190; 3],
            log_side_offset: [0.385537; 3],
            lin_side_slope: [5.555556; 3],
            lin_side_offset: [0.052272; 3],
            lin_side_break: [0.010591; 3],
            linear_slope: Some([5.367655; 3]),
            direction: TransformDirection::Forward,
        }
    }

    /// Creates Sony S-Log3-like transform.
    pub fn slog3() -> Self {
        Self {
            base: 10.0,
            log_side_slope: [0.255620723; 3],
            log_side_offset: [0.41055718; 3],
            lin_side_slope: [5.26315789; 3],
            lin_side_offset: [0.052272; 3],
            lin_side_break: [0.01125; 3],
            linear_slope: None,
            direction: TransformDirection::Forward,
        }
    }

    /// Applies the transform to RGB values.
    pub fn apply(&self, rgb: [f64; 3]) -> [f64; 3] {
        let mut out = [0.0; 3];
        let ln_base = self.base.ln();

        for i in 0..3 {
            if rgb[i] <= self.lin_side_break[i] {
                // Linear segment
                let linear_slope = self.linear_slope
                    .map(|s| s[i])
                    .unwrap_or(self.log_side_slope[i] * self.lin_side_slope[i] /
                              ((self.lin_side_slope[i] * self.lin_side_break[i] + self.lin_side_offset[i]) * ln_base));
                out[i] = linear_slope * rgb[i];
            } else {
                // Log segment
                let lin = self.lin_side_slope[i] * rgb[i] + self.lin_side_offset[i];
                out[i] = self.log_side_slope[i] * lin.ln() / ln_base + self.log_side_offset[i];
            }
        }
        out
    }
}

/// Exponent with linear segment transform (OCIO v2).
///
/// Applies gamma with a linear segment in the shadows.
/// Commonly used for sRGB and Rec.709 transfer functions.
#[derive(Debug, Clone)]
pub struct ExponentWithLinearTransform {
    /// Exponent (gamma) per channel [R, G, B, A].
    pub gamma: [f64; 4],
    /// Offset per channel (for linear segment calculation).
    pub offset: [f64; 4],
    /// Negative value handling.
    pub negative_style: NegativeStyle,
    /// Direction.
    pub direction: TransformDirection,
}

impl Default for ExponentWithLinearTransform {
    fn default() -> Self {
        Self {
            gamma: [1.0, 1.0, 1.0, 1.0],
            offset: [0.0, 0.0, 0.0, 0.0],
            negative_style: NegativeStyle::Linear,
            direction: TransformDirection::Forward,
        }
    }
}

impl ExponentWithLinearTransform {
    /// Creates a new ExponentWithLinearTransform.
    pub fn new(gamma: [f64; 4], offset: [f64; 4]) -> Self {
        Self {
            gamma,
            offset,
            negative_style: NegativeStyle::Clamp,
            direction: TransformDirection::Forward,
        }
    }

    /// Creates sRGB transfer function.
    pub fn srgb() -> Self {
        Self {
            gamma: [2.4, 2.4, 2.4, 1.0],
            offset: [0.055, 0.055, 0.055, 0.0],
            negative_style: NegativeStyle::Clamp,
            direction: TransformDirection::Forward,
        }
    }

    /// Creates Rec.709 transfer function.
    pub fn rec709() -> Self {
        Self {
            gamma: [1.0 / 0.45, 1.0 / 0.45, 1.0 / 0.45, 1.0],
            offset: [0.099, 0.099, 0.099, 0.0],
            negative_style: NegativeStyle::Clamp,
            direction: TransformDirection::Forward,
        }
    }

    /// Sets negative handling style.
    pub fn with_negative_style(mut self, style: NegativeStyle) -> Self {
        self.negative_style = style;
        self
    }

    /// Calculates the break point for the linear segment.
    fn break_point(&self, channel: usize) -> f64 {
        let g = self.gamma[channel];
        let o = self.offset[channel];
        if o.abs() < 1e-10 || g.abs() < 1e-10 {
            return 0.0;
        }
        // Break point where derivative of power = derivative of linear
        // For (1+offset) * x^gamma - offset, the linear slope at 0 determines break
        o / (g * (1.0 + o).powf(g - 1.0) - (g - 1.0) * o)
    }

    /// Applies the transform to a single value.
    pub fn apply_channel(&self, value: f64, channel: usize) -> f64 {
        let g = self.gamma[channel];
        let o = self.offset[channel];

        if g.abs() < 1e-10 || (g - 1.0).abs() < 1e-10 {
            return value;
        }

        let brk = self.break_point(channel);
        let linear_slope = if brk.abs() > 1e-10 {
            ((1.0 + o) * brk.powf(g) - o) / brk
        } else {
            1.0
        };

        match self.negative_style {
            NegativeStyle::Clamp => {
                let v = value.max(0.0);
                if v <= brk {
                    linear_slope * v
                } else {
                    (1.0 + o) * v.powf(g) - o
                }
            }
            NegativeStyle::Mirror => {
                let sign = value.signum();
                let v = value.abs();
                let result = if v <= brk {
                    linear_slope * v
                } else {
                    (1.0 + o) * v.powf(g) - o
                };
                sign * result
            }
            NegativeStyle::PassThru => {
                if value < 0.0 {
                    value
                } else if value <= brk {
                    linear_slope * value
                } else {
                    (1.0 + o) * value.powf(g) - o
                }
            }
            NegativeStyle::Linear => {
                // Linear segment continues into negatives
                if value <= brk {
                    linear_slope * value
                } else {
                    (1.0 + o) * value.powf(g) - o
                }
            }
        }
    }

    /// Applies the transform to RGBA values.
    pub fn apply(&self, rgba: [f64; 4]) -> [f64; 4] {
        [
            self.apply_channel(rgba[0], 0),
            self.apply_channel(rgba[1], 1),
            self.apply_channel(rgba[2], 2),
            self.apply_channel(rgba[3], 3),
        ]
    }
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
    /// ACES red modifier (v0.3/0.7).
    AcesRedMod03,
    /// ACES red modifier (v1.0).
    AcesRedMod10,
    /// ACES glow (v0.3/0.7).
    AcesGlow03,
    /// ACES glow (v1.0).
    AcesGlow10,
    /// ACES dark-to-dim surround correction (v1.0). Param: gamma.
    AcesDarkToDim10,
    /// ACES gamut compress (v1.3).
    AcesGamutComp13,
    /// Rec.2100 surround correction. Param: gamma.
    Rec2100Surround,
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
    /// Linear to PQ (ST-2084). 1.0 = 100 nits.
    LinToPq,
    /// PQ to Linear.
    PqToLin,
    /// Linear to parameterized gamma+log curve. Params: [mirrorPt, breakPt, gamma, logSlope, linSlope, logOffset, linOffset].
    LinToGammaLog,
    /// Gamma+log to Linear (inverse).
    GammaLogToLin,
    /// Linear to double-log curve. Params: [base, breakPt, logSlope, logOffset, linSlope, linOffset, linearSlope, log2Slope, log2Offset].
    LinToDoubleLog,
    /// Double-log to Linear (inverse).
    DoubleLogToLin,
    /// ACES 2.0 Output Transform (experimental). Params: peak_lum.
    AcesOutputTransform20,
    /// ACES 2.0 RGB to JMh (experimental).
    AcesRgbToJmh20,
    /// ACES 2.0 JMh to RGB (experimental).
    AcesJmhToRgb20,
    /// ACES 2.0 Tonescale + chroma compress (experimental).
    AcesTonescaleCompress20,
    /// ACES 2.0 Gamut compress (experimental).
    AcesGamutCompress20,
    /// RGB to HSY (linear variant).
    RgbToHsyLin,
    /// HSY (linear) to RGB.
    HsyLinToRgb,
    /// RGB to HSY (log variant).
    RgbToHsyLog,
    /// HSY (log) to RGB.
    HsyLogToRgb,
    /// RGB to HSY (video variant).
    RgbToHsyVid,
    /// HSY (video) to RGB.
    HsyVidToRgb,
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

/// Grading hue curve transform.
///
/// Applies 8 curve types for hue-based color adjustments:
/// - HUE_HUE: shift hue based on input hue
/// - HUE_SAT: adjust saturation based on hue
/// - HUE_LUM: adjust luminance based on hue
/// - LUM_SAT: adjust saturation based on luminance
/// - SAT_SAT: adjust saturation based on saturation
/// - LUM_LUM: adjust luminance based on luminance
/// - SAT_LUM: adjust luminance based on saturation
/// - HUE_FX: special effects hue shift
///
/// Reference: OCIO GradingHueCurveTransform
#[derive(Debug, Clone)]
pub struct GradingHueCurveTransform {
    /// Grading style (Log, Linear, Video).
    pub style: GradingHueCurveStyle,
    /// HUE_HUE curve control points [(hue, shift), ...].
    pub hue_hue: Vec<[f64; 2]>,
    /// HUE_SAT curve control points [(hue, gain), ...].
    pub hue_sat: Vec<[f64; 2]>,
    /// HUE_LUM curve control points [(hue, gain), ...].
    pub hue_lum: Vec<[f64; 2]>,
    /// LUM_SAT curve control points [(lum, gain), ...].
    pub lum_sat: Vec<[f64; 2]>,
    /// SAT_SAT curve control points [(sat, sat), ...].
    pub sat_sat: Vec<[f64; 2]>,
    /// LUM_LUM curve control points [(lum, lum), ...].
    pub lum_lum: Vec<[f64; 2]>,
    /// SAT_LUM curve control points [(sat, gain), ...].
    pub sat_lum: Vec<[f64; 2]>,
    /// HUE_FX curve control points [(hue, shift), ...].
    pub hue_fx: Vec<[f64; 2]>,
    /// Direction.
    pub direction: TransformDirection,
}

/// Grading hue curve style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GradingHueCurveStyle {
    /// Log style (for log-encoded footage).
    #[default]
    Log,
    /// Linear style (for scene-linear footage).
    Linear,
    /// Video style (for display-referred footage).
    Video,
}

impl Default for GradingHueCurveTransform {
    fn default() -> Self {
        // Identity curves
        let hue_6pts_identity = |v: f64| vec![
            [0.0, v], [1.0/6.0, v], [2.0/6.0, v],
            [0.5, v], [4.0/6.0, v], [5.0/6.0, v],
        ];
        let hue_hue_identity = vec![
            [0.0, 0.0], [1.0/6.0, 1.0/6.0], [2.0/6.0, 2.0/6.0],
            [0.5, 0.5], [4.0/6.0, 4.0/6.0], [5.0/6.0, 5.0/6.0],
        ];
        let sat_diag = vec![[0.0, 0.0], [0.5, 0.5], [1.0, 1.0]];
        let lum_diag = vec![[0.0, 0.0], [0.5, 0.5], [1.0, 1.0]];
        let horiz_1 = vec![[0.0, 1.0], [0.5, 1.0], [1.0, 1.0]];

        Self {
            style: GradingHueCurveStyle::Log,
            hue_hue: hue_hue_identity,
            hue_sat: hue_6pts_identity(1.0),
            hue_lum: hue_6pts_identity(1.0),
            lum_sat: horiz_1.clone(),
            sat_sat: sat_diag,
            lum_lum: lum_diag,
            sat_lum: horiz_1,
            hue_fx: hue_6pts_identity(0.0),
            direction: TransformDirection::Forward,
        }
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
