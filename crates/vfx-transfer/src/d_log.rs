//! DJI D-Log transfer function.
//!
//! D-Log is DJI's logarithmic encoding for their Zenmuse cameras
//! (X5S, X7, X9). Based on film scan characteristics.
//!
//! # Range
//!
//! - Encoded: [0, 1] (signal range)
//! - Linear: Scene-referred
//!
//! # Reference
//!
//! DJI Cinema Color System whitepaper (D-Log/D-Gamut)

// D-Log constants (from DJI whitepaper)
const LIN_CUT: f32 = 0.0078;
const LOG_CUT: f32 = 0.14;
const A: f32 = 6.025;
const B: f32 = 0.0929;
const C: f32 = 0.9892;
const D: f32 = 0.0108;
const E: f32 = 0.256663;
const F: f32 = 0.584555;

/// D-Log encode: Linear to D-Log.
///
/// # Example
///
/// ```rust
/// use vfx_transfer::d_log::encode;
///
/// // 18% gray in D-Log is approximately 0.399
/// let log = encode(0.18);
/// assert!((log - 0.399).abs() < 0.01);
/// ```
#[inline]
pub fn encode(linear: f32) -> f32 {
    if linear <= LIN_CUT {
        A * linear + B
    } else {
        (linear * C + D).log10() * E + F
    }
}

/// D-Log decode: D-Log to linear.
///
/// # Example
///
/// ```rust
/// use vfx_transfer::d_log::decode;
///
/// let linear = decode(0.399);
/// assert!((linear - 0.18).abs() < 0.01);
/// ```
#[inline]
pub fn decode(log: f32) -> f32 {
    if log <= LOG_CUT {
        (log - B) / A
    } else {
        // Inverse: 10^((log - F) / E) = linear * C + D
        // linear = (10^((log - F) / E) - D) / C
        (10.0_f32.powf((log - F) / E) - D) / C
    }
}

/// Applies D-Log encoding to RGB.
#[inline]
pub fn encode_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [encode(rgb[0]), encode(rgb[1]), encode(rgb[2])]
}

/// Applies D-Log decoding to RGB.
#[inline]
pub fn decode_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [decode(rgb[0]), decode(rgb[1]), decode(rgb[2])]
}

/// Returns the D-Log value for 18% gray.
#[inline]
pub fn middle_gray() -> f32 {
    encode(0.18)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let test_values = [0.001, 0.005, 0.18, 0.5, 1.0, 2.0];
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
        // 18% gray maps to ~408/1023 ≈ 0.399
        assert!((log - 0.399).abs() < 0.01, "middle gray = {}", log);
    }

    #[test]
    fn test_black() {
        let log = encode(0.0);
        // Black is lifted: 95/1023 ≈ 0.093
        assert!((log - 0.093).abs() < 0.01, "black = {}", log);
    }

    #[test]
    fn test_white() {
        let log = encode(0.9);
        // 90% reflectance maps to ~586/1023 ≈ 0.573
        assert!((log - 0.573).abs() < 0.02, "white = {}", log);
    }
}
