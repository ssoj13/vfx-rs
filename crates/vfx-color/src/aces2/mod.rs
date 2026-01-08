//! ACES 2.0 Output Transform
//!
//! Full implementation of the ACES 2.0 Output Transform based on OCIO reference.
//! This is the modern CAM16-based tone mapping and gamut mapping system.
//!
//! # Pipeline
//!
//! ```text
//! ACEScg (AP1) -> JMh -> Tonescale -> Chroma Compress -> Gamut Compress -> Display RGB
//! ```
//!
//! # Key Components
//!
//! - **CAM16**: Color Appearance Model for perceptual color representation
//! - **JMh**: Lightness (J), Colorfulness (M), Hue (h) color space
//! - **Tonescale**: HDR to SDR mapping with configurable peak luminance
//! - **Chroma Compression**: Saturation rolloff to prevent clipping
//! - **Gamut Compression**: Map out-of-gamut colors to display gamut
//!
//! # Reference
//!
//! Based on OpenColorIO ACES2 implementation:
//! - OCIO/src/OpenColorIO/ops/fixedfunction/ACES2/

mod common;
mod cam;
mod tonescale;
mod chroma;
mod gamut;
mod tables;
mod transform;

pub use common::*;
pub use cam::*;
pub use tonescale::*;
pub use chroma::*;
pub use gamut::*;
pub use transform::*;
