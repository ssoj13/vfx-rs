//! OpenColorIO-compatible color management for VFX.
//!
//! This crate provides a Rust implementation of OCIO functionality:
//! - Load and parse `.ocio` configuration files
//! - Define and manage color spaces
//! - Build transform chains for color conversion
//! - Built-in ACES configurations
//!
//! # Quick Start
//!
//! ```
//! use vfx_ocio::{Config, builtin};
//!
//! // Use built-in ACES config
//! let config = builtin::aces_1_3();
//!
//! // Look up color spaces
//! let acescg = config.colorspace("ACEScg").unwrap();
//! println!("Working space: {}", acescg.name());
//!
//! // Create a processor
//! let processor = config.processor("ACEScg", "sRGB").unwrap();
//!
//! // Apply to pixels
//! let mut pixels = [[0.18_f32, 0.18, 0.18]];
//! processor.apply_rgb(&mut pixels);
//! ```
//!
//! # Loading External Configs
//!
//! ```ignore
//! use vfx_ocio::Config;
//!
//! let config = Config::from_file("path/to/config.ocio")?;
//!
//! // List available color spaces
//! for cs in config.colorspaces() {
//!     println!("{}: {:?}", cs.name(), cs.encoding());
//! }
//! ```
//!
//! # Roles
//!
//! Roles provide semantic access to color spaces:
//!
//! ```
//! use vfx_ocio::builtin;
//!
//! let config = builtin::aces_1_3();
//!
//! // Access by role name
//! let linear = config.colorspace("scene_linear").unwrap();
//! assert_eq!(linear.name(), "ACEScg");
//! ```
//!
//! # Display Pipeline
//!
//! ```ignore
//! use vfx_ocio::Config;
//!
//! let config = Config::from_file("config.ocio")?;
//!
//! // Create display processor
//! let proc = config.display_processor("ACEScg", "sRGB", "Film")?;
//! proc.apply_rgb(&mut pixels);
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

mod error;
mod config;
mod colorspace;
mod transform;
mod processor;
mod display;
mod look;
mod role;
mod context;

pub mod builtin;
pub mod validate;

// Re-exports
pub use error::{OcioError, OcioResult};
pub use config::{Config, ConfigVersion, FileRule, NamedTransform, SharedView, ViewingRule};
pub use colorspace::{ColorSpace, Encoding, Family, BitDepth, AllocationInfo, AllocationType, ColorSpaceBuilder};
pub use transform::{
    Transform, TransformDirection, Interpolation,
    MatrixTransform, CdlTransform, CdlStyle,
    ExponentTransform, NegativeStyle,
    LogTransform, FileTransform,
    RangeTransform, RangeStyle,
    GroupTransform, BuiltinTransform,
    ColorSpaceTransform, LookTransform, DisplayViewTransform,
    FixedFunctionTransform, FixedFunctionStyle,
    ExposureContrastTransform, ExposureContrastStyle,
    AllocationTransform, AllocationType as TransformAllocationType,
    BuiltinTransferTransform,
    GradingPrimaryTransform, GradingRgbCurveTransform, GradingToneTransform,
    Lut1DTransform, Lut3DTransform,
};
pub use processor::{Processor, OptimizationLevel, BitDepth as ProcessorBitDepth};
pub use display::{Display, View, ViewTransform, DisplayManager};
pub use look::{Look, LookManager, parse_looks};
pub use role::{Roles, names as role_names};
pub use context::Context;
pub use validate::{check as validate_config, Issue, Severity, IssueCategory, has_errors, has_warnings};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_start_example() {
        // Use built-in ACES config
        let config = builtin::aces_1_3();

        // Look up color spaces
        let acescg = config.colorspace("ACEScg").unwrap();
        assert_eq!(acescg.name(), "ACEScg");

        // Create a processor
        let processor = config.processor("ACEScg", "sRGB").unwrap();

        // Apply to pixels
        let mut pixels = [[0.18_f32, 0.18, 0.18]];
        processor.apply_rgb(&mut pixels);
    }

    #[test]
    fn role_access() {
        let config = builtin::aces_1_3();

        // Access by role name
        let linear = config.colorspace("scene_linear").unwrap();
        assert_eq!(linear.name(), "ACEScg");
    }

    #[test]
    fn context_variables() {
        let mut ctx = Context::new();
        ctx.set("SHOT", "sh010");
        
        let resolved = ctx.resolve("/shows/$SHOT/luts/grade.cube");
        assert_eq!(resolved, "/shows/sh010/luts/grade.cube");
    }

    #[test]
    fn transform_chain() {
        let cdl = Transform::Cdl(CdlTransform {
            slope: [1.1, 1.0, 0.9],
            offset: [0.0, 0.0, 0.0],
            power: [1.0, 1.0, 1.0],
            saturation: 1.0,
            ..Default::default()
        });

        let processor = Processor::from_transform(&cdl, TransformDirection::Forward).unwrap();
        
        let mut pixels = [[0.5_f32, 0.5, 0.5]];
        processor.apply_rgb(&mut pixels);
        
        assert!((pixels[0][0] - 0.55).abs() < 0.01);
    }
}
