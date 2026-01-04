//! # vfx-lut
//!
//! Look-Up Table (LUT) types and operations for VFX color pipelines.
//!
//! This crate provides data structures for 1D and 3D LUTs, commonly used
//! for color grading, display calibration, and color space conversions.
//!
//! # LUT Types
//!
//! - [`Lut1D`] - 1-dimensional lookup table (per-channel curves)
//! - [`Lut3D`] - 3-dimensional lookup table (full RGB cube)
//!
//! # Supported Formats
//!
//! - `.cube` - Adobe/Resolve LUT format (parsing in vfx-io)
//! - `.clf` - Academy Common LUT Format ([`clf`] module)
//! - `.spi1d` / `.spi3d` - Sony Pictures Imageworks ([`spi`] module)
//!
//! # Usage
//!
//! ```rust
//! use vfx_lut::{Lut1D, Lut3D, Interpolation};
//!
//! // Create a 1D LUT (e.g., gamma curve)
//! let lut = Lut1D::gamma(1024, 2.2);
//! let output = lut.apply(0.5);
//!
//! // Create a 3D LUT (e.g., color grade)
//! let lut = Lut3D::identity(33);
//! let rgb = lut.apply([0.5, 0.3, 0.2]);
//! ```
//!
//! # Interpolation
//!
//! - 1D LUTs: Linear interpolation
//! - 3D LUTs: Trilinear or tetrahedral interpolation
//!
//! # Dependencies
//!
//! - [`vfx-core`] - Core types
//! - [`thiserror`] - Error handling
//!
//! # Used By
//!
//! - `vfx-color` - Color transformations
//! - `vfx-io` - LUT file loading

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

mod lut1d;
mod lut3d;
mod interp;
mod error;
pub mod cube;
pub mod clf;
pub mod spi;
pub mod threedl;

pub use lut1d::Lut1D;
pub use lut3d::Lut3D;
pub use interp::Interpolation;
pub use error::{LutError, LutResult};
pub use cube::{read_1d as read_cube_1d, read_3d as read_cube_3d, write_1d as write_cube_1d, write_3d as write_cube_3d};
pub use clf::{ProcessList, ProcessNode, read_clf, write_clf, read_ctf, write_ctf, parse_ctf};
pub use spi::{read_spi1d, read_spi3d, write_spi1d, write_spi3d};
pub use threedl::{read_3dl, parse_3dl, write_3dl, write_3dl_with_depth};
