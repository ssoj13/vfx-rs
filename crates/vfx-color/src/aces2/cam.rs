//! CAM16 Color Appearance Model for ACES2.
//!
//! Converts between RGB and JMh (Lightness, Colorfulness, Hue) perceptual space.

use super::common::*;

// ============================================================================
// JMh Parameters
// ============================================================================

/// Parameters for JMh color space conversion.
#[derive(Debug, Clone)]
pub struct JMhParams {
    /// RGB to CAM16 cone response matrix (with chromatic adaptation)
    pub matrix_rgb_to_cam16_c: M33,
    /// CAM16 cone response to RGB matrix (inverse)
    pub matrix_cam16_c_to_rgb: M33,
    /// Cone response to Aab matrix
    pub matrix_cone_to_aab: M33,
    /// Aab to cone response matrix (inverse)
    pub matrix_aab_to_cone: M33,
    /// F_L normalized
    pub f_l_n: f32,
    /// c * z nonlinearity
    pub cz: f32,
    /// 1/cz
    pub inv_cz: f32,
    /// Achromatic response at white
    pub a_w_j: f32,
    /// 1/a_w_j
    pub inv_a_w_j: f32,
}

impl JMhParams {
    /// Initialize JMh parameters for given primaries.
    ///
    /// # Arguments
    /// * `rgb_to_xyz` - 3x3 matrix converting RGB to XYZ
    pub fn new(rgb_to_xyz: &M33) -> Self {
        // CAM16 primaries to XYZ matrix
        let cam16_to_xyz = cam16_rgb_to_xyz();
        let xyz_to_cam16 = invert_m33(&cam16_to_xyz);
        
        // Reference white in XYZ
        let xyz_w = mult_f3_m33(&f3_from_f(REFERENCE_LUMINANCE), rgb_to_xyz);
        let y_w = xyz_w[1];
        
        // Reference white in CAM16 LMS
        let rgb_w = mult_f3_m33(&xyz_w, &xyz_to_cam16);
        
        // Viewing condition parameters
        let k = 1.0 / (5.0 * L_A + 1.0);
        let k4 = k * k * k * k;
        let f_l = 0.2 * k4 * (5.0 * L_A) + 0.1 * (1.0 - k4).powi(2) * (5.0 * L_A).powf(1.0 / 3.0);
        
        let f_l_n = f_l / REFERENCE_LUMINANCE;
        let cz = model_gamma();
        let inv_cz = 1.0 / cz;
        
        // Degree of adaptation (D_RGB)
        let d_rgb = [
            f_l_n * y_w / rgb_w[0],
            f_l_n * y_w / rgb_w[1],
            f_l_n * y_w / rgb_w[2],
        ];
        
        // Adapted white cone responses
        let rgb_wc = [
            d_rgb[0] * rgb_w[0],
            d_rgb[1] * rgb_w[1],
            d_rgb[2] * rgb_w[2],
        ];
        
        // Post-adaptation compression of white
        let rgb_aw = [
            post_adaptation_compress_fwd(rgb_wc[0]),
            post_adaptation_compress_fwd(rgb_wc[1]),
            post_adaptation_compress_fwd(rgb_wc[2]),
        ];
        
        // Base cone response to Aab matrix
        let base_cone_to_aab: M33 = [
            2.0, 1.0, 1.0 / 20.0,
            1.0, -12.0 / 11.0, 1.0 / 11.0,
            1.0 / 9.0, 1.0 / 9.0, -2.0 / 9.0,
        ];
        
        // Scale by CAM_NL_SCALE
        let cone_to_aab = mult_m33_m33(
            &scale_m33(&IDENTITY_M33, &f3_from_f(CAM_NL_SCALE)),
            &base_cone_to_aab,
        );
        
        // Achromatic response at white
        let a_w = cone_to_aab[0] * rgb_aw[0] + cone_to_aab[1] * rgb_aw[1] + cone_to_aab[2] * rgb_aw[2];
        let a_w_j = post_adaptation_compress_fwd_inner(f_l);
        let inv_a_w_j = 1.0 / a_w_j;
        
        // Build RGB to CAM16 matrix with chromatic adaptation
        let rgb_to_cam16 = mult_m33_m33(
            &rgb_to_cam16_from_primaries(rgb_to_xyz),
            &scale_m33(&IDENTITY_M33, &f3_from_f(REFERENCE_LUMINANCE)),
        );
        let matrix_rgb_to_cam16_c = mult_m33_m33(
            &scale_m33(&IDENTITY_M33, &d_rgb),
            &rgb_to_cam16,
        );
        let matrix_cam16_c_to_rgb = invert_m33(&matrix_rgb_to_cam16_c);
        
        // Normalize cone response to Aab matrix
        let nc = 43.0 * SURROUND[2];
        let matrix_cone_to_aab: M33 = [
            cone_to_aab[0] / a_w, cone_to_aab[1] / a_w, cone_to_aab[2] / a_w,
            cone_to_aab[3] * nc, cone_to_aab[4] * nc, cone_to_aab[5] * nc,
            cone_to_aab[6] * nc, cone_to_aab[7] * nc, cone_to_aab[8] * nc,
        ];
        let matrix_aab_to_cone = invert_m33(&matrix_cone_to_aab);
        
        Self {
            matrix_rgb_to_cam16_c,
            matrix_cam16_c_to_rgb,
            matrix_cone_to_aab,
            matrix_aab_to_cone,
            f_l_n,
            cz,
            inv_cz,
            a_w_j,
            inv_a_w_j,
        }
    }
}

// ============================================================================
// CAM16 Helper Functions
// ============================================================================

/// Model gamma (c * z nonlinearity)
#[inline]
fn model_gamma() -> f32 {
    SURROUND[1] * (1.48 + (Y_B / REFERENCE_LUMINANCE).sqrt())
}

/// CAM16 RGB to XYZ matrix
fn cam16_rgb_to_xyz() -> M33 {
    // CAM16 primaries (unusual "imaginary" primaries for the color model)
    // These create a well-conditioned matrix for the appearance model
    primaries_to_xyz_matrix(
        CAM16_RED_X, CAM16_RED_Y,
        CAM16_GREEN_X, CAM16_GREEN_Y,
        CAM16_BLUE_X, CAM16_BLUE_Y,
        CAM16_WHITE_X, CAM16_WHITE_Y,
    )
}

/// RGB to CAM16 matrix from primaries
fn rgb_to_cam16_from_primaries(rgb_to_xyz: &M33) -> M33 {
    let cam16_to_xyz = cam16_rgb_to_xyz();
    let xyz_to_cam16 = invert_m33(&cam16_to_xyz);
    mult_m33_m33(&xyz_to_cam16, rgb_to_xyz)
}

/// Build XYZ matrix from chromaticity coordinates
fn primaries_to_xyz_matrix(
    rx: f32, ry: f32,
    gx: f32, gy: f32,
    bx: f32, by: f32,
    wx: f32, wy: f32,
) -> M33 {
    // Calculate XYZ for each primary
    let rz = 1.0 - rx - ry;
    let gz = 1.0 - gx - gy;
    let bz = 1.0 - bx - by;
    let wz = 1.0 - wx - wy;
    
    // White point XYZ (normalized to Y=1)
    let w_y = 1.0;
    let w_x = wx / wy * w_y;
    let w_z = wz / wy * w_y;
    
    // Primaries matrix (before scaling)
    let m: M33 = [
        rx, gx, bx,
        ry, gy, by,
        rz, gz, bz,
    ];
    
    // Solve for scaling factors S such that M * S = W
    let m_inv = invert_m33(&m);
    let s = mult_f3_m33(&[w_x, w_y, w_z], &m_inv);
    
    // Scale columns by S
    [
        rx * s[0], gx * s[1], bx * s[2],
        ry * s[0], gy * s[1], by * s[2],
        rz * s[0], gz * s[1], bz * s[2],
    ]
}

// ============================================================================
// Post-Adaptation Compression
// ============================================================================

/// Post-adaptation cone response compression (forward, inner)
#[inline]
fn post_adaptation_compress_fwd_inner(rc: f32) -> f32 {
    let f_l_y = rc.powf(0.42);
    f_l_y / (CAM_NL_OFFSET + f_l_y)
}

/// Post-adaptation cone response compression (inverse, inner)
#[inline]
fn post_adaptation_compress_inv_inner(ra: f32) -> f32 {
    let ra_lim = ra.min(0.99);
    let f_l_y = (CAM_NL_OFFSET * ra_lim) / (1.0 - ra_lim);
    f_l_y.powf(1.0 / 0.42)
}

/// Post-adaptation cone response compression (forward)
#[inline]
pub fn post_adaptation_compress_fwd(v: f32) -> f32 {
    let ra = post_adaptation_compress_fwd_inner(v.abs());
    ra.copysign(v)
}

/// Post-adaptation cone response compression (inverse)
#[inline]
pub fn post_adaptation_compress_inv(v: f32) -> f32 {
    let rc = post_adaptation_compress_inv_inner(v.abs());
    rc.copysign(v)
}

// ============================================================================
// J (Lightness) Conversions
// ============================================================================

/// Achromatic response to J
#[inline]
fn achromatic_to_j(a: f32, cz: f32) -> f32 {
    J_SCALE * a.powf(cz)
}

/// J to achromatic response
#[inline]
fn j_to_achromatic(j: f32, inv_cz: f32) -> f32 {
    (j / J_SCALE).powf(inv_cz)
}

/// Achromatic to Y (luminance)
#[inline]
fn a_to_y(a: f32, p: &JMhParams) -> f32 {
    let ra = p.a_w_j * a;
    post_adaptation_compress_inv_inner(ra) / p.f_l_n
}

/// J to Y (luminance)
#[inline]
pub fn j_to_y(j: f32, p: &JMhParams) -> f32 {
    a_to_y(j_to_achromatic(j.abs(), p.inv_cz), p)
}

/// Y (luminance) to J
#[inline]
pub fn y_to_j(y: f32, p: &JMhParams) -> f32 {
    let ra = post_adaptation_compress_fwd_inner(y.abs() * p.f_l_n);
    let j = achromatic_to_j(ra * p.inv_a_w_j, p.cz);
    j.copysign(y)
}

// ============================================================================
// RGB <-> JMh Conversions
// ============================================================================

/// RGB to Aab (achromatic, a, b)
pub fn rgb_to_aab(rgb: &F3, p: &JMhParams) -> F3 {
    // Apply chromatic adaptation
    let rgb_m = mult_f3_m33(rgb, &p.matrix_rgb_to_cam16_c);
    
    // Post-adaptation compression
    let rgb_a = [
        post_adaptation_compress_fwd(rgb_m[0]),
        post_adaptation_compress_fwd(rgb_m[1]),
        post_adaptation_compress_fwd(rgb_m[2]),
    ];
    
    // Convert to Aab
    mult_f3_m33(&rgb_a, &p.matrix_cone_to_aab)
}

/// Aab to JMh
pub fn aab_to_jmh(aab: &F3, p: &JMhParams) -> F3 {
    if aab[0] <= 0.0 {
        return [0.0, 0.0, 0.0];
    }
    
    let j = achromatic_to_j(aab[0], p.cz);
    let m = (aab[1] * aab[1] + aab[2] * aab[2]).sqrt();
    let h_rad = aab[2].atan2(aab[1]);
    let h = from_radians(h_rad);
    
    [j, m, h]
}

/// RGB to JMh
pub fn rgb_to_jmh(rgb: &F3, p: &JMhParams) -> F3 {
    let aab = rgb_to_aab(rgb, p);
    aab_to_jmh(&aab, p)
}

/// JMh to Aab (with precomputed trig)
pub fn jmh_to_aab_trig(jmh: &F3, cos_h: f32, sin_h: f32, p: &JMhParams) -> F3 {
    let j = jmh[0];
    let m = jmh[1];
    
    let a = j_to_achromatic(j, p.inv_cz);
    let aa = m * cos_h;
    let b = m * sin_h;
    
    [a, aa, b]
}

/// JMh to Aab
pub fn jmh_to_aab(jmh: &F3, p: &JMhParams) -> F3 {
    let h_rad = to_radians(jmh[2]);
    jmh_to_aab_trig(jmh, h_rad.cos(), h_rad.sin(), p)
}

/// Aab to RGB
pub fn aab_to_rgb(aab: &F3, p: &JMhParams) -> F3 {
    // Convert Aab to cone response
    let rgb_a = mult_f3_m33(aab, &p.matrix_aab_to_cone);
    
    // Inverse post-adaptation compression
    let rgb_m = [
        post_adaptation_compress_inv(rgb_a[0]),
        post_adaptation_compress_inv(rgb_a[1]),
        post_adaptation_compress_inv(rgb_a[2]),
    ];
    
    // Remove chromatic adaptation
    mult_f3_m33(&rgb_m, &p.matrix_cam16_c_to_rgb)
}

/// JMh to RGB
pub fn jmh_to_rgb(jmh: &F3, p: &JMhParams) -> F3 {
    let aab = jmh_to_aab(jmh, p);
    aab_to_rgb(&aab, p)
}

#[cfg(test)]
mod tests {
    use super::*;

    // sRGB/Rec.709 to XYZ matrix
    fn srgb_to_xyz() -> M33 {
        [
            0.4124564, 0.3575761, 0.1804375,
            0.2126729, 0.7151522, 0.0721750,
            0.0193339, 0.1191920, 0.9503041,
        ]
    }

    #[test]
    fn test_jmh_params_init() {
        let p = JMhParams::new(&srgb_to_xyz());
        assert!(p.cz > 0.0);
        assert!(p.f_l_n > 0.0);
        assert!(p.a_w_j > 0.0);
    }

    #[test]
    fn test_rgb_jmh_roundtrip() {
        let p = JMhParams::new(&srgb_to_xyz());
        
        let test_values = [
            [0.5, 0.3, 0.2],
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        
        for rgb in &test_values {
            let jmh = rgb_to_jmh(rgb, &p);
            let rgb2 = jmh_to_rgb(&jmh, &p);
            
            for i in 0..3 {
                assert!(
                    (rgb[i] - rgb2[i]).abs() < 1e-4,
                    "RGB roundtrip failed for {:?}: got {:?}", rgb, rgb2
                );
            }
        }
    }

    #[test]
    fn test_j_y_roundtrip() {
        let p = JMhParams::new(&srgb_to_xyz());
        
        for y in [0.01, 0.1, 0.5, 1.0, 10.0, 100.0] {
            let j = y_to_j(y, &p);
            let y2 = j_to_y(j, &p);
            assert!(
                (y - y2).abs() < 1e-4,
                "Y roundtrip failed for {}: got {}", y, y2
            );
        }
    }

    #[test]
    fn test_black_is_black() {
        let p = JMhParams::new(&srgb_to_xyz());
        let jmh = rgb_to_jmh(&[0.0, 0.0, 0.0], &p);
        assert!(jmh[0].abs() < 1e-6, "J for black should be 0");
        assert!(jmh[1].abs() < 1e-6, "M for black should be 0");
    }

    #[test]
    fn test_white_has_zero_chroma() {
        let p = JMhParams::new(&srgb_to_xyz());
        let jmh = rgb_to_jmh(&[1.0, 1.0, 1.0], &p);
        assert!(jmh[0] > 0.0, "J for white should be positive");
        assert!(jmh[1] < 0.1, "M for white should be near zero, got {}", jmh[1]);
    }
}
