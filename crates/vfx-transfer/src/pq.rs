//! SMPTE ST 2084 Perceptual Quantizer (PQ) transfer function.
//!
//! PQ is designed for HDR content, encoding luminance up to 10,000 cd/m2
//! in a perceptually uniform way.
//!
//! # Range
//!
//! - Encoded: [0, 1]
//! - Linear: [0, 10000] cd/m2 (nits)
//!
//! # Reference
//!
//! SMPTE ST 2084:2014
//!
//! # Usage
//!
//! ```rust
//! use vfx_transfer::pq;
//!
//! // Decode PQ signal to absolute luminance
//! let nits = pq::eotf(0.5);
//!
//! // Encode luminance to PQ
//! let signal = pq::oetf(100.0);
//! ```

/// Maximum luminance in cd/m2 (nits).
pub const L_MAX: f32 = 10000.0;

// PQ constants from SMPTE ST 2084
const M1: f32 = 2610.0 / 16384.0;
const M2: f32 = 2523.0 / 4096.0 * 128.0;
const C1: f32 = 3424.0 / 4096.0;
const C2: f32 = 2413.0 / 4096.0 * 32.0;
const C3: f32 = 2392.0 / 4096.0 * 32.0;

/// PQ EOTF: Decodes PQ signal to absolute luminance (cd/m2).
///
/// # Arguments
///
/// * `v` - PQ encoded value [0, 1]
///
/// # Returns
///
/// Absolute luminance in cd/m2 [0, 10000].
///
/// # Example
///
/// ```rust
/// use vfx_transfer::pq::eotf;
///
/// // Reference white (100 nits)
/// let nits = eotf(0.508);
/// assert!((nits - 100.0).abs() < 1.0);
/// ```
#[inline]
pub fn eotf(v: f32) -> f32 {
    if v <= 0.0 {
        return 0.0;
    }

    let vp = v.powf(1.0 / M2);
    let num = (vp - C1).max(0.0);
    let den = C2 - C3 * vp;

    L_MAX * (num / den).powf(1.0 / M1)
}

/// PQ OETF: Encodes absolute luminance to PQ signal.
///
/// # Arguments
///
/// * `l` - Luminance in cd/m2 [0, 10000]
///
/// # Returns
///
/// PQ encoded value [0, 1].
///
/// # Example
///
/// ```rust
/// use vfx_transfer::pq::oetf;
///
/// // Encode 100 nits (reference white)
/// let signal = oetf(100.0);
/// assert!((signal - 0.508).abs() < 0.01);
/// ```
#[inline]
pub fn oetf(l: f32) -> f32 {
    if l <= 0.0 {
        return 0.0;
    }

    let y = (l / L_MAX).clamp(0.0, 1.0);
    let yp = y.powf(M1);
    let num = C1 + C2 * yp;
    let den = 1.0 + C3 * yp;

    (num / den).powf(M2)
}

/// Normalized PQ EOTF: Returns [0, 1] normalized linear.
///
/// Divides absolute luminance by reference white (100 nits).
#[inline]
pub fn eotf_normalized(v: f32) -> f32 {
    eotf(v) / 100.0
}

/// Normalized PQ OETF: Accepts [0, 1] normalized linear.
///
/// Multiplies by reference white (100 nits) before encoding.
#[inline]
pub fn oetf_normalized(l: f32) -> f32 {
    oetf(l * 100.0)
}

/// Applies PQ EOTF to RGB, returning luminance in nits.
#[inline]
pub fn eotf_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [eotf(rgb[0]), eotf(rgb[1]), eotf(rgb[2])]
}

/// Applies PQ OETF to RGB luminance values.
#[inline]
pub fn oetf_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [oetf(rgb[0]), oetf(rgb[1]), oetf(rgb[2])]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let test_values = [0.0, 0.1, 0.5, 1.0, 10.0, 100.0, 1000.0, 10000.0];
        for &l in &test_values {
            let encoded = oetf(l);
            let decoded = eotf(encoded);
            assert!(
                (l - decoded).abs() < l * 0.001 + 0.001,
                "l={}, decoded={}",
                l,
                decoded
            );
        }
    }

    #[test]
    fn test_reference_white() {
        // 100 nits should be around 0.508 in PQ
        let signal = oetf(100.0);
        assert!((signal - 0.508).abs() < 0.01);
    }

    #[test]
    fn test_boundaries() {
        assert_eq!(eotf(0.0), 0.0);
        assert!((eotf(1.0) - L_MAX).abs() < 1.0);
    }
}
