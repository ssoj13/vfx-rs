//! Built-in OCIO configurations.
//!
//! Provides pre-configured ACES and studio configs that can be used
//! without loading external files.
//!
//! # Example
//!
//! ```
//! use vfx_ocio::builtin;
//!
//! // Get ACES 1.3 config
//! let config = builtin::aces_1_3();
//!
//! // List color spaces
//! for cs in config.colorspaces() {
//!     println!("{}", cs.name());
//! }
//! ```

use crate::colorspace::{AllocationInfo, AllocationType, BitDepth, ColorSpace, Encoding, Family};
use crate::config::Config;
use crate::display::{Display, View};
use crate::role;
use crate::transform::*;

/// Creates an ACES 1.3 compatible configuration.
///
/// Includes standard ACES color spaces:
/// - ACES2065-1 (reference)
/// - ACEScg (working space)
/// - ACEScct (color timing)
/// - ACEScc (legacy timing)
/// - Output transforms for sRGB, Rec.709, etc.
pub fn aces_1_3() -> Config {
    let mut config = Config::new();

    // Set version
    // config.version = ConfigVersion::V2;

    // Define color spaces
    config.add_colorspace(aces2065_1());
    config.add_colorspace(acescg());
    config.add_colorspace(acescct());
    config.add_colorspace(acescc());
    config.add_colorspace(raw());
    config.add_colorspace(srgb_linear());
    config.add_colorspace(srgb());
    config.add_colorspace(rec709());

    // Define roles
    config.set_role(role::names::REFERENCE, "ACES2065-1");
    config.set_role(role::names::SCENE_LINEAR, "ACEScg");
    config.set_role(role::names::COMPOSITING_LINEAR, "ACEScg");
    config.set_role(role::names::COLOR_TIMING, "ACEScct");
    config.set_role(role::names::DATA, "Raw");
    config.set_role(role::names::DEFAULT, "ACEScg");
    config.set_role(role::names::ACES_INTERCHANGE, "ACES2065-1");

    // Define displays
    let mut srgb_display = Display::new("sRGB");
    srgb_display.add_view(View::new("ACES 1.0 - SDR Video", "sRGB"));
    srgb_display.add_view(View::new("Raw", "Raw"));
    config.add_display(srgb_display);

    let mut rec709_display = Display::new("Rec.709");
    rec709_display.add_view(View::new("ACES 1.0 - SDR Video", "Rec.709"));
    rec709_display.add_view(View::new("Raw", "Raw"));
    config.add_display(rec709_display);

    config
}

/// Creates a simple sRGB studio config.
///
/// A minimal config suitable for sRGB-only workflows:
/// - Linear sRGB (reference/working)
/// - sRGB (display)
/// - Raw (data)
pub fn srgb_studio() -> Config {
    let mut config = Config::new();

    config.add_colorspace(srgb_linear());
    config.add_colorspace(srgb());
    config.add_colorspace(raw());

    config.set_role(role::names::REFERENCE, "Linear sRGB");
    config.set_role(role::names::SCENE_LINEAR, "Linear sRGB");
    config.set_role(role::names::DEFAULT, "sRGB");
    config.set_role(role::names::DATA, "Raw");

    config
}

/// Creates a Rec.709 broadcast config.
pub fn rec709_studio() -> Config {
    let mut config = Config::new();

    config.add_colorspace(srgb_linear());
    config.add_colorspace(rec709());
    config.add_colorspace(raw());

    config.set_role(role::names::REFERENCE, "Linear sRGB");
    config.set_role(role::names::SCENE_LINEAR, "Linear sRGB");
    config.set_role(role::names::DEFAULT, "Rec.709");
    config.set_role(role::names::DATA, "Raw");

    config
}

// ============================================================================
// Color space definitions
// ============================================================================

/// ACES 2065-1 reference color space (AP0 primaries).
fn aces2065_1() -> ColorSpace {
    ColorSpace::builder("ACES2065-1")
        .alias("aces")
        .alias("ACES - ACES2065-1")
        .family(Family::Aces)
        .encoding(Encoding::SceneLinear)
        .description("ACES 2065-1 reference color space (AP0 primaries)")
        .build()
}

/// ACEScg working color space (AP1 primaries).
fn acescg() -> ColorSpace {
    // AP1 to AP0 matrix
    let to_ref = Transform::matrix([
        0.6954522414, 0.1406786965, 0.1638690622, 0.0,
        0.0447945634, 0.8596711185, 0.0955343182, 0.0,
        -0.0055258826, 0.0040252103, 1.0015006723, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]);

    let from_ref = Transform::matrix([
        1.4514393161, -0.2365107469, -0.2149285693, 0.0,
        -0.0765537734, 1.1762296998, -0.0996759264, 0.0,
        0.0083161484, -0.0060324498, 0.9977163014, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]);

    ColorSpace::builder("ACEScg")
        .alias("ACES - ACEScg")
        .family(Family::Aces)
        .encoding(Encoding::SceneLinear)
        .description("ACEScg working space (AP1 primaries, linear)")
        .to_reference(to_ref)
        .from_reference(from_ref)
        .build()
}

/// ACEScct color timing space (log encoding).
fn acescct() -> ColorSpace {
    // ACEScct uses AP1 primaries (same as ACEScg) + log encoding
    // To reference: decode ACEScct -> linear AP1 -> AP0
    let to_ref = Transform::group(vec![
        // Decode ACEScct log to linear
        Transform::BuiltinTransfer(BuiltinTransferTransform {
            style: "ACEScct".into(),
            direction: TransformDirection::Inverse,
        }),
        // AP1 to AP0 matrix
        Transform::matrix([
            0.6954522414, 0.1406786965, 0.1638690622, 0.0,
            0.0447945634, 0.8596711185, 0.0955343182, 0.0,
            -0.0055258826, 0.0040252103, 1.0015006723, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]),
    ]);

    // From reference: AP0 -> linear AP1 -> encode ACEScct
    let from_ref = Transform::group(vec![
        // AP0 to AP1 matrix
        Transform::matrix([
            1.4514393161, -0.2365107469, -0.2149285693, 0.0,
            -0.0765537734, 1.1762296998, -0.0996759264, 0.0,
            0.0083161484, -0.0060324498, 0.9977163014, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]),
        // Encode to ACEScct log
        Transform::BuiltinTransfer(BuiltinTransferTransform {
            style: "ACEScct".into(),
            direction: TransformDirection::Forward,
        }),
    ]);

    ColorSpace::builder("ACEScct")
        .alias("ACES - ACEScct")
        .family(Family::Aces)
        .encoding(Encoding::Log)
        .description("ACEScct logarithmic color timing space")
        .allocation(AllocationInfo {
            alloc_type: AllocationType::Uniform,
            vars: [-0.35, 1.55],
        })
        .to_reference(to_ref)
        .from_reference(from_ref)
        .build()
}

/// ACEScc color timing space (pure log, legacy).
fn acescc() -> ColorSpace {
    // ACEScc uses AP1 primaries + pure log encoding (no toe)
    let to_ref = Transform::group(vec![
        Transform::BuiltinTransfer(BuiltinTransferTransform {
            style: "ACEScc".into(),
            direction: TransformDirection::Inverse,
        }),
        Transform::matrix([
            0.6954522414, 0.1406786965, 0.1638690622, 0.0,
            0.0447945634, 0.8596711185, 0.0955343182, 0.0,
            -0.0055258826, 0.0040252103, 1.0015006723, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]),
    ]);

    let from_ref = Transform::group(vec![
        Transform::matrix([
            1.4514393161, -0.2365107469, -0.2149285693, 0.0,
            -0.0765537734, 1.1762296998, -0.0996759264, 0.0,
            0.0083161484, -0.0060324498, 0.9977163014, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]),
        Transform::BuiltinTransfer(BuiltinTransferTransform {
            style: "ACEScc".into(),
            direction: TransformDirection::Forward,
        }),
    ]);

    ColorSpace::builder("ACEScc")
        .alias("ACES - ACEScc")
        .family(Family::Aces)
        .encoding(Encoding::Log)
        .description("ACEScc pure logarithmic color timing space")
        .allocation(AllocationInfo {
            alloc_type: AllocationType::Uniform,
            vars: [-0.3584, 1.468],
        })
        .to_reference(to_ref)
        .from_reference(from_ref)
        .build()
}

/// Raw/non-color data space.
fn raw() -> ColorSpace {
    ColorSpace::builder("Raw")
        .alias("raw")
        .alias("Non-Color")
        .alias("Utility - Raw")
        .family(Family::Utility)
        .encoding(Encoding::Data)
        .description("Non-color data (normals, masks, etc.)")
        .is_data(true)
        .build()
}

/// Linear sRGB / Rec.709 primaries.
fn srgb_linear() -> ColorSpace {
    // sRGB to AP0 matrix (via XYZ D65)
    let to_ref = Transform::matrix([
        0.4396658251, 0.3829824270, 0.1773517479, 0.0,
        0.0897912300, 0.8134346456, 0.0967741244, 0.0,
        0.0175437929, 0.1115441718, 0.8709120353, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]);

    let from_ref = Transform::matrix([
        2.5216994244, -1.1368885542, -0.3848108702, 0.0,
        -0.2752540897, 1.3697051137, -0.0944510240, 0.0,
        -0.0159374562, -0.1478051662, 1.1637426224, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]);

    ColorSpace::builder("Linear sRGB")
        .alias("Linear Rec.709")
        .alias("lin_srgb")
        .alias("Utility - Linear - sRGB")
        .family(Family::Utility)
        .encoding(Encoding::SceneLinear)
        .description("Linear sRGB / Rec.709 primaries")
        .to_reference(to_ref)
        .from_reference(from_ref)
        .build()
}

/// sRGB display color space.
fn srgb() -> ColorSpace {
    // sRGB: linear sRGB primaries + sRGB EOTF/OETF
    let to_ref = Transform::group(vec![
        // Decode sRGB gamma to linear
        Transform::BuiltinTransfer(BuiltinTransferTransform {
            style: "sRGB".into(),
            direction: TransformDirection::Inverse,
        }),
        // sRGB to AP0 matrix
        Transform::matrix([
            0.4396658251, 0.3829824270, 0.1773517479, 0.0,
            0.0897912300, 0.8134346456, 0.0967741244, 0.0,
            0.0175437929, 0.1115441718, 0.8709120353, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]),
    ]);

    let from_ref = Transform::group(vec![
        // AP0 to sRGB matrix
        Transform::matrix([
            2.5216994244, -1.1368885542, -0.3848108702, 0.0,
            -0.2752540897, 1.3697051137, -0.0944510240, 0.0,
            -0.0159374562, -0.1478051662, 1.1637426224, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]),
        // Encode linear to sRGB gamma
        Transform::BuiltinTransfer(BuiltinTransferTransform {
            style: "sRGB".into(),
            direction: TransformDirection::Forward,
        }),
    ]);

    ColorSpace::builder("sRGB")
        .alias("srgb")
        .alias("Output - sRGB")
        .family(Family::Display)
        .encoding(Encoding::Sdr)
        .bit_depth(BitDepth::U8)
        .description("sRGB display color space")
        .to_reference(to_ref)
        .from_reference(from_ref)
        .build()
}

/// Rec.709 video color space.
fn rec709() -> ColorSpace {
    // Rec.709: sRGB primaries + Rec.709 OETF
    let to_ref = Transform::group(vec![
        Transform::BuiltinTransfer(BuiltinTransferTransform {
            style: "Rec709".into(),
            direction: TransformDirection::Inverse,
        }),
        Transform::matrix([
            0.4396658251, 0.3829824270, 0.1773517479, 0.0,
            0.0897912300, 0.8134346456, 0.0967741244, 0.0,
            0.0175437929, 0.1115441718, 0.8709120353, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]),
    ]);

    let from_ref = Transform::group(vec![
        Transform::matrix([
            2.5216994244, -1.1368885542, -0.3848108702, 0.0,
            -0.2752540897, 1.3697051137, -0.0944510240, 0.0,
            -0.0159374562, -0.1478051662, 1.1637426224, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]),
        Transform::BuiltinTransfer(BuiltinTransferTransform {
            style: "Rec709".into(),
            direction: TransformDirection::Forward,
        }),
    ]);

    ColorSpace::builder("Rec.709")
        .alias("rec709")
        .alias("BT.709")
        .alias("Output - Rec.709")
        .family(Family::Display)
        .encoding(Encoding::Sdr)
        .description("Rec.709 / BT.709 video color space")
        .to_reference(to_ref)
        .from_reference(from_ref)
        .build()
}

// ============================================================================
// Builtin transform registry
// ============================================================================

/// Built-in transform name registry.
///
/// Maps OCIO v2 builtin transform names to implementations.
pub mod transforms {
    /// ACEScct to ACES2065-1.
    pub const ACESCCT_TO_ACES2065_1: &str = "ACEScct_to_ACES2065-1";
    /// ACES2065-1 to ACEScct.
    pub const ACES2065_1_TO_ACESCCT: &str = "ACES2065-1_to_ACEScct";
    /// ACEScg to ACES2065-1.
    pub const ACESCG_TO_ACES2065_1: &str = "ACEScg_to_ACES2065-1";
    /// ACES2065-1 to ACEScg.
    pub const ACES2065_1_TO_ACESCG: &str = "ACES2065-1_to_ACEScg";
    /// sRGB to linear sRGB.
    pub const SRGB_TO_LINEAR: &str = "sRGB_to_Linear";
    /// Linear sRGB to sRGB.
    pub const LINEAR_TO_SRGB: &str = "Linear_to_sRGB";
    /// Rec.709 to linear.
    pub const REC709_TO_LINEAR: &str = "Rec709_to_Linear";
    /// Linear to Rec.709.
    pub const LINEAR_TO_REC709: &str = "Linear_to_Rec709";
}

/// Returns list of all available builtin config names.
pub fn available_configs() -> &'static [&'static str] {
    &["aces_1.3", "srgb_studio", "rec709_studio"]
}

/// Gets a built-in config by name.
pub fn get_config(name: &str) -> Option<Config> {
    match name.to_lowercase().as_str() {
        "aces_1.3" | "aces" | "aces1.3" => Some(aces_1_3()),
        "srgb_studio" | "srgb" => Some(srgb_studio()),
        "rec709_studio" | "rec709" | "rec.709" => Some(rec709_studio()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aces_config_has_colorspaces() {
        let config = aces_1_3();
        assert!(config.colorspaces().len() >= 8);
    }

    #[test]
    fn aces_config_has_roles() {
        let config = aces_1_3();
        assert!(config.roles().has_reference());
        assert_eq!(config.roles().reference(), Some("ACES2065-1"));
        assert_eq!(config.roles().scene_linear(), Some("ACEScg"));
    }

    #[test]
    fn aces_colorspace_lookup() {
        let config = aces_1_3();

        // By name
        assert!(config.colorspace("ACEScg").is_some());
        
        // By alias
        assert!(config.colorspace("ACES - ACEScg").is_some());
        
        // By role
        assert!(config.colorspace("scene_linear").is_some());
    }

    #[test]
    fn srgb_studio_config() {
        let config = srgb_studio();
        assert!(config.colorspace("Linear sRGB").is_some());
        assert!(config.colorspace("sRGB").is_some());
    }

    #[test]
    fn get_builtin_by_name() {
        assert!(get_config("aces_1.3").is_some());
        assert!(get_config("ACES").is_some());
        assert!(get_config("srgb_studio").is_some());
        assert!(get_config("unknown").is_none());
    }

    #[test]
    fn available_configs_not_empty() {
        assert!(!available_configs().is_empty());
    }

    #[test]
    fn acescg_to_srgb_conversion() {
        let config = aces_1_3();
        let processor = config.processor("ACEScg", "sRGB").unwrap();
        
        // 18% gray in ACEScg
        let mut pixels = [[0.18_f32, 0.18, 0.18]];
        processor.apply_rgb(&mut pixels);
        
        // Should be around 0.46-0.47 in sRGB (gamma encoded 18% gray)
        assert!((pixels[0][0] - 0.46).abs() < 0.05, "R: {}", pixels[0][0]);
        assert!((pixels[0][1] - 0.46).abs() < 0.05, "G: {}", pixels[0][1]);
        assert!((pixels[0][2] - 0.46).abs() < 0.05, "B: {}", pixels[0][2]);
    }

    #[test]
    fn acescct_roundtrip() {
        let config = aces_1_3();
        
        // ACEScg -> ACEScct
        let to_cct = config.processor("ACEScg", "ACEScct").unwrap();
        // ACEScct -> ACEScg
        let from_cct = config.processor("ACEScct", "ACEScg").unwrap();
        
        let original = [0.18_f32, 0.5, 1.0];
        let mut pixels = [original];
        
        to_cct.apply_rgb(&mut pixels);
        from_cct.apply_rgb(&mut pixels);
        
        // Should be back to original within tolerance
        assert!((pixels[0][0] - original[0]).abs() < 0.001, "R: {} vs {}", pixels[0][0], original[0]);
        assert!((pixels[0][1] - original[1]).abs() < 0.001, "G: {} vs {}", pixels[0][1], original[1]);
        assert!((pixels[0][2] - original[2]).abs() < 0.001, "B: {} vs {}", pixels[0][2], original[2]);
    }
}
