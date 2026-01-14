//! # vfx-core
//!
//! Core types for VFX image and color processing.
//!
//! This crate provides the foundational types used throughout the VFX-RS ecosystem:
//!
//! - [`ColorSpace`] - Trait and marker types for compile-time color space safety
//! - [`Pixel`], [`Rgb`], [`Rgba`] - Generic pixel types with color space tracking
//! - [`Image`] - Zero-copy image buffer with color space awareness
//! - [`ImageSpec`] - Image metadata and specifications
//! - [`Rect`], [`Roi`] - Region of interest types
//!
//! ## Design Philosophy
//!
//! The core principle is **compile-time color space safety**. An image in sRGB
//! cannot be accidentally mixed with an image in ACEScg without explicit conversion:
//!
//! ```ignore
//! let srgb: Image<Srgb, u8> = read("photo.jpg")?;
//! let aces: Image<AcesCg, f32> = srgb.convert(); // Explicit conversion
//! // let bad = srgb + aces; // Compile error!
//! ```
//!
//! ## Crate Structure
//!
//! This crate is the foundation of VFX-RS and has no internal dependencies.
//! All other VFX-RS crates depend on `vfx-core`:
//!
//! ```text
//! vfx-core (this crate)
//!    ^
//!    |
//!    +-- vfx-math (matrices, interpolation)
//!    +-- vfx-lut (LUT types)
//!    +-- vfx-transfer (transfer functions)
//!    +-- vfx-primaries (color primaries)
//!    +-- vfx-io (image I/O)
//!    +-- vfx-ops (image operations)
//!    +-- ... (all other crates)
//! ```
//!
//! ## Feature Flags
//!
//! - `serde` - Enable serialization for metadata types
//! - `rayon` - Enable parallel pixel iteration (enabled by default)

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod colorspace;
pub mod error;
pub mod format;
pub mod image;
pub mod pixel;
pub mod rect;
pub mod spec;

// Re-exports for convenience
pub use colorspace::*;
pub use error::*;
pub use format::*;
pub use image::*;
pub use pixel::{luminance_rec709, PixelFormat, Rgb, Rgba, REC709_LUMA, REC709_LUMA_B, REC709_LUMA_G, REC709_LUMA_R};
pub use rect::*;
pub use spec::*;

/// Prelude module for convenient imports.
///
/// # Usage
///
/// ```
/// use vfx_core::prelude::*;
/// ```
pub mod prelude {
    pub use crate::colorspace::{
        Aces2065, AcesCc, AcesCct, AcesCg, ColorSpace, ColorSpaceId, DciP3, DisplayP3,
        LinearSrgb, Rec2020, Rec709, Srgb,
    };
    pub use crate::error::{Error, Result};
    pub use crate::format::{
        Aggregate, BaseType, BitDepth, DataFormat, TypeDesc, VecSemantics,
    };
    pub use crate::image::{Image, ImageView, ImageViewMut};
    pub use crate::pixel::{
        luminance_rec709, PixelFormat, Rgb, Rgba, REC709_LUMA, REC709_LUMA_B,
        REC709_LUMA_G, REC709_LUMA_R,
    };
    pub use crate::rect::{Rect, Roi, Roi3D};
    pub use crate::spec::ImageSpec;
}
