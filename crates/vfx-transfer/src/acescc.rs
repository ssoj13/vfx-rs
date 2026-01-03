//! ACEScc transfer function.
//!
//! ACEScc is a pure logarithmic color space for color grading in ACES.
//! Unlike ACEScct, it has no linear toe segment - it's pure log throughout.
//! This can cause issues with very dark values but provides simpler math.
//!
//! # Range
//!
//! - Linear input: scene-referred, typically [0, 65504]
//! - Encoded output: approximately [-0.36, 1.47]
//!
//! # Reference
//!
//! AMPAS S-2014-003 - ACEScc specification

// ACEScc uses pure log2 with no linear toe
const MIN_VAL: f32 = 1.0 / 65536.0; // 2^-16, avoids log(0)

/// ACEScc encode: Converts ACES linear to ACEScc.
///
/// # Formula
///
/// ```text
/// if linear <= 0:
///     ACEScc = (log2(2^-16) + 9.72) / 17.52  // = -0.3584
/// else if linear < 2^-15:
///     ACEScc = (log2(2^-16 + linear * 0.5) + 9.72) / 17.52
/// else:
///     ACEScc = (log2(linear) + 9.72) / 17.52
/// ```
///
/// # Example
///
/// ```rust
/// use vfx_transfer::acescc::encode;
///
/// let cc = encode(0.18);
/// assert!((cc - 0.4135).abs() < 0.001);
/// ```
#[inline]
pub fn encode(linear: f32) -> f32 {
    const LN2: f32 = std::f32::consts::LN_2;
    const THRESHOLD: f32 = 1.0 / 32768.0; // 2^-15
    
    if linear <= 0.0 {
        // Clamp to minimum representable value
        (MIN_VAL.ln() / LN2 + 9.72) / 17.52
    } else if linear < THRESHOLD {
        // Special handling for very small values
        ((MIN_VAL + linear * 0.5).ln() / LN2 + 9.72) / 17.52
    } else {
        (linear.ln() / LN2 + 9.72) / 17.52
    }
}

/// ACEScc decode: Converts ACEScc to ACES linear.
///
/// # Formula
///
/// ```text
/// if ACEScc < (9.72 - 15) / 17.52:
///     linear = (2^(ACEScc * 17.52 - 9.72) - 2^-16) * 2
/// else:
///     linear = 2^(ACEScc * 17.52 - 9.72)
/// ```
///
/// # Example
///
/// ```rust
/// use vfx_transfer::acescc::decode;
///
/// let linear = decode(0.4135);
/// assert!((linear - 0.18).abs() < 0.001);
/// ```
#[inline]
pub fn decode(cc: f32) -> f32 {
    const THRESHOLD: f32 = (9.72 - 15.0) / 17.52; // ≈ -0.3014
    
    if cc < THRESHOLD {
        // Inverse of the small-value encoding
        (2.0_f32.powf(cc * 17.52 - 9.72) - MIN_VAL) * 2.0
    } else {
        2.0_f32.powf(cc * 17.52 - 9.72)
    }
}

/// Applies ACEScc encode to an RGB triplet.
#[inline]
pub fn encode_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [encode(rgb[0]), encode(rgb[1]), encode(rgb[2])]
}

/// Applies ACEScc decode to an RGB triplet.
#[inline]
pub fn decode_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [decode(rgb[0]), decode(rgb[1]), decode(rgb[2])]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        // ACEScc has issues near zero, so test from 0.001
        let values = [0.001, 0.01, 0.18, 1.0, 10.0, 100.0];
        for &v in &values {
            let encoded = encode(v);
            let back = decode(encoded);
            let tol = 1e-4 * v.max(0.001);
            assert!((v - back).abs() < tol, "v={}, back={}, diff={}", v, back, (v - back).abs());
        }
    }

    #[test]
    fn test_midgray() {
        // 18% gray should encode to approximately 0.4135
        let encoded = encode(0.18);
        assert!((encoded - 0.4135).abs() < 0.001);
    }

    #[test]
    fn test_negative_clamp() {
        // Negative values should clamp to minimum
        let neg = encode(-1.0);
        let zero = encode(0.0);
        assert!((neg - zero).abs() < 1e-6);
    }
}
