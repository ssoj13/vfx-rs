//! # vfx-ops
//!
//! Image processing operations for VFX color pipelines.
//!
//! This crate provides fundamental image operations used in compositing
//! and visual effects workflows.
//!
//! # Modules
//!
//! - [`resize`] - Image scaling and resampling
//! - [`composite`] - Layer blending operations
//! - [`filter`] - Convolution and filtering
//! - [`transform`] - Geometric transformations
//!
//! # Example
//!
//! ```rust
//! use vfx_ops::{resize, composite, BlendMode};
//!
//! // Resize an image using Lanczos filter
//! // let scaled = resize::resize(&image, 1920, 1080, resize::Filter::Lanczos3);
//!
//! // Composite two layers
//! // let result = composite::over(&fg, &bg);
//! ```
//!
//! # Common Operations
//!
//! ## Resize/Scale
//!
//! ```rust,ignore
//! use vfx_ops::resize::{resize, Filter};
//!
//! let scaled = resize(&image, new_width, new_height, Filter::Lanczos3)?;
//! ```
//!
//! ## Composite
//!
//! ```rust,ignore
//! use vfx_ops::composite::{over, BlendMode};
//!
//! // Porter-Duff Over
//! let result = over(&foreground, &background)?;
//!
//! // Blend modes
//! let multiplied = blend(&a, &b, BlendMode::Multiply)?;
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

mod error;
pub mod guard;
pub mod layer_ops;
pub mod resize;
pub mod composite;
pub mod filter;
pub mod transform;
pub mod warp;

#[cfg(feature = "parallel")]
pub mod parallel;

pub use error::{OpsError, OpsResult};
pub use resize::Filter;
pub use composite::BlendMode;
