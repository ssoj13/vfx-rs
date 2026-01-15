//! ACES2 Tonescale (HDR to SDR mapping).
//!
//! Maps scene-referred luminance to display-referred luminance
//! with configurable peak luminance.

use super::common::*;
use super::cam::*;

// ============================================================================
// Tonescale Parameters
// ============================================================================

/// Parameters for the tonescale curve.
#[derive(Debug, Clone, Copy)]
pub struct TonescaleParams {
    /// Normalized peak luminance
    pub n: f32,
    /// Reciprocal of normalized peak
    pub n_r: f32,
    /// Gamma (contrast)
    pub g: f32,
    /// t_1 parameter
    pub t_1: f32,
    /// c_t parameter
    pub c_t: f32,
    /// s_2 parameter
    pub s_2: f32,
    /// u_2 parameter  
    pub u_2: f32,
    /// m_2 parameter
    pub m_2: f32,
    /// Forward limit
    pub forward_limit: f32,
    /// Inverse limit
    pub inverse_limit: f32,
    /// Log of peak luminance
    pub log_peak: f32,
}

/// Minimum allowed peak luminance (cd/m²).
/// Below this value, tonescale calculations become numerically unstable.
const MIN_PEAK_LUMINANCE: f32 = 1.0;

/// Maximum allowed peak luminance (cd/m²).
/// Above this value is beyond any practical display capability.
const MAX_PEAK_LUMINANCE: f32 = 100_000.0;

impl TonescaleParams {
    /// Initialize tonescale parameters for given peak luminance.
    ///
    /// # Arguments
    /// * `peak_luminance` - Display peak luminance in cd/m² (nits).
    ///   Must be in range [1.0, 100000.0].
    ///
    /// # Panics
    /// Panics if `peak_luminance` is outside the valid range.
    pub fn new(peak_luminance: f32) -> Self {
        // Validate input to prevent division by zero and numerical instability
        assert!(
            peak_luminance >= MIN_PEAK_LUMINANCE && peak_luminance <= MAX_PEAK_LUMINANCE,
            "peak_luminance must be in range [{}, {}], got {}",
            MIN_PEAK_LUMINANCE, MAX_PEAK_LUMINANCE, peak_luminance
        );
        
        // Normalized peak
        let n = peak_luminance / REFERENCE_LUMINANCE;
        let n_r = REFERENCE_LUMINANCE / peak_luminance;
        
        // Tonescale curve parameters
        // These are carefully tuned for perceptually uniform contrast
        let g = 1.15; // Gamma/contrast
        
        // Calculate derived parameters
        let c: f32 = 0.18; // Mid-gray reference
        let c_d: f32 = 10.013; // Dark compression
        let w_g: f32 = 0.14; // White gain
        
        let m_1 = (c * c_d + 1.0).ln() / c_d.ln();
        let s_1 = (c * c_d + 1.0).ln() / (c * c_d * c_d.ln());
        
        let u_2 = ((n * c_d + 1.0).ln() / c_d.ln() - m_1) / s_1;
        let m_2 = m_1 + s_1 * u_2;
        let s_2 = s_1 * u_2 / m_2;
        
        let t_1 = m_2 * w_g;
        let c_t = (1.0 + t_1) / m_2;
        
        // Limits for safe inverse
        let forward_limit = n;
        let inverse_limit = m_2 * 0.9999;
        
        let log_peak = n.ln();
        
        Self {
            n,
            n_r,
            g,
            t_1,
            c_t,
            s_2,
            u_2,
            m_2,
            forward_limit,
            inverse_limit,
            log_peak,
        }
    }

    /// Create tonescale for SDR (100 nits)
    pub fn sdr() -> Self {
        Self::new(100.0)
    }

    /// Create tonescale for HDR (1000 nits)
    pub fn hdr_1000() -> Self {
        Self::new(1000.0)
    }

    /// Create tonescale for HDR (4000 nits)
    pub fn hdr_4000() -> Self {
        Self::new(4000.0)
    }
}

// ============================================================================
// Tonescale Functions
// ============================================================================

/// ACES tonescale forward (scene Y to display Y).
#[inline]
pub fn aces_tonescale_fwd(y: f32, p: &TonescaleParams) -> f32 {
    let f = p.m_2 * (y / (y + p.s_2)).powf(p.g);
    let y_ts = (f * f / (f + p.t_1)).max(0.0) * p.n_r;
    y_ts
}

/// ACES tonescale inverse (display Y to scene Y).
#[inline]
pub fn aces_tonescale_inv(y: f32, p: &TonescaleParams) -> f32 {
    let y_ts_norm = y / REFERENCE_LUMINANCE;
    let z = y_ts_norm.max(0.0).min(p.inverse_limit);
    
    // Protect against division by zero when z is very small
    if z < 1e-10 {
        return 0.0;
    }
    
    let f = (z + (z * (4.0 * p.t_1 + z)).sqrt()) / 2.0;
    
    // Protect against division by zero in the final calculation
    // This can happen when f approaches m_2 (denominator approaches 0)
    let ratio = p.m_2 / f.max(1e-10);
    let denom = ratio.powf(1.0 / p.g) - 1.0;
    
    if denom.abs() < 1e-10 {
        // At the limit, return a large but finite value
        return p.s_2 * 1e6;
    }
    
    p.s_2 / denom
}

/// Apply tonescale to J (lightness) forward.
pub fn tonescale_j_fwd(j: f32, jmh_p: &JMhParams, ts_p: &TonescaleParams) -> f32 {
    let j_abs = j.abs();
    let y_in = j_to_y(j_abs, jmh_p);
    let y_out = aces_tonescale_fwd(y_in, ts_p);
    let j_out = y_to_j(y_out, jmh_p);
    j_out.copysign(j)
}

/// Apply tonescale to J (lightness) inverse.
pub fn tonescale_j_inv(j: f32, jmh_p: &JMhParams, ts_p: &TonescaleParams) -> f32 {
    let j_abs = j.abs();
    let y_in = j_to_y(j_abs, jmh_p);
    let y_out = aces_tonescale_inv(y_in, ts_p);
    let j_out = y_to_j(y_out, jmh_p);
    j_out.copysign(j)
}

/// Apply tonescale from achromatic (A) to J.
pub fn tonescale_a_to_j_fwd(a: f32, jmh_p: &JMhParams, ts_p: &TonescaleParams) -> f32 {
    // Convert A to Y
    let y_in = if a <= 0.0 { 0.0 } else {
        let ra = jmh_p.a_w_j * a;
        let rc = if ra >= 0.99 { 
            ra 
        } else { 
            (CAM_NL_OFFSET * ra) / (1.0 - ra)
        };
        rc.powf(1.0 / 0.42) / jmh_p.f_l_n
    };
    
    // Apply tonescale
    let y_out = aces_tonescale_fwd(y_in, ts_p);
    
    // Convert back to J
    y_to_j(y_out, jmh_p).copysign(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn srgb_to_xyz() -> M33 {
        [
            0.4124564, 0.3575761, 0.1804375,
            0.2126729, 0.7151522, 0.0721750,
            0.0193339, 0.1191920, 0.9503041,
        ]
    }

    #[test]
    fn test_tonescale_params() {
        let p = TonescaleParams::new(100.0);
        assert!(p.n > 0.0);
        assert!(p.m_2 > 0.0);
        assert!(p.s_2 > 0.0);
    }

    #[test]
    fn test_tonescale_black_is_black() {
        let p = TonescaleParams::sdr();
        let y_out = aces_tonescale_fwd(0.0, &p);
        assert!(y_out.abs() < 1e-6);
    }

    #[test]
    fn test_tonescale_monotonic() {
        let p = TonescaleParams::sdr();
        let mut prev = 0.0;
        
        for i in 1..100 {
            let y = i as f32 * 0.1;
            let y_out = aces_tonescale_fwd(y, &p);
            assert!(y_out >= prev, "Tonescale not monotonic at y={}", y);
            prev = y_out;
        }
    }

    #[test]
    fn test_tonescale_clamped() {
        let p = TonescaleParams::sdr();
        
        // Very high values should not exceed limit
        let y_out = aces_tonescale_fwd(1000.0, &p);
        assert!(y_out <= p.n * 1.01, "Tonescale output {} exceeds limit", y_out);
    }

    #[test]
    fn test_tonescale_j_basic() {
        let jmh_p = JMhParams::new(&srgb_to_xyz());
        let ts_p = TonescaleParams::sdr();
        
        // Test that tonescale produces valid output
        for j in [10.0, 30.0, 50.0, 70.0, 90.0] {
            let j_ts = tonescale_j_fwd(j, &jmh_p, &ts_p);
            assert!(j_ts > 0.0, "Tonescale output should be positive for j={}", j);
            assert!(j_ts < 200.0, "Tonescale output should be reasonable for j={}", j);
        }
    }

    #[test]
    fn test_hdr_vs_sdr_params() {
        let sdr = TonescaleParams::sdr();
        let hdr = TonescaleParams::hdr_1000();
        
        // HDR should have higher peak
        assert!(hdr.n > sdr.n, "HDR peak should be higher than SDR");
        assert!(hdr.m_2 > sdr.m_2, "HDR m_2 should be higher");
    }

    #[test]
    fn test_tonescale_inv_zero_input() {
        let p = TonescaleParams::sdr();
        // Should not panic, should return 0
        let result = aces_tonescale_inv(0.0, &p);
        assert!(result.is_finite(), "Inverse tonescale should return finite value for zero input");
        assert!(result.abs() < 1e-6, "Inverse tonescale of zero should be near zero");
    }

    #[test]
    fn test_tonescale_inv_near_limit() {
        let p = TonescaleParams::sdr();
        // Test near the inverse limit - should not panic or return NaN/Inf
        let near_limit = p.inverse_limit * REFERENCE_LUMINANCE * 0.9999;
        let result = aces_tonescale_inv(near_limit, &p);
        assert!(result.is_finite(), "Inverse tonescale should return finite value near limit");
    }

    #[test]
    #[should_panic(expected = "peak_luminance must be in range")]
    fn test_invalid_peak_luminance_zero() {
        let _ = TonescaleParams::new(0.0);
    }

    #[test]
    #[should_panic(expected = "peak_luminance must be in range")]
    fn test_invalid_peak_luminance_negative() {
        let _ = TonescaleParams::new(-100.0);
    }

    #[test]
    fn test_valid_peak_luminance_boundary() {
        // Should not panic at valid boundaries
        let min_valid = TonescaleParams::new(1.0);
        assert!(min_valid.n > 0.0);
        
        let max_valid = TonescaleParams::new(100_000.0);
        assert!(max_valid.n > 0.0);
    }
}
