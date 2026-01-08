//! FixedFunction operations - specialized color space conversions.
//!
//! Reference: OCIO FixedFunctionOpCPU.cpp
//!
//! This module provides fixed-function color operations including:
//! - CIE chromaticity conversions (XYZ <-> xyY, uvY)
//! - ACES Red Modifier (1.0) - hue-based red channel correction
//! - ACES Glow (1.0) - glow effect based on saturation
//! - REC.2100 Surround - HDR surround correction

// ============================================================================
// XYZ <-> xyY (CIE 1931 chromaticity)
// ============================================================================

/// Convert CIE XYZ to xyY (1931 chromaticity coordinates).
///
/// - x, y = chromaticity coordinates
/// - Y = luminance (unchanged)
///
/// Formula:
/// - d = X + Y + Z (divisor)
/// - x = X / d
/// - y = Y / d
#[inline]
pub fn xyz_to_xyy(xyz: [f32; 3]) -> [f32; 3] {
    let x_val = xyz[0];
    let y_val = xyz[1];
    let z_val = xyz[2];
    
    let d = x_val + y_val + z_val;
    let d = if d == 0.0 { 0.0 } else { 1.0 / d };
    
    [x_val * d, y_val * d, y_val]  // (x, y, Y)
}

/// Convert xyY to CIE XYZ.
///
/// Formula:
/// - d = 1 / y (divisor)
/// - X = Y * x / y
/// - Z = Y * (1 - x - y) / y
#[inline]
pub fn xyy_to_xyz(xyy: [f32; 3]) -> [f32; 3] {
    let x = xyy[0];
    let y = xyy[1];
    let y_lum = xyy[2];  // Y luminance
    
    let d = if y == 0.0 { 0.0 } else { 1.0 / y };
    let x_val = y_lum * x * d;
    let z_val = y_lum * (1.0 - x - y) * d;
    
    [x_val, y_lum, z_val]
}

/// Apply XYZ to xyY conversion in-place.
#[inline]
pub fn apply_xyz_to_xyy(rgb: &mut [f32; 3]) {
    *rgb = xyz_to_xyy(*rgb);
}

/// Apply xyY to XYZ conversion in-place.
#[inline]
pub fn apply_xyy_to_xyz(rgb: &mut [f32; 3]) {
    *rgb = xyy_to_xyz(*rgb);
}

/// Apply XYZ to xyY to RGBA buffer.
pub fn apply_xyz_to_xyy_rgba(pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let xyz = [chunk[0], chunk[1], chunk[2]];
        let xyy = xyz_to_xyy(xyz);
        chunk[0] = xyy[0];
        chunk[1] = xyy[1];
        chunk[2] = xyy[2];
        // Alpha unchanged
    }
}

/// Apply xyY to XYZ to RGBA buffer.
pub fn apply_xyy_to_xyz_rgba(pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let xyy = [chunk[0], chunk[1], chunk[2]];
        let xyz = xyy_to_xyz(xyy);
        chunk[0] = xyz[0];
        chunk[1] = xyz[1];
        chunk[2] = xyz[2];
        // Alpha unchanged
    }
}

// ============================================================================
// XYZ <-> uvY (CIE 1976 u'v' chromaticity)
// ============================================================================

/// Convert CIE XYZ to u'v'Y (1976 chromaticity coordinates).
///
/// Formula:
/// - d = X + 15*Y + 3*Z
/// - u' = 4*X / d
/// - v' = 9*Y / d
#[inline]
pub fn xyz_to_uvy(xyz: [f32; 3]) -> [f32; 3] {
    let x_val = xyz[0];
    let y_val = xyz[1];
    let z_val = xyz[2];
    
    let d = x_val + 15.0 * y_val + 3.0 * z_val;
    let d = if d == 0.0 { 0.0 } else { 1.0 / d };
    
    let u = 4.0 * x_val * d;
    let v = 9.0 * y_val * d;
    
    [u, v, y_val]  // (u', v', Y)
}

/// Convert u'v'Y to CIE XYZ.
///
/// Formula:
/// - d = 1 / v'
/// - X = (9/4) * Y * u' / v'
/// - Z = (3/4) * Y * (4 - u' - (20/3)*v') / v'
#[inline]
pub fn uvy_to_xyz(uvy: [f32; 3]) -> [f32; 3] {
    let u = uvy[0];
    let v = uvy[1];
    let y_lum = uvy[2];
    
    let d = if v == 0.0 { 0.0 } else { 1.0 / v };
    
    // X = (9/4) * Y * u' / v' = 2.25 * Y * u * d
    let x_val = 2.25 * y_lum * u * d;
    
    // Z = (3/4) * Y * (4 - u' - (20/3)*v') / v'
    // = 0.75 * Y * (4 - u - 6.666... * v) * d
    let z_val = 0.75 * y_lum * (4.0 - u - 6.666666666666667 * v) * d;
    
    [x_val, y_lum, z_val]
}

/// Apply XYZ to uvY conversion in-place.
#[inline]
pub fn apply_xyz_to_uvy(rgb: &mut [f32; 3]) {
    *rgb = xyz_to_uvy(*rgb);
}

/// Apply uvY to XYZ conversion in-place.
#[inline]
pub fn apply_uvy_to_xyz(rgb: &mut [f32; 3]) {
    *rgb = uvy_to_xyz(*rgb);
}

/// Apply XYZ to uvY to RGBA buffer.
pub fn apply_xyz_to_uvy_rgba(pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let xyz = [chunk[0], chunk[1], chunk[2]];
        let uvy = xyz_to_uvy(xyz);
        chunk[0] = uvy[0];
        chunk[1] = uvy[1];
        chunk[2] = uvy[2];
    }
}

/// Apply uvY to XYZ to RGBA buffer.
pub fn apply_uvy_to_xyz_rgba(pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let uvy = [chunk[0], chunk[1], chunk[2]];
        let xyz = uvy_to_xyz(uvy);
        chunk[0] = xyz[0];
        chunk[1] = xyz[1];
        chunk[2] = xyz[2];
    }
}

// ============================================================================
// ACES Red Modifier 1.0
// ============================================================================

/// Constants for ACES Red Modifier 1.0
mod red_mod_10 {
    pub const SCALE: f32 = 0.82;
    pub const ONE_MINUS_SCALE: f32 = 1.0 - SCALE;  // 0.18
    pub const PIVOT: f32 = 0.03;
    /// 4 / (135 * pi/180) = 4 / 2.3562 ≈ 1.6977
    pub const INV_WIDTH: f32 = 1.6976527263135504;
    pub const NOISE_LIMIT: f32 = 1e-2;
}

/// B-spline coefficients for hue weighting (ACES).
const HUE_BSPLINE_M: [[f32; 4]; 4] = [
    [ 0.25,  0.00,  0.00,  0.00],
    [-0.75,  0.75,  0.75,  0.25],
    [ 0.75, -1.50,  0.00,  1.00],
    [-0.25,  0.75, -0.75,  0.25],
];

/// Calculate saturation weight for ACES functions.
///
/// Returns (max - min) / max with noise limiting.
#[inline]
fn calc_sat_weight(red: f32, grn: f32, blu: f32, noise_limit: f32) -> f32 {
    let min_val = red.min(grn.min(blu));
    let max_val = red.max(grn.max(blu));
    
    // Clamp to prevent problems from negative values
    let numerator = max_val.max(1e-10) - min_val.max(1e-10);
    let denominator = max_val.max(noise_limit);
    
    numerator / denominator
}

/// Calculate hue weight for ACES Red Modifier.
///
/// Uses B-spline for smooth window around red hue.
#[inline]
fn calc_hue_weight(red: f32, grn: f32, blu: f32, inv_width: f32) -> f32 {
    // Convert RGB to Yab (luma/chroma representation)
    let a = 2.0 * red - (grn + blu);
    let sqrt3 = 1.7320508075688772_f32;
    let b = sqrt3 * (grn - blu);
    
    let hue = b.atan2(a);
    
    // Determine normalized input coords to B-spline
    let knot_coord = hue * inv_width + 2.0;
    let j = knot_coord as i32;
    
    // Hue is in range of the window, calculate weight
    if j >= 0 && j < 4 {
        let t = knot_coord - j as f32;  // fractional component
        
        // Calculate quadratic B-spline weighting function
        let coefs = &HUE_BSPLINE_M[j as usize];
        coefs[3] + t * (coefs[2] + t * (coefs[1] + t * coefs[0]))
    } else {
        0.0
    }
}

/// ACES Red Modifier 1.0 forward.
///
/// Applies hue-weighted saturation-based red channel modification.
#[inline]
pub fn aces_red_mod_10_fwd(rgb: &mut [f32; 3]) {
    use red_mod_10::*;
    
    let f_h = calc_hue_weight(rgb[0], rgb[1], rgb[2], INV_WIDTH);
    
    if f_h > 0.0 {
        let f_s = calc_sat_weight(rgb[0], rgb[1], rgb[2], NOISE_LIMIT);
        
        // red = red + f_H * f_S * (pivot - red) * (1 - scale)
        rgb[0] = rgb[0] + f_h * f_s * (PIVOT - rgb[0]) * ONE_MINUS_SCALE;
    }
}

/// ACES Red Modifier 1.0 inverse.
///
/// Solves quadratic equation to invert the modification.
#[inline]
pub fn aces_red_mod_10_inv(rgb: &mut [f32; 3]) {
    use red_mod_10::*;
    
    let f_h = calc_hue_weight(rgb[0], rgb[1], rgb[2], INV_WIDTH);
    
    if f_h > 0.0 {
        let min_chan = rgb[1].min(rgb[2]);
        
        // Quadratic formula coefficients: a*x^2 + b*x + c = 0
        // Derived from: red_out = red + f_H * f_S * (pivot - red) * (1-scale)
        // where f_S = (red - min) / red
        let a = f_h * ONE_MINUS_SCALE - 1.0;
        let b = rgb[0] - f_h * (PIVOT + min_chan) * ONE_MINUS_SCALE;
        let c = f_h * PIVOT * min_chan * ONE_MINUS_SCALE;
        
        // Use negative root: (-b - sqrt(b^2 - 4ac)) / 2a
        let discriminant = b * b - 4.0 * a * c;
        rgb[0] = (-b - discriminant.sqrt()) / (2.0 * a);
    }
}

/// Apply ACES Red Modifier 1.0 forward to RGBA buffer.
pub fn apply_aces_red_mod_10_fwd_rgba(pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        aces_red_mod_10_fwd(&mut rgb);
        chunk[0] = rgb[0];
        chunk[1] = rgb[1];
        chunk[2] = rgb[2];
    }
}

/// Apply ACES Red Modifier 1.0 inverse to RGBA buffer.
pub fn apply_aces_red_mod_10_inv_rgba(pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        aces_red_mod_10_inv(&mut rgb);
        chunk[0] = rgb[0];
        chunk[1] = rgb[1];
        chunk[2] = rgb[2];
    }
}

// ============================================================================
// ACES Glow 1.0
// ============================================================================

/// Constants for ACES Glow 1.0
mod glow_10 {
    pub const GLOW_GAIN: f32 = 0.05;
    pub const GLOW_MID: f32 = 0.08;
    pub const NOISE_LIMIT: f32 = 1e-2;
}

/// Convert RGB to YC (luma + chroma factor).
#[inline]
fn rgb_to_yc(red: f32, grn: f32, blu: f32) -> f32 {
    const YC_RADIUS_WEIGHT: f32 = 1.75;
    let chroma = (blu * (blu - grn) + grn * (grn - red) + red * (red - blu)).sqrt();
    (blu + grn + red + YC_RADIUS_WEIGHT * chroma) / 3.0
}

/// Sigmoid shaper for saturation.
#[inline]
fn sigmoid_shaper(sat: f32) -> f32 {
    let x = (sat - 0.4) * 5.0;
    let sign = x.signum();
    let t = (1.0 - 0.5 * sign * x).max(0.0);
    (1.0 + sign * (1.0 - t * t)) * 0.5
}

/// ACES Glow 1.0 forward.
///
/// Applies a glow effect based on saturation and luminance.
#[inline]
pub fn aces_glow_10_fwd(rgb: &mut [f32; 3]) {
    use glow_10::*;
    
    let yc = rgb_to_yc(rgb[0], rgb[1], rgb[2]);
    let sat = calc_sat_weight(rgb[0], rgb[1], rgb[2], NOISE_LIMIT);
    let s = sigmoid_shaper(sat);
    
    let glow_gain = GLOW_GAIN * s;
    
    // Calculate glow amount based on YC
    let glow_gain_out = if yc >= GLOW_MID * 2.0 {
        0.0
    } else if yc <= GLOW_MID * 2.0 / 3.0 {
        glow_gain
    } else {
        glow_gain * (GLOW_MID / yc - 0.5)
    };
    
    let added_glow = 1.0 + glow_gain_out;
    rgb[0] *= added_glow;
    rgb[1] *= added_glow;
    rgb[2] *= added_glow;
}

/// ACES Glow 1.0 inverse.
#[inline]
pub fn aces_glow_10_inv(rgb: &mut [f32; 3]) {
    use glow_10::*;
    
    let yc = rgb_to_yc(rgb[0], rgb[1], rgb[2]);
    let sat = calc_sat_weight(rgb[0], rgb[1], rgb[2], NOISE_LIMIT);
    let s = sigmoid_shaper(sat);
    
    let glow_gain = GLOW_GAIN * s;
    
    // Inverse glow calculation
    let glow_gain_out = if yc >= GLOW_MID * 2.0 {
        0.0
    } else if yc <= (1.0 + glow_gain) * GLOW_MID * 2.0 / 3.0 {
        -glow_gain / (1.0 + glow_gain)
    } else {
        glow_gain * (GLOW_MID / yc - 0.5) / (glow_gain * 0.5 - 1.0)
    };
    
    let reduced_glow = 1.0 + glow_gain_out;
    rgb[0] *= reduced_glow;
    rgb[1] *= reduced_glow;
    rgb[2] *= reduced_glow;
}

/// Apply ACES Glow 1.0 forward to RGBA buffer.
pub fn apply_aces_glow_10_fwd_rgba(pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        aces_glow_10_fwd(&mut rgb);
        chunk[0] = rgb[0];
        chunk[1] = rgb[1];
        chunk[2] = rgb[2];
    }
}

/// Apply ACES Glow 1.0 inverse to RGBA buffer.
pub fn apply_aces_glow_10_inv_rgba(pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        aces_glow_10_inv(&mut rgb);
        chunk[0] = rgb[0];
        chunk[1] = rgb[1];
        chunk[2] = rgb[2];
    }
}

// ============================================================================
// REC.2100 Surround
// ============================================================================

/// Rec.2100 luminance coefficients.
const REC2100_Y_R: f32 = 0.2627;
const REC2100_Y_G: f32 = 0.6780;
const REC2100_Y_B: f32 = 0.0593;

/// REC.2100 Surround correction forward.
///
/// Applies surround correction for HDR content.
/// `gamma` is the surround adjustment parameter (typically 0.78 - 1.0 for HLG).
#[inline]
pub fn rec2100_surround_fwd(rgb: &mut [f32; 3], gamma: f32) {
    // Min luminance threshold
    let min_lum = 1e-4_f32;
    
    // Calculate luminance
    let mut y = REC2100_Y_R * rgb[0] + REC2100_Y_G * rgb[1] + REC2100_Y_B * rgb[2];
    
    // Mirror around origin
    y = y.abs();
    
    // Clamp to prevent extreme gain in dark colors
    y = y.max(min_lum);
    
    // Y^gamma / Y = Y^(gamma-1)
    let y_pow_over_y = y.powf(gamma - 1.0);
    
    rgb[0] *= y_pow_over_y;
    rgb[1] *= y_pow_over_y;
    rgb[2] *= y_pow_over_y;
}

/// REC.2100 Surround correction inverse.
///
/// Applies inverse surround correction for HDR content.
#[inline]
pub fn rec2100_surround_inv(rgb: &mut [f32; 3], gamma: f32) {
    // For inverse: use 1/gamma and adjust min_lum
    let inv_gamma = 1.0 / gamma;
    let min_lum = (1e-4_f32).powf(gamma);
    
    // Calculate luminance
    let mut y = REC2100_Y_R * rgb[0] + REC2100_Y_G * rgb[1] + REC2100_Y_B * rgb[2];
    
    // Mirror around origin
    y = y.abs();
    
    // Clamp to prevent extreme gain
    y = y.max(min_lum);
    
    // Y^(1/gamma) / Y = Y^(1/gamma - 1)
    let y_pow_over_y = y.powf(inv_gamma - 1.0);
    
    rgb[0] *= y_pow_over_y;
    rgb[1] *= y_pow_over_y;
    rgb[2] *= y_pow_over_y;
}

/// Apply REC.2100 Surround forward to RGBA buffer.
pub fn apply_rec2100_surround_fwd_rgba(pixels: &mut [f32], gamma: f32) {
    for chunk in pixels.chunks_exact_mut(4) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        rec2100_surround_fwd(&mut rgb, gamma);
        chunk[0] = rgb[0];
        chunk[1] = rgb[1];
        chunk[2] = rgb[2];
    }
}

/// Apply REC.2100 Surround inverse to RGBA buffer.
pub fn apply_rec2100_surround_inv_rgba(pixels: &mut [f32], gamma: f32) {
    for chunk in pixels.chunks_exact_mut(4) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        rec2100_surround_inv(&mut rgb, gamma);
        chunk[0] = rgb[0];
        chunk[1] = rgb[1];
        chunk[2] = rgb[2];
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    const EPSILON: f32 = 1e-5;
    
    // ========================================================================
    // XYZ <-> xyY tests
    // ========================================================================
    
    #[test]
    fn test_xyz_to_xyy_d65() {
        // D65 white point: X=0.95047, Y=1.0, Z=1.08883
        let xyz = [0.95047, 1.0, 1.08883];
        let xyy = xyz_to_xyy(xyz);
        
        // Should give x≈0.3127, y≈0.3290 (D65 chromaticity)
        assert!((xyy[0] - 0.3127).abs() < 0.001);
        assert!((xyy[1] - 0.3290).abs() < 0.001);
        assert!((xyy[2] - 1.0).abs() < EPSILON); // Y unchanged
    }
    
    #[test]
    fn test_xyz_xyy_roundtrip() {
        let original = [0.5, 0.3, 0.2];
        let xyy = xyz_to_xyy(original);
        let back = xyy_to_xyz(xyy);
        
        assert!((back[0] - original[0]).abs() < EPSILON);
        assert!((back[1] - original[1]).abs() < EPSILON);
        assert!((back[2] - original[2]).abs() < EPSILON);
    }
    
    #[test]
    fn test_xyz_xyy_roundtrip_various() {
        let test_values = [
            [0.2, 0.3, 0.4],
            [0.95047, 1.0, 1.08883],  // D65
            [0.1, 0.1, 0.1],
            [0.8, 0.5, 0.2],
            [0.01, 0.02, 0.03],
        ];
        
        for original in test_values {
            let xyy = xyz_to_xyy(original);
            let back = xyy_to_xyz(xyy);
            
            assert!(
                (back[0] - original[0]).abs() < EPSILON,
                "X mismatch for {:?}: {} vs {}", original, back[0], original[0]
            );
            assert!(
                (back[1] - original[1]).abs() < EPSILON,
                "Y mismatch for {:?}: {} vs {}", original, back[1], original[1]
            );
            assert!(
                (back[2] - original[2]).abs() < EPSILON,
                "Z mismatch for {:?}: {} vs {}", original, back[2], original[2]
            );
        }
    }
    
    #[test]
    fn test_xyz_xyy_black() {
        // Black (0,0,0) should handle gracefully
        let xyz = [0.0, 0.0, 0.0];
        let xyy = xyz_to_xyy(xyz);
        
        // All zeros when divisor is 0
        assert!((xyy[0]).abs() < EPSILON);
        assert!((xyy[1]).abs() < EPSILON);
        assert!((xyy[2]).abs() < EPSILON);
    }
    
    // ========================================================================
    // XYZ <-> uvY tests
    // ========================================================================
    
    #[test]
    fn test_xyz_to_uvy_d65() {
        // D65 white point
        let xyz = [0.95047, 1.0, 1.08883];
        let uvy = xyz_to_uvy(xyz);
        
        // D65 u'v' chromaticity: u'≈0.1978, v'≈0.4683
        assert!((uvy[0] - 0.1978).abs() < 0.001);
        assert!((uvy[1] - 0.4683).abs() < 0.001);
        assert!((uvy[2] - 1.0).abs() < EPSILON); // Y unchanged
    }
    
    #[test]
    fn test_xyz_uvy_roundtrip() {
        let original = [0.5, 0.3, 0.2];
        let uvy = xyz_to_uvy(original);
        let back = uvy_to_xyz(uvy);
        
        assert!((back[0] - original[0]).abs() < EPSILON);
        assert!((back[1] - original[1]).abs() < EPSILON);
        assert!((back[2] - original[2]).abs() < EPSILON);
    }
    
    #[test]
    fn test_xyz_uvy_roundtrip_various() {
        let test_values = [
            [0.2, 0.3, 0.4],
            [0.95047, 1.0, 1.08883],  // D65
            [0.1, 0.1, 0.1],
            [0.8, 0.5, 0.2],
            [0.01, 0.02, 0.03],
        ];
        
        for original in test_values {
            let uvy = xyz_to_uvy(original);
            let back = uvy_to_xyz(uvy);
            
            assert!(
                (back[0] - original[0]).abs() < EPSILON,
                "X mismatch for {:?}: {} vs {}", original, back[0], original[0]
            );
            assert!(
                (back[1] - original[1]).abs() < EPSILON,
                "Y mismatch for {:?}: {} vs {}", original, back[1], original[1]
            );
            assert!(
                (back[2] - original[2]).abs() < EPSILON,
                "Z mismatch for {:?}: {} vs {}", original, back[2], original[2]
            );
        }
    }
    
    #[test]
    fn test_xyz_uvy_black() {
        // Black (0,0,0) should handle gracefully
        let xyz = [0.0, 0.0, 0.0];
        let uvy = xyz_to_uvy(xyz);
        
        assert!((uvy[0]).abs() < EPSILON);
        assert!((uvy[1]).abs() < EPSILON);
        assert!((uvy[2]).abs() < EPSILON);
    }
    
    // ========================================================================
    // RGBA buffer tests
    // ========================================================================
    
    #[test]
    fn test_rgba_xyz_xyy_roundtrip() {
        let original = [
            0.5, 0.3, 0.2, 1.0,
            0.95047, 1.0, 1.08883, 0.5,
        ];
        let mut pixels = original;
        
        apply_xyz_to_xyy_rgba(&mut pixels);
        apply_xyy_to_xyz_rgba(&mut pixels);
        
        for i in 0..8 {
            assert!(
                (pixels[i] - original[i]).abs() < EPSILON,
                "Mismatch at {i}: {} vs {}", pixels[i], original[i]
            );
        }
    }
    
    #[test]
    fn test_rgba_xyz_uvy_roundtrip() {
        let original = [
            0.5, 0.3, 0.2, 1.0,
            0.95047, 1.0, 1.08883, 0.5,
        ];
        let mut pixels = original;
        
        apply_xyz_to_uvy_rgba(&mut pixels);
        apply_uvy_to_xyz_rgba(&mut pixels);
        
        for i in 0..8 {
            assert!(
                (pixels[i] - original[i]).abs() < EPSILON,
                "Mismatch at {i}: {} vs {}", pixels[i], original[i]
            );
        }
    }
    
    // ========================================================================
    // Known value tests
    // ========================================================================
    
    #[test]
    fn test_primary_colors_xyy() {
        // sRGB primaries in XYZ (approximate)
        // Red: x=0.64, y=0.33
        // Green: x=0.30, y=0.60
        // Blue: x=0.15, y=0.06
        
        // Test red primary chromaticity
        // XYZ for pure sRGB red: ~(0.4124, 0.2126, 0.0193)
        let red_xyz = [0.4124, 0.2126, 0.0193];
        let red_xyy = xyz_to_xyy(red_xyz);
        
        assert!((red_xyy[0] - 0.64).abs() < 0.01, "Red x: {}", red_xyy[0]);
        assert!((red_xyy[1] - 0.33).abs() < 0.01, "Red y: {}", red_xyy[1]);
    }
    
    // ========================================================================
    // ACES Red Modifier 1.0 tests
    // ========================================================================
    
    #[test]
    fn test_aces_red_mod_10_identity_non_red() {
        // Non-red colors should be unaffected (hue weight = 0)
        let mut blue = [0.1, 0.2, 0.8];
        let original = blue;
        aces_red_mod_10_fwd(&mut blue);
        
        // Should be unchanged (blue has no red hue weight)
        assert!((blue[0] - original[0]).abs() < EPSILON);
        assert!((blue[1] - original[1]).abs() < EPSILON);
        assert!((blue[2] - original[2]).abs() < EPSILON);
    }
    
    #[test]
    fn test_aces_red_mod_10_affects_red() {
        // Saturated red should be modified
        let mut red = [0.8, 0.1, 0.1];
        let original_red = red[0];
        aces_red_mod_10_fwd(&mut red);
        
        // Red channel should have changed (reduced toward pivot)
        assert!(red[0] != original_red, "Red should be modified");
        // Green and blue unchanged
        assert!((red[1] - 0.1).abs() < EPSILON);
        assert!((red[2] - 0.1).abs() < EPSILON);
    }
    
    #[test]
    fn test_aces_red_mod_10_roundtrip() {
        // Test roundtrip for various red-ish colors.
        // Note: roundtrip is approximate because hue weight is calculated
        // from current RGB values, not stored original values.
        let test_values = [
            [0.8, 0.1, 0.1],   // pure-ish red
            [0.5, 0.2, 0.15],  // orangey red
            [0.6, 0.3, 0.2],   // less saturated
        ];
        
        for original in test_values {
            let mut rgb = original;
            aces_red_mod_10_fwd(&mut rgb);
            aces_red_mod_10_inv(&mut rgb);
            
            // Tolerance relaxed due to hue recalculation in inverse
            assert!(
                (rgb[0] - original[0]).abs() < 0.01,
                "Red roundtrip failed for {:?}: got {}", original, rgb[0]
            );
        }
    }
    
    #[test]
    fn test_aces_red_mod_10_rgba() {
        let mut pixels = [
            0.8, 0.1, 0.1, 1.0,
            0.1, 0.8, 0.1, 0.5,  // green (unaffected)
        ];
        let original = pixels;
        
        apply_aces_red_mod_10_fwd_rgba(&mut pixels);
        apply_aces_red_mod_10_inv_rgba(&mut pixels);
        
        // Alpha should be unchanged
        assert!((pixels[3] - 1.0).abs() < EPSILON);
        assert!((pixels[7] - 0.5).abs() < EPSILON);
        
        // Should roundtrip (approximately)
        assert!((pixels[0] - original[0]).abs() < 1e-4);
    }
    
    // ========================================================================
    // ACES Glow 1.0 tests
    // ========================================================================
    
    #[test]
    fn test_aces_glow_10_identity_bright() {
        // Very bright colors (YC >= GlowMid*2) should be unaffected
        let mut bright = [1.0, 1.0, 1.0];
        let original = bright;
        aces_glow_10_fwd(&mut bright);
        
        // Should be unchanged (YC is high)
        assert!((bright[0] - original[0]).abs() < EPSILON);
        assert!((bright[1] - original[1]).abs() < EPSILON);
        assert!((bright[2] - original[2]).abs() < EPSILON);
    }
    
    #[test]
    fn test_aces_glow_10_affects_dark_saturated() {
        // Dark saturated colors should have glow applied
        let mut dark = [0.05, 0.01, 0.01];
        let original_sum = dark[0] + dark[1] + dark[2];
        aces_glow_10_fwd(&mut dark);
        
        let new_sum = dark[0] + dark[1] + dark[2];
        // Glow adds luminance
        assert!(new_sum > original_sum, "Glow should increase luminance");
    }
    
    #[test]
    fn test_aces_glow_10_roundtrip() {
        let test_values = [
            [0.05, 0.03, 0.02],  // dark
            [0.1, 0.05, 0.05],   // darker saturated
            [0.3, 0.1, 0.1],     // medium red
        ];
        
        for original in test_values {
            let mut rgb = original;
            aces_glow_10_fwd(&mut rgb);
            aces_glow_10_inv(&mut rgb);
            
            assert!(
                (rgb[0] - original[0]).abs() < 1e-4,
                "Glow roundtrip failed for {:?}: got {:?}", original, rgb
            );
        }
    }
    
    #[test]
    fn test_aces_glow_10_rgba() {
        let mut pixels = [
            0.05, 0.03, 0.02, 1.0,
            1.0, 1.0, 1.0, 0.5,  // bright (unaffected)
        ];
        let original = pixels;
        
        apply_aces_glow_10_fwd_rgba(&mut pixels);
        apply_aces_glow_10_inv_rgba(&mut pixels);
        
        // Alpha should be unchanged
        assert!((pixels[3] - 1.0).abs() < EPSILON);
        assert!((pixels[7] - 0.5).abs() < EPSILON);
        
        // Should roundtrip
        assert!((pixels[0] - original[0]).abs() < 1e-4);
    }
    
    // ========================================================================
    // REC.2100 Surround tests
    // ========================================================================
    
    #[test]
    fn test_rec2100_surround_identity() {
        // With gamma = 1.0, should be nearly identity
        let mut rgb = [0.5, 0.3, 0.2];
        let original = rgb;
        rec2100_surround_fwd(&mut rgb, 1.0);
        
        // gamma-1 = 0, so Y^0 = 1
        assert!((rgb[0] - original[0]).abs() < EPSILON);
        assert!((rgb[1] - original[1]).abs() < EPSILON);
        assert!((rgb[2] - original[2]).abs() < EPSILON);
    }
    
    #[test]
    fn test_rec2100_surround_affects_values() {
        // With gamma != 1.0, should modify values
        let mut rgb = [0.5, 0.3, 0.2];
        let original = rgb;
        rec2100_surround_fwd(&mut rgb, 0.8);
        
        // Values should have changed
        assert!(rgb[0] != original[0] || rgb[1] != original[1] || rgb[2] != original[2]);
    }
    
    #[test]
    fn test_rec2100_surround_roundtrip() {
        let test_gammas = [0.78, 0.85, 0.9, 0.95];
        let test_values = [
            [0.5, 0.3, 0.2],
            [0.1, 0.1, 0.1],
            [0.8, 0.6, 0.4],
        ];
        
        for gamma in test_gammas {
            for original in test_values {
                let mut rgb = original;
                rec2100_surround_fwd(&mut rgb, gamma);
                rec2100_surround_inv(&mut rgb, gamma);
                
                assert!(
                    (rgb[0] - original[0]).abs() < 1e-4,
                    "REC2100 roundtrip failed for gamma={}, {:?}: got {:?}", 
                    gamma, original, rgb
                );
            }
        }
    }
    
    #[test]
    fn test_rec2100_surround_hlg_gamma() {
        // Typical HLG gamma is 0.78 (dark surround)
        let gamma = 0.78;
        let mut rgb = [0.5, 0.3, 0.2];
        rec2100_surround_fwd(&mut rgb, gamma);
        
        // Values should all be positive and reasonable
        assert!(rgb[0] > 0.0 && rgb[0].is_finite());
        assert!(rgb[1] > 0.0 && rgb[1].is_finite());
        assert!(rgb[2] > 0.0 && rgb[2].is_finite());
    }
    
    #[test]
    fn test_rec2100_surround_rgba() {
        let gamma = 0.85;
        let mut pixels = [
            0.5, 0.3, 0.2, 1.0,
            0.8, 0.6, 0.4, 0.5,
        ];
        let original = pixels;
        
        apply_rec2100_surround_fwd_rgba(&mut pixels, gamma);
        apply_rec2100_surround_inv_rgba(&mut pixels, gamma);
        
        // Alpha should be unchanged
        assert!((pixels[3] - 1.0).abs() < EPSILON);
        assert!((pixels[7] - 0.5).abs() < EPSILON);
        
        // Should roundtrip
        assert!((pixels[0] - original[0]).abs() < 1e-4);
        assert!((pixels[4] - original[4]).abs() < 1e-4);
    }
}
