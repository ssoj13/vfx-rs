//! Color transformation pipeline.
//!
//! A pipeline is a sequence of operations that transform RGB values.
//! Operations are applied in order:
//!
//! 1. Input transfer (EOTF - decode)
//! 2. Matrix transforms (color space conversion)
//! 3. 3D LUT (color grading)
//! 4. Output transfer (OETF - encode)
//!
//! # Example
//!
//! ```rust
//! use vfx_color::{Pipeline, TransformOp};
//! use vfx_color::transfer::srgb;
//! use vfx_color::primaries::{SRGB, rgb_to_xyz_matrix};
//! use vfx_math::Mat3;
//!
//! // sRGB -> Linear -> XYZ -> Linear sRGB
//! let pipeline = Pipeline::new()
//!     .push(TransformOp::TransferIn(srgb::eotf))
//!     .push(TransformOp::Matrix(rgb_to_xyz_matrix(&SRGB)))
//!     .push(TransformOp::Matrix(rgb_to_xyz_matrix(&SRGB).inverse().unwrap()));
//! ```

use vfx_lut::{Lut1D, Lut3D};
use vfx_math::{Mat3, Vec3};

/// Transfer function type (scalar to scalar).
pub type TransferFn = fn(f32) -> f32;

/// A single operation in the color pipeline.
///
/// Operations are applied in sequence to transform RGB values.
/// The order matters - typically: decode -> matrix -> LUT -> encode.
#[derive(Clone)]
pub enum TransformOp {
    /// Input transfer function (EOTF - decode from display).
    ///
    /// Applied to each channel independently.
    /// Example: sRGB EOTF converts display values to linear light.
    TransferIn(TransferFn),

    /// Output transfer function (OETF - encode for display).
    ///
    /// Applied to each channel independently.
    /// Example: sRGB OETF converts linear light to display values.
    TransferOut(TransferFn),

    /// 3x3 matrix transform.
    ///
    /// Applied as: `[R', G', B'] = M * [R, G, B]`
    /// Used for color space conversions (RGB <-> XYZ).
    Matrix(Mat3),

    /// 1D LUT (per-channel curve).
    ///
    /// Applied to each channel using the same or separate curves.
    Lut1D(Lut1D),

    /// 3D LUT (cube interpolation).
    ///
    /// Full RGB -> RGB mapping with trilinear or tetrahedral interpolation.
    Lut3D(Lut3D),

    /// Per-channel scale.
    ///
    /// `[R', G', B'] = [R*s[0], G*s[1], B*s[2]]`
    Scale([f32; 3]),

    /// Per-channel offset.
    ///
    /// `[R', G', B'] = [R+o[0], G+o[1], B+o[2]]`
    Offset([f32; 3]),

    /// Clamp to range.
    ///
    /// `[R', G', B'] = clamp([R, G, B], min, max)`
    Clamp { 
        /// Minimum value per channel.
        min: [f32; 3], 
        /// Maximum value per channel.
        max: [f32; 3] 
    },
}

impl std::fmt::Debug for TransformOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TransferIn(_) => f.write_str("TransferIn(fn)"),
            Self::TransferOut(_) => f.write_str("TransferOut(fn)"),
            Self::Matrix(m) => f.debug_tuple("Matrix").field(m).finish(),
            Self::Lut1D(lut) => f.debug_tuple("Lut1D").field(&lut.size()).finish(),
            Self::Lut3D(lut) => f.debug_tuple("Lut3D").field(&lut.size).finish(),
            Self::Scale(s) => f.debug_tuple("Scale").field(s).finish(),
            Self::Offset(o) => f.debug_tuple("Offset").field(o).finish(),
            Self::Clamp { min, max } => f.debug_struct("Clamp")
                .field("min", min)
                .field("max", max)
                .finish(),
        }
    }
}

/// A color transformation pipeline.
///
/// Stores a sequence of operations to transform RGB values.
/// Operations are applied in order, left to right.
///
/// # Example
///
/// ```rust
/// use vfx_color::Pipeline;
/// use vfx_color::transfer::{srgb, pq};
/// use vfx_color::primaries::{SRGB, REC2020, rgb_to_xyz_matrix, xyz_to_rgb_matrix};
///
/// // sRGB -> Linear -> Rec.2020 -> PQ
/// let pipeline = Pipeline::new()
///     .transfer_in(srgb::eotf)
///     .matrix(rgb_to_xyz_matrix(&SRGB))
///     .matrix(xyz_to_rgb_matrix(&REC2020))
///     .transfer_out(pq::oetf);
/// ```
#[derive(Debug, Clone, Default)]
pub struct Pipeline {
    ops: Vec<TransformOp>,
}

impl Pipeline {
    /// Creates an empty pipeline.
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    /// Creates a pipeline with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self { ops: Vec::with_capacity(capacity) }
    }

    /// Adds an operation to the pipeline.
    pub fn push(mut self, op: TransformOp) -> Self {
        self.ops.push(op);
        self
    }

    /// Adds an input transfer function (EOTF).
    ///
    /// Decodes from display encoding to linear light.
    pub fn transfer_in(self, f: TransferFn) -> Self {
        self.push(TransformOp::TransferIn(f))
    }

    /// Adds an output transfer function (OETF).
    ///
    /// Encodes from linear light to display encoding.
    pub fn transfer_out(self, f: TransferFn) -> Self {
        self.push(TransformOp::TransferOut(f))
    }

    /// Adds a matrix transform.
    ///
    /// Used for color space conversions.
    pub fn matrix(self, m: Mat3) -> Self {
        self.push(TransformOp::Matrix(m))
    }

    /// Adds a 1D LUT.
    pub fn lut1d(self, lut: Lut1D) -> Self {
        self.push(TransformOp::Lut1D(lut))
    }

    /// Adds a 3D LUT.
    pub fn lut3d(self, lut: Lut3D) -> Self {
        self.push(TransformOp::Lut3D(lut))
    }

    /// Adds a scale operation.
    pub fn scale(self, s: [f32; 3]) -> Self {
        self.push(TransformOp::Scale(s))
    }

    /// Adds an offset operation.
    pub fn offset(self, o: [f32; 3]) -> Self {
        self.push(TransformOp::Offset(o))
    }

    /// Adds a clamp operation.
    pub fn clamp(self, min: [f32; 3], max: [f32; 3]) -> Self {
        self.push(TransformOp::Clamp { min, max })
    }

    /// Adds a 0-1 clamp (common for display output).
    pub fn clamp_01(self) -> Self {
        self.clamp([0.0; 3], [1.0; 3])
    }

    /// Returns the number of operations in the pipeline.
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// Returns true if the pipeline is empty.
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Returns the operations in the pipeline.
    pub fn ops(&self) -> &[TransformOp] {
        &self.ops
    }

    /// Applies the pipeline to an RGB value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_color::Pipeline;
    /// use vfx_color::transfer::srgb;
    ///
    /// let pipeline = Pipeline::new()
    ///     .transfer_in(srgb::eotf)
    ///     .transfer_out(srgb::oetf);
    ///
    /// let rgb = [0.5, 0.3, 0.2];
    /// let result = pipeline.apply(rgb);
    /// // result should be approximately equal to input (round-trip)
    /// ```
    pub fn apply(&self, mut rgb: [f32; 3]) -> [f32; 3] {
        for op in &self.ops {
            rgb = match op {
                TransformOp::TransferIn(f) | TransformOp::TransferOut(f) => {
                    [f(rgb[0]), f(rgb[1]), f(rgb[2])]
                }
                TransformOp::Matrix(m) => {
                    let v = Vec3::from_array(rgb);
                    m.transform(v).to_array()
                }
                TransformOp::Lut1D(lut) => {
                    lut.apply_rgb(rgb)
                }
                TransformOp::Lut3D(lut) => {
                    lut.apply(rgb)
                }
                TransformOp::Scale(s) => {
                    [rgb[0] * s[0], rgb[1] * s[1], rgb[2] * s[2]]
                }
                TransformOp::Offset(o) => {
                    [rgb[0] + o[0], rgb[1] + o[1], rgb[2] + o[2]]
                }
                TransformOp::Clamp { min, max } => {
                    [
                        rgb[0].clamp(min[0], max[0]),
                        rgb[1].clamp(min[1], max[1]),
                        rgb[2].clamp(min[2], max[2]),
                    ]
                }
            };
        }
        rgb
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vfx_transfer::srgb;
    use vfx_primaries::{SRGB, rgb_to_xyz_matrix, xyz_to_rgb_matrix};

    #[test]
    fn test_empty_pipeline() {
        let pipeline = Pipeline::new();
        let rgb = [0.5, 0.3, 0.2];
        let result = pipeline.apply(rgb);
        assert_eq!(result, rgb);
    }

    #[test]
    fn test_transfer_roundtrip() {
        let pipeline = Pipeline::new()
            .transfer_in(srgb::eotf)
            .transfer_out(srgb::oetf);
        
        let rgb = [0.5, 0.3, 0.2];
        let result = pipeline.apply(rgb);
        
        assert!((result[0] - rgb[0]).abs() < 0.001);
        assert!((result[1] - rgb[1]).abs() < 0.001);
        assert!((result[2] - rgb[2]).abs() < 0.001);
    }

    #[test]
    fn test_matrix_roundtrip() {
        let pipeline = Pipeline::new()
            .matrix(rgb_to_xyz_matrix(&SRGB))
            .matrix(xyz_to_rgb_matrix(&SRGB));
        
        let rgb = [0.5, 0.3, 0.2];
        let result = pipeline.apply(rgb);
        
        assert!((result[0] - rgb[0]).abs() < 0.001);
        assert!((result[1] - rgb[1]).abs() < 0.001);
        assert!((result[2] - rgb[2]).abs() < 0.001);
    }

    #[test]
    fn test_scale_offset() {
        let pipeline = Pipeline::new()
            .scale([2.0, 2.0, 2.0])
            .offset([0.1, 0.1, 0.1]);
        
        let rgb = [0.5, 0.3, 0.2];
        let result = pipeline.apply(rgb);
        
        assert!((result[0] - 1.1).abs() < 0.001);
        assert!((result[1] - 0.7).abs() < 0.001);
        assert!((result[2] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_clamp() {
        let pipeline = Pipeline::new()
            .scale([2.0, 2.0, 2.0])
            .clamp_01();
        
        let rgb = [0.7, 0.3, 0.2];
        let result = pipeline.apply(rgb);
        
        assert_eq!(result[0], 1.0); // 1.4 clamped to 1.0
        assert!((result[1] - 0.6).abs() < 0.001);
        assert!((result[2] - 0.4).abs() < 0.001);
    }
}
