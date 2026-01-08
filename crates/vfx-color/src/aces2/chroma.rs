//! ACES2 Chroma Compression.
//!
//! Compresses chroma (colorfulness) to prevent oversaturation and clipping
//! when mapping from scene-referred to display-referred values.

use super::common::*;
use super::tonescale::TonescaleParams;

// ============================================================================
// Chroma Compression Parameters
// ============================================================================

/// Shared compression parameters (used by both chroma and gamut compression).
#[derive(Debug, Clone)]
pub struct SharedCompressionParams {
    /// Maximum J for the limiting gamut
    pub limit_j_max: f32,
    /// Inverse of model gamma
    pub model_gamma_inv: f32,
    /// Reach M table (360 entries + 2 for wrap)
    pub reach_m_table: Vec<f32>,
}

/// Resolved shared compression parameters for a specific hue.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedCompressionParams {
    /// Maximum J for the limiting gamut
    pub limit_j_max: f32,
    /// Inverse of model gamma
    pub model_gamma_inv: f32,
    /// Maximum M at reach for this hue
    pub reach_max_m: f32,
}

/// Chroma compression parameters.
#[derive(Debug, Clone, Copy)]
pub struct ChromaCompressParams {
    /// Saturation parameter
    pub sat: f32,
    /// Saturation threshold
    pub sat_thr: f32,
    /// Compression strength
    pub compr: f32,
    /// Chroma compression scale
    pub chroma_compress_scale: f32,
}

impl ChromaCompressParams {
    /// Cusp mid blend constant
    pub const CUSP_MID_BLEND: f32 = 1.3;

    /// Initialize chroma compression parameters.
    pub fn new(peak_luminance: f32, ts_params: &TonescaleParams) -> Self {
        // Calculate saturation and compression parameters based on peak luminance
        let sat = CHROMA_COMPRESS;
        let sat_thr = CHROMA_COMPRESS_FACT * (peak_luminance / REFERENCE_LUMINANCE).ln();
        let compr = CHROMA_EXPAND;
        let chroma_compress_scale = 1.0 / (CHROMA_EXPAND_FACT * ts_params.m_2);
        
        Self {
            sat,
            sat_thr: sat_thr.max(0.001),
            compr,
            chroma_compress_scale,
        }
    }
}

// ============================================================================
// Toe Functions (for smooth compression)
// ============================================================================

/// Toe function forward (smooth compression near zero).
#[inline]
pub fn toe_fwd(x: f32, limit: f32, k1_in: f32, k2_in: f32) -> f32 {
    if x > limit {
        return x;
    }
    
    let k2 = k2_in.max(0.001);
    let k1 = (k1_in * k1_in + k2 * k2).sqrt();
    let k3 = (limit + k1) / (limit + k2);
    
    let minus_b = k3 * x - k1;
    let minus_ac = k2 * k3 * x; // a is 1.0
    
    0.5 * (minus_b + (minus_b * minus_b + 4.0 * minus_ac).sqrt())
}

/// Toe function inverse.
#[inline]
pub fn toe_inv(x: f32, limit: f32, k1_in: f32, k2_in: f32) -> f32 {
    if x > limit {
        return x;
    }
    
    let k2 = k2_in.max(0.001);
    let k1 = (k1_in * k1_in + k2 * k2).sqrt();
    let k3 = (limit + k1) / (limit + k2);
    
    (x * x + k1 * x) / (k3 * (x + k2))
}

// ============================================================================
// Chroma Compression Norm
// ============================================================================

/// Calculate chroma compression normalization factor for a hue.
///
/// This uses a Fourier-based approximation of the gamut boundary.
pub fn chroma_compress_norm(cos_h: f32, sin_h: f32, scale: f32) -> f32 {
    // Compute higher-order trig terms
    let cos_h2 = 2.0 * cos_h * cos_h - 1.0;
    let sin_h2 = 2.0 * cos_h * sin_h;
    let cos_h3 = 4.0 * cos_h * cos_h * cos_h - 3.0 * cos_h;
    let sin_h3 = 3.0 * sin_h - 4.0 * sin_h * sin_h * sin_h;
    
    // Weights from OCIO reference
    const WEIGHTS: [f32; 8] = [
        11.34072, 16.46899, 7.88380, 0.0,
        14.66441, -6.37224, 9.19364, 77.12896,
    ];
    
    let m = WEIGHTS[0] * cos_h
          + WEIGHTS[1] * cos_h2
          + WEIGHTS[2] * cos_h3
          + WEIGHTS[4] * sin_h
          + WEIGHTS[5] * sin_h2
          + WEIGHTS[6] * sin_h3
          + WEIGHTS[7];
    
    m * scale
}

// ============================================================================
// Chroma Compression
// ============================================================================

/// Apply chroma compression forward.
///
/// Compresses chroma values to fit within the display gamut while
/// maintaining smooth rolloff.
pub fn chroma_compress_fwd(
    jmh: &F3,
    j_ts: f32,
    m_norm: f32,
    pr: &ResolvedCompressionParams,
    pc: &ChromaCompressParams,
) -> F3 {
    let j = jmh[0];
    let m = jmh[1];
    let h = jmh[2];
    
    if m == 0.0 {
        return [j_ts, 0.0, h];
    }
    
    // Normalized J
    let nj = j_ts / pr.limit_j_max;
    let snj = (1.0 - nj).max(0.0);
    
    // Calculate limit based on reach at this hue
    let limit = nj.powf(pr.model_gamma_inv) * pr.reach_max_m / m_norm;
    
    // Apply compression
    let mut m_cp = m * (j_ts / j).powf(pr.model_gamma_inv);
    m_cp /= m_norm;
    
    // Two-stage toe compression
    m_cp = limit - toe_fwd(limit - m_cp, limit - 0.001, snj * pc.sat, (nj * nj + pc.sat_thr).sqrt());
    m_cp = toe_fwd(m_cp, limit, nj * pc.compr, snj);
    
    m_cp *= m_norm;
    
    [j_ts, m_cp, h]
}

/// Apply chroma compression inverse.
pub fn chroma_compress_inv(
    jmh: &F3,
    j: f32,
    m_norm: f32,
    pr: &ResolvedCompressionParams,
    pc: &ChromaCompressParams,
) -> F3 {
    let j_ts = jmh[0];
    let m_cp = jmh[1];
    let h = jmh[2];
    
    if m_cp == 0.0 {
        return [j, 0.0, h];
    }
    
    let nj = j_ts / pr.limit_j_max;
    let snj = (1.0 - nj).max(0.0);
    let limit = nj.powf(pr.model_gamma_inv) * pr.reach_max_m / m_norm;
    
    // Inverse compression
    let mut m = m_cp / m_norm;
    m = toe_inv(m, limit, nj * pc.compr, snj);
    m = limit - toe_inv(limit - m, limit - 0.001, snj * pc.sat, (nj * nj + pc.sat_thr).sqrt());
    m *= m_norm;
    m *= (j_ts / j).powf(-pr.model_gamma_inv);
    
    [j, m, h]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toe_fwd_passthrough() {
        // Values above limit should pass through
        let result = toe_fwd(1.5, 1.0, 0.1, 0.1);
        assert!((result - 1.5).abs() < 1e-6);
    }

    #[test]
    fn test_toe_roundtrip() {
        let limit = 1.0;
        let k1 = 0.1;
        let k2 = 0.1;
        
        for x in [0.1, 0.3, 0.5, 0.7, 0.9] {
            let fwd = toe_fwd(x, limit, k1, k2);
            let inv = toe_inv(fwd, limit, k1, k2);
            assert!(
                (x - inv).abs() < 1e-5,
                "Toe roundtrip failed: {} -> {} -> {}", x, fwd, inv
            );
        }
    }

    #[test]
    fn test_chroma_compress_norm_range() {
        let scale = 0.01;
        
        // Test various hues
        for i in 0..360 {
            let h_rad = (i as f32).to_radians();
            let norm = chroma_compress_norm(h_rad.cos(), h_rad.sin(), scale);
            assert!(norm > 0.0, "Chroma norm should be positive at hue {}", i);
            assert!(norm < 10.0, "Chroma norm too large at hue {}", i);
        }
    }

    #[test]
    fn test_chroma_compress_zero() {
        let pr = ResolvedCompressionParams {
            limit_j_max: 100.0,
            model_gamma_inv: 1.0,
            reach_max_m: 50.0,
        };
        let pc = ChromaCompressParams {
            sat: 2.4,
            sat_thr: 0.5,
            compr: 1.3,
            chroma_compress_scale: 0.01,
        };
        
        let jmh = [50.0, 0.0, 180.0];
        let result = chroma_compress_fwd(&jmh, 50.0, 1.0, &pr, &pc);
        
        assert!((result[1] - 0.0).abs() < 1e-6, "Zero chroma should remain zero");
    }
}
