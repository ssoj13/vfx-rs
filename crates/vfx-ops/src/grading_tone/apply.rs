//! Apply functions for GradingTone operation.
//!
//! Reference: OCIO ops/gradingtone/GradingToneOpCPU.cpp

use super::curves::*;
use super::linlog::{lin_to_log, log_to_lin};
use super::prerender::GradingTonePreRender;
use super::types::{GradingTone, RGBMChannel};
use crate::GradingStyle;

/// Maximum half-float value to prevent overflow.
const MAX_HALF_FLOAT: f32 = 65504.0;

/// Clamp RGB values to max half-float.
#[inline]
fn clamp_max_rgb(rgb: &mut [f32; 3]) {
    for c in rgb.iter_mut() {
        *c = c.min(MAX_HALF_FLOAT);
    }
}

// ============================================================================
// Forward Apply
// ============================================================================

/// Apply GradingTone forward (LOG/VIDEO style).
///
/// Order: mids -> highlights -> whites -> shadows -> blacks -> scontrast
#[inline]
pub fn apply_fwd(pr: &GradingTonePreRender, tone: &GradingTone, rgb: &mut [f32; 3]) {
    if pr.local_bypass {
        return;
    }

    // Midtones: R, G, B, then Master
    for channel in RGBMChannel::RGB {
        rgb[channel as usize] = mids_fwd_single(pr, tone, channel, rgb[channel as usize]);
    }
    mids_fwd_rgb(pr, tone, rgb);

    // Highlights: R, G, B, then Master
    for channel in RGBMChannel::RGB {
        rgb[channel as usize] = hs_fwd_single(pr, tone, channel, false, rgb[channel as usize]);
    }
    hs_fwd_rgb(pr, tone, false, rgb);

    // Whites: R, G, B, then Master
    for channel in RGBMChannel::RGB {
        rgb[channel as usize] = wb_fwd_single(pr, tone, channel, false, rgb[channel as usize]);
    }
    wb_fwd_rgb(pr, tone, false, rgb);

    // Shadows: R, G, B, then Master
    for channel in RGBMChannel::RGB {
        rgb[channel as usize] = hs_fwd_single(pr, tone, channel, true, rgb[channel as usize]);
    }
    hs_fwd_rgb(pr, tone, true, rgb);

    // Blacks: R, G, B, then Master
    for channel in RGBMChannel::RGB {
        rgb[channel as usize] = wb_fwd_single(pr, tone, channel, true, rgb[channel as usize]);
    }
    wb_fwd_rgb(pr, tone, true, rgb);

    // S-Contrast
    scontrast_fwd(pr, tone, rgb);

    clamp_max_rgb(rgb);
}

/// Apply GradingTone forward (LINEAR style).
///
/// Converts to log space first, applies grading, then converts back.
#[inline]
pub fn apply_linear_fwd(pr: &GradingTonePreRender, tone: &GradingTone, rgb: &mut [f32; 3]) {
    if pr.local_bypass {
        return;
    }

    // Convert linear to log-like space
    lin_to_log(rgb);

    // Apply grading in log space
    apply_fwd_core(pr, tone, rgb);

    // Convert back to linear
    log_to_lin(rgb);

    clamp_max_rgb(rgb);
}

/// Core forward apply (used by both LOG/VIDEO and LINEAR).
#[inline]
fn apply_fwd_core(pr: &GradingTonePreRender, tone: &GradingTone, rgb: &mut [f32; 3]) {
    // Midtones: R, G, B, then Master
    for channel in RGBMChannel::RGB {
        rgb[channel as usize] = mids_fwd_single(pr, tone, channel, rgb[channel as usize]);
    }
    mids_fwd_rgb(pr, tone, rgb);

    // Highlights: R, G, B, then Master
    for channel in RGBMChannel::RGB {
        rgb[channel as usize] = hs_fwd_single(pr, tone, channel, false, rgb[channel as usize]);
    }
    hs_fwd_rgb(pr, tone, false, rgb);

    // Whites: R, G, B, then Master
    for channel in RGBMChannel::RGB {
        rgb[channel as usize] = wb_fwd_single(pr, tone, channel, false, rgb[channel as usize]);
    }
    wb_fwd_rgb(pr, tone, false, rgb);

    // Shadows: R, G, B, then Master
    for channel in RGBMChannel::RGB {
        rgb[channel as usize] = hs_fwd_single(pr, tone, channel, true, rgb[channel as usize]);
    }
    hs_fwd_rgb(pr, tone, true, rgb);

    // Blacks: R, G, B, then Master
    for channel in RGBMChannel::RGB {
        rgb[channel as usize] = wb_fwd_single(pr, tone, channel, true, rgb[channel as usize]);
    }
    wb_fwd_rgb(pr, tone, true, rgb);

    // S-Contrast
    scontrast_fwd(pr, tone, rgb);
}

// ============================================================================
// Reverse Apply
// ============================================================================

/// Apply GradingTone reverse (LOG/VIDEO style).
///
/// Order: scontrast -> blacks -> shadows -> whites -> highlights -> mids
/// (reverse order of forward, and each operation uses its reverse function)
#[inline]
pub fn apply_rev(pr: &GradingTonePreRender, tone: &GradingTone, rgb: &mut [f32; 3]) {
    if pr.local_bypass {
        return;
    }

    // S-Contrast (reverse)
    scontrast_rev(pr, tone, rgb);

    // Blacks: Master first, then B, G, R
    wb_rev_rgb(pr, tone, true, rgb);
    for channel in RGBMChannel::RGB.iter().rev() {
        rgb[*channel as usize] = wb_rev_single(pr, tone, *channel, true, rgb[*channel as usize]);
    }

    // Shadows: Master first, then B, G, R
    hs_rev_rgb(pr, tone, true, rgb);
    for channel in RGBMChannel::RGB.iter().rev() {
        rgb[*channel as usize] = hs_rev_single(pr, tone, *channel, true, rgb[*channel as usize]);
    }

    // Whites: Master first, then B, G, R
    wb_rev_rgb(pr, tone, false, rgb);
    for channel in RGBMChannel::RGB.iter().rev() {
        rgb[*channel as usize] = wb_rev_single(pr, tone, *channel, false, rgb[*channel as usize]);
    }

    // Highlights: Master first, then B, G, R
    hs_rev_rgb(pr, tone, false, rgb);
    for channel in RGBMChannel::RGB.iter().rev() {
        rgb[*channel as usize] = hs_rev_single(pr, tone, *channel, false, rgb[*channel as usize]);
    }

    // Midtones: Master first, then B, G, R
    mids_rev_rgb(pr, tone, rgb);
    for channel in RGBMChannel::RGB.iter().rev() {
        rgb[*channel as usize] = mids_rev_single(pr, tone, *channel, rgb[*channel as usize]);
    }

    clamp_max_rgb(rgb);
}

/// Apply GradingTone reverse (LINEAR style).
#[inline]
pub fn apply_linear_rev(pr: &GradingTonePreRender, tone: &GradingTone, rgb: &mut [f32; 3]) {
    if pr.local_bypass {
        return;
    }

    // Convert linear to log-like space
    lin_to_log(rgb);

    // Apply reverse grading in log space
    apply_rev_core(pr, tone, rgb);

    // Convert back to linear
    log_to_lin(rgb);

    clamp_max_rgb(rgb);
}

/// Core reverse apply.
#[inline]
fn apply_rev_core(pr: &GradingTonePreRender, tone: &GradingTone, rgb: &mut [f32; 3]) {
    // S-Contrast (reverse)
    scontrast_rev(pr, tone, rgb);

    // Blacks: Master first, then B, G, R
    wb_rev_rgb(pr, tone, true, rgb);
    for channel in RGBMChannel::RGB.iter().rev() {
        rgb[*channel as usize] = wb_rev_single(pr, tone, *channel, true, rgb[*channel as usize]);
    }

    // Shadows: Master first, then B, G, R
    hs_rev_rgb(pr, tone, true, rgb);
    for channel in RGBMChannel::RGB.iter().rev() {
        rgb[*channel as usize] = hs_rev_single(pr, tone, *channel, true, rgb[*channel as usize]);
    }

    // Whites: Master first, then B, G, R
    wb_rev_rgb(pr, tone, false, rgb);
    for channel in RGBMChannel::RGB.iter().rev() {
        rgb[*channel as usize] = wb_rev_single(pr, tone, *channel, false, rgb[*channel as usize]);
    }

    // Highlights: Master first, then B, G, R
    hs_rev_rgb(pr, tone, false, rgb);
    for channel in RGBMChannel::RGB.iter().rev() {
        rgb[*channel as usize] = hs_rev_single(pr, tone, *channel, false, rgb[*channel as usize]);
    }

    // Midtones: Master first, then B, G, R
    mids_rev_rgb(pr, tone, rgb);
    for channel in RGBMChannel::RGB.iter().rev() {
        rgb[*channel as usize] = mids_rev_single(pr, tone, *channel, rgb[*channel as usize]);
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Apply GradingTone to RGB pixel.
///
/// Uses precomputed values for efficient rendering.
#[inline]
pub fn apply_grading_tone(
    style: GradingStyle,
    pr: &GradingTonePreRender,
    tone: &GradingTone,
    forward: bool,
    rgb: &mut [f32; 3],
) {
    match (style, forward) {
        (GradingStyle::Linear, true) => apply_linear_fwd(pr, tone, rgb),
        (GradingStyle::Linear, false) => apply_linear_rev(pr, tone, rgb),
        (_, true) => apply_fwd(pr, tone, rgb),
        (_, false) => apply_rev(pr, tone, rgb),
    }
}

/// Apply GradingTone to RGBA buffer.
#[inline]
pub fn apply_grading_tone_rgba(
    style: GradingStyle,
    pr: &GradingTonePreRender,
    tone: &GradingTone,
    forward: bool,
    pixels: &mut [f32],
) {
    debug_assert!(pixels.len() % 4 == 0);

    if pr.local_bypass {
        return;
    }

    for chunk in pixels.chunks_exact_mut(4) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        apply_grading_tone(style, pr, tone, forward, &mut rgb);
        chunk[0] = rgb[0];
        chunk[1] = rgb[1];
        chunk[2] = rgb[2];
        // Alpha unchanged
    }
}

/// Convenience function: apply GradingTone forward without pre-existing prerender.
#[inline]
pub fn apply_grading_tone_simple(
    style: GradingStyle,
    tone: &GradingTone,
    rgb: &mut [f32; 3],
) {
    let pr = GradingTonePreRender::new(style, tone);
    apply_grading_tone(style, &pr, tone, true, rgb);
}

/// Convenience function: apply GradingTone reverse without pre-existing prerender.
#[inline]
pub fn apply_grading_tone_simple_rev(
    style: GradingStyle,
    tone: &GradingTone,
    rgb: &mut [f32; 3],
) {
    let pr = GradingTonePreRender::new(style, tone);
    apply_grading_tone(style, &pr, tone, false, rgb);
}
