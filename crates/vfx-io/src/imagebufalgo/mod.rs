//! OIIO-compatible ImageBufAlgo implementation.
//!
//! ImageBufAlgo provides a comprehensive set of image processing operations
//! compatible with OpenImageIO's ImageBufAlgo namespace.
//!
//! # Modules
//!
//! - [`patterns`] - Pattern generation (fill, checker, noise)
//! - [`channels`] - Channel operations (shuffle, append, extract)
//! - [`geometry`] - Geometric operations (crop, flip, rotate, resize)
//! - [`arithmetic`] - Arithmetic operations (add, sub, mul, div, over)
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::imagebuf::{ImageBuf, InitializePixels};
//! use vfx_io::imagebufalgo::{fill, add, flip};
//! use vfx_core::ImageSpec;
//!
//! // Create and fill an image
//! let spec = ImageSpec::rgba(100, 100);
//! let mut buf = ImageBuf::new(spec, InitializePixels::No);
//! fill(&mut buf, &[1.0, 0.0, 0.0, 1.0], None);
//!
//! // Apply operations
//! let flipped = flip(&buf, None);
//! ```

pub mod patterns;
pub mod channels;
pub mod geometry;
pub mod arithmetic;

// Re-export commonly used functions
pub use patterns::{zero, fill, checker, noise};
pub use channels::{channels, channel_append, channel_sum};
pub use geometry::{crop, cut, flip, flop, transpose, rotate90, rotate180, rotate270, resize};
pub use arithmetic::{add, sub, mul, div, abs, absdiff, pow, clamp, invert, over};
