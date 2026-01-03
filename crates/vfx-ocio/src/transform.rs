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
}
