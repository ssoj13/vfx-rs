//! Color space definitions and compile-time color space safety.
//!
//! This module provides the [`ColorSpace`] trait and marker types for various
//! color spaces used in VFX production.
//!
//! # Architecture
//!
//! The color system has two complementary representations:
//!
//! 1. **Compile-time markers** ([`ColorSpace`] trait) - Zero-sized types like
//!    [`Srgb`], [`AcesCg`] that enable type-safe image operations.
//!    An `Image<Srgb>` cannot be accidentally mixed with `Image<AcesCg>`.
//!
//! 2. **Runtime identifiers** ([`ColorSpaceId`] enum) - For serialization,
//!    dynamic dispatch, OCIO integration, and metadata storage.
//!
//! The bridge between them:
//! ```text
//! Compile-time          Runtime              Math
//! ────────────          ───────              ────
//! trait ColorSpace  ──► ColorSpaceId  ──► vfx_primaries::Primaries
//!   const ID            .name()            ::from_id()
//!   const NAME          .is_linear()       conversion_matrix()
//!   const PRIMARIES     .primaries()
//! ```
//!
//! # Design Rationale
//!
//! - **Why both?** Compile-time safety catches errors at build time, but
//!   runtime IDs are needed for I/O, configs, and user-facing code.
//! - **Why `const ID`?** Links marker types to runtime enum without overhead.
//! - **Why not just enum?** Would lose compile-time type safety.
//!
//! # Supported Color Spaces
//!
//! ## Scene-Referred (Linear)
//! - [`AcesCg`] - ACEScg (AP1 primaries, linear) - Primary working space
//! - [`Aces2065`] - ACES2065-1 (AP0 primaries, linear) - Archival format
//! - [`LinearSrgb`] - Linear sRGB (Rec.709 primaries, linear)
//! - [`Rec2020`] - ITU-R BT.2020 (wide gamut, linear)
//!
//! ## Display-Referred (Non-Linear)
//! - [`Srgb`] - sRGB with standard transfer function
//! - [`Rec709`] - ITU-R BT.709 with gamma 2.4
//! - [`DciP3`] - DCI-P3 (theater)
//! - [`DisplayP3`] - Display P3 (Apple devices)
//!
//! ## Log-Encoded
//! - [`AcesCct`] - ACEScct (AP1 primaries, log-like curve)
//! - [`AcesCc`] - ACEScc (AP1 primaries, pure log)
//!
//! # Usage
//!
//! ```
//! use vfx_core::prelude::*;
//!
//! // Color spaces are used as type parameters
//! fn process_image<C: ColorSpace>(img: &Image<C, f32, 3>) {
//!     println!("Processing image in {} color space", C::NAME);
//! }
//! ```
//!
//! # Dependencies
//!
//! This module has no external dependencies. It defines the core abstractions
//! used by:
//! - `vfx-primaries` - RGB to XYZ matrices based on primaries
//! - `vfx-transfer` - Transfer functions based on `IS_LINEAR` flag
//! - `vfx-aces` - ACES-specific color space handling

use std::fmt;

// ============================================================================
// ColorSpaceId - Runtime color space identifier
// ============================================================================

/// Runtime identifier for color spaces.
///
/// Bridges compile-time marker types with runtime operations.
/// Use this for:
/// - Serialization/deserialization
/// - Dynamic dispatch
/// - OCIO integration
/// - Metadata storage
///
/// # Example
///
/// ```rust
/// use vfx_core::{ColorSpace, ColorSpaceId, Srgb, AcesCg};
///
/// // Get ID from compile-time type
/// let id = Srgb::ID;
/// assert_eq!(id.name(), "sRGB");
///
/// // Parse from string
/// let id = ColorSpaceId::from_name("ACEScg").unwrap();
/// assert_eq!(id, ColorSpaceId::AcesCg);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ColorSpaceId {
    /// sRGB with standard transfer function.
    Srgb = 0,
    /// Linear sRGB (Rec.709 primaries, linear).
    LinearSrgb = 1,
    /// ACEScg - standard ACES working space.
    AcesCg = 2,
    /// ACES2065-1 - archival ACES format.
    Aces2065 = 3,
    /// ACEScct - log-encoded ACES for grading.
    AcesCct = 4,
    /// ACEScc - pure log ACES.
    AcesCc = 5,
    /// Rec.709 broadcast standard.
    Rec709 = 6,
    /// Rec.2020 wide gamut (linear).
    Rec2020 = 7,
    /// DCI-P3 theatrical.
    DciP3 = 8,
    /// Display P3 (Apple).
    DisplayP3 = 9,
    /// Unknown or unspecified.
    Unknown = 255,
}

impl ColorSpaceId {
    /// All known color space IDs (excluding Unknown).
    pub const ALL: &'static [ColorSpaceId] = &[
        Self::Srgb,
        Self::LinearSrgb,
        Self::AcesCg,
        Self::Aces2065,
        Self::AcesCct,
        Self::AcesCc,
        Self::Rec709,
        Self::Rec2020,
        Self::DciP3,
        Self::DisplayP3,
    ];

    /// Human-readable name.
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Srgb => "sRGB",
            Self::LinearSrgb => "Linear sRGB",
            Self::AcesCg => "ACEScg",
            Self::Aces2065 => "ACES2065-1",
            Self::AcesCct => "ACEScct",
            Self::AcesCc => "ACEScc",
            Self::Rec709 => "Rec.709",
            Self::Rec2020 => "Rec.2020",
            Self::DciP3 => "DCI-P3",
            Self::DisplayP3 => "Display P3",
            Self::Unknown => "Unknown",
        }
    }

    /// Whether this color space uses linear encoding.
    #[inline]
    pub const fn is_linear(&self) -> bool {
        matches!(
            self,
            Self::LinearSrgb | Self::AcesCg | Self::Aces2065 | Self::Rec2020
        )
    }

    /// CIE xy white point chromaticity.
    #[inline]
    pub const fn white_point(&self) -> (f32, f32) {
        match self {
            // D65 white point
            Self::Srgb | Self::LinearSrgb | Self::Rec709 | Self::Rec2020 | Self::DisplayP3 => {
                (0.3127, 0.3290)
            }
            // ACES white (~D60)
            Self::AcesCg | Self::Aces2065 | Self::AcesCct | Self::AcesCc => (0.32168, 0.33767),
            // DCI white
            Self::DciP3 => (0.314, 0.351),
            Self::Unknown => (0.3127, 0.3290),
        }
    }

    /// CIE xy primaries [Red, Green, Blue].
    #[inline]
    pub const fn primaries(&self) -> [(f32, f32); 3] {
        match self {
            // sRGB / Rec.709 primaries
            Self::Srgb | Self::LinearSrgb | Self::Rec709 | Self::Unknown => {
                [(0.640, 0.330), (0.300, 0.600), (0.150, 0.060)]
            }
            // ACES AP1 primaries
            Self::AcesCg | Self::AcesCct | Self::AcesCc => {
                [(0.713, 0.293), (0.165, 0.830), (0.128, 0.044)]
            }
            // ACES AP0 primaries
            Self::Aces2065 => [(0.7347, 0.2653), (0.0000, 1.0000), (0.0001, -0.0770)],
            // Rec.2020 primaries
            Self::Rec2020 => [(0.708, 0.292), (0.170, 0.797), (0.131, 0.046)],
            // P3 primaries
            Self::DciP3 | Self::DisplayP3 => [(0.680, 0.320), (0.265, 0.690), (0.150, 0.060)],
        }
    }

    /// Color space family (ACES, sRGB, Rec, P3).
    #[inline]
    pub const fn family(&self) -> Option<&'static str> {
        match self {
            Self::AcesCg | Self::Aces2065 | Self::AcesCct | Self::AcesCc => Some("ACES"),
            Self::Srgb | Self::LinearSrgb => Some("sRGB"),
            Self::Rec709 | Self::Rec2020 => Some("Rec"),
            Self::DciP3 | Self::DisplayP3 => Some("P3"),
            Self::Unknown => None,
        }
    }

    /// Parse from name string (case-insensitive).
    pub fn from_name(name: &str) -> Option<Self> {
        let lower = name.to_lowercase();
        match lower.as_str() {
            "srgb" | "s-rgb" => Some(Self::Srgb),
            "linear srgb" | "linearsrgb" | "linear_srgb" | "srgb-linear" => Some(Self::LinearSrgb),
            "acescg" | "aces_cg" | "aces-cg" => Some(Self::AcesCg),
            "aces2065-1" | "aces2065" | "ap0" => Some(Self::Aces2065),
            "acescct" | "aces_cct" | "aces-cct" => Some(Self::AcesCct),
            "acescc" | "aces_cc" | "aces-cc" => Some(Self::AcesCc),
            "rec709" | "rec.709" | "bt709" | "bt.709" => Some(Self::Rec709),
            "rec2020" | "rec.2020" | "bt2020" | "bt.2020" => Some(Self::Rec2020),
            "dci-p3" | "dcip3" | "dci_p3" => Some(Self::DciP3),
            "display p3" | "displayp3" | "display_p3" | "p3-display" => Some(Self::DisplayP3),
            "unknown" | "" => Some(Self::Unknown),
            _ => None,
        }
    }
}

impl Default for ColorSpaceId {
    fn default() -> Self {
        Self::Unknown
    }
}

impl fmt::Display for ColorSpaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

// ============================================================================
// ColorSpace Trait
// ============================================================================

/// Trait for color space marker types.
///
/// This trait provides compile-time information about color spaces,
/// enabling type-safe color operations.
///
/// # Implementing Custom Color Spaces
///
/// ```
/// use vfx_core::{ColorSpace, ColorSpaceId};
///
/// #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
/// pub struct MyCustomSpace;
///
/// impl ColorSpace for MyCustomSpace {
///     const ID: ColorSpaceId = ColorSpaceId::Unknown;
///     const NAME: &'static str = "MyCustom";
///     const IS_LINEAR: bool = true;
///     const WHITE_POINT: (f32, f32) = (0.3127, 0.3290); // D65
///     const PRIMARIES: [(f32, f32); 3] = [
///         (0.640, 0.330), // Red
///         (0.300, 0.600), // Green
///         (0.150, 0.060), // Blue
///     ];
/// }
/// ```
pub trait ColorSpace: Copy + Clone + Default + Send + Sync + fmt::Debug + 'static {
    /// Runtime identifier for this color space.
    ///
    /// Enables bridging between compile-time and runtime representations.
    const ID: ColorSpaceId;

    /// Human-readable name of the color space.
    ///
    /// Used for display, logging, and metadata.
    const NAME: &'static str;

    /// Whether this color space uses linear light encoding.
    ///
    /// - `true` for scene-referred linear spaces (ACEScg, Linear sRGB)
    /// - `false` for display-referred or log-encoded spaces (sRGB, ACEScct)
    const IS_LINEAR: bool;

    /// CIE xy chromaticity coordinates of the white point.
    ///
    /// Common values:
    /// - D65: (0.3127, 0.3290) - sRGB, Rec.709, Rec.2020
    /// - D60: (0.32168, 0.33767) - ACES
    /// - DCI: (0.314, 0.351) - DCI-P3
    const WHITE_POINT: (f32, f32);

    /// CIE xy chromaticity coordinates of RGB primaries.
    ///
    /// Order: `[Red, Green, Blue]`
    ///
    /// These define the color gamut of the color space.
    const PRIMARIES: [(f32, f32); 3];

    /// Optional family/category for grouping related color spaces.
    ///
    /// Examples: "ACES", "Rec", "Display"
    const FAMILY: Option<&'static str> = None;

    /// Whether this is a scene-referred color space.
    ///
    /// Scene-referred spaces can represent values outside [0, 1].
    #[inline]
    fn is_scene_referred() -> bool {
        Self::IS_LINEAR
    }

    /// Whether this is a display-referred color space.
    ///
    /// Display-referred spaces are typically clamped to [0, 1].
    #[inline]
    fn is_display_referred() -> bool {
        !Self::IS_LINEAR
    }
}

// ============================================================================
// ACES Color Spaces
// ============================================================================

/// ACES2065-1 - The archival ACES color space.
///
/// Uses AP0 primaries which encompass the entire visible spectrum.
/// This is the interchange format for ACES.
///
/// # Characteristics
/// - **Primaries**: AP0 (spectral locus)
/// - **White Point**: ACES white (~D60)
/// - **Transfer**: Linear
/// - **Usage**: Archival, interchange
///
/// # When to Use
/// - Long-term archival storage
/// - Interchange between facilities
/// - Maximum color preservation
///
/// # References
/// - [ACES Documentation](https://docs.acescentral.com)
/// - SMPTE ST 2065-1
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Aces2065;

impl ColorSpace for Aces2065 {
    const ID: ColorSpaceId = ColorSpaceId::Aces2065;
    const NAME: &'static str = "ACES2065-1";
    const IS_LINEAR: bool = true;
    const WHITE_POINT: (f32, f32) = (0.32168, 0.33767); // ACES white (~D60)
    const PRIMARIES: [(f32, f32); 3] = [
        (0.7347, 0.2653),  // Red (spectral locus)
        (0.0000, 1.0000),  // Green (spectral locus)
        (0.0001, -0.0770), // Blue (spectral locus, imaginary)
    ];
    const FAMILY: Option<&'static str> = Some("ACES");
}

/// ACEScg - The standard ACES working color space.
///
/// Uses AP1 primaries which are more practical for CG rendering
/// while still being wide-gamut.
///
/// # Characteristics
/// - **Primaries**: AP1
/// - **White Point**: ACES white (~D60)
/// - **Transfer**: Linear
/// - **Usage**: CG rendering, compositing
///
/// # When to Use
/// - 3D rendering
/// - Compositing
/// - Color grading working space
///
/// # References
/// - [ACEScg Specification](https://docs.acescentral.com)
/// - S-2014-004
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AcesCg;

impl ColorSpace for AcesCg {
    const ID: ColorSpaceId = ColorSpaceId::AcesCg;
    const NAME: &'static str = "ACEScg";
    const IS_LINEAR: bool = true;
    const WHITE_POINT: (f32, f32) = (0.32168, 0.33767);
    const PRIMARIES: [(f32, f32); 3] = [
        (0.713, 0.293), // Red
        (0.165, 0.830), // Green
        (0.128, 0.044), // Blue
    ];
    const FAMILY: Option<&'static str> = Some("ACES");
}

/// ACEScct - Log-encoded ACES for color correction.
///
/// Uses AP1 primaries with a log-like transfer function optimized
/// for color grading in DaVinci Resolve and similar tools.
///
/// # Characteristics
/// - **Primaries**: AP1
/// - **White Point**: ACES white (~D60)
/// - **Transfer**: ACEScct log curve (toe + log)
/// - **Usage**: Color grading
///
/// # Transfer Function
/// The ACEScct curve has a toe region for better shadow handling:
/// - Below 0.0078125: linear portion
/// - Above: logarithmic portion
///
/// # When to Use
/// - Color grading sessions
/// - When you need CDL-like controls
/// - DaVinci Resolve, Baselight
///
/// # Note
/// The actual transfer function is implemented in `vfx-transfer`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AcesCct;

impl ColorSpace for AcesCct {
    const ID: ColorSpaceId = ColorSpaceId::AcesCct;
    const NAME: &'static str = "ACEScct";
    const IS_LINEAR: bool = false;
    const WHITE_POINT: (f32, f32) = (0.32168, 0.33767);
    const PRIMARIES: [(f32, f32); 3] = [
        (0.713, 0.293),
        (0.165, 0.830),
        (0.128, 0.044),
    ];
    const FAMILY: Option<&'static str> = Some("ACES");
}

/// ACEScc - Pure log-encoded ACES for color correction.
///
/// Similar to ACEScct but with a pure logarithmic curve (no toe).
///
/// # Characteristics
/// - **Primaries**: AP1
/// - **White Point**: ACES white (~D60)
/// - **Transfer**: Pure log curve
/// - **Usage**: Color grading (legacy)
///
/// # Note
/// ACEScct is generally preferred over ACEScc for modern workflows.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AcesCc;

impl ColorSpace for AcesCc {
    const ID: ColorSpaceId = ColorSpaceId::AcesCc;
    const NAME: &'static str = "ACEScc";
    const IS_LINEAR: bool = false;
    const WHITE_POINT: (f32, f32) = (0.32168, 0.33767);
    const PRIMARIES: [(f32, f32); 3] = [
        (0.713, 0.293),
        (0.165, 0.830),
        (0.128, 0.044),
    ];
    const FAMILY: Option<&'static str> = Some("ACES");
}

// ============================================================================
// sRGB / Rec.709 Color Spaces
// ============================================================================

/// sRGB - Standard RGB color space for web and consumer displays.
///
/// The most common color space for images on the web and in consumer
/// applications.
///
/// # Characteristics
/// - **Primaries**: Rec.709 / sRGB
/// - **White Point**: D65
/// - **Transfer**: sRGB curve (linear near black, ~2.2 gamma)
/// - **Usage**: Web, photography, consumer displays
///
/// # Transfer Function
/// ```text
/// if L <= 0.0031308:
///     V = 12.92 * L
/// else:
///     V = 1.055 * L^(1/2.4) - 0.055
/// ```
///
/// # When to Use
/// - Final output for web
/// - JPEG/PNG export
/// - Consumer display viewing
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Srgb;

impl ColorSpace for Srgb {
    const ID: ColorSpaceId = ColorSpaceId::Srgb;
    const NAME: &'static str = "sRGB";
    const IS_LINEAR: bool = false;
    const WHITE_POINT: (f32, f32) = (0.3127, 0.3290); // D65
    const PRIMARIES: [(f32, f32); 3] = [
        (0.640, 0.330), // Red
        (0.300, 0.600), // Green
        (0.150, 0.060), // Blue
    ];
    const FAMILY: Option<&'static str> = Some("sRGB");
}

/// Linear sRGB - sRGB primaries with linear transfer.
///
/// Same primaries as sRGB but without the gamma curve.
/// Common intermediate space for rendering and compositing.
///
/// # Characteristics
/// - **Primaries**: Rec.709 / sRGB
/// - **White Point**: D65
/// - **Transfer**: Linear (1.0 gamma)
/// - **Usage**: Rendering, compositing, intermediate
///
/// # When to Use
/// - Blender, Unity, Unreal (linear workflow)
/// - Compositing in Nuke, After Effects
/// - When sRGB output is the final target
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct LinearSrgb;

impl ColorSpace for LinearSrgb {
    const ID: ColorSpaceId = ColorSpaceId::LinearSrgb;
    const NAME: &'static str = "Linear sRGB";
    const IS_LINEAR: bool = true;
    const WHITE_POINT: (f32, f32) = (0.3127, 0.3290);
    const PRIMARIES: [(f32, f32); 3] = [
        (0.640, 0.330),
        (0.300, 0.600),
        (0.150, 0.060),
    ];
    const FAMILY: Option<&'static str> = Some("sRGB");
}

/// Rec.709 - ITU-R BT.709 broadcast color space.
///
/// The HD video standard. Same primaries as sRGB but with
/// BT.1886 transfer function (2.4 gamma).
///
/// # Characteristics
/// - **Primaries**: Rec.709
/// - **White Point**: D65
/// - **Transfer**: BT.1886 (2.4 gamma)
/// - **Usage**: HD broadcast, video production
///
/// # When to Use
/// - Video output for broadcast
/// - HD video editing
/// - When targeting TV displays
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rec709;

impl ColorSpace for Rec709 {
    const ID: ColorSpaceId = ColorSpaceId::Rec709;
    const NAME: &'static str = "Rec.709";
    const IS_LINEAR: bool = false;
    const WHITE_POINT: (f32, f32) = (0.3127, 0.3290);
    const PRIMARIES: [(f32, f32); 3] = [
        (0.640, 0.330),
        (0.300, 0.600),
        (0.150, 0.060),
    ];
    const FAMILY: Option<&'static str> = Some("Rec");
}

// ============================================================================
// Wide Gamut Color Spaces
// ============================================================================

/// Rec.2020 - ITU-R BT.2020 wide gamut color space.
///
/// The UHD/4K/HDR video standard with significantly wider gamut
/// than Rec.709.
///
/// # Characteristics
/// - **Primaries**: Rec.2020 (wide gamut)
/// - **White Point**: D65
/// - **Transfer**: Can be linear, PQ, or HLG
/// - **Usage**: HDR video, UHD broadcast
///
/// # Note
/// This type represents Linear Rec.2020. For HDR output,
/// apply PQ or HLG transfer function via `vfx-transfer`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rec2020;

impl ColorSpace for Rec2020 {
    const ID: ColorSpaceId = ColorSpaceId::Rec2020;
    const NAME: &'static str = "Rec.2020";
    const IS_LINEAR: bool = true; // Linear variant
    const WHITE_POINT: (f32, f32) = (0.3127, 0.3290);
    const PRIMARIES: [(f32, f32); 3] = [
        (0.708, 0.292), // Red
        (0.170, 0.797), // Green
        (0.131, 0.046), // Blue
    ];
    const FAMILY: Option<&'static str> = Some("Rec");
}

/// DCI-P3 - Digital Cinema Initiative P3 color space.
///
/// The theatrical digital cinema standard.
///
/// # Characteristics
/// - **Primaries**: DCI-P3
/// - **White Point**: DCI white (greenish, ~6300K)
/// - **Transfer**: 2.6 gamma
/// - **Usage**: Digital cinema projection
///
/// # When to Use
/// - Creating DCPs (Digital Cinema Packages)
/// - Theatrical release
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DciP3;

impl ColorSpace for DciP3 {
    const ID: ColorSpaceId = ColorSpaceId::DciP3;
    const NAME: &'static str = "DCI-P3";
    const IS_LINEAR: bool = false;
    const WHITE_POINT: (f32, f32) = (0.314, 0.351); // DCI white
    const PRIMARIES: [(f32, f32); 3] = [
        (0.680, 0.320), // Red
        (0.265, 0.690), // Green
        (0.150, 0.060), // Blue
    ];
    const FAMILY: Option<&'static str> = Some("P3");
}

/// Display P3 - Apple's P3 color space for displays.
///
/// Uses DCI-P3 primaries but with D65 white point and sRGB transfer.
///
/// # Characteristics
/// - **Primaries**: DCI-P3
/// - **White Point**: D65
/// - **Transfer**: sRGB curve
/// - **Usage**: Apple devices, modern wide-gamut displays
///
/// # When to Use
/// - Apple device displays
/// - Wide-gamut web content
/// - Modern HDR displays
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DisplayP3;

impl ColorSpace for DisplayP3 {
    const ID: ColorSpaceId = ColorSpaceId::DisplayP3;
    const NAME: &'static str = "Display P3";
    const IS_LINEAR: bool = false;
    const WHITE_POINT: (f32, f32) = (0.3127, 0.3290); // D65
    const PRIMARIES: [(f32, f32); 3] = [
        (0.680, 0.320),
        (0.265, 0.690),
        (0.150, 0.060),
    ];
    const FAMILY: Option<&'static str> = Some("P3");
}

// ============================================================================
// Generic / Unknown Color Space
// ============================================================================

/// Unknown or unspecified color space.
///
/// Used when the color space is not known or not relevant.
/// Operations between `Unknown` and typed color spaces should
/// be avoided.
///
/// # When to Use
/// - Reading images without color space metadata
/// - Legacy workflows
/// - Testing
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Unknown;

impl ColorSpace for Unknown {
    const ID: ColorSpaceId = ColorSpaceId::Unknown;
    const NAME: &'static str = "Unknown";
    const IS_LINEAR: bool = false;
    const WHITE_POINT: (f32, f32) = (0.3127, 0.3290); // Assume D65
    const PRIMARIES: [(f32, f32); 3] = [
        (0.640, 0.330), // Assume sRGB
        (0.300, 0.600),
        (0.150, 0.060),
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colorspace_id_roundtrip() {
        // All marker types have correct ID
        assert_eq!(Srgb::ID, ColorSpaceId::Srgb);
        assert_eq!(AcesCg::ID, ColorSpaceId::AcesCg);
        assert_eq!(Aces2065::ID, ColorSpaceId::Aces2065);
        assert_eq!(Rec2020::ID, ColorSpaceId::Rec2020);
    }

    #[test]
    fn test_colorspace_id_properties() {
        let id = ColorSpaceId::AcesCg;
        assert_eq!(id.name(), "ACEScg");
        assert!(id.is_linear());
        assert_eq!(id.family(), Some("ACES"));
    }

    #[test]
    fn test_colorspace_id_from_name() {
        assert_eq!(ColorSpaceId::from_name("sRGB"), Some(ColorSpaceId::Srgb));
        assert_eq!(ColorSpaceId::from_name("ACEScg"), Some(ColorSpaceId::AcesCg));
        assert_eq!(ColorSpaceId::from_name("rec.709"), Some(ColorSpaceId::Rec709));
        assert_eq!(ColorSpaceId::from_name("unknown_space"), None);
    }

    #[test]
    fn test_colorspace_id_all() {
        // ALL should contain all known spaces
        assert_eq!(ColorSpaceId::ALL.len(), 10);
        assert!(ColorSpaceId::ALL.contains(&ColorSpaceId::Srgb));
        assert!(ColorSpaceId::ALL.contains(&ColorSpaceId::AcesCg));
        assert!(!ColorSpaceId::ALL.contains(&ColorSpaceId::Unknown));
    }

    #[test]
    fn test_colorspace_names() {
        assert_eq!(Srgb::NAME, "sRGB");
        assert_eq!(AcesCg::NAME, "ACEScg");
        assert_eq!(Rec2020::NAME, "Rec.2020");
    }

    #[test]
    fn test_colorspace_linearity() {
        assert!(!Srgb::IS_LINEAR);
        assert!(AcesCg::IS_LINEAR);
        assert!(LinearSrgb::IS_LINEAR);
        assert!(!AcesCct::IS_LINEAR);
    }

    #[test]
    fn test_scene_display_referred() {
        assert!(AcesCg::is_scene_referred());
        assert!(!AcesCg::is_display_referred());
        assert!(!Srgb::is_scene_referred());
        assert!(Srgb::is_display_referred());
    }

    #[test]
    fn test_aces_family() {
        assert_eq!(AcesCg::FAMILY, Some("ACES"));
        assert_eq!(AcesCct::FAMILY, Some("ACES"));
        assert_eq!(Aces2065::FAMILY, Some("ACES"));
    }
}
