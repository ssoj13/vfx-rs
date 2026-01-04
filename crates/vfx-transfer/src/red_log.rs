//! RED camera log transfer functions.
//!
//! Includes REDLogFilm and REDLog3G10 encodings used by RED cameras.
//!
//! # REDLogFilm
//!
//! Older RED log encoding with Cineon-style curve.
//!
//! # REDLog3G10
//!
//! Modern RED log encoding with extended dynamic range.
//! Handles negative values via linear extension below break point.
//!
//! # Reference
//!
//! RED Digital Cinema - Technical White Papers
//! OCIO BuiltinTransforms - RedCameras.cpp

// === REDLogFilm Constants ===
// From OCIO: RED_REDLOGFILM_RWG_to_LINEAR

const REDLOGFILM_REF_WHITE: f64 = 685.0 / 1023.0;
const REDLOGFILM_REF_BLACK: f64 = 95.0 / 1023.0;
const REDLOGFILM_GAMMA: f64 = 0.6;
const REDLOGFILM_RANGE: f64 = 0.002 * 1023.0;
const REDLOGFILM_HIGHLIGHT: f64 = 1.0;
const REDLOGFILM_SHADOW: f64 = 0.0;

/// REDLogFilm encode: Linear to REDLogFilm.
///
/// # Example
///
/// ```rust
/// use vfx_transfer::red_log::redlogfilm_encode;
///
/// let log = redlogfilm_encode(0.18);
/// assert!((log - 0.5).abs() < 0.1);
/// ```
#[inline]
pub fn redlogfilm_encode(linear: f32) -> f32 {
    // Precompute constants
    let multi_factor = REDLOGFILM_RANGE / REDLOGFILM_GAMMA;
    let gain = (REDLOGFILM_HIGHLIGHT - REDLOGFILM_SHADOW)
        / (1.0 - 10.0_f64.powf(multi_factor * (REDLOGFILM_REF_BLACK - REDLOGFILM_REF_WHITE)));
    let offset = gain - (REDLOGFILM_HIGHLIGHT - REDLOGFILM_SHADOW);

    // LogAffine parameters
    let log_side_slope = 1.0 / multi_factor;
    let log_side_offset = REDLOGFILM_REF_WHITE;
    let lin_side_slope = 1.0 / gain;
    let lin_side_offset = (offset - REDLOGFILM_SHADOW) / gain;

    // Apply: y = logSideSlope * log10(linSideSlope * x + linSideOffset) + logSideOffset
    let lin_val = lin_side_slope * (linear as f64) + lin_side_offset;
    if lin_val <= 0.0 {
        return 0.0;
    }
    (log_side_slope * lin_val.log10() + log_side_offset) as f32
}

/// REDLogFilm decode: REDLogFilm to linear.
///
/// # Example
///
/// ```rust
/// use vfx_transfer::red_log::redlogfilm_decode;
///
/// let linear = redlogfilm_decode(0.5);
/// assert!(linear > 0.0 && linear < 1.0);
/// ```
#[inline]
pub fn redlogfilm_decode(log: f32) -> f32 {
    // Precompute constants
    let multi_factor = REDLOGFILM_RANGE / REDLOGFILM_GAMMA;
    let gain = (REDLOGFILM_HIGHLIGHT - REDLOGFILM_SHADOW)
        / (1.0 - 10.0_f64.powf(multi_factor * (REDLOGFILM_REF_BLACK - REDLOGFILM_REF_WHITE)));
    let offset = gain - (REDLOGFILM_HIGHLIGHT - REDLOGFILM_SHADOW);

    // LogAffine parameters
    let log_side_slope = 1.0 / multi_factor;
    let log_side_offset = REDLOGFILM_REF_WHITE;
    let lin_side_slope = 1.0 / gain;
    let lin_side_offset = (offset - REDLOGFILM_SHADOW) / gain;

    // Inverse: x = (10^((y - logSideOffset) / logSideSlope) - linSideOffset) / linSideSlope
    let log_val = (log as f64 - log_side_offset) / log_side_slope;
    let lin_val = 10.0_f64.powf(log_val);
    ((lin_val - lin_side_offset) / lin_side_slope) as f32
}

// === REDLog3G10 Constants ===
// From OCIO: RED_LOG3G10_RWG_to_LINEAR

const LOG3G10_LIN_SIDE_SLOPE: f64 = 155.975327;
const LOG3G10_LIN_SIDE_OFFSET: f64 = 0.01 * LOG3G10_LIN_SIDE_SLOPE + 1.0;
const LOG3G10_LOG_SIDE_SLOPE: f64 = 0.224282;
const LOG3G10_LOG_SIDE_OFFSET: f64 = 0.0;
const LOG3G10_LIN_SIDE_BREAK: f64 = -0.01;
const LOG3G10_BASE: f64 = 10.0;

/// REDLog3G10 encode: Linear to REDLog3G10.
///
/// Handles negative values via linear extension.
///
/// # Example
///
/// ```rust
/// use vfx_transfer::red_log::log3g10_encode;
///
/// let log = log3g10_encode(0.18);
/// assert!(log > 0.3 && log < 0.5);
/// ```
#[inline]
pub fn log3g10_encode(linear: f32) -> f32 {
    let x = linear as f64;

    if x >= LOG3G10_LIN_SIDE_BREAK {
        // Log region
        let lin_val = LOG3G10_LIN_SIDE_SLOPE * x + LOG3G10_LIN_SIDE_OFFSET;
        (LOG3G10_LOG_SIDE_SLOPE * lin_val.log10() + LOG3G10_LOG_SIDE_OFFSET) as f32
    } else {
        // Linear extension for negative values
        // Calculate slope at break point
        let break_log = LOG3G10_LIN_SIDE_SLOPE * LOG3G10_LIN_SIDE_BREAK + LOG3G10_LIN_SIDE_OFFSET;
        let log_at_break = LOG3G10_LOG_SIDE_SLOPE * break_log.log10() + LOG3G10_LOG_SIDE_OFFSET;
        let slope_at_break =
            LOG3G10_LOG_SIDE_SLOPE * LOG3G10_LIN_SIDE_SLOPE / (break_log * LOG3G10_BASE.ln());

        (log_at_break + slope_at_break * (x - LOG3G10_LIN_SIDE_BREAK)) as f32
    }
}

/// REDLog3G10 decode: REDLog3G10 to linear.
///
/// # Example
///
/// ```rust
/// use vfx_transfer::red_log::log3g10_decode;
///
/// let linear = log3g10_decode(0.4);
/// assert!(linear > 0.0 && linear < 1.0);
/// ```
#[inline]
pub fn log3g10_decode(log: f32) -> f32 {
    let y = log as f64;

    // Calculate log value at break point
    let break_log = LOG3G10_LIN_SIDE_SLOPE * LOG3G10_LIN_SIDE_BREAK + LOG3G10_LIN_SIDE_OFFSET;
    let log_at_break = LOG3G10_LOG_SIDE_SLOPE * break_log.log10() + LOG3G10_LOG_SIDE_OFFSET;

    if y >= log_at_break {
        // Log region
        let lin_val = LOG3G10_BASE.powf((y - LOG3G10_LOG_SIDE_OFFSET) / LOG3G10_LOG_SIDE_SLOPE);
        ((lin_val - LOG3G10_LIN_SIDE_OFFSET) / LOG3G10_LIN_SIDE_SLOPE) as f32
    } else {
        // Linear extension
        let slope_at_break =
            LOG3G10_LOG_SIDE_SLOPE * LOG3G10_LIN_SIDE_SLOPE / (break_log * LOG3G10_BASE.ln());
        (LOG3G10_LIN_SIDE_BREAK + (y - log_at_break) / slope_at_break) as f32
    }
}

/// Applies REDLogFilm encoding to RGB.
#[inline]
pub fn redlogfilm_encode_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [
        redlogfilm_encode(rgb[0]),
        redlogfilm_encode(rgb[1]),
        redlogfilm_encode(rgb[2]),
    ]
}

/// Applies REDLogFilm decoding to RGB.
#[inline]
pub fn redlogfilm_decode_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [
        redlogfilm_decode(rgb[0]),
        redlogfilm_decode(rgb[1]),
        redlogfilm_decode(rgb[2]),
    ]
}

/// Applies REDLog3G10 encoding to RGB.
#[inline]
pub fn log3g10_encode_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [
        log3g10_encode(rgb[0]),
        log3g10_encode(rgb[1]),
        log3g10_encode(rgb[2]),
    ]
}

/// Applies REDLog3G10 decoding to RGB.
#[inline]
pub fn log3g10_decode_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [
        log3g10_decode(rgb[0]),
        log3g10_decode(rgb[1]),
        log3g10_decode(rgb[2]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redlogfilm_roundtrip() {
        let test_values = [0.01, 0.18, 0.5, 1.0, 2.0];
        for &l in &test_values {
            let encoded = redlogfilm_encode(l);
            let decoded = redlogfilm_decode(encoded);
            assert!(
                (l - decoded).abs() < l * 0.01 + 0.001,
                "l={}, encoded={}, decoded={}",
                l,
                encoded,
                decoded
            );
        }
    }

    #[test]
    fn test_log3g10_roundtrip() {
        let test_values = [-0.005, 0.0, 0.01, 0.18, 0.5, 1.0, 2.0];
        for &l in &test_values {
            let encoded = log3g10_encode(l);
            let decoded = log3g10_decode(encoded);
            assert!(
                (l - decoded).abs() < l.abs() * 0.01 + 0.001,
                "l={}, encoded={}, decoded={}",
                l,
                encoded,
                decoded
            );
        }
    }

    #[test]
    fn test_log3g10_negative_handling() {
        // REDLog3G10 should handle negative values
        let neg = log3g10_encode(-0.005);
        assert!(neg < 0.0, "Negative linear should give negative log");

        let decoded = log3g10_decode(neg);
        assert!(
            (decoded - (-0.005)).abs() < 0.001,
            "Negative roundtrip failed"
        );
    }
}
