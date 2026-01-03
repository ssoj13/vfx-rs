//! ARRI LogC transfer function.
//!
//! LogC is ARRI's logarithmic encoding for their digital cinema cameras.
//! This implementation uses LogC3 (current standard).
//!
//! # Range
//!
//! - Encoded: [0, 1] (signal range)
//! - Linear: Scene-referred (can be negative for out-of-gamut)
//!
//! # Exposure Index
//!
//! LogC parameters vary with camera EI (Exposure Index) setting.
//! This implementation uses EI 800 (most common).
//!
//! # Reference
//!
//! ARRI LogC3 Specification

// LogC3 constants for EI 800
const CUT: f32 = 0.010591;
const A: f32 = 5.555556;
const B: f32 = 0.052272;
const C: f32 = 0.247190;
const D: f32 = 0.385537;
const E: f32 = 5.367655;
const F: f32 = 0.092809;

/// LogC encode: Linear to LogC.
///
/// Converts linear scene light to LogC encoded values.
///
/// # Example
///
/// ```rust
/// use vfx_transfer::log_c::encode;
///
/// // 18% gray in LogC is approximately 0.391
/// let log = encode(0.18);
/// assert!((log - 0.391).abs() < 0.01);
/// ```
#[inline]
pub fn encode(linear: f32) -> f32 {
    if linear > CUT {
        C * (A * linear + B).log10() + D
    } else {
        E * linear + F
    }
}

/// LogC decode: LogC to linear.
///
/// Converts LogC encoded values to linear scene light.
///
/// # Example
///
/// ```rust
/// use vfx_transfer::log_c::decode;
///
/// let linear = decode(0.391);
/// assert!((linear - 0.18).abs() < 0.01);
/// ```
#[inline]
pub fn decode(log: f32) -> f32 {
    if log > E * CUT + F {
        (10.0_f32.powf((log - D) / C) - B) / A
    } else {
        (log - F) / E
    }
}

/// Applies LogC encoding to RGB.
#[inline]
pub fn encode_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [encode(rgb[0]), encode(rgb[1]), encode(rgb[2])]
}

/// Applies LogC decoding to RGB.
#[inline]
pub fn decode_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [decode(rgb[0]), decode(rgb[1]), decode(rgb[2])]
}

/// Returns the LogC value for 18% gray (middle gray).
#[inline]
pub fn middle_gray() -> f32 {
    encode(0.18)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let test_values = [0.0, 0.01, 0.18, 0.5, 1.0, 2.0, 10.0];
        for &l in &test_values {
            let encoded = encode(l);
            let decoded = decode(encoded);
            assert!(
                (l - decoded).abs() < l * 0.001 + 0.001,
                "l={}, decoded={}",
                l,
                decoded
            );
        }
    }

    #[test]
    fn test_middle_gray() {
        // 18% gray should be around 0.391 in LogC
        let log = encode(0.18);
        assert!((log - 0.391).abs() < 0.01);
    }

    #[test]
    fn test_black() {
        assert!(encode(0.0) < 0.1);
    }

    #[test]
    fn test_clipping() {
        // Above 1.0 linear should still encode smoothly
        let log = encode(2.0);
        assert!(log > encode(1.0));
    }
}
