//! ACES2 Gamut Compression.
//!
//! Maps out-of-gamut colors to the display gamut boundary while
//! preserving hue and smooth rolloff.

use super::common::*;
use super::cam::JMhParams;
use super::chroma::{ResolvedCompressionParams, toe_fwd, toe_inv};

// ============================================================================
// Gamut Compression Parameters
// ============================================================================

/// Hue-dependent gamut parameters.
#[derive(Debug, Clone, Copy)]
pub struct HueDependentGamutParams {
    /// Inverse gamma for bottom region
    pub gamma_bottom_inv: f32,
    /// Cusp JM coordinates
    pub jm_cusp: F2,
    /// Inverse gamma for top region
    pub gamma_top_inv: f32,
    /// Focus J value
    pub focus_j: f32,
    /// Analytical threshold
    pub analytical_threshold: f32,
}

/// Gamut compression parameters.
#[derive(Debug, Clone)]
pub struct GamutCompressParams {
    /// Mid J value
    pub mid_j: f32,
    /// Focus distance
    pub focus_dist: f32,
    /// Lower hull gamma inverse
    pub lower_hull_gamma_inv: f32,
    /// Hue linearity search range
    pub hue_linearity_search_range: [i32; 2],
    /// Hue table (362 entries)
    pub hue_table: Vec<f32>,
    /// Gamut cusp table (362 x 3 entries: J, M, h)
    pub gamut_cusp_table: Vec<F3>,
}

impl GamutCompressParams {
    /// Create new gamut compression parameters.
    pub fn new(
        peak_luminance: f32,
        _input_jmh_params: &JMhParams,
        _limit_jmh_params: &JMhParams,
    ) -> Self {
        // Calculate mid J
        let mid_j = 0.5 * (100.0 + y_to_j_simple(peak_luminance / REFERENCE_LUMINANCE));
        
        // Focus distance based on peak
        let focus_dist = FOCUS_DISTANCE + FOCUS_DISTANCE_SCALING * (peak_luminance / 1000.0).ln().max(0.0);
        
        // Lower hull gamma
        let lower_hull_gamma_inv = 1.0 / (1.14 + 0.07 * (peak_luminance / 1000.0).ln().max(0.0));
        
        // Search range for hue table lookups
        let hue_linearity_search_range = [-3, 4];
        
        // Initialize tables (will be filled by build functions)
        let hue_table = vec![0.0; 362];
        let gamut_cusp_table = vec![[0.0, 0.0, 0.0]; 362];
        
        Self {
            mid_j,
            focus_dist,
            lower_hull_gamma_inv,
            hue_linearity_search_range,
            hue_table,
            gamut_cusp_table,
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Simplified Y to J (without full JMh params)
fn y_to_j_simple(y: f32) -> f32 {
    let ra = (y.abs() * 0.42).powf(0.42) / (CAM_NL_OFFSET + (y.abs() * 0.42).powf(0.42));
    let cz = SURROUND[1] * (1.48 + (Y_B / REFERENCE_LUMINANCE).sqrt());
    J_SCALE * ra.powf(cz)
}

/// Generate unit cube cusp corner RGB values.
///
/// Returns RGB for corners: R, Y, G, C, B, M (in rotating order)
pub fn generate_cusp_corner(corner: usize) -> F3 {
    // Generation order: R, Y, G, C, B, M
    [
        if ((corner + 1) % CUSP_CORNER_COUNT) < 3 { 1.0 } else { 0.0 },
        if ((corner + 5) % CUSP_CORNER_COUNT) < 3 { 1.0 } else { 0.0 },
        if ((corner + 3) % CUSP_CORNER_COUNT) < 3 { 1.0 } else { 0.0 },
    ]
}

/// Find cusp (maximum chroma point) for a given hue.
pub fn find_cusp_for_hue(
    hue: f32,
    rgb_corners: &[[f32; 3]; TOTAL_CORNER_COUNT],
    jmh_corners: &[[f32; 3]; TOTAL_CORNER_COUNT],
    params: &JMhParams,
) -> F2 {
    // Find the interval containing this hue
    let mut upper_corner = 1;
    for i in 1..TOTAL_CORNER_COUNT {
        if jmh_corners[i][2] > hue {
            upper_corner = i;
            break;
        }
    }
    let lower_corner = upper_corner - 1;
    
    // Check for exact match
    if (jmh_corners[lower_corner][2] - hue).abs() < 1e-6 {
        return [jmh_corners[lower_corner][0], jmh_corners[lower_corner][1]];
    }
    
    // Binary search along the edge between corners
    let cusp_lower = &rgb_corners[lower_corner];
    let cusp_upper = &rgb_corners[upper_corner];
    
    let mut lower_t = 0.0;
    let mut upper_t = 1.0;
    
    while (upper_t - lower_t) > DISPLAY_CUSP_TOLERANCE {
        let sample_t = midpoint(lower_t, upper_t);
        let sample = lerp_f3(cusp_lower, cusp_upper, sample_t);
        let jmh = super::cam::rgb_to_jmh(&sample, params);
        
        if jmh[2] < jmh_corners[lower_corner][2] {
            upper_t = sample_t;
        } else if jmh[2] >= jmh_corners[upper_corner][2] {
            lower_t = sample_t;
        } else if jmh[2] > hue {
            upper_t = sample_t;
        } else {
            lower_t = sample_t;
        }
    }
    
    let final_t = midpoint(lower_t, upper_t);
    let sample = lerp_f3(cusp_lower, cusp_upper, final_t);
    let jmh = super::cam::rgb_to_jmh(&sample, params);
    
    [jmh[0], jmh[1]]
}

// ============================================================================
// Gamut Compression
// ============================================================================

/// Resolve hue-dependent gamut parameters for a specific hue.
pub fn resolve_hue_dependent_params(
    _hue: f32,
    cusp_jm: &F2,
    gamut_params: &GamutCompressParams,
    shared_params: &ResolvedCompressionParams,
) -> HueDependentGamutParams {
    let cusp_j = cusp_jm[0];
    let cusp_m = cusp_jm[1];
    
    // Calculate gamma for bottom and top regions
    let gamma_bottom = cusp_j / (cusp_m + 1e-6);
    let gamma_top = (shared_params.limit_j_max - cusp_j) / (cusp_m + 1e-6);
    
    // Focus J is the J value where compression focuses
    let focus_j = lerp(cusp_j, gamut_params.mid_j, FOCUS_GAIN_BLEND);
    
    // Analytical threshold for switching between compression methods
    let analytical_threshold = COMPRESSION_THRESHOLD * cusp_m;
    
    HueDependentGamutParams {
        gamma_bottom_inv: 1.0 / gamma_bottom.max(0.001),
        jm_cusp: *cusp_jm,
        gamma_top_inv: 1.0 / gamma_top.max(0.001),
        focus_j,
        analytical_threshold,
    }
}

/// Apply gamut compression forward.
pub fn gamut_compress_fwd(
    jmh: &F3,
    shared_params: &ResolvedCompressionParams,
    gamut_params: &GamutCompressParams,
    hue_params: &HueDependentGamutParams,
) -> F3 {
    let j = jmh[0];
    let m = jmh[1];
    let h = jmh[2];
    
    if m < 1e-6 {
        return [j, m, h];
    }
    
    let cusp_j = hue_params.jm_cusp[0];
    let cusp_m = hue_params.jm_cusp[1];
    
    // Determine if we're above or below the cusp
    let in_upper = j > cusp_j;
    
    // Calculate the boundary M at this J
    let boundary_m = if in_upper {
        let t = (j - cusp_j) / (shared_params.limit_j_max - cusp_j + 1e-6);
        cusp_m * (1.0 - t.powf(hue_params.gamma_top_inv))
    } else {
        let t = j / (cusp_j + 1e-6);
        cusp_m * t.powf(hue_params.gamma_bottom_inv)
    };
    
    // If already inside gamut, no compression needed
    if m <= boundary_m * COMPRESSION_THRESHOLD {
        return [j, m, h];
    }
    
    // Apply compression using toe function
    let normalized_m = m / (boundary_m + 1e-6);
    
    // Focus-based compression
    let focus_dist = gamut_params.focus_dist;
    let compressed = toe_fwd(normalized_m, 1.0, focus_dist * 0.5, focus_dist * 0.3);
    
    let m_out = compressed * boundary_m;
    
    [j, m_out.min(boundary_m), h]
}

/// Apply gamut compression inverse.
pub fn gamut_compress_inv(
    jmh: &F3,
    shared_params: &ResolvedCompressionParams,
    gamut_params: &GamutCompressParams,
    hue_params: &HueDependentGamutParams,
) -> F3 {
    let j = jmh[0];
    let m = jmh[1];
    let h = jmh[2];
    
    if m < 1e-6 {
        return [j, m, h];
    }
    
    let cusp_j = hue_params.jm_cusp[0];
    let cusp_m = hue_params.jm_cusp[1];
    
    let in_upper = j > cusp_j;
    
    let boundary_m = if in_upper {
        let t = (j - cusp_j) / (shared_params.limit_j_max - cusp_j + 1e-6);
        cusp_m * (1.0 - t.powf(hue_params.gamma_top_inv))
    } else {
        let t = j / (cusp_j + 1e-6);
        cusp_m * t.powf(hue_params.gamma_bottom_inv)
    };
    
    if m <= boundary_m * COMPRESSION_THRESHOLD {
        return [j, m, h];
    }
    
    // Inverse compression
    let normalized_m = m / (boundary_m + 1e-6);
    let focus_dist = gamut_params.focus_dist;
    let expanded = toe_inv(normalized_m, 1.0, focus_dist * 0.5, focus_dist * 0.3);
    
    let m_out = expanded * boundary_m;
    
    [j, m_out, h]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_cusp_corners() {
        // R corner
        let r = generate_cusp_corner(0);
        assert!((r[0] - 1.0).abs() < 1e-6);
        assert!((r[1] - 0.0).abs() < 1e-6);
        assert!((r[2] - 0.0).abs() < 1e-6);
        
        // G corner
        let g = generate_cusp_corner(2);
        assert!((g[0] - 0.0).abs() < 1e-6);
        assert!((g[1] - 1.0).abs() < 1e-6);
        assert!((g[2] - 0.0).abs() < 1e-6);
        
        // B corner
        let b = generate_cusp_corner(4);
        assert!((b[0] - 0.0).abs() < 1e-6);
        assert!((b[1] - 0.0).abs() < 1e-6);
        assert!((b[2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_gamut_compress_achromatic() {
        let shared = ResolvedCompressionParams {
            limit_j_max: 100.0,
            model_gamma_inv: 1.0,
            reach_max_m: 50.0,
        };
        let gamut = GamutCompressParams {
            mid_j: 50.0,
            focus_dist: 1.35,
            lower_hull_gamma_inv: 1.0,
            hue_linearity_search_range: [-3, 4],
            hue_table: vec![0.0; 362],
            gamut_cusp_table: vec![[0.0, 0.0, 0.0]; 362],
        };
        let hue_dep = HueDependentGamutParams {
            gamma_bottom_inv: 1.0,
            jm_cusp: [50.0, 40.0],
            gamma_top_inv: 1.0,
            focus_j: 50.0,
            analytical_threshold: 30.0,
        };
        
        // Achromatic (M=0) should pass through
        let jmh = [50.0, 0.0, 180.0];
        let result = gamut_compress_fwd(&jmh, &shared, &gamut, &hue_dep);
        assert!((result[1] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_gamut_compress_inside_gamut() {
        let shared = ResolvedCompressionParams {
            limit_j_max: 100.0,
            model_gamma_inv: 1.0,
            reach_max_m: 50.0,
        };
        let gamut = GamutCompressParams {
            mid_j: 50.0,
            focus_dist: 1.35,
            lower_hull_gamma_inv: 1.0,
            hue_linearity_search_range: [-3, 4],
            hue_table: vec![0.0; 362],
            gamut_cusp_table: vec![[0.0, 0.0, 0.0]; 362],
        };
        let hue_dep = HueDependentGamutParams {
            gamma_bottom_inv: 1.0,
            jm_cusp: [50.0, 40.0],
            gamma_top_inv: 1.0,
            focus_j: 50.0,
            analytical_threshold: 30.0,
        };
        
        // Small M (inside gamut) should pass through mostly unchanged
        let jmh = [50.0, 5.0, 180.0];
        let result = gamut_compress_fwd(&jmh, &shared, &gamut, &hue_dep);
        assert!((result[1] - 5.0).abs() < 1.0, "Inside-gamut value changed too much");
    }
}
