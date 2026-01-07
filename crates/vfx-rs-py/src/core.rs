//! Core types module - OIIO-compatible TypeDesc, ImageSpec, Roi3D.
//!
//! Provides foundational types for VFX image processing compatible with OpenImageIO.

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use std::collections::HashMap;

// =============================================================================
// BaseType Enum
// =============================================================================

/// Base data type for TypeDesc (matches OIIO BASETYPE).
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BaseType {
    /// Unknown or unspecified type
    Unknown = 0,
    /// No type (void)
    None = 1,
    /// 8-bit unsigned integer
    UInt8 = 2,
    /// 8-bit signed integer
    Int8 = 3,
    /// 16-bit unsigned integer
    UInt16 = 4,
    /// 16-bit signed integer
    Int16 = 5,
    /// 32-bit unsigned integer
    UInt32 = 6,
    /// 32-bit signed integer
    Int32 = 7,
    /// 64-bit unsigned integer
    UInt64 = 8,
    /// 64-bit signed integer
    Int64 = 9,
    /// 16-bit IEEE floating point (half)
    Half = 10,
    /// 32-bit IEEE floating point
    Float = 11,
    /// 64-bit IEEE floating point
    Double = 12,
    /// String type
    String = 13,
    /// Pointer type
    Ptr = 14,
}

#[pymethods]
impl BaseType {
    /// Size in bytes of one element.
    #[getter]
    fn size(&self) -> usize {
        match self {
            Self::Unknown | Self::None => 0,
            Self::UInt8 | Self::Int8 => 1,
            Self::UInt16 | Self::Int16 | Self::Half => 2,
            Self::UInt32 | Self::Int32 | Self::Float => 4,
            Self::UInt64 | Self::Int64 | Self::Double => 8,
            Self::String | Self::Ptr => 8, // pointer size
        }
    }

    /// Returns true if this is a floating-point type.
    fn is_floating_point(&self) -> bool {
        matches!(self, Self::Half | Self::Float | Self::Double)
    }

    /// Returns true if this is a signed type.
    fn is_signed(&self) -> bool {
        matches!(
            self,
            Self::Int8 | Self::Int16 | Self::Int32 | Self::Int64
            | Self::Half | Self::Float | Self::Double
        )
    }

    /// Returns true if this is an integer type.
    fn is_integer(&self) -> bool {
        matches!(
            self,
            Self::UInt8 | Self::Int8 | Self::UInt16 | Self::Int16
            | Self::UInt32 | Self::Int32 | Self::UInt64 | Self::Int64
        )
    }

    /// Short name for the type.
    #[getter]
    fn name(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::None => "none",
            Self::UInt8 => "uint8",
            Self::Int8 => "int8",
            Self::UInt16 => "uint16",
            Self::Int16 => "int16",
            Self::UInt32 => "uint32",
            Self::Int32 => "int32",
            Self::UInt64 => "uint64",
            Self::Int64 => "int64",
            Self::Half => "half",
            Self::Float => "float",
            Self::Double => "double",
            Self::String => "string",
            Self::Ptr => "ptr",
        }
    }

    fn __repr__(&self) -> String {
        format!("BaseType.{:?}", self)
    }
}

// =============================================================================
// Aggregate Enum
// =============================================================================

/// Aggregate type for TypeDesc (matches OIIO AGGREGATE).
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Aggregate {
    /// Single scalar value
    Scalar = 1,
    /// 2-component vector
    Vec2 = 2,
    /// 3-component vector
    Vec3 = 3,
    /// 4-component vector
    Vec4 = 4,
    /// 3x3 matrix (9 elements)
    Matrix33 = 9,
    /// 4x4 matrix (16 elements)
    Matrix44 = 16,
}

#[pymethods]
impl Aggregate {
    /// Number of base type elements in this aggregate.
    #[getter]
    fn count(&self) -> usize {
        match self {
            Self::Scalar => 1,
            Self::Vec2 => 2,
            Self::Vec3 => 3,
            Self::Vec4 => 4,
            Self::Matrix33 => 9,
            Self::Matrix44 => 16,
        }
    }

    /// Returns true if this is a scalar.
    fn is_scalar(&self) -> bool {
        matches!(self, Self::Scalar)
    }

    /// Returns true if this is a vector type.
    fn is_vector(&self) -> bool {
        matches!(self, Self::Vec2 | Self::Vec3 | Self::Vec4)
    }

    /// Returns true if this is a matrix type.
    fn is_matrix(&self) -> bool {
        matches!(self, Self::Matrix33 | Self::Matrix44)
    }

    fn __repr__(&self) -> String {
        format!("Aggregate.{:?}", self)
    }
}

// =============================================================================
// VecSemantics Enum
// =============================================================================

/// Vector semantics for TypeDesc (matches OIIO VECSEMANTICS).
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VecSemantics {
    /// No transformation needed
    NoSemantics = 0,
    /// Color value
    Color = 1,
    /// 3D point (full matrix transform)
    Point = 2,
    /// 3D direction vector (no translation)
    Vector = 3,
    /// Surface normal (inverse transpose)
    Normal = 4,
    /// SMPTE timecode
    Timecode = 5,
    /// Film keycode
    Keycode = 6,
    /// Rational number
    Rational = 7,
}

#[pymethods]
impl VecSemantics {
    /// Returns true if this requires special transformation handling.
    fn requires_transform(&self) -> bool {
        matches!(self, Self::Point | Self::Vector | Self::Normal)
    }

    fn __repr__(&self) -> String {
        format!("VecSemantics.{:?}", self)
    }
}

// =============================================================================
// DataFormat Enum
// =============================================================================

/// Runtime pixel data format.
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DataFormat {
    /// 8-bit unsigned integer
    U8 = 0,
    /// 16-bit unsigned integer
    #[default]
    U16 = 1,
    /// 32-bit unsigned integer
    U32 = 2,
    /// 16-bit half-precision float
    F16 = 3,
    /// 32-bit single-precision float
    F32 = 4,
}

#[pymethods]
impl DataFormat {
    /// Number of bytes per channel.
    #[getter]
    fn bytes_per_channel(&self) -> usize {
        match self {
            Self::U8 => 1,
            Self::U16 | Self::F16 => 2,
            Self::U32 | Self::F32 => 4,
        }
    }

    /// Number of bits per channel.
    #[getter]
    fn bits(&self) -> u32 {
        match self {
            Self::U8 => 8,
            Self::U16 | Self::F16 => 16,
            Self::U32 | Self::F32 => 32,
        }
    }

    /// Whether this is a floating-point format.
    fn is_float(&self) -> bool {
        matches!(self, Self::F16 | Self::F32)
    }

    /// Whether this is an integer format.
    fn is_integer(&self) -> bool {
        !self.is_float()
    }

    /// Short name for display.
    #[getter]
    fn name(&self) -> &'static str {
        match self {
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::F16 => "f16",
            Self::F32 => "f32",
        }
    }

    fn __repr__(&self) -> String {
        format!("DataFormat.{:?}", self)
    }
}

// =============================================================================
// TypeDesc
// =============================================================================

/// Complete type descriptor (matches OIIO TypeDesc).
///
/// Describes the complete type of a value including base type, aggregation,
/// array size, and vector semantics.
///
/// Example:
///     >>> td = TypeDesc(BaseType.Float, Aggregate.Vec3)
///     >>> td.size()
///     12
///     >>> TypeDesc.color().size()
///     12
#[pyclass]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDesc {
    /// Base data type
    #[pyo3(get, set)]
    pub basetype: BaseType,
    /// Aggregation of base type
    #[pyo3(get, set)]
    pub aggregate: Aggregate,
    /// Vector semantics
    #[pyo3(get, set)]
    pub vecsemantics: VecSemantics,
    /// Array length (0 = not array, -1 = unsized array)
    #[pyo3(get, set)]
    pub arraylen: i32,
}

#[pymethods]
impl TypeDesc {
    // Type constants as class attributes
    #[classattr]
    const UNKNOWN: TypeDesc = TypeDesc::scalar(BaseType::Unknown);
    #[classattr]
    const NONE: TypeDesc = TypeDesc::scalar(BaseType::None);
    #[classattr]
    const UINT8: TypeDesc = TypeDesc::scalar(BaseType::UInt8);
    #[classattr]
    const INT8: TypeDesc = TypeDesc::scalar(BaseType::Int8);
    #[classattr]
    const UINT16: TypeDesc = TypeDesc::scalar(BaseType::UInt16);
    #[classattr]
    const INT16: TypeDesc = TypeDesc::scalar(BaseType::Int16);
    #[classattr]
    const UINT32: TypeDesc = TypeDesc::scalar(BaseType::UInt32);
    #[classattr]
    const INT32: TypeDesc = TypeDesc::scalar(BaseType::Int32);
    #[classattr]
    const UINT64: TypeDesc = TypeDesc::scalar(BaseType::UInt64);
    #[classattr]
    const INT64: TypeDesc = TypeDesc::scalar(BaseType::Int64);
    #[classattr]
    const HALF: TypeDesc = TypeDesc::scalar(BaseType::Half);
    #[classattr]
    const FLOAT: TypeDesc = TypeDesc::scalar(BaseType::Float);
    #[classattr]
    const DOUBLE: TypeDesc = TypeDesc::scalar(BaseType::Double);
    #[classattr]
    const STRING: TypeDesc = TypeDesc::scalar(BaseType::String);

    /// Create a new TypeDesc.
    #[new]
    #[pyo3(signature = (basetype=BaseType::Float, aggregate=Aggregate::Scalar, vecsemantics=VecSemantics::NoSemantics, arraylen=0))]
    fn new(
        basetype: BaseType,
        aggregate: Aggregate,
        vecsemantics: VecSemantics,
        arraylen: i32,
    ) -> Self {
        Self {
            basetype,
            aggregate,
            vecsemantics,
            arraylen,
        }
    }

    /// Creates a scalar TypeDesc.
    #[staticmethod]
    const fn scalar(basetype: BaseType) -> Self {
        Self {
            basetype,
            aggregate: Aggregate::Scalar,
            vecsemantics: VecSemantics::NoSemantics,
            arraylen: 0,
        }
    }

    /// Creates a color type (Vec3 float with color semantics).
    #[staticmethod]
    fn color() -> Self {
        Self {
            basetype: BaseType::Float,
            aggregate: Aggregate::Vec3,
            vecsemantics: VecSemantics::Color,
            arraylen: 0,
        }
    }

    /// Creates a point type (Vec3 float with point semantics).
    #[staticmethod]
    fn point() -> Self {
        Self {
            basetype: BaseType::Float,
            aggregate: Aggregate::Vec3,
            vecsemantics: VecSemantics::Point,
            arraylen: 0,
        }
    }

    /// Creates a vector type (Vec3 float with vector semantics).
    #[staticmethod]
    fn vector() -> Self {
        Self {
            basetype: BaseType::Float,
            aggregate: Aggregate::Vec3,
            vecsemantics: VecSemantics::Vector,
            arraylen: 0,
        }
    }

    /// Creates a normal type (Vec3 float with normal semantics).
    #[staticmethod]
    fn normal() -> Self {
        Self {
            basetype: BaseType::Float,
            aggregate: Aggregate::Vec3,
            vecsemantics: VecSemantics::Normal,
            arraylen: 0,
        }
    }

    /// Creates a 4x4 matrix type.
    #[staticmethod]
    fn matrix44() -> Self {
        Self {
            basetype: BaseType::Float,
            aggregate: Aggregate::Matrix44,
            vecsemantics: VecSemantics::NoSemantics,
            arraylen: 0,
        }
    }

    /// Creates a 3x3 matrix type.
    #[staticmethod]
    fn matrix33() -> Self {
        Self {
            basetype: BaseType::Float,
            aggregate: Aggregate::Matrix33,
            vecsemantics: VecSemantics::NoSemantics,
            arraylen: 0,
        }
    }

    /// Creates an int array type.
    #[staticmethod]
    fn int_array(len: i32) -> Self {
        Self {
            basetype: BaseType::Int32,
            aggregate: Aggregate::Scalar,
            vecsemantics: VecSemantics::NoSemantics,
            arraylen: len,
        }
    }

    /// Creates a float array type.
    #[staticmethod]
    fn float_array(len: i32) -> Self {
        Self {
            basetype: BaseType::Float,
            aggregate: Aggregate::Scalar,
            vecsemantics: VecSemantics::NoSemantics,
            arraylen: len,
        }
    }

    /// Size of one base element in bytes.
    fn basesize(&self) -> usize {
        self.basetype.size()
    }

    /// Size of one aggregate element (base * aggregate count).
    fn elementsize(&self) -> usize {
        self.basetype.size() * self.aggregate.count()
    }

    /// Total size in bytes.
    fn size(&self) -> usize {
        let base = self.elementsize();
        if self.arraylen > 0 {
            base * self.arraylen as usize
        } else {
            base
        }
    }

    /// Number of base type elements in one aggregate.
    fn basevalues(&self) -> usize {
        self.aggregate.count()
    }

    /// Number of array elements (1 if not an array).
    fn numelements(&self) -> usize {
        if self.arraylen > 0 {
            self.arraylen as usize
        } else {
            1
        }
    }

    /// Returns true if this is an array type.
    fn is_array(&self) -> bool {
        self.arraylen != 0
    }

    /// Returns true if this is a sized array.
    fn is_sized_array(&self) -> bool {
        self.arraylen > 0
    }

    /// Returns true if this is an unsized array.
    fn is_unsized_array(&self) -> bool {
        self.arraylen < 0
    }

    /// Returns true if this is a floating-point type.
    fn is_floating_point(&self) -> bool {
        self.basetype.is_floating_point()
    }

    /// Returns true if this is a signed type.
    fn is_signed(&self) -> bool {
        self.basetype.is_signed()
    }

    /// Returns true if this type is unknown.
    fn is_unknown(&self) -> bool {
        matches!(self.basetype, BaseType::Unknown)
    }

    /// Returns the type of a single element (strips array).
    fn elementtype(&self) -> Self {
        Self {
            basetype: self.basetype,
            aggregate: self.aggregate,
            vecsemantics: self.vecsemantics,
            arraylen: 0,
        }
    }

    /// Returns the scalar base type (strips aggregate and array).
    fn scalartype(&self) -> Self {
        Self::scalar(self.basetype)
    }

    /// Returns this type without array specification.
    fn unarray(&self) -> Self {
        self.elementtype()
    }

    /// Returns a copy with array specification.
    fn array(&self, len: i32) -> Self {
        Self {
            basetype: self.basetype,
            aggregate: self.aggregate,
            vecsemantics: self.vecsemantics,
            arraylen: len,
        }
    }

    /// Returns a copy with vector semantics.
    fn with_semantics(&self, semantics: VecSemantics) -> Self {
        Self {
            basetype: self.basetype,
            aggregate: self.aggregate,
            vecsemantics: semantics,
            arraylen: self.arraylen,
        }
    }

    /// Equivalent types (same base and aggregate, ignoring semantics and array).
    fn equivalent(&self, other: &Self) -> bool {
        self.basetype == other.basetype && self.aggregate == other.aggregate
    }

    fn __repr__(&self) -> String {
        let sem = match self.vecsemantics {
            VecSemantics::Color => "color",
            VecSemantics::Point => "point",
            VecSemantics::Vector => "vector",
            VecSemantics::Normal => "normal",
            _ => "",
        };

        let base = if !sem.is_empty() {
            sem.to_string()
        } else {
            match self.aggregate {
                Aggregate::Scalar => self.basetype.name().to_string(),
                Aggregate::Vec2 => format!("{}[2]", self.basetype.name()),
                Aggregate::Vec3 => format!("{}[3]", self.basetype.name()),
                Aggregate::Vec4 => format!("{}[4]", self.basetype.name()),
                Aggregate::Matrix33 => "matrix33".to_string(),
                Aggregate::Matrix44 => "matrix44".to_string(),
            }
        };

        let arr = if self.arraylen > 0 {
            format!("[{}]", self.arraylen)
        } else if self.arraylen < 0 {
            "[]".to_string()
        } else {
            String::new()
        };

        format!("TypeDesc({}{})", base, arr)
    }
}

// =============================================================================
// Roi3D
// =============================================================================

/// 3D Region of Interest with channel range (OIIO-compatible).
///
/// Defines a 3D subregion of an image including channel range.
/// All ranges are [begin, end) - end is exclusive.
///
/// Example:
///     >>> roi = Roi3D(0, 1920, 0, 1080, 0, 1, 0, 4)  # Full HD, 4 channels
///     >>> roi.width
///     1920
///     >>> roi.npixels()
///     2073600
#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Roi3D {
    #[pyo3(get, set)]
    pub xbegin: i32,
    #[pyo3(get, set)]
    pub xend: i32,
    #[pyo3(get, set)]
    pub ybegin: i32,
    #[pyo3(get, set)]
    pub yend: i32,
    #[pyo3(get, set)]
    pub zbegin: i32,
    #[pyo3(get, set)]
    pub zend: i32,
    #[pyo3(get, set)]
    pub chbegin: i32,
    #[pyo3(get, set)]
    pub chend: i32,
}

#[pymethods]
impl Roi3D {
    /// Create a new Roi3D.
    #[new]
    #[pyo3(signature = (xbegin=0, xend=0, ybegin=0, yend=0, zbegin=0, zend=1, chbegin=0, chend=-1))]
    fn new(
        xbegin: i32,
        xend: i32,
        ybegin: i32,
        yend: i32,
        zbegin: i32,
        zend: i32,
        chbegin: i32,
        chend: i32,
    ) -> Self {
        Self {
            xbegin,
            xend,
            ybegin,
            yend,
            zbegin,
            zend,
            chbegin,
            chend,
        }
    }

    /// Create a 2D ROI (z range is 0-1).
    #[staticmethod]
    fn new_2d(xbegin: i32, xend: i32, ybegin: i32, yend: i32) -> Self {
        Self::new(xbegin, xend, ybegin, yend, 0, 1, 0, -1)
    }

    /// Create a 2D ROI with channel range.
    #[staticmethod]
    fn new_2d_with_channels(
        xbegin: i32,
        xend: i32,
        ybegin: i32,
        yend: i32,
        chbegin: i32,
        chend: i32,
    ) -> Self {
        Self::new(xbegin, xend, ybegin, yend, 0, 1, chbegin, chend)
    }

    /// Create a ROI from dimensions (origin at 0,0).
    #[staticmethod]
    fn from_size(width: i32, height: i32) -> Self {
        Self::new(0, width, 0, height, 0, 1, 0, -1)
    }

    /// Create an unlimited ROI (all pixels).
    #[staticmethod]
    fn all() -> Self {
        Self {
            xbegin: i32::MIN,
            xend: i32::MAX,
            ybegin: i32::MIN,
            yend: i32::MAX,
            zbegin: i32::MIN,
            zend: i32::MAX,
            chbegin: 0,
            chend: i32::MAX,
        }
    }

    /// Returns true if this is an "all" (unlimited) ROI.
    fn is_all(&self) -> bool {
        self.xbegin == i32::MIN
            && self.xend == i32::MAX
            && self.ybegin == i32::MIN
            && self.yend == i32::MAX
    }

    /// Returns true if this ROI is defined (not empty).
    fn defined(&self) -> bool {
        self.xbegin < self.xend && self.ybegin < self.yend && self.zbegin < self.zend
    }

    /// Width of the ROI in pixels.
    #[getter]
    fn width(&self) -> i32 {
        (self.xend - self.xbegin).max(0)
    }

    /// Height of the ROI in pixels.
    #[getter]
    fn height(&self) -> i32 {
        (self.yend - self.ybegin).max(0)
    }

    /// Depth of the ROI (for 3D images).
    #[getter]
    fn depth(&self) -> i32 {
        (self.zend - self.zbegin).max(0)
    }

    /// Number of channels.
    #[getter]
    fn nchannels(&self) -> i32 {
        if self.chend < 0 {
            -1 // unlimited
        } else {
            (self.chend - self.chbegin).max(0)
        }
    }

    /// Total number of pixels.
    fn npixels(&self) -> u64 {
        (self.width() as u64) * (self.height() as u64) * (self.depth() as u64)
    }

    /// Returns true if the point (x, y, z) is inside the ROI.
    #[pyo3(signature = (x, y, z=0))]
    fn contains(&self, x: i32, y: i32, z: i32) -> bool {
        x >= self.xbegin
            && x < self.xend
            && y >= self.ybegin
            && y < self.yend
            && z >= self.zbegin
            && z < self.zend
    }

    /// Returns true if the point with channel is inside the ROI.
    fn contains_with_channel(&self, x: i32, y: i32, z: i32, ch: i32) -> bool {
        self.contains(x, y, z)
            && ch >= self.chbegin
            && (self.chend < 0 || ch < self.chend)
    }

    /// Returns true if this ROI fully contains another.
    fn contains_roi(&self, other: &Roi3D) -> bool {
        self.xbegin <= other.xbegin
            && self.xend >= other.xend
            && self.ybegin <= other.ybegin
            && self.yend >= other.yend
            && self.zbegin <= other.zbegin
            && self.zend >= other.zend
    }

    /// Returns the union of two ROIs.
    fn union(&self, other: &Roi3D) -> Roi3D {
        Roi3D {
            xbegin: self.xbegin.min(other.xbegin),
            xend: self.xend.max(other.xend),
            ybegin: self.ybegin.min(other.ybegin),
            yend: self.yend.max(other.yend),
            zbegin: self.zbegin.min(other.zbegin),
            zend: self.zend.max(other.zend),
            chbegin: self.chbegin.min(other.chbegin),
            chend: if self.chend < 0 || other.chend < 0 {
                -1
            } else {
                self.chend.max(other.chend)
            },
        }
    }

    /// Returns the intersection of two ROIs, or None if they don't overlap.
    fn intersection(&self, other: &Roi3D) -> Option<Roi3D> {
        let xbegin = self.xbegin.max(other.xbegin);
        let xend = self.xend.min(other.xend);
        let ybegin = self.ybegin.max(other.ybegin);
        let yend = self.yend.min(other.yend);
        let zbegin = self.zbegin.max(other.zbegin);
        let zend = self.zend.min(other.zend);

        if xbegin < xend && ybegin < yend && zbegin < zend {
            Some(Roi3D {
                xbegin,
                xend,
                ybegin,
                yend,
                zbegin,
                zend,
                chbegin: self.chbegin.max(other.chbegin),
                chend: if self.chend < 0 {
                    other.chend
                } else if other.chend < 0 {
                    self.chend
                } else {
                    self.chend.min(other.chend)
                },
            })
        } else {
            None
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Roi3D(x=[{}, {}), y=[{}, {}), z=[{}, {}), ch=[{}, {}))",
            self.xbegin, self.xend,
            self.ybegin, self.yend,
            self.zbegin, self.zend,
            self.chbegin, if self.chend < 0 { "∞".to_string() } else { self.chend.to_string() }
        )
    }
}

// =============================================================================
// ImageSpec
// =============================================================================

/// Comprehensive image specification (OIIO-compatible).
///
/// Contains all information needed to interpret raw pixel data:
/// dimensions, channel layout, data type, and arbitrary metadata.
///
/// Example:
///     >>> spec = ImageSpec(1920, 1080, 4, DataFormat.F16)
///     >>> spec.bytes_per_pixel()
///     8
///     >>> spec.image_bytes()
///     16588800
#[pyclass]
#[derive(Debug, Clone)]
pub struct ImageSpec {
    // Dimensions (data window)
    #[pyo3(get, set)]
    pub width: u32,
    #[pyo3(get, set)]
    pub height: u32,
    #[pyo3(get, set)]
    pub depth: u32,

    // Data window origin
    #[pyo3(get, set)]
    pub x: i32,
    #[pyo3(get, set)]
    pub y: i32,
    #[pyo3(get, set)]
    pub z: i32,

    // Full/Display window
    #[pyo3(get, set)]
    pub full_width: u32,
    #[pyo3(get, set)]
    pub full_height: u32,
    #[pyo3(get, set)]
    pub full_depth: u32,
    #[pyo3(get, set)]
    pub full_x: i32,
    #[pyo3(get, set)]
    pub full_y: i32,
    #[pyo3(get, set)]
    pub full_z: i32,

    // Tiling
    #[pyo3(get, set)]
    pub tile_width: u32,
    #[pyo3(get, set)]
    pub tile_height: u32,
    #[pyo3(get, set)]
    pub tile_depth: u32,

    // Channels
    #[pyo3(get, set)]
    pub nchannels: u8,
    #[pyo3(get, set)]
    pub format: DataFormat,
    #[pyo3(get, set)]
    pub channel_names: Vec<String>,
    #[pyo3(get, set)]
    pub alpha_channel: i32,
    #[pyo3(get, set)]
    pub z_channel: i32,

    // Deep image
    #[pyo3(get, set)]
    pub deep: bool,

    // Attributes stored as Python dict equivalent
    attributes: HashMap<String, PyAttrValue>,
}

/// Internal attribute value representation
#[derive(Debug, Clone)]
enum PyAttrValue {
    Int(i64),
    Float(f64),
    String(String),
    IntArray(Vec<i64>),
    FloatArray(Vec<f64>),
}

// TODO: Migrate to IntoPyObject when pyo3 0.24 stabilizes
#[allow(deprecated)]
impl IntoPy<PyObject> for PyAttrValue {
    fn into_py(self, py: Python<'_>) -> PyObject {
        match self {
            Self::Int(v) => v.into_py(py),
            Self::Float(v) => v.into_py(py),
            Self::String(v) => v.into_py(py),
            Self::IntArray(v) => v.into_py(py),
            Self::FloatArray(v) => v.into_py(py),
        }
    }
}

#[pymethods]
impl ImageSpec {
    /// Create a new ImageSpec.
    #[new]
    #[pyo3(signature = (width, height, nchannels=4, format=DataFormat::F16))]
    fn new(width: u32, height: u32, nchannels: u8, format: DataFormat) -> Self {
        Self {
            width,
            height,
            depth: 1,
            x: 0,
            y: 0,
            z: 0,
            full_width: width,
            full_height: height,
            full_depth: 1,
            full_x: 0,
            full_y: 0,
            full_z: 0,
            tile_width: 0,
            tile_height: 0,
            tile_depth: 0,
            nchannels,
            format,
            channel_names: Vec::new(),
            alpha_channel: -1,
            z_channel: -1,
            deep: false,
            attributes: HashMap::new(),
        }
    }

    /// Create a spec for an RGB image (3 channels, F16).
    #[staticmethod]
    fn rgb(width: u32, height: u32) -> Self {
        let mut spec = Self::new(width, height, 3, DataFormat::F16);
        spec.channel_names = vec!["R".into(), "G".into(), "B".into()];
        spec
    }

    /// Create a spec for an RGBA image (4 channels, F16).
    #[staticmethod]
    fn rgba(width: u32, height: u32) -> Self {
        let mut spec = Self::new(width, height, 4, DataFormat::F16);
        spec.channel_names = vec!["R".into(), "G".into(), "B".into(), "A".into()];
        spec.alpha_channel = 3;
        spec
    }

    /// Create a spec for a grayscale image (1 channel).
    #[staticmethod]
    fn gray(width: u32, height: u32) -> Self {
        let mut spec = Self::new(width, height, 1, DataFormat::F16);
        spec.channel_names = vec!["Y".into()];
        spec
    }

    /// Create a spec for a grayscale+alpha image (2 channels).
    #[staticmethod]
    fn gray_alpha(width: u32, height: u32) -> Self {
        let mut spec = Self::new(width, height, 2, DataFormat::F16);
        spec.channel_names = vec!["Y".into(), "A".into()];
        spec.alpha_channel = 1;
        spec
    }

    /// Create a spec from a Roi3D.
    #[staticmethod]
    fn from_roi(roi: &Roi3D) -> Self {
        let width = roi.width().max(0) as u32;
        let height = roi.height().max(0) as u32;
        let depth = roi.depth().max(0) as u32;
        let nchannels = roi.nchannels().max(0).min(255) as u8;

        let mut spec = Self::new(width, height, nchannels, DataFormat::F32);
        spec.depth = depth;
        spec.x = roi.xbegin;
        spec.y = roi.ybegin;
        spec.z = roi.zbegin;
        spec.default_channel_names();
        spec
    }

    /// Sets default channel names based on channel count.
    fn default_channel_names(&mut self) {
        self.channel_names = match self.nchannels {
            1 => vec!["Y".into()],
            2 => vec!["Y".into(), "A".into()],
            3 => vec!["R".into(), "G".into(), "B".into()],
            4 => vec!["R".into(), "G".into(), "B".into(), "A".into()],
            n => {
                let mut names = vec!["R".into(), "G".into(), "B".into(), "A".into()];
                for i in 4..n {
                    names.push(format!("channel{}", i));
                }
                names
            }
        };

        // Update alpha/z channel indices
        self.alpha_channel = self
            .channel_names
            .iter()
            .position(|n| n.eq_ignore_ascii_case("a") || n.eq_ignore_ascii_case("alpha"))
            .map(|i| i as i32)
            .unwrap_or(-1);

        self.z_channel = self
            .channel_names
            .iter()
            .position(|n| n.eq_ignore_ascii_case("z") || n.eq_ignore_ascii_case("depth"))
            .map(|i| i as i32)
            .unwrap_or(-1);
    }

    /// Number of bytes per pixel.
    fn bytes_per_pixel(&self) -> usize {
        self.nchannels as usize * self.format.bytes_per_channel()
    }

    /// Number of bytes per scanline (row).
    fn bytes_per_row(&self) -> usize {
        self.width as usize * self.bytes_per_pixel()
    }

    /// Alias for bytes_per_pixel (OIIO compatibility).
    #[pyo3(signature = (native=false))]
    fn pixel_bytes(&self, native: bool) -> usize {
        let _ = native; // TODO: per-channel formats
        self.bytes_per_pixel()
    }

    /// Alias for bytes_per_row (OIIO compatibility).
    #[pyo3(signature = (native=false))]
    fn scanline_bytes(&self, native: bool) -> usize {
        self.width as usize * self.pixel_bytes(native)
    }

    /// Total number of pixels.
    fn pixel_count(&self) -> u64 {
        (self.width as u64) * (self.height as u64) * (self.depth as u64)
    }

    /// Alias for pixel_count (OIIO compatibility).
    fn image_pixels(&self) -> u64 {
        self.pixel_count()
    }

    /// Total image size in bytes.
    #[pyo3(signature = (native=false))]
    fn image_bytes(&self, native: bool) -> u64 {
        self.image_pixels() * self.pixel_bytes(native) as u64
    }

    /// Total data size in bytes.
    fn data_size(&self) -> usize {
        self.pixel_count() as usize * self.bytes_per_pixel()
    }

    /// Returns true if the image has an alpha channel.
    fn has_alpha(&self) -> bool {
        if self.alpha_channel >= 0 {
            return true;
        }
        self.channel_names.iter().any(|name| {
            let lower = name.to_lowercase();
            lower == "a" || lower == "alpha"
        })
    }

    /// Returns the alpha channel index, or None.
    fn get_alpha_channel(&self) -> Option<usize> {
        if self.alpha_channel >= 0 {
            return Some(self.alpha_channel as usize);
        }
        self.channel_names.iter().position(|name| {
            let lower = name.to_lowercase();
            lower == "a" || lower == "alpha"
        })
    }

    /// Returns the depth/Z channel index, or None.
    fn get_z_channel(&self) -> Option<usize> {
        if self.z_channel >= 0 {
            return Some(self.z_channel as usize);
        }
        self.channel_names.iter().position(|name| {
            let lower = name.to_lowercase();
            lower == "z" || lower == "depth"
        })
    }

    /// Returns true if data and display windows differ (overscan).
    fn has_overscan(&self) -> bool {
        self.x != self.full_x
            || self.y != self.full_y
            || self.z != self.full_z
            || self.width != self.full_width
            || self.height != self.full_height
            || self.depth != self.full_depth
    }

    /// Returns true if the image is tiled.
    fn is_tiled(&self) -> bool {
        self.tile_width > 0 && self.tile_height > 0
    }

    /// Returns true if this is a 3D (volumetric) texture.
    fn is_3d(&self) -> bool {
        self.depth > 1
    }

    /// Returns true if the spec is undefined/invalid.
    fn undefined(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Returns number of pixels per tile.
    fn tile_pixels(&self) -> u64 {
        if self.tile_width == 0 || self.tile_height == 0 {
            0
        } else {
            let d = self.tile_depth.max(1) as u64;
            (self.tile_width as u64) * (self.tile_height as u64) * d
        }
    }

    /// Returns bytes per tile.
    #[pyo3(signature = (native=false))]
    fn tile_bytes(&self, native: bool) -> usize {
        self.tile_pixels() as usize * self.pixel_bytes(native)
    }

    /// Creates a ROI from the image dimensions.
    fn roi(&self) -> Roi3D {
        Roi3D::new(
            self.x,
            self.x + self.width as i32,
            self.y,
            self.y + self.height as i32,
            self.z,
            self.z + self.depth as i32,
            0,
            self.nchannels as i32,
        )
    }

    /// Creates a ROI from the full/display dimensions.
    fn roi_full(&self) -> Roi3D {
        Roi3D::new(
            self.full_x,
            self.full_x + self.full_width as i32,
            self.full_y,
            self.full_y + self.full_height as i32,
            self.full_z,
            self.full_z + self.full_depth as i32,
            0,
            self.nchannels as i32,
        )
    }

    /// Sets dimensions from a ROI.
    fn set_roi(&mut self, roi: &Roi3D) {
        self.x = roi.xbegin;
        self.y = roi.ybegin;
        self.z = roi.zbegin;
        self.width = roi.width() as u32;
        self.height = roi.height() as u32;
        self.depth = roi.depth() as u32;
    }

    /// Sets full/display dimensions from a ROI.
    fn set_roi_full(&mut self, roi: &Roi3D) {
        self.full_x = roi.xbegin;
        self.full_y = roi.ybegin;
        self.full_z = roi.zbegin;
        self.full_width = roi.width() as u32;
        self.full_height = roi.height() as u32;
        self.full_depth = roi.depth() as u32;
    }

    /// Set an attribute value.
    fn set_attr(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let val = if let Ok(v) = value.extract::<i64>() {
            PyAttrValue::Int(v)
        } else if let Ok(v) = value.extract::<f64>() {
            PyAttrValue::Float(v)
        } else if let Ok(v) = value.extract::<String>() {
            PyAttrValue::String(v)
        } else if let Ok(v) = value.extract::<Vec<i64>>() {
            PyAttrValue::IntArray(v)
        } else if let Ok(v) = value.extract::<Vec<f64>>() {
            PyAttrValue::FloatArray(v)
        } else {
            return Err(PyValueError::new_err("Unsupported attribute type"));
        };
        self.attributes.insert(key.to_string(), val);
        Ok(())
    }

    /// Get an attribute value.
    #[allow(deprecated)]  // TODO: Migrate to IntoPyObject
    fn get_attr(&self, py: Python<'_>, key: &str) -> Option<PyObject> {
        self.attributes.get(key).map(|v| v.clone().into_py(py))
    }

    /// Get a string attribute.
    fn get_string(&self, key: &str) -> Option<String> {
        match self.attributes.get(key) {
            Some(PyAttrValue::String(s)) => Some(s.clone()),
            _ => None,
        }
    }

    /// Get an integer attribute.
    fn get_int(&self, key: &str) -> Option<i64> {
        match self.attributes.get(key) {
            Some(PyAttrValue::Int(v)) => Some(*v),
            Some(PyAttrValue::Float(v)) => Some(*v as i64),
            _ => None,
        }
    }

    /// Get a float attribute.
    fn get_float(&self, key: &str) -> Option<f64> {
        match self.attributes.get(key) {
            Some(PyAttrValue::Int(v)) => Some(*v as f64),
            Some(PyAttrValue::Float(v)) => Some(*v),
            _ => None,
        }
    }

    /// Get an integer attribute with default.
    fn get_int_attribute(&self, key: &str, default: i64) -> i64 {
        self.get_int(key).unwrap_or(default)
    }

    /// Get a float attribute with default.
    fn get_float_attribute(&self, key: &str, default: f64) -> f64 {
        self.get_float(key).unwrap_or(default)
    }

    /// Get a string attribute with default.
    fn get_string_attribute(&self, key: &str, default: &str) -> String {
        self.get_string(key).unwrap_or_else(|| default.to_string())
    }

    /// Remove an attribute.
    fn erase_attribute(&mut self, key: &str) -> bool {
        self.attributes.remove(key).is_some()
    }

    /// Sets the colorspace metadata attribute.
    fn set_colorspace(&mut self, colorspace: &str) -> PyResult<()> {
        self.attributes.insert(
            "oiio:ColorSpace".to_string(),
            PyAttrValue::String(colorspace.to_string()),
        );
        Ok(())
    }

    /// Gets the colorspace metadata attribute.
    fn get_colorspace(&self) -> Option<String> {
        self.get_string("oiio:ColorSpace")
    }

    // Note: format property has automatic setter via #[pyo3(get, set)]

    /// Creates a copy with different dimensions.
    fn with_size(&self, width: u32, height: u32) -> Self {
        let mut spec = self.clone();
        spec.width = width;
        spec.height = height;
        spec.full_width = width;
        spec.full_height = height;
        spec
    }

    /// Creates a copy with different format.
    fn with_format(&self, format: DataFormat) -> Self {
        let mut spec = self.clone();
        spec.format = format;
        spec
    }

    fn __repr__(&self) -> String {
        if self.depth > 1 {
            format!(
                "ImageSpec({}x{}x{} {} {}ch)",
                self.width, self.height, self.depth, self.format.name(), self.nchannels
            )
        } else {
            format!(
                "ImageSpec({}x{} {} {}ch)",
                self.width, self.height, self.format.name(), self.nchannels
            )
        }
    }
}

// =============================================================================
// Module registration
// =============================================================================

/// Register core types with the module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Enums
    m.add_class::<BaseType>()?;
    m.add_class::<Aggregate>()?;
    m.add_class::<VecSemantics>()?;
    m.add_class::<DataFormat>()?;

    // Core types
    m.add_class::<TypeDesc>()?;
    m.add_class::<Roi3D>()?;
    m.add_class::<ImageSpec>()?;

    Ok(())
}
