//! ACES2 Lookup Tables.
//!
//! Pre-computed tables for efficient gamut and chroma compression.

use super::common::*;
use super::cam::{JMhParams, rgb_to_jmh, rgb_to_aab};
use super::common::{CUSP_CORNER_COUNT, TOTAL_CORNER_COUNT};
use super::gamut::generate_cusp_corner;

// ============================================================================
// Table Constants
// ============================================================================

/// Number of table entries (360 degrees + 2 for wrap)
pub const TABLE_SIZE: usize = 362;

/// Nominal table size (360 degrees)
pub const TABLE_NOMINAL_SIZE: usize = 360;

/// Base index (after lower wrap)
pub const TABLE_BASE_INDEX: usize = 1;

/// Lower wrap index
pub const TABLE_LOWER_WRAP: usize = 0;

/// Upper wrap index
pub const TABLE_UPPER_WRAP: usize = TABLE_BASE_INDEX + TABLE_NOMINAL_SIZE;

// ============================================================================
// Table Structures
// ============================================================================

/// 1D table for hue-dependent values (e.g., reach M).
#[derive(Debug, Clone)]
pub struct Table1D {
    /// Table data
    pub data: Vec<f32>,
}

impl Table1D {
    /// Create new table with zeros.
    pub fn new() -> Self {
        Self {
            data: vec![0.0; TABLE_SIZE],
        }
    }

    /// Get hue position in uniform table.
    #[inline]
    pub fn hue_position(&self, hue: f32) -> usize {
        (hue as usize) % TABLE_NOMINAL_SIZE
    }

    /// Get nominal position (with base offset).
    #[inline]
    pub fn nominal_position(&self, hue: f32) -> usize {
        TABLE_BASE_INDEX + self.hue_position(hue)
    }

    /// Interpolate value at hue.
    pub fn lookup(&self, hue: f32) -> f32 {
        let base = self.hue_position(hue);
        let t = hue - base as f32;
        let i_lo = base + TABLE_BASE_INDEX;
        let i_hi = i_lo + 1;
        
        lerp(self.data[i_lo], self.data[i_hi], t)
    }

    /// Set wrap entries for continuity.
    pub fn set_wrap(&mut self) {
        self.data[TABLE_LOWER_WRAP] = self.data[TABLE_NOMINAL_SIZE];
        self.data[TABLE_UPPER_WRAP] = self.data[TABLE_BASE_INDEX];
    }
}

impl Default for Table1D {
    fn default() -> Self {
        Self::new()
    }
}

/// 3D table for JMh cusp values.
#[derive(Debug, Clone)]
pub struct Table3D {
    /// Table data (J, M, h per entry)
    pub data: Vec<F3>,
}

impl Table3D {
    /// Create new table with zeros.
    pub fn new() -> Self {
        Self {
            data: vec![[0.0, 0.0, 0.0]; TABLE_SIZE],
        }
    }

    /// Interpolate value at hue.
    pub fn lookup(&self, hue: f32) -> F3 {
        let base = (hue as usize) % TABLE_NOMINAL_SIZE;
        let t = hue - base as f32;
        let i_lo = base + TABLE_BASE_INDEX;
        let i_hi = i_lo + 1;
        
        lerp_f3(&self.data[i_lo], &self.data[i_hi], t)
    }

    /// Set wrap entries for continuity.
    pub fn set_wrap(&mut self) {
        self.data[TABLE_LOWER_WRAP] = self.data[TABLE_NOMINAL_SIZE];
        self.data[TABLE_UPPER_WRAP] = self.data[TABLE_BASE_INDEX];
        
        // Adjust hue for wrap
        self.data[TABLE_LOWER_WRAP][2] -= HUE_LIMIT;
        self.data[TABLE_UPPER_WRAP][2] += HUE_LIMIT;
    }
}

impl Default for Table3D {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Table Building Functions
// ============================================================================

/// Build limiting gamut cusp corner tables.
///
/// Calculates RGB and JMh for the 6 cusp corners (R, Y, G, C, B, M)
/// plus 2 wrap entries for interpolation.
pub fn build_cusp_corners(
    params: &JMhParams,
    peak_luminance: f32,
) -> (Vec<F3>, Vec<F3>) {
    let mut rgb_corners = vec![[0.0f32; 3]; TOTAL_CORNER_COUNT];
    let mut jmh_corners = vec![[0.0f32; 3]; TOTAL_CORNER_COUNT];
    
    // Temporary storage for sorting
    let mut temp_rgb = vec![[0.0f32; 3]; CUSP_CORNER_COUNT];
    let mut temp_jmh = vec![[0.0f32; 3]; CUSP_CORNER_COUNT];
    
    // Calculate corners
    let scale = peak_luminance / REFERENCE_LUMINANCE;
    let mut min_index = 0;
    
    for i in 0..CUSP_CORNER_COUNT {
        let corner_rgb = generate_cusp_corner(i);
        temp_rgb[i] = mult_f_f3(scale, &corner_rgb);
        temp_jmh[i] = rgb_to_jmh(&temp_rgb[i], params);
        
        // Track minimum hue for rotation
        if temp_jmh[i][2] < temp_jmh[min_index][2] {
            min_index = i;
        }
    }
    
    // Rotate entries so minimum hue is at index 1
    for i in 0..CUSP_CORNER_COUNT {
        let src = (i + min_index) % CUSP_CORNER_COUNT;
        rgb_corners[i + 1] = temp_rgb[src];
        jmh_corners[i + 1] = temp_jmh[src];
    }
    
    // Copy wrap entries
    rgb_corners[0] = rgb_corners[CUSP_CORNER_COUNT];
    rgb_corners[CUSP_CORNER_COUNT + 1] = rgb_corners[1];
    jmh_corners[0] = jmh_corners[CUSP_CORNER_COUNT];
    jmh_corners[CUSP_CORNER_COUNT + 1] = jmh_corners[1];
    
    // Adjust hue wrap
    jmh_corners[0][2] -= HUE_LIMIT;
    jmh_corners[CUSP_CORNER_COUNT + 1][2] += HUE_LIMIT;
    
    (rgb_corners, jmh_corners)
}

/// Build reach M table (maximum M at each hue that maps to limit J).
pub fn build_reach_m_table(
    params: &JMhParams,
    limit_j: f32,
    max_source: f32,
) -> Table1D {
    let mut table = Table1D::new();
    
    let limit_a = (limit_j / J_SCALE).powf(1.0 / params.cz);
    
    // For each cusp corner, find the scaling that gives limit_j
    let mut temp_jmh = vec![[0.0f32; 3]; CUSP_CORNER_COUNT];
    let mut min_index = 0;
    
    for i in 0..CUSP_CORNER_COUNT {
        let rgb_vector = generate_cusp_corner(i);
        
        // Binary search for the scale that gives limit_a
        let mut lower = 0.0;
        let mut upper = max_source;
        
        while (upper - lower) > REACH_CUSP_TOLERANCE {
            let test = midpoint(lower, upper);
            let test_rgb = mult_f_f3(test, &rgb_vector);
            let a = rgb_to_aab(&test_rgb, params)[0];
            
            if a < limit_a {
                lower = test;
            } else {
                upper = test;
            }
        }
        
        let final_rgb = mult_f_f3(upper, &rgb_vector);
        temp_jmh[i] = rgb_to_jmh(&final_rgb, params);
        
        if temp_jmh[i][2] < temp_jmh[min_index][2] {
            min_index = i;
        }
    }
    
    // Rotate to put minimum hue first
    let mut sorted_jmh = vec![[0.0f32; 3]; CUSP_CORNER_COUNT];
    for i in 0..CUSP_CORNER_COUNT {
        sorted_jmh[i] = temp_jmh[(i + min_index) % CUSP_CORNER_COUNT];
    }
    
    // Interpolate to fill 360-degree table
    for deg in 0..TABLE_NOMINAL_SIZE {
        let hue = deg as f32;
        
        // Find bracketing corners
        let mut upper_idx = 0;
        for i in 0..CUSP_CORNER_COUNT {
            if sorted_jmh[i][2] > hue || (i == 0 && sorted_jmh[i][2] > hue - HUE_LIMIT) {
                upper_idx = i;
                break;
            }
            upper_idx = i + 1;
        }
        
        let lower_idx = if upper_idx == 0 { CUSP_CORNER_COUNT - 1 } else { upper_idx - 1 };
        
        // Handle wrap
        let lower_hue = if upper_idx == 0 {
            sorted_jmh[lower_idx][2] - HUE_LIMIT
        } else {
            sorted_jmh[lower_idx][2]
        };
        let upper_hue = if upper_idx >= CUSP_CORNER_COUNT {
            sorted_jmh[0][2] + HUE_LIMIT
        } else {
            sorted_jmh[upper_idx % CUSP_CORNER_COUNT][2]
        };
        
        let t = if (upper_hue - lower_hue).abs() > 1e-6 {
            (hue - lower_hue) / (upper_hue - lower_hue)
        } else {
            0.0
        };
        
        let lower_m = sorted_jmh[lower_idx][1];
        let upper_m = sorted_jmh[upper_idx % CUSP_CORNER_COUNT][1];
        
        table.data[deg + TABLE_BASE_INDEX] = lerp(lower_m, upper_m, t.clamp(0.0, 1.0));
    }
    
    table.set_wrap();
    table
}

/// Build gamut cusp table (JMh of cusp at each hue degree).
pub fn build_gamut_cusp_table(
    rgb_corners: &[F3],
    jmh_corners: &[F3],
    params: &JMhParams,
) -> Table3D {
    let mut table = Table3D::new();
    
    for deg in 0..TABLE_NOMINAL_SIZE {
        let hue = deg as f32;
        
        // Find the cusp for this hue via binary search
        let jm = find_cusp_jm_for_hue(hue, rgb_corners, jmh_corners, params);
        
        table.data[deg + TABLE_BASE_INDEX] = [jm[0], jm[1], hue];
    }
    
    table.set_wrap();
    table
}

/// Find cusp JM for a specific hue.
fn find_cusp_jm_for_hue(
    hue: f32,
    rgb_corners: &[F3],
    jmh_corners: &[F3],
    params: &JMhParams,
) -> F2 {
    // Find bracketing corners
    let mut upper_corner = 1;
    for i in 1..TOTAL_CORNER_COUNT {
        if jmh_corners[i][2] > hue {
            upper_corner = i;
            break;
        }
    }
    let lower_corner = upper_corner - 1;
    
    // Exact match check
    if (jmh_corners[lower_corner][2] - hue).abs() < 1e-6 {
        return [jmh_corners[lower_corner][0], jmh_corners[lower_corner][1]];
    }
    
    // Binary search between corners
    let cusp_lower = &rgb_corners[lower_corner];
    let cusp_upper = &rgb_corners[upper_corner];
    
    let mut lower_t = 0.0f32;
    let mut upper_t = 1.0f32;
    
    while (upper_t - lower_t) > DISPLAY_CUSP_TOLERANCE {
        let sample_t = midpoint(lower_t, upper_t);
        let sample = lerp_f3(cusp_lower, cusp_upper, sample_t);
        let jmh = rgb_to_jmh(&sample, params);
        
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
    let jmh = rgb_to_jmh(&sample, params);
    
    [jmh[0], jmh[1]]
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
    fn test_table1d_lookup() {
        let mut table = Table1D::new();
        
        // Fill with linear ramp
        for i in 0..TABLE_NOMINAL_SIZE {
            table.data[i + TABLE_BASE_INDEX] = i as f32;
        }
        table.set_wrap();
        
        // Test exact values
        assert!((table.lookup(0.0) - 0.0).abs() < 0.01);
        assert!((table.lookup(180.0) - 180.0).abs() < 0.01);
        
        // Test interpolation
        let v = table.lookup(0.5);
        assert!(v > 0.0 && v < 1.0);
    }

    #[test]
    fn test_build_cusp_corners() {
        let params = JMhParams::new(&srgb_to_xyz());
        let (rgb, jmh) = build_cusp_corners(&params, 100.0);
        
        assert_eq!(rgb.len(), TOTAL_CORNER_COUNT);
        assert_eq!(jmh.len(), TOTAL_CORNER_COUNT);
        
        // All corners should have positive J and M
        for i in 1..=CUSP_CORNER_COUNT {
            assert!(jmh[i][0] > 0.0, "J should be positive at corner {}", i);
            assert!(jmh[i][1] > 0.0, "M should be positive at corner {}", i);
        }
    }

    #[test]
    fn test_reach_m_table() {
        let params = JMhParams::new(&srgb_to_xyz());
        let table = build_reach_m_table(&params, 100.0, 10000.0);
        
        // All values should be positive
        for i in TABLE_BASE_INDEX..TABLE_UPPER_WRAP {
            assert!(table.data[i] >= 0.0, "Reach M should be non-negative at {}", i);
        }
    }
}
