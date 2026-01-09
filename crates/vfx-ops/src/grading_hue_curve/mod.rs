//! Hue-based grading curves for selective color adjustments.
//!
//! Implements three curve types commonly found in color grading tools:
//! - **Hue vs Hue**: Shift hue based on input hue (e.g., make reds more orange)
//! - **Hue vs Sat**: Adjust saturation based on input hue (e.g., desaturate greens)
//! - **Hue vs Lum**: Adjust luminance based on input hue (e.g., darken blues)
//!
//! # Example
//!
//! ```
//! use vfx_ops::grading_hue_curve::{GradingHueCurves, HueCurve, HueControlPoint, apply_hue_curves_fwd};
//!
//! // Create curves with control points
//! let mut curves = GradingHueCurves::default();
//!
//! // Shift reds (hue ~0.0) toward orange (+0.05)
//! curves.hue_vs_hue.points.push(HueControlPoint { hue: 0.0, value: 0.05 });
//! curves.hue_vs_hue.points.push(HueControlPoint { hue: 0.1, value: 0.0 });
//!
//! // Desaturate greens (hue ~0.33)
//! curves.hue_vs_sat.points.push(HueControlPoint { hue: 0.3, value: 1.0 });
//! curves.hue_vs_sat.points.push(HueControlPoint { hue: 0.33, value: 0.5 });
//! curves.hue_vs_sat.points.push(HueControlPoint { hue: 0.36, value: 1.0 });
//!
//! // Apply to RGB pixel
//! let mut rgb = [0.8_f32, 0.2, 0.1]; // reddish
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

pub use types::{HueControlPoint, HueCurve, GradingHueCurves};
pub use apply::{
    apply_hue_curves_fwd,
    apply_hue_curves_rev,
    apply_hue_curves_rgb,
    apply_hue_curves_rgba,
    rgb_to_hsl,
    hsl_to_rgb,
};
