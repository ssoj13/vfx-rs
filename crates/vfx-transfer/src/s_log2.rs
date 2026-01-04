//! Sony S-Log2 transfer function.
//!
//! S-Log2 is Sony's earlier logarithmic encoding, used in cameras like
//! F65, F55, F5, FS700, and FS7 (before S-Log3 became standard).
//!
//! # Range
//!
//! - Encoded: [0, 1] (signal range)
//! - Linear: Scene-referred
//!
//! # Reference
//!
//! - Sony S-Log2 Technical Summary
//! - OpenColorIO/src/OpenColorIO/ops/log/LogOpData.cpp
//! - ACES config: Sony S-Gamut S-Log2 colorspace

// S-Log2 constants from Sony documentation
// Ref: OpenColorIO/src/OpenColorIO/transforms/builtins/Cameras.cpp

/// Log side slope: 0.432699
const LOG_SLOPE: f32 = 0.432699;

/// Log side offset added before division
const LOG_OFFSET: f32 = 0.616596;

/// Linear side offset (added to linear before log)
const LIN_OFFSET: f32 = 0.037584;

/// Output scale factor
const SCALE: f32 = 1.03;

/// Output offset
const OUT_OFFSET: f32 = 0.03;

/// Linear slope for negative values (derived from continuity)
/// slope = LOG_SLOPE * log10(e) / LIN_OFFSET
const LIN_SLOPE: f32 = 5.0; // Approximate: 0.432699 * 0.4342944819 / 0.037584

/// Break point in code values
const CODE_BREAK: f32 = 0.030001222; // Value at linear = 0

/// S-Log2 encode: Linear to S-Log2.
///
/// # Formula
///
/// For linear >= 0:
///   y = (0.432699 * log10(linear + 0.037584) + 0.616596 + 0.03) / 1.03
///
/// For linear < 0:
///   y = (linear * slope + 0.030001222) / 1.03
///
/// # Example
///
/// ```rust
/// use vfx_transfer::s_log2::encode;
///
/// // 18% gray in S-Log2 is approximately 0.35
/// let log = encode(0.18);
/// assert!((log - 0.35).abs() < 0.02);
/// ```
#[inline]
pub fn encode(linear: f32) -> f32 {
    if linear >= 0.0 {
        // Log formula
        (LOG_SLOPE * (linear + LIN_OFFSET).log10() + LOG_OFFSET + OUT_OFFSET) / SCALE
    } else {
        // Linear extension for negative values
        (linear * LIN_SLOPE + CODE_BREAK) / SCALE
    }
}

/// S-Log2 decode: S-Log2 to linear.
///
/// # Example
///
/// ```rust
/// use vfx_transfer::s_log2::decode;
///
/// let linear = decode(0.35);
/// assert!((linear - 0.18).abs() < 0.02);
/// ```
#[inline]
pub fn decode(log: f32) -> f32 {
    let code_break_norm = CODE_BREAK / SCALE;
    
    if log >= code_break_norm {
        // Inverse log formula
        10.0_f32.powf((log * SCALE - OUT_OFFSET - LOG_OFFSET) / LOG_SLOPE) - LIN_OFFSET
    } else {
        // Inverse linear extension
        (log * SCALE - CODE_BREAK) / LIN_SLOPE
    }
}

/// Applies S-Log2 encoding to RGB.
#[inline]
pub fn encode_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [encode(rgb[0]), encode(rgb[1]), encode(rgb[2])]
}

/// Applies S-Log2 decoding to RGB.
#[inline]
pub fn decode_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [decode(rgb[0]), decode(rgb[1]), decode(rgb[2])]
}

/// Returns the S-Log2 value for 18% gray.
#[inline]
pub fn middle_gray() -> f32 {
    encode(0.18)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let test_values = [0.0, 0.001, 0.01, 0.18, 0.5, 1.0, 2.0, 5.0];
        for &l in &test_values {
            let encoded = encode(l);
            let decoded = decode(encoded);
            let tol = l.abs() * 0.001 + 0.0001;
            assert!(
                (l - decoded).abs() < tol,
                "l={}, encoded={}, decoded={}, diff={}",
                l, encoded, decoded, (l - decoded).abs()
            );
        }
    }

    #[test]
    fn test_roundtrip_negative() {
        let test_values = [-0.01, -0.001];
        for &l in &test_values {
            let encoded = encode(l);
            let decoded = decode(encoded);
            let tol = l.abs() * 0.01 + 0.0001;
            assert!(
                (l - decoded).abs() < tol,
                "l={}, encoded={}, decoded={}, diff={}",
                l, encoded, decoded, (l - decoded).abs()
            );
        }
    }

    #[test]
    fn test_middle_gray() {
        let log = encode(0.18);
        // S-Log2 middle gray is around 0.35
        // (differs from S-Log3 which is ~0.41)
        assert!(log > 0.34 && log < 0.38, "log={}", log);
    }

    #[test]
    fn test_zero() {
        let encoded = encode(0.0);
        // At linear=0, we get the break point
        let expected = CODE_BREAK / SCALE;
        assert!(
            (encoded - expected).abs() < 0.0001,
            "encoded={}, expected={}",
            encoded, expected
        );
    }

    #[test]
    fn test_monotonic() {
        let mut prev = encode(-0.01);
        for i in 0..100 {
            let linear = i as f32 / 50.0; // 0 to 2
            let encoded = encode(linear);
            assert!(
                encoded > prev,
                "Not monotonic at {}: {} <= {}",
                linear, encoded, prev
            );
            prev = encoded;
        }
    }
}
