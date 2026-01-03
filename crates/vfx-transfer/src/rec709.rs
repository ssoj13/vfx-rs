//! Rec.709 (BT.709) transfer function.
//!
//! The Rec.709 OETF is used for HDTV encoding. Note that the commonly
//! used Rec.709 EOTF is actually BT.1886 (gamma 2.4), not the inverse
//! of the OETF.
//!
//! # Range
//!
//! - Input/Output: [0, 1]
//!
//! # Reference
//!
//! ITU-R BT.709-6

/// Rec.709 OETF: Encodes linear to Rec.709.
///
/// # Formula
///
/// ```text
/// if L < 0.018:
///     V = 4.5 * L
/// else:
///     V = 1.099 * L^0.45 - 0.099
/// ```
#[inline]
pub fn oetf(l: f32) -> f32 {
    if l < 0.018 {
        4.5 * l
    } else {
        1.099 * l.powf(0.45) - 0.099
    }
}

/// Rec.709 inverse OETF: Decodes Rec.709 to linear.
///
/// Note: For display, use BT.1886 (gamma 2.4) instead.
#[inline]
pub fn eotf(v: f32) -> f32 {
    if v < 0.081 {
        v / 4.5
    } else {
        ((v + 0.099) / 1.099).powf(1.0 / 0.45)
    }
}

/// Applies Rec.709 EOTF to an RGB triplet.
#[inline]
pub fn eotf_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [eotf(rgb[0]), eotf(rgb[1]), eotf(rgb[2])]
}

/// Applies Rec.709 OETF to an RGB triplet.
#[inline]
pub fn oetf_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [oetf(rgb[0]), oetf(rgb[1]), oetf(rgb[2])]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        for i in 0..=100 {
            let v = i as f32 / 100.0;
            let linear = eotf(v);
            let back = oetf(linear);
            assert!((v - back).abs() < 1e-4, "v={}, back={}", v, back);
        }
    }

    #[test]
    fn test_boundaries() {
        assert_eq!(oetf(0.0), 0.0);
        assert!((oetf(1.0) - 1.0).abs() < 1e-6);
    }
}
