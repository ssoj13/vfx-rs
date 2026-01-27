//! Hue-based grading curves for selective color adjustments.
//!
//! Implements 8 curve types matching OCIO GradingHueCurveTransform:
//! HUE_HUE, HUE_SAT, HUE_LUM, LUM_SAT, SAT_SAT, LUM_LUM, SAT_LUM, HUE_FX.
//!
//! Uses HSY color space with Lin/Log/Vid variants.
//!
//! # Example
//!
//! ```
//! use vfx_ops::grading_hue_curve::{GradingHueCurves, HueCurve, HueControlPoint, apply_hue_curves_fwd};
//!
//! let mut curves = GradingHueCurves::default();
//!
//! // Boost saturation for reds via hue_sat curve
//! curves.hue_sat = HueCurve::new(vec![
//!     HueControlPoint::new(0.0, 1.5),
//!     HueControlPoint::new(0.1, 1.0),
//!     HueControlPoint::new(0.5, 1.0),
//! ]);
//!
//! let mut rgb = [0.8_f32, 0.2, 0.1];
//! apply_hue_curves_fwd(&curves, &mut rgb);
//! ```
//!
//! # Hue Values
//!
//! Hue is normalized to [0, 1) range:
//! - 0.0 = Red
//! - 0.167 = Yellow
//! - 0.333 = Green
//! - 0.5 = Cyan
//! - 0.667 = Blue
//! - 0.833 = Magenta
//!
//! The curve wraps around, so 0.0 and 1.0 represent the same hue.

mod types;
mod apply;

pub use types::{
    HueCurveType, GradingStyle, HueControlPoint, HueCurve, GradingHueCurves,
};
pub use apply::{
    apply_hue_curves_fwd,
    apply_hue_curves_rev,
    apply_hue_curves_rgb,
    apply_hue_curves_rgba,
};
