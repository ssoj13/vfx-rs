//! Color space definition and properties.
//!
//! A color space in OCIO defines:
//! - How to convert to/from the reference space
//! - Categorization (family, encoding, etc.)
//! - Metadata (description, aliases)
//!
//! # Example
//!
//! ```
//! use vfx_ocio::{ColorSpace, Encoding, Family};
//!
//! let cs = ColorSpace::builder("ACEScg")
//!     .family(Family::Scene)
//!     .encoding(Encoding::SceneLinear)
//!     .description("ACES CG working space")
//!     .build();
//!
//! assert_eq!(cs.name(), "ACEScg");
//! assert_eq!(cs.encoding(), Encoding::SceneLinear);
//! ```

use crate::transform::Transform;

/// Color encoding type.
///
/// Indicates the data encoding/interpretation of pixel values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Encoding {
    /// Scene-referred linear.
    SceneLinear,
    /// Display-referred linear.
    DisplayLinear,
    /// Logarithmic encoding.
    Log,
    /// OETF-encoded (sRGB, Rec.709, etc.).
    Sdr,
    /// HDR display encoding (PQ, HLG).
    Hdr,
    /// Non-color data (normals, masks).
    Data,
    /// Unknown/unspecified encoding.
    #[default]
    Unknown,
}

impl Encoding {
    /// Parses encoding from OCIO config string.
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "scene-linear" | "scene_linear" | "linear" => Self::SceneLinear,
            "display-linear" | "display_linear" => Self::DisplayLinear,
            "log" => Self::Log,
            "sdr-video" | "sdr_video" | "sdr" => Self::Sdr,
            "hdr-video" | "hdr_video" | "hdr" => Self::Hdr,
            "data" => Self::Data,
            _ => Self::Unknown,
        }
    }

    /// Returns OCIO config string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SceneLinear => "scene-linear",
            Self::DisplayLinear => "display-linear",
            Self::Log => "log",
            Self::Sdr => "sdr-video",
            Self::Hdr => "hdr-video",
            Self::Data => "data",
            Self::Unknown => "",
        }
    }

    /// Checks if this is a linear encoding.
    #[inline]
    pub fn is_linear(&self) -> bool {
        matches!(self, Self::SceneLinear | Self::DisplayLinear)
    }
}

/// Color space family/category.
///
/// Groups related color spaces for UI organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Family {
    /// Scene-referred spaces (linear, log).
    Scene,
    /// Display-referred spaces.
    Display,
    /// Input/camera spaces.
    Input,
    /// Output/delivery spaces.
    Output,
    /// Utility spaces.
    Utility,
    /// ACES spaces.
    Aces,
    /// Custom/uncategorized.
    #[default]
    Other,
}

impl Family {
    /// Parses family from config string.
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "scene" | "scene-referred" => Self::Scene,
            "display" | "display-referred" => Self::Display,
            "input" | "camera" => Self::Input,
            "output" | "delivery" => Self::Output,
            "utility" | "utilities" => Self::Utility,
            "aces" => Self::Aces,
            _ => Self::Other,
        }
    }

    /// Returns display string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Scene => "Scene",
            Self::Display => "Display",
            Self::Input => "Input",
            Self::Output => "Output",
            Self::Utility => "Utility",
            Self::Aces => "ACES",
            Self::Other => "",
        }
    }
}

/// Bit depth hint for the color space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BitDepth {
    /// 8-bit unsigned integer.
    Uint8,
    /// 10-bit unsigned integer.
    Uint10,
    /// 12-bit unsigned integer.
    Uint12,
    /// 16-bit unsigned integer.
    Uint16,
    /// 16-bit float.
    F16,
    /// 32-bit float.
    #[default]
    F32,
}

/// Color space definition.
///
/// Represents a named color space with transforms to/from reference space.
#[derive(Debug, Clone)]
pub struct ColorSpace {
    /// Unique name.
    name: String,
    /// Alternative names.
    aliases: Vec<String>,
    /// Human-readable description.
    description: String,
    /// Family/category.
    family: Family,
    /// Encoding type.
    encoding: Encoding,
    /// Bit depth hint.
    bit_depth: BitDepth,
    /// Whether this is for non-color data.
    is_data: bool,
    /// Transform from this space to reference.
    to_reference: Option<Transform>,
    /// Transform from reference to this space.
    from_reference: Option<Transform>,
    /// Allocation variables for GPU optimization.
    allocation: AllocationInfo,
}

/// GPU allocation hints.
#[derive(Debug, Clone, Default)]
pub struct AllocationInfo {
    /// Allocation type (uniform or log).
    pub alloc_type: AllocationType,
    /// Min/max range.
    pub vars: [f32; 2],
}

/// Allocation type for GPU texture mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AllocationType {
    /// Uniform distribution.
    #[default]
    Uniform,
    /// Logarithmic distribution.
    Log,
}

impl ColorSpace {
    /// Creates a new color space with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            aliases: Vec::new(),
            description: String::new(),
            family: Family::default(),
            encoding: Encoding::default(),
            bit_depth: BitDepth::default(),
            is_data: false,
            to_reference: None,
            from_reference: None,
            allocation: AllocationInfo::default(),
        }
    }

    /// Creates a builder for constructing color spaces.
    #[inline]
    pub fn builder(name: impl Into<String>) -> ColorSpaceBuilder {
        ColorSpaceBuilder::new(name)
    }

    /// Returns the color space name.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns all aliases.
    #[inline]
    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }

    /// Returns the description.
    #[inline]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the family/category.
    #[inline]
    pub fn family(&self) -> Family {
        self.family
    }

    /// Returns the encoding type.
    #[inline]
    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    /// Returns the bit depth hint.
    #[inline]
    pub fn bit_depth(&self) -> BitDepth {
        self.bit_depth
    }

    /// Checks if this is a data (non-color) space.
    #[inline]
    pub fn is_data(&self) -> bool {
        self.is_data
    }

    /// Returns the transform to reference space.
    #[inline]
    pub fn to_reference(&self) -> Option<&Transform> {
        self.to_reference.as_ref()
    }

    /// Returns the transform from reference space.
    #[inline]
    pub fn from_reference(&self) -> Option<&Transform> {
        self.from_reference.as_ref()
    }

    /// Returns GPU allocation info.
    #[inline]
    pub fn allocation(&self) -> &AllocationInfo {
        &self.allocation
    }

    /// Checks if a name or alias matches.
    pub fn matches_name(&self, name: &str) -> bool {
        self.name.eq_ignore_ascii_case(name)
            || self.aliases.iter().any(|a| a.eq_ignore_ascii_case(name))
    }
}

/// Builder for constructing color spaces.
#[derive(Debug)]
pub struct ColorSpaceBuilder {
    inner: ColorSpace,
}

impl ColorSpaceBuilder {
    /// Creates a new builder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            inner: ColorSpace::new(name),
        }
    }

    /// Adds an alias.
    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        self.inner.aliases.push(alias.into());
        self
    }

    /// Sets the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.inner.description = desc.into();
        self
    }

    /// Sets the family.
    pub fn family(mut self, family: Family) -> Self {
        self.inner.family = family;
        self
    }

    /// Sets the encoding.
    pub fn encoding(mut self, encoding: Encoding) -> Self {
        self.inner.encoding = encoding;
        if encoding == Encoding::Data {
            self.inner.is_data = true;
        }
        self
    }

    /// Sets the bit depth.
    pub fn bit_depth(mut self, depth: BitDepth) -> Self {
        self.inner.bit_depth = depth;
        self
    }

    /// Marks as data (non-color) space.
    pub fn is_data(mut self, is_data: bool) -> Self {
        self.inner.is_data = is_data;
        self
    }

    /// Sets the transform to reference space.
    pub fn to_reference(mut self, transform: Transform) -> Self {
        self.inner.to_reference = Some(transform);
        self
    }

    /// Sets the transform from reference space.
    pub fn from_reference(mut self, transform: Transform) -> Self {
        self.inner.from_reference = Some(transform);
        self
    }

    /// Sets allocation info.
    pub fn allocation(mut self, alloc: AllocationInfo) -> Self {
        self.inner.allocation = alloc;
        self
    }

    /// Builds the color space.
    pub fn build(self) -> ColorSpace {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_colorspace() {
        let cs = ColorSpace::builder("ACEScg")
            .alias("ACES - ACEScg")
            .family(Family::Scene)
            .encoding(Encoding::SceneLinear)
            .description("ACES CG working space")
            .build();

        assert_eq!(cs.name(), "ACEScg");
        assert_eq!(cs.family(), Family::Scene);
        assert_eq!(cs.encoding(), Encoding::SceneLinear);
        assert!(cs.encoding().is_linear());
        assert!(cs.matches_name("acescg"));
        assert!(cs.matches_name("ACES - ACEScg"));
    }

    #[test]
    fn encoding_parse() {
        assert_eq!(Encoding::parse("scene-linear"), Encoding::SceneLinear);
        assert_eq!(Encoding::parse("log"), Encoding::Log);
        assert_eq!(Encoding::parse("data"), Encoding::Data);
    }

    #[test]
    fn data_colorspace() {
        let cs = ColorSpace::builder("Raw")
            .encoding(Encoding::Data)
            .build();

        assert!(cs.is_data());
    }
}
