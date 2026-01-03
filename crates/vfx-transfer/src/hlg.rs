//! Hybrid Log-Gamma (HLG) transfer function.
//!
//! HLG is designed for HDR broadcast, compatible with SDR displays.
//! Uses a logarithmic curve for highlights and a gamma curve for shadows.
//!
//! # Range
//!
//! - Encoded: [0, 1]
//! - Linear: [0, 1] (scene-referred, relative)
//!
//! # Reference
//!
//! ITU-R BT.2100-2

// HLG constants
const A: f32 = 0.17883277;
const B: f32 = 0.28466892; // 1 - 4*A
const C: f32 = 0.55991073; // 0.5 - A*ln(4*A)

/// HLG OETF: Encodes linear scene light to HLG signal.
///
/// # Formula
///
/// ```text
/// if E <= 1/12:
///     E' = sqrt(3 * E)
/// else:
///     E' = A * ln(12*E - B) + C
/// ```
///
/// # Example
///
/// ```rust
/// use vfx_transfer::hlg::oetf;
///
/// let signal = oetf(0.5);
/// ```
#[inline]
pub fn oetf(e: f32) -> f32 {
    if e <= 0.0 {
        0.0
    } else if e <= 1.0 / 12.0 {
        (3.0 * e).sqrt()
    } else {
        A * (12.0 * e - B).ln() + C
    }
}

/// HLG inverse OETF: Decodes HLG signal to linear scene light.
///
/// # Example
///
/// ```rust
/// use vfx_transfer::hlg::eotf;
///
/// let linear = eotf(0.5);
/// ```
#[inline]
pub fn eotf(ep: f32) -> f32 {
    if ep <= 0.0 {
        0.0
    } else if ep <= 0.5 {
        ep * ep / 3.0
    } else {
        (((ep - C) / A).exp() + B) / 12.0
    }
}

/// HLG OOTF (Opto-Optical Transfer Function).
///
/// Converts scene linear to display linear with system gamma.
/// The full HLG display pipeline is: EOTF -> OOTF -> Display.
///
/// # Arguments
///
/// * `y` - Scene linear luminance
/// * `gamma` - System gamma (typically 1.2 for dim surround)
///
/// # Returns
///
/// Display linear luminance.
#[inline]
pub fn ootf(y: f32, gamma: f32) -> f32 {
    y.powf(gamma - 1.0) * y
}

/// Applies HLG OETF to RGB.
#[inline]
pub fn oetf_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [oetf(rgb[0]), oetf(rgb[1]), oetf(rgb[2])]
}

/// Applies HLG EOTF to RGB.
#[inline]
pub fn eotf_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [eotf(rgb[0]), eotf(rgb[1]), eotf(rgb[2])]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        for i in 0..=100 {
            let e = i as f32 / 100.0;
            let encoded = oetf(e);
            let decoded = eotf(encoded);
            assert!((e - decoded).abs() < 1e-4, "e={}, decoded={}", e, decoded);
        }
    }

    #[test]
    fn test_boundaries() {
        assert_eq!(oetf(0.0), 0.0);
        assert!((oetf(1.0) - 1.0).abs() < 1e-6);
        assert_eq!(eotf(0.0), 0.0);
        assert!((eotf(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_transition_point() {
        // Test around 1/12 boundary
        let e = 1.0 / 12.0;
        let encoded = oetf(e);
        assert!(encoded > 0.0 && encoded < 1.0);
    }
}
