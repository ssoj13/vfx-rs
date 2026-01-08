//! # vfx-color
//!
//! Unified color transformation API for VFX pipelines.
//!
//! This crate combines all color-related functionality into a single,
//! easy-to-use API:
//!
//! - **Transfer functions** - Gamma, sRGB, PQ, HLG, LogC, S-Log3, V-Log
//! - **Color primaries** - RGB/XYZ matrices for all major color spaces
//! - **Chromatic adaptation** - Bradford, CAT02, Von Kries
//! - **LUTs** - 1D and 3D lookup tables with interpolation
//! - **CDL** - ASC Color Decision List (Slope/Offset/Power/Saturation)
//!
//! # Architecture
//!
//! ```text
//!                    vfx-color
//!                        |
//!     +------------------+------------------+
//!     |                  |                  |
//! vfx-transfer    vfx-primaries        vfx-lut
//!     |                  |                  |
//!     +--------+---------+                  |
//!              |                            |
//!          vfx-math                         |
//!              |                            |
//!              +----------------------------+
//!                          |
//!                      vfx-core
//! ```
//!
//! # Quick Start
//!
//! ```rust
//! use vfx_color::{ColorProcessor, Pipeline};
//! use vfx_color::transfer::{srgb, pq};
//! use vfx_color::primaries::{SRGB, REC2020, rgb_to_xyz_matrix, xyz_to_rgb_matrix};
//!
//! // Create a color processor
//! let mut proc = ColorProcessor::new();
//!
//! // Build a transform pipeline: sRGB -> Linear -> Rec.2020 PQ
//! let pipeline = Pipeline::new()
//!     .transfer_in(srgb::eotf)           // sRGB EOTF (decode)
//!     .matrix(rgb_to_xyz_matrix(&SRGB))
//!     .matrix(xyz_to_rgb_matrix(&REC2020))
//!     .transfer_out(pq::oetf);            // PQ OETF (encode)
//!
//! // Apply to RGB
//! let rgb = [0.5, 0.3, 0.2];
//! let result = proc.apply(&pipeline, rgb);
//! ```
//!
//! # Color Spaces
//!
//! Common color spaces and their components:
//!
//! | Space | Primaries | Transfer | White Point |
//! |-------|-----------|----------|-------------|
//! | sRGB | Rec.709 | sRGB EOTF | D65 |
//! | Rec.709 | Rec.709 | BT.1886 | D65 |
//! | Rec.2020 | Rec.2020 | BT.1886 | D65 |
//! | Rec.2100 PQ | Rec.2020 | PQ | D65 |
//! | Rec.2100 HLG | Rec.2020 | HLG | D65 |
//! | DCI-P3 | P3 | Gamma 2.6 | DCI White |
//! | Display P3 | P3 | sRGB | D65 |
//! | ACEScg | AP1 | Linear | D60 |
//! | ACES 2065-1 | AP0 | Linear | D60 |
//! | ARRI LogC3 | ARRI Wide | LogC3 | D65 |
//! | Sony S-Log3 | S-Gamut3 | S-Log3 | D65 |
//! | Panasonic V-Log | V-Gamut | V-Log | D65 |
//!
//! # Dependencies
//!
//! - [`vfx-core`] - Core types (`ColorSpace`, `Image`)
//! - [`vfx-math`] - Math utilities (Vec3, Mat3, chromatic adaptation)
//! - [`vfx-transfer`] - Transfer function implementations
//! - [`vfx-primaries`] - Color space primaries and matrices
//! - [`vfx-lut`] - 1D and 3D LUT types
//!
//! # Used By
//!
//! - `vfx-io` - Color conversion during I/O
//! - `vfx-ops` - Color grading operations
//! - `vfx` - Unified API

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

mod error;
mod pipeline;
mod processor;
pub mod convert;
pub mod cdl;
pub mod aces;
pub mod aces2;

pub use error::{ColorError, ColorResult};
pub use pipeline::{Pipeline, TransformOp};
pub use processor::ColorProcessor;
pub use convert::{Convert, RgbConvert};

// Re-export sub-crates for convenience
pub use vfx_transfer as transfer;
pub use vfx_primaries as primaries;
pub use vfx_lut as lut;
pub use vfx_math as math;

/// Prelude with commonly used types
pub mod prelude {
    pub use crate::{
        ColorProcessor,
        Pipeline,
        TransformOp,
        Convert,
        RgbConvert,
    };
    
    // Re-export common transfer functions
    pub use vfx_transfer::{srgb, gamma, pq, hlg};
    
    // Re-export primaries and matrix functions
    pub use vfx_primaries::{
        Primaries, SRGB, REC709, REC2020, DCI_P3, DISPLAY_P3,
        ACES_AP0, ACES_AP1, ADOBE_RGB,
        rgb_to_xyz_matrix, xyz_to_rgb_matrix, rgb_to_rgb_matrix,
    };
    
    // Re-export LUT types
    pub use vfx_lut::{Lut1D, Lut3D, Interpolation};
    
    // Re-export math
    pub use vfx_math::{Vec3, Mat3};
    
    // Re-export adaptation
    pub use vfx_math::{adapt_matrix, BRADFORD, CAT02, VON_KRIES, D65, D50, D60};
}
