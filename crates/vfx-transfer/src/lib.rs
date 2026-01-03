//! # vfx-transfer
//!
//! Transfer functions (OETF/EOTF) for color encoding and decoding.
//!
//! Transfer functions convert between linear light values and encoded values
//! for storage, display, or transmission.
//!
//! # Terminology
//!
//! - **OETF** (Opto-Electronic Transfer Function): Linear -> Encoded (e.g., for recording)
//! - **EOTF** (Electro-Optical Transfer Function): Encoded -> Linear (e.g., for display)
//! - **Gamma**: The exponent in a power-law transfer function
//!
//! # Supported Transfer Functions
//!
//! | Function | Use Case | Range |
//! |----------|----------|-------|
//! | [`srgb`] | Web, consumer displays | [0, 1] |
//! | [`gamma22`], [`gamma24`] | Legacy CRT simulation | [0, 1] |
//! | [`rec709`] | HDTV broadcast | [0, 1] |
//! | [`pq`] | HDR (HDR10, Dolby Vision) | [0, 10000] cd/m2 |
//! | [`hlg`] | HDR broadcast (HLG) | [0, 1] |
//! | [`log_c`] | ARRI cameras | Scene-referred |
//! | [`s_log3`] | Sony cameras | Scene-referred |
//! | [`v_log`] | Panasonic cameras | Scene-referred |
//!
//! # Usage
//!
//! ```rust
//! use vfx_transfer::{srgb, pq};
//!
//! // Decode sRGB to linear
//! let linear = srgb::eotf(0.5);
//!
//! // Encode linear to sRGB
//! let encoded = srgb::oetf(linear);
//!
//! // HDR: decode PQ (returns absolute luminance in cd/m2)
//! let nits = pq::eotf(0.5);
//! ```
//!
//! # Scene vs Display Referred
//!
//! - **Display-referred** (sRGB, Rec.709): Values represent display output
//! - **Scene-referred** (ACES, Log): Values represent scene light ratios
//!
//! Camera log curves (LogC, S-Log, V-Log) are scene-referred and can
//! represent very wide dynamic ranges (14+ stops).
//!
//! # Dependencies
//!
//! - [`vfx-core`] - Core types
//!
//! # Used By
//!
//! - `vfx-color` - Full color space conversions
//! - `vfx-io` - Image file encoding/decoding

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod srgb;
pub mod gamma;
pub mod rec709;
pub mod pq;
pub mod hlg;
pub mod log_c;
pub mod s_log3;
pub mod v_log;

// Re-export common functions
pub use srgb::{eotf as srgb_eotf, oetf as srgb_oetf};
pub use gamma::{gamma_eotf, gamma_oetf};
pub use rec709::{eotf as rec709_eotf, oetf as rec709_oetf};
pub use pq::{eotf as pq_eotf, oetf as pq_oetf};
pub use hlg::{eotf as hlg_eotf, oetf as hlg_oetf};
pub use log_c::{decode as log_c_decode, encode as log_c_encode};
pub use s_log3::{decode as s_log3_decode, encode as s_log3_encode};
pub use v_log::{decode as v_log_decode, encode as v_log_encode};
