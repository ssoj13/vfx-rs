//! Panasonic V-Log transfer function.
//!
//! V-Log is Panasonic's logarithmic encoding for their VariCam
//! and Lumix cameras.
//!
//! # Range
//!
//! - Encoded: [0, 1] (signal range)
//! - Linear: Scene-referred
//!
//! # Reference
//!
//! Panasonic V-Log/V-Gamut Technical Documentation

// V-Log constants
const CUT1: f32 = 0.01;
const B: f32 = 0.00873;
const C: f32 = 0.241514;
const D: f32 = 0.598206;

/// V-Log encode: Linear to V-Log.
///
/// # Example
///
/// ```rust
/// use vfx_transfer::v_log::encode;
///
/// // 18% gray in V-Log is approximately 0.423
/// let log = encode(0.18);
/// assert!((log - 0.423).abs() < 0.01);
/// ```
#[inline]
pub fn encode(linear: f32) -> f32 {
    if linear < CUT1 {
        5.6 * linear + 0.125
    } else {
        C * (linear + B).log10() + D
    }
}

/// V-Log decode: V-Log to linear.
///
/// # Example
///
/// ```rust
/// use vfx_transfer::v_log::decode;
///
/// let linear = decode(0.423);
/// assert!((linear - 0.18).abs() < 0.01);
/// ```
#[inline]
pub fn decode(log: f32) -> f32 {
    if log < 0.181 {
        (log - 0.125) / 5.6
    } else {
        10.0_f32.powf((log - D) / C) - B
    }
}

/// Applies V-Log encoding to RGB.
#[inline]
pub fn encode_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [encode(rgb[0]), encode(rgb[1]), encode(rgb[2])]
}

/// Applies V-Log decoding to RGB.
#[inline]
pub fn decode_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [decode(rgb[0]), decode(rgb[1]), decode(rgb[2])]
}

/// Returns the V-Log value for 18% gray.
#[inline]
pub fn middle_gray() -> f32 {
    encode(0.18)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let test_values = [0.005, 0.18, 0.5, 1.0, 2.0];
        for &l in &test_values {
            let encoded = encode(l);
            let decoded = decode(encoded);
            assert!(
                (l - decoded).abs() < l * 0.01 + 0.001,
                "l={}, decoded={}",
                l,
                decoded
            );
        }
    }

    #[test]
    fn test_middle_gray() {
        let log = encode(0.18);
        assert!((log - 0.423).abs() < 0.01);
    }

    #[test]
    fn test_black() {
        let log = encode(0.0);
        assert!(log > 0.0); // V-Log has lifted black
    }
}
