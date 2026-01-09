//! Gamma transfer functions (pure and moncurve).
//!
//! Simple gamma curves and moncurve (gamma with linear segment).
//!
//! ## Pure Gamma
//! - 2.2: Legacy CRT approximation
//! - 2.4: BT.1886 reference EOTF
//! - 2.6: DCI theatrical projection
//!
//! ## Moncurve
//! Gamma with linear segment near black (ExponentWithLinear style).
//! Parameters: gamma (exponent >= 1) and offset (0 to 0.9).
//!
//! Based on OCIO GammaOp implementation.
//!
//! # Range
//!
//! - Input/Output: [0, 1] for basic, mirrors negatives for mirror mode

const EPS: f32 = 1e-6;

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

// ============================================================================
// Moncurve (gamma with linear segment) - OCIO compatible
// ============================================================================

/// Computed parameters for moncurve rendering.
#[derive(Debug, Clone, Copy)]
pub struct MoncurveParams {
    /// Gamma exponent
    pub gamma: f32,
    /// Offset for power function
    pub offset: f32,
    /// Break point between linear and power
    pub break_pnt: f32,
    /// Slope of linear segment
    pub slope: f32,
    /// Scale factor
    pub scale: f32,
}

/// Computes moncurve parameters for forward direction (linear to encoded).
///
/// Based on OCIO GammaOpUtils.cpp ComputeParamsFwd.
///
/// # Arguments
/// * `gamma_param` - Gamma exponent (must be >= 1)
/// * `offset_param` - Offset parameter (0 to 0.9)
pub fn moncurve_params_fwd(gamma_param: f32, offset_param: f32) -> MoncurveParams {
    let gamma = (gamma_param).max(1.0 + EPS);
    let offset = offset_param.max(EPS);
    
    // Break point
    let break_pnt = offset / (gamma - 1.0);
    
    // Scale and rendered offset
    let scale = 1.0 / (1.0 + offset);
    let offset_rendered = offset / (1.0 + offset);
    
    // Slope: ((gamma - 1) / offset) * pow(offset * gamma / ((gamma - 1) * (1 + offset)), gamma)
    let a = (gamma - 1.0) / offset;
    let b = offset * gamma / ((gamma - 1.0) * (1.0 + offset));
    let slope = a * b.powf(gamma);
    
    MoncurveParams {
        gamma,
        offset: offset_rendered,
        break_pnt,
        slope,
        scale,
    }
}

/// Computes moncurve parameters for reverse direction (encoded to linear).
///
/// Based on OCIO GammaOpUtils.cpp ComputeParamsRev.
pub fn moncurve_params_rev(gamma_param: f32, offset_param: f32) -> MoncurveParams {
    let gamma = (gamma_param).max(1.0 + EPS);
    let offset = offset_param.max(EPS);
    
    // Inverse gamma
    let gamma_inv = 1.0 / gamma;
    
    // Break point for reverse
    let a = offset * gamma;
    let b = (gamma - 1.0) * (1.0 + offset);
    let break_pnt = (a / b).powf(gamma);
    
    // Slope for reverse
    let slope_a = (gamma - 1.0) / offset;
    let slope_b = (1.0 + offset) / gamma;
    let slope = slope_a.powf(gamma - 1.0) * slope_b.powf(gamma);
    
    // Scale and offset for reverse
    let scale = 1.0 + offset;
    
    MoncurveParams {
        gamma: gamma_inv,
        offset,
        break_pnt,
        slope,
        scale,
    }
}

/// Moncurve forward (linear to encoded).
///
/// Formula (OCIO compatible):
/// - if x <= break_pnt: out = slope * x
/// - else: out = pow(x * scale + offset, gamma)
///
/// # Example
///
/// ```rust
/// use vfx_transfer::gamma::moncurve_fwd;
///
/// // With gamma=2.2, offset=0.1
/// let encoded = moncurve_fwd(0.5, 2.2, 0.1);
/// assert!(encoded > 0.0 && encoded < 1.0);
/// ```
#[inline]
pub fn moncurve_fwd(x: f32, gamma: f32, offset: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    let p = moncurve_params_fwd(gamma, offset);
    if x <= p.break_pnt {
        p.slope * x
    } else {
        (x * p.scale + p.offset).powf(p.gamma)
    }
}

/// Moncurve reverse (encoded to linear).
///
/// Formula (OCIO compatible):
/// - if y <= break_pnt: out = y / slope
/// - else: out = pow(y, gamma) * scale - offset
///
/// # Example
///
/// ```rust
/// use vfx_transfer::gamma::moncurve_rev;
///
/// let linear = moncurve_rev(0.5, 2.2, 0.1);
/// assert!(linear > 0.0 && linear < 1.0);
/// ```
#[inline]
pub fn moncurve_rev(y: f32, gamma: f32, offset: f32) -> f32 {
    if y <= 0.0 {
        return 0.0;
    }
    let p = moncurve_params_rev(gamma, offset);
    if y <= p.break_pnt {
        y * p.slope  // slope in rev is actually 1/slope_fwd for linear segment
    } else {
        y.powf(p.gamma) * p.scale - p.offset
    }
}

/// Moncurve forward with mirror mode for negative values.
///
/// Negative inputs are mirrored: sign(x) * moncurve_fwd(|x|)
///
/// # Example
///
/// ```rust
/// use vfx_transfer::gamma::moncurve_mirror_fwd;
///
/// let pos = moncurve_mirror_fwd(0.5, 2.2, 0.1);
/// let neg = moncurve_mirror_fwd(-0.5, 2.2, 0.1);
/// assert!((pos + neg).abs() < 1e-6);
/// ```
#[inline]
pub fn moncurve_mirror_fwd(x: f32, gamma: f32, offset: f32) -> f32 {
    if x >= 0.0 {
        moncurve_fwd(x, gamma, offset)
    } else {
        -moncurve_fwd(-x, gamma, offset)
    }
}

/// Moncurve reverse with mirror mode for negative values.
///
/// # Example
///
/// ```rust
/// use vfx_transfer::gamma::moncurve_mirror_rev;
///
/// let pos = moncurve_mirror_rev(0.5, 2.2, 0.1);
/// let neg = moncurve_mirror_rev(-0.5, 2.2, 0.1);
/// assert!((pos + neg).abs() < 1e-6);
/// ```
#[inline]
pub fn moncurve_mirror_rev(y: f32, gamma: f32, offset: f32) -> f32 {
    if y >= 0.0 {
        moncurve_rev(y, gamma, offset)
    } else {
        -moncurve_rev(-y, gamma, offset)
    }
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

    #[test]
    fn test_moncurve_roundtrip() {
        // OCIO-compatible parameters
        let gamma = 2.2;
        let offset = 0.1;
        
        for i in 0..=100 {
            let x = i as f32 / 100.0;
            let encoded = moncurve_fwd(x, gamma, offset);
            let back = moncurve_rev(encoded, gamma, offset);
            assert!((x - back).abs() < 1e-4, "x={}, encoded={}, back={}", x, encoded, back);
        }
    }

    #[test]
    fn test_moncurve_mirror_roundtrip() {
        let gamma = 2.2;
        let offset = 0.1;
        
        for i in -100..=100 {
            let x = i as f32 / 100.0;
            let encoded = moncurve_mirror_fwd(x, gamma, offset);
            let back = moncurve_mirror_rev(encoded, gamma, offset);
            assert!((x - back).abs() < 1e-4, "x={}, back={}", x, back);
        }
    }

    #[test]
    fn test_moncurve_negative_mirrored() {
        let gamma = 2.2;
        let offset = 0.1;
        
        let pos = moncurve_mirror_fwd(0.5, gamma, offset);
        let neg = moncurve_mirror_fwd(-0.5, gamma, offset);
        assert!((pos + neg).abs() < 1e-6, "should mirror: {} vs {}", pos, neg);
    }

    #[test]
    fn test_moncurve_continuity() {
        // Check continuity at break point
        let gamma = 2.2;
        let offset = 0.1;
        let p = moncurve_params_fwd(gamma, offset);
        
        let below = moncurve_fwd(p.break_pnt - 1e-6, gamma, offset);
        let above = moncurve_fwd(p.break_pnt + 1e-6, gamma, offset);
        assert!((below - above).abs() < 1e-4, "discontinuity at break: {} vs {}", below, above);
    }

    #[test]
    fn test_moncurve_monotonic() {
        let gamma = 2.2;
        let offset = 0.1;
        
        let mut prev = 0.0;
        for i in 1..=100 {
            let x = i as f32 / 100.0;
            let y = moncurve_fwd(x, gamma, offset);
            assert!(y > prev, "not monotonic at x={}: {} <= {}", x, y, prev);
            prev = y;
        }
    }
}
