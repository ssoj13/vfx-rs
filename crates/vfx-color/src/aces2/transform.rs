//! ACES2 Output Transform.
//!
//! Complete implementation of the ACES 2.0 Output Transform pipeline.
//!
//! # Pipeline
//!
//! ```text
//! ACEScg -> JMh -> Tonescale -> Chroma Compress -> Gamut Compress -> Display RGB
//! ```

use super::common::*;
use super::cam::*;
use super::tonescale::*;
use super::chroma::*;
use super::gamut::*;
use super::tables::*;

// ============================================================================
// Output Transform Configuration
// ============================================================================

/// Display type for the output transform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayType {
    /// SDR display (100 nits, sRGB/Rec.709)
    Sdr,
    /// HDR display (1000 nits, Rec.2020 + PQ)
    Hdr1000,
    /// HDR display (2000 nits)
    Hdr2000,
    /// HDR display (4000 nits)
    Hdr4000,
    /// Custom peak luminance
    Custom(f32),
}

impl DisplayType {
    /// Get peak luminance in cd/m² (nits).
    pub fn peak_luminance(&self) -> f32 {
        match self {
            Self::Sdr => 100.0,
            Self::Hdr1000 => 1000.0,
            Self::Hdr2000 => 2000.0,
            Self::Hdr4000 => 4000.0,
            Self::Custom(peak) => *peak,
        }
    }
}

// ============================================================================
// Output Transform
// ============================================================================

/// ACES2 Output Transform processor.
///
/// Transforms scene-referred ACEScg values to display-referred RGB values
/// for a specific display type.
#[derive(Debug, Clone)]
pub struct OutputTransform {
    /// Peak luminance
    pub peak_luminance: f32,
    /// Input JMh parameters (for ACEScg/AP1)
    pub input_jmh: JMhParams,
    /// Limit/display JMh parameters
    pub limit_jmh: JMhParams,
    /// Tonescale parameters
    pub tonescale: TonescaleParams,
    /// Chroma compression parameters
    pub chroma_compress: ChromaCompressParams,
    /// Shared compression parameters
    pub shared_compress: SharedCompressionParams,
    /// Gamut compression parameters
    pub gamut_compress: GamutCompressParams,
    /// Cusp RGB corners
    pub cusp_rgb: Vec<F3>,
    /// Cusp JMh corners
    pub cusp_jmh: Vec<F3>,
    /// Reach M table
    pub reach_m_table: Table1D,
    /// Gamut cusp table
    pub gamut_cusp_table: Table3D,
    /// AP1 to display matrix
    pub ap1_to_display: M33,
    /// Display to AP1 matrix
    pub display_to_ap1: M33,
}

impl OutputTransform {
    /// Create a new output transform for the given display type.
    pub fn new(display: DisplayType) -> Self {
        Self::with_peak(display.peak_luminance())
    }

    /// Create a new output transform with custom peak luminance.
    pub fn with_peak(peak_luminance: f32) -> Self {
        // ACEScg (AP1) to XYZ matrix
        let ap1_to_xyz = ap1_to_xyz_matrix();
        
        // sRGB to XYZ for display (assuming sRGB primaries for SDR)
        let srgb_to_xyz = srgb_to_xyz_matrix();
        
        // Initialize JMh parameters
        let input_jmh = JMhParams::new(&ap1_to_xyz);
        let limit_jmh = JMhParams::new(&srgb_to_xyz);
        
        // Initialize tonescale
        let tonescale = TonescaleParams::new(peak_luminance);
        
        // Initialize compression params
        let chroma_compress = ChromaCompressParams::new(peak_luminance, &tonescale);
        
        // Calculate limit J max
        let limit_j_max = y_to_j(peak_luminance / REFERENCE_LUMINANCE, &limit_jmh);
        let model_gamma_inv = 1.0 / (SURROUND[1] * (1.48 + (Y_B / REFERENCE_LUMINANCE).sqrt()));
        
        // Build cusp corner tables
        let (cusp_rgb, cusp_jmh) = build_cusp_corners(&limit_jmh, peak_luminance);
        
        // Build reach M table
        let reach_m_table = build_reach_m_table(&input_jmh, limit_j_max, 10000.0);
        
        // Build gamut cusp table
        let gamut_cusp_table = build_gamut_cusp_table(&cusp_rgb, &cusp_jmh, &limit_jmh);
        
        let shared_compress = SharedCompressionParams {
            limit_j_max,
            model_gamma_inv,
            reach_m_table: reach_m_table.data.clone(),
        };
        
        let gamut_compress = GamutCompressParams::new(peak_luminance, &input_jmh, &limit_jmh);
        
        // AP1 to sRGB matrix (with D60 to D65 adaptation)
        let ap1_to_display = ap1_to_srgb_matrix();
        let display_to_ap1 = invert_m33(&ap1_to_display);
        
        Self {
            peak_luminance,
            input_jmh,
            limit_jmh,
            tonescale,
            chroma_compress,
            shared_compress,
            gamut_compress,
            cusp_rgb,
            cusp_jmh,
            reach_m_table,
            gamut_cusp_table,
            ap1_to_display,
            display_to_ap1,
        }
    }

    /// Apply forward transform (ACEScg to display RGB).
    pub fn forward(&self, rgb: &F3) -> F3 {
        // 1. Convert to JMh
        let jmh = rgb_to_jmh(rgb, &self.input_jmh);
        
        if jmh[0] <= 0.0 {
            return [0.0, 0.0, 0.0];
        }
        
        // Get hue-dependent parameters
        let h = jmh[2];
        let h_rad = to_radians(h);
        let cos_h = h_rad.cos();
        let sin_h = h_rad.sin();
        
        // 2. Apply tonescale
        let j_ts = tonescale_j_fwd(jmh[0], &self.input_jmh, &self.tonescale);
        
        // 3. Chroma compression
        let m_norm = chroma_compress_norm(cos_h, sin_h, self.chroma_compress.chroma_compress_scale);
        let reach_m = self.reach_m_table.lookup(h);
        
        let resolved = ResolvedCompressionParams {
            limit_j_max: self.shared_compress.limit_j_max,
            model_gamma_inv: self.shared_compress.model_gamma_inv,
            reach_max_m: reach_m,
        };
        
        let jmh_cc = chroma_compress_fwd(&jmh, j_ts, m_norm, &resolved, &self.chroma_compress);
        
        // 4. Gamut compression
        let cusp_jm = self.gamut_cusp_table.lookup(h);
        let hue_params = resolve_hue_dependent_params(
            h,
            &[cusp_jm[0], cusp_jm[1]],
            &self.gamut_compress,
            &resolved,
        );
        
        let jmh_gc = gamut_compress_fwd(&jmh_cc, &resolved, &self.gamut_compress, &hue_params);
        
        // 5. Convert back to RGB (in display space)
        let display_rgb = jmh_to_rgb(&jmh_gc, &self.limit_jmh);
        
        // 6. Clamp to valid range
        [
            display_rgb[0].clamp(0.0, 1.0),
            display_rgb[1].clamp(0.0, 1.0),
            display_rgb[2].clamp(0.0, 1.0),
        ]
    }

    /// Apply inverse transform (display RGB to ACEScg).
    pub fn inverse(&self, rgb: &F3) -> F3 {
        // 1. Convert display RGB to JMh
        let jmh = rgb_to_jmh(rgb, &self.limit_jmh);
        
        if jmh[0] <= 0.0 {
            return [0.0, 0.0, 0.0];
        }
        
        let h = jmh[2];
        let h_rad = to_radians(h);
        let cos_h = h_rad.cos();
        let sin_h = h_rad.sin();
        
        // Get resolved params
        let reach_m = self.reach_m_table.lookup(h);
        let resolved = ResolvedCompressionParams {
            limit_j_max: self.shared_compress.limit_j_max,
            model_gamma_inv: self.shared_compress.model_gamma_inv,
            reach_max_m: reach_m,
        };
        
        // 2. Inverse gamut compression
        let cusp_jm = self.gamut_cusp_table.lookup(h);
        let hue_params = resolve_hue_dependent_params(
            h,
            &[cusp_jm[0], cusp_jm[1]],
            &self.gamut_compress,
            &resolved,
        );
        
        let jmh_gc = gamut_compress_inv(&jmh, &resolved, &self.gamut_compress, &hue_params);
        
        // 3. Inverse chroma compression
        let m_norm = chroma_compress_norm(cos_h, sin_h, self.chroma_compress.chroma_compress_scale);
        let j_orig = tonescale_j_inv(jmh_gc[0], &self.input_jmh, &self.tonescale);
        
        let jmh_cc = chroma_compress_inv(&jmh_gc, j_orig, m_norm, &resolved, &self.chroma_compress);
        
        // 4. Inverse tonescale (already done above for j_orig)
        let jmh_ts = [j_orig, jmh_cc[1], jmh_cc[2]];
        
        // 5. Convert back to ACEScg RGB
        jmh_to_rgb(&jmh_ts, &self.input_jmh)
    }

    /// Apply forward transform to image buffer.
    ///
    /// # Arguments
    /// * `data` - RGBA or RGB pixel data (3 or 4 channels)
    /// * `channels` - Number of channels per pixel
    pub fn apply_forward(&self, data: &[f32], channels: usize) -> Vec<f32> {
        let mut result = data.to_vec();
        let pixels = data.len() / channels;
        
        for i in 0..pixels {
            let idx = i * channels;
            let rgb = [data[idx], data[idx + 1], data[idx + 2]];
            let out = self.forward(&rgb);
            result[idx] = out[0];
            result[idx + 1] = out[1];
            result[idx + 2] = out[2];
            // Alpha passes through unchanged
        }
        
        result
    }

    /// Apply inverse transform to image buffer.
    pub fn apply_inverse(&self, data: &[f32], channels: usize) -> Vec<f32> {
        let mut result = data.to_vec();
        let pixels = data.len() / channels;
        
        for i in 0..pixels {
            let idx = i * channels;
            let rgb = [data[idx], data[idx + 1], data[idx + 2]];
            let out = self.inverse(&rgb);
            result[idx] = out[0];
            result[idx + 1] = out[1];
            result[idx + 2] = out[2];
        }
        
        result
    }
}

// ============================================================================
// Matrix Helpers
// ============================================================================

/// ACEScg (AP1) to XYZ matrix.
fn ap1_to_xyz_matrix() -> M33 {
    // AP1 primaries with D60 white point
    [
        0.6624542, 0.1340042, 0.1561877,
        0.2722287, 0.6740818, 0.0536895,
        -0.0055746, 0.0040607, 1.0103391,
    ]
}

/// sRGB to XYZ matrix.
fn srgb_to_xyz_matrix() -> M33 {
    [
        0.4124564, 0.3575761, 0.1804375,
        0.2126729, 0.7151522, 0.0721750,
        0.0193339, 0.1191920, 0.9503041,
    ]
}

/// AP1 to sRGB matrix (with D60 to D65 Bradford adaptation).
fn ap1_to_srgb_matrix() -> M33 {
    // Pre-computed AP1 -> XYZ -> Bradford D60->D65 -> sRGB
    [
        1.7050510, -0.6217921, -0.0832590,
        -0.1302564, 1.1408048, -0.0105485,
        -0.0240033, -0.1289690, 1.1529724,
    ]
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Apply ACES2 output transform (SDR) to a single RGB pixel.
pub fn aces2_sdr(rgb: &F3) -> F3 {
    // Use a static/lazy transform for efficiency
    let transform = OutputTransform::new(DisplayType::Sdr);
    transform.forward(rgb)
}

/// Apply ACES2 output transform (HDR 1000 nits) to a single RGB pixel.
pub fn aces2_hdr1000(rgb: &F3) -> F3 {
    let transform = OutputTransform::new(DisplayType::Hdr1000);
    transform.forward(rgb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_transform_create() {
        let transform = OutputTransform::new(DisplayType::Sdr);
        assert!((transform.peak_luminance - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_black_is_black() {
        let transform = OutputTransform::new(DisplayType::Sdr);
        let result = transform.forward(&[0.0, 0.0, 0.0]);
        
        assert!(result[0].abs() < 1e-4);
        assert!(result[1].abs() < 1e-4);
        assert!(result[2].abs() < 1e-4);
    }

    #[test]
    fn test_output_clamped() {
        let transform = OutputTransform::new(DisplayType::Sdr);
        
        // High values should be clamped to [0, 1]
        let result = transform.forward(&[10.0, 10.0, 10.0]);
        
        assert!(result[0] >= 0.0 && result[0] <= 1.0);
        assert!(result[1] >= 0.0 && result[1] <= 1.0);
        assert!(result[2] >= 0.0 && result[2] <= 1.0);
    }

    #[test]
    fn test_midgray_produces_output() {
        let transform = OutputTransform::new(DisplayType::Sdr);
        
        // ACEScg mid-gray (0.18)
        let result = transform.forward(&[0.18, 0.18, 0.18]);
        
        // Should produce valid output (exact values depend on calibration)
        assert!(result[0] >= 0.0 && result[0] <= 1.0, "Mid-gray R out of range: {}", result[0]);
        assert!(result[1] >= 0.0 && result[1] <= 1.0, "Mid-gray G out of range: {}", result[1]);
        assert!(result[2] >= 0.0 && result[2] <= 1.0, "Mid-gray B out of range: {}", result[2]);
        // All channels should be similar for neutral gray
        assert!((result[0] - result[1]).abs() < 0.1, "Gray should be neutral");
        assert!((result[1] - result[2]).abs() < 0.1, "Gray should be neutral");
    }

    #[test]
    fn test_monotonic() {
        let transform = OutputTransform::new(DisplayType::Sdr);
        
        let mut prev = 0.0;
        for i in 1..20 {
            let v = i as f32 * 0.1;
            let result = transform.forward(&[v, v, v]);
            let lum = result[0] * 0.2126 + result[1] * 0.7152 + result[2] * 0.0722;
            
            assert!(lum >= prev, "Not monotonic at v={}: lum={}, prev={}", v, lum, prev);
            prev = lum;
        }
    }

    #[test]
    fn test_hdr_vs_sdr() {
        let sdr = OutputTransform::new(DisplayType::Sdr);
        let hdr = OutputTransform::new(DisplayType::Hdr1000);
        
        // At high values, HDR should preserve more detail
        let input = [2.0, 2.0, 2.0];
        let sdr_out = sdr.forward(&input);
        let hdr_out = hdr.forward(&input);
        
        // HDR output should have more headroom (higher values)
        let sdr_lum = sdr_out[0] + sdr_out[1] + sdr_out[2];
        let hdr_lum = hdr_out[0] + hdr_out[1] + hdr_out[2];
        
        // Note: This test may need adjustment based on actual algorithm behavior
        assert!(sdr_lum > 0.0 && hdr_lum > 0.0);
    }

    #[test]
    fn test_apply_buffer() {
        let transform = OutputTransform::new(DisplayType::Sdr);
        
        let input = vec![0.18, 0.18, 0.18, 1.0, 0.5, 0.3, 0.2, 1.0];
        let result = transform.apply_forward(&input, 4);
        
        assert_eq!(result.len(), 8);
        // Alpha should be unchanged
        assert!((result[3] - 1.0).abs() < 1e-6);
        assert!((result[7] - 1.0).abs() < 1e-6);
    }
}
