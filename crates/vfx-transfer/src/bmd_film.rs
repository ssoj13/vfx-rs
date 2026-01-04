//! Blackmagic Design Film transfer functions.
//!
//! Implements BMDFilm Gen5 encoding used by Blackmagic cameras (URSA, Pocket Cinema, etc).
//!
//! # BMDFilm Gen5
//!
//! LogCameraTransform with natural log base (e = 2.71828...).
//! Features a linear extension below break point for handling shadows.
//!
//! # Reference
//!
//! Source: OpenColorIO studio-config-v1.0.0_aces-v1.3_ocio-v2.1.ocio
//! Blackmagic Design Film Gen 5 - LogCameraTransform
//! <https://github.com/AcademySoftwareFoundation/OpenColorIO-Config-ACES>

use std::f64::consts::E;

// === BMDFilm Gen5 Constants ===
// Source: OCIO ACES config - Blackmagic Design Film Gen 5
// LogCameraTransform with base=e (2.71828...)
const BMD_GEN5_BASE: f64 = E;
const BMD_GEN5_LOG_SIDE_SLOPE: f64 = 0.0869287606549122;
const BMD_GEN5_LOG_SIDE_OFFSET: f64 = 0.530013339229194;
const BMD_GEN5_LIN_SIDE_SLOPE: f64 = 1.0; // default
const BMD_GEN5_LIN_SIDE_OFFSET: f64 = 0.00549407243225781;
const BMD_GEN5_LIN_SIDE_BREAK: f64 = 0.005;

/// BMDFilm Gen5 encode: Linear to BMDFilm Gen5.
///
/// # Example
///
/// ```rust
/// use vfx_transfer::bmd_film::bmd_film_gen5_encode;
///
/// let log = bmd_film_gen5_encode(0.18);
/// assert!(log > 0.3 && log < 0.6);
/// ```
#[inline]
pub fn bmd_film_gen5_encode(linear: f32) -> f32 {
    let x = linear as f64;

    if x >= BMD_GEN5_LIN_SIDE_BREAK {
        // Log region: y = logSideSlope * log_base(linSideSlope * x + linSideOffset) + logSideOffset
        let lin_val = BMD_GEN5_LIN_SIDE_SLOPE * x + BMD_GEN5_LIN_SIDE_OFFSET;
        (BMD_GEN5_LOG_SIDE_SLOPE * lin_val.ln() / BMD_GEN5_BASE.ln() + BMD_GEN5_LOG_SIDE_OFFSET)
            as f32
    } else {
        // Linear extension below break point
        let break_lin = BMD_GEN5_LIN_SIDE_SLOPE * BMD_GEN5_LIN_SIDE_BREAK + BMD_GEN5_LIN_SIDE_OFFSET;
        let log_at_break =
            BMD_GEN5_LOG_SIDE_SLOPE * break_lin.ln() / BMD_GEN5_BASE.ln() + BMD_GEN5_LOG_SIDE_OFFSET;
        // Slope at break: d/dx[logSideSlope * ln(linSideSlope * x + linSideOffset) / ln(base)]
        // = logSideSlope * linSideSlope / (ln(base) * (linSideSlope * x + linSideOffset))
        let slope_at_break = BMD_GEN5_LOG_SIDE_SLOPE * BMD_GEN5_LIN_SIDE_SLOPE
            / (BMD_GEN5_BASE.ln() * break_lin);

        (log_at_break + slope_at_break * (x - BMD_GEN5_LIN_SIDE_BREAK)) as f32
    }
}

/// BMDFilm Gen5 decode: BMDFilm Gen5 to linear.
///
/// # Example
///
/// ```rust
/// use vfx_transfer::bmd_film::bmd_film_gen5_decode;
///
/// let linear = bmd_film_gen5_decode(0.45);
/// assert!(linear > 0.0 && linear < 1.0);
/// ```
#[inline]
pub fn bmd_film_gen5_decode(log: f32) -> f32 {
    let y = log as f64;

    // Calculate log value at break point
    let break_lin = BMD_GEN5_LIN_SIDE_SLOPE * BMD_GEN5_LIN_SIDE_BREAK + BMD_GEN5_LIN_SIDE_OFFSET;
    let log_at_break =
        BMD_GEN5_LOG_SIDE_SLOPE * break_lin.ln() / BMD_GEN5_BASE.ln() + BMD_GEN5_LOG_SIDE_OFFSET;

    if y >= log_at_break {
        // Log region: x = (base^((y - logSideOffset) / logSideSlope) - linSideOffset) / linSideSlope
        let log_val = (y - BMD_GEN5_LOG_SIDE_OFFSET) / BMD_GEN5_LOG_SIDE_SLOPE;
        let lin_val = BMD_GEN5_BASE.powf(log_val);
        ((lin_val - BMD_GEN5_LIN_SIDE_OFFSET) / BMD_GEN5_LIN_SIDE_SLOPE) as f32
    } else {
        // Linear extension
        let slope_at_break = BMD_GEN5_LOG_SIDE_SLOPE * BMD_GEN5_LIN_SIDE_SLOPE
            / (BMD_GEN5_BASE.ln() * break_lin);
        (BMD_GEN5_LIN_SIDE_BREAK + (y - log_at_break) / slope_at_break) as f32
    }
}

/// Applies BMDFilm Gen5 encoding to RGB.
#[inline]
pub fn bmd_film_gen5_encode_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [
        bmd_film_gen5_encode(rgb[0]),
        bmd_film_gen5_encode(rgb[1]),
        bmd_film_gen5_encode(rgb[2]),
    ]
}

/// Applies BMDFilm Gen5 decoding to RGB.
#[inline]
pub fn bmd_film_gen5_decode_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [
        bmd_film_gen5_decode(rgb[0]),
        bmd_film_gen5_decode(rgb[1]),
        bmd_film_gen5_decode(rgb[2]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bmd_gen5_roundtrip() {
        let test_values = [0.001, 0.01, 0.18, 0.5, 1.0, 2.0];
        for &l in &test_values {
            let encoded = bmd_film_gen5_encode(l);
            let decoded = bmd_film_gen5_decode(encoded);
            assert!(
                (l - decoded).abs() < l * 0.01 + 0.0001,
                "l={}, encoded={}, decoded={}",
                l,
                encoded,
                decoded
            );
        }
    }

    #[test]
    fn test_bmd_gen5_below_break() {
        // Test values below break point (0.005)
        let small = bmd_film_gen5_encode(0.002);
        let decoded = bmd_film_gen5_decode(small);
        assert!(
            (decoded - 0.002).abs() < 0.0001,
            "Below-break roundtrip failed: {}",
            decoded
        );
    }

    #[test]
    fn test_bmd_gen5_18_percent_gray() {
        // 18% gray should give roughly mid-range log value
        let log = bmd_film_gen5_encode(0.18);
        assert!(
            log > 0.35 && log < 0.55,
            "18% gray should be mid-range, got {}",
            log
        );
    }
}
