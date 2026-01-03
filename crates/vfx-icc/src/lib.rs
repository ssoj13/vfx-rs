//! # vfx-icc
//!
//! ICC color profile support for VFX color pipelines.
//!
//! This crate provides a high-level interface for working with ICC color profiles,
//! built on top of the industry-standard Little CMS 2 library.
//!
//! # Features
//!
//! - Load ICC profiles from files or embedded data
//! - Create standard profiles (sRGB, Adobe RGB, Display P3, ACES, etc.)
//! - Transform colors between profiles
//! - Support for different rendering intents
//! - High-precision 32-bit float processing
//!
//! # Example
//!
//! ```rust,no_run
//! use vfx_icc::{Profile, Transform, Intent};
//! use std::path::Path;
//!
//! // Load a camera profile
//! let camera = Profile::from_file(Path::new("camera.icc")).unwrap();
//!
//! // Get the working space profile
//! let aces = Profile::aces_ap0();
//!
//! // Create a transform
//! let transform = Transform::new(&camera, &aces, Intent::Perceptual).unwrap();
//!
//! // Transform pixels
//! let mut pixels = vec![[0.5f32, 0.3, 0.2]; 100];
//! transform.apply(&mut pixels);
//! ```
//!
//! # Supported Color Spaces
//!
//! Built-in profiles include:
//! - sRGB (IEC 61966-2-1)
//! - Adobe RGB (1998)
//! - Display P3
//! - DCI-P3
//! - ACES AP0 (linear)
//! - ACES AP1 / ACEScg (linear)
//! - Rec. 709
//! - Rec. 2020
//!
//! # Thread Safety
//!
//! Transforms can be shared between threads when created with caching disabled.
//! Use [`Transform::new_uncached`] for thread-safe transforms.

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

mod profile;
mod transform;
mod error;
mod standard;

pub use profile::Profile;
pub use transform::{Transform, convert_rgb};
pub use error::{IccError, IccResult};
pub use standard::StandardProfile;

/// Rendering intent for color transformations.
///
/// Determines how out-of-gamut colors are handled during conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Intent {
    /// Maintains color accuracy within the destination gamut.
    ///
    /// Out-of-gamut colors are clipped to the nearest in-gamut color.
    /// Best for proofing and accurate color reproduction.
    #[default]
    Perceptual,

    /// Preserves the relationship between colors.
    ///
    /// Compresses the entire source gamut to fit within the destination.
    /// Best for photographic images.
    RelativeColorimetric,

    /// Maintains saturation at the expense of accuracy.
    ///
    /// Best for business graphics where vivid colors are more important
    /// than exact color matching.
    Saturation,

    /// Like relative colorimetric but with white point adaptation.
    ///
    /// Maps source white to destination white exactly.
    /// Best for spot color matching.
    AbsoluteColorimetric,
}

impl From<Intent> for lcms2::Intent {
    fn from(intent: Intent) -> Self {
        match intent {
            Intent::Perceptual => lcms2::Intent::Perceptual,
            Intent::RelativeColorimetric => lcms2::Intent::RelativeColorimetric,
            Intent::Saturation => lcms2::Intent::Saturation,
            Intent::AbsoluteColorimetric => lcms2::Intent::AbsoluteColorimetric,
        }
    }
}


