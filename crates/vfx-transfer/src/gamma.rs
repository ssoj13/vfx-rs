//! Pure gamma (power law) transfer functions.
//!
//! Simple gamma curves without the linear segment used by sRGB.
//! Common values:
//! - 2.2: Legacy CRT approximation
//! - 2.4: BT.1886 reference EOTF
//! - 2.6: DCI theatrical projection
//!
//! # Range
//!
//! - Input/Output: [0, 1]

/// EOTF for arbitrary gamma: `v^gamma`
///
/// # Example
///
/// ```rust
/// use vfx_transfer::gamma::gamma_eotf;
///
/// let linear = gamma_eotf(0.5, 2.2);
/// ```
#[inline]
pub fn gamma_eotf(v: f32, gamma: f32) -> f32 {
    if v <= 0.0 {
        0.0
    } else {
        v.powf(gamma)
    }
}

/// OETF for arbitrary gamma: `l^(1/gamma)`
///
/// # Example
///
/// ```rust
/// use vfx_transfer::gamma::gamma_oetf;
///
/// let encoded = gamma_oetf(0.218, 2.2);
/// assert!((encoded - 0.5).abs() < 0.01);
/// ```
#[inline]
pub fn gamma_oetf(l: f32, gamma: f32) -> f32 {
    if l <= 0.0 {
        0.0
    } else {
        l.powf(1.0 / gamma)
    }
}

/// Gamma 2.2 EOTF.
#[inline]
pub fn eotf_22(v: f32) -> f32 {
    gamma_eotf(v, 2.2)
}

/// Gamma 2.2 OETF.
#[inline]
pub fn oetf_22(l: f32) -> f32 {
    gamma_oetf(l, 2.2)
}

/// Gamma 2.4 EOTF (BT.1886 reference).
#[inline]
pub fn eotf_24(v: f32) -> f32 {
    gamma_eotf(v, 2.4)
}

/// Gamma 2.4 OETF.
#[inline]
pub fn oetf_24(l: f32) -> f32 {
    gamma_oetf(l, 2.4)
}

/// Gamma 2.6 EOTF (DCI theatrical).
#[inline]
pub fn eotf_26(v: f32) -> f32 {
    gamma_eotf(v, 2.6)
}

/// Gamma 2.6 OETF.
#[inline]
pub fn oetf_26(l: f32) -> f32 {
    gamma_oetf(l, 2.6)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gamma22_roundtrip() {
        for i in 0..=100 {
            let v = i as f32 / 100.0;
            let linear = eotf_22(v);
            let back = oetf_22(linear);
            assert!((v - back).abs() < 1e-5);
        }
    }

    #[test]
    fn test_gamma_identity() {
        // gamma 1.0 should be identity
        assert_eq!(gamma_eotf(0.5, 1.0), 0.5);
        assert_eq!(gamma_oetf(0.5, 1.0), 0.5);
    }
}
