//! FixedFunction operations - specialized color space conversions.
//!
//! Reference: OCIO FixedFunctionOpCPU.cpp
//!
//! This module provides fixed-function color operations including:
//! - CIE chromaticity conversions (XYZ <-> xyY, uvY)
//! - ACES Red Modifier (1.0) - hue-based red channel correction
//! - ACES Glow (1.0) - glow effect based on saturation
//! - REC.2100 Surround - HDR surround correction

use vfx_core::pixel::{REC709_LUMA_R, REC709_LUMA_G, REC709_LUMA_B};

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
// XYZ <-> L*u*v* (CIELUV)
// ============================================================================

/// D65 white point chromaticity constants for CIELUV.
mod luv_d65 {
    /// D65 u' = 4 * 0.95047 / (0.95047 + 15 * 1.0 + 3 * 1.08883)
    pub const U_N: f32 = 0.19783001;
    /// D65 v' = 9 * 1.0 / (0.95047 + 15 * 1.0 + 3 * 1.08883)
    pub const V_N: f32 = 0.46831999;
    /// L* linear segment threshold (Y value)
    pub const Y_BREAK: f32 = 0.008856451679;
    /// L* linear segment threshold (L* value)
    pub const L_BREAK: f32 = 0.08;
    /// L* linear coefficient: 903.3 / 100 (= 9.033 for percentage scale)
    pub const KAPPA: f32 = 9.0329629629629608;
    /// 1/KAPPA for inverse
    pub const INV_KAPPA: f32 = 0.11070564598794539;
    /// L* power coefficient
    pub const L_SCALE: f32 = 1.16;
    /// L* power offset
    pub const L_OFFSET: f32 = 0.16;
    /// 1/1.16 for inverse
    pub const INV_L_SCALE: f32 = 0.86206896551724144;
    /// 1/13 for u*, v* calculation
    pub const INV_13: f32 = 0.076923076923076927;
}

/// Convert CIE XYZ to L*u*v* (CIELUV).
///
/// This is the perceptually uniform color space standardized by CIE.
/// - L* = lightness (0-100 scale mapped to 0-1)
/// - u*, v* = chromaticity coordinates
///
/// Uses D65 reference white.
#[inline]
pub fn xyz_to_luv(xyz: [f32; 3]) -> [f32; 3] {
    use luv_d65::*;
    
    let x = xyz[0];
    let y = xyz[1];
    let z = xyz[2];
    
    // Calculate u'v' chromaticity
    let d = x + 15.0 * y + 3.0 * z;
    let d = if d == 0.0 { 0.0 } else { 1.0 / d };
    let u = 4.0 * x * d;
    let v = 9.0 * y * d;
    
    // Calculate L*
    let l_star = if y <= Y_BREAK {
        KAPPA * y
    } else {
        L_SCALE * y.powf(1.0 / 3.0) - L_OFFSET
    };
    
    // Calculate u*, v* relative to D65 white
    let u_star = 13.0 * l_star * (u - U_N);
    let v_star = 13.0 * l_star * (v - V_N);
    
    [l_star, u_star, v_star]
}

/// Convert L*u*v* (CIELUV) to CIE XYZ.
///
/// Uses D65 reference white.
#[inline]
pub fn luv_to_xyz(luv: [f32; 3]) -> [f32; 3] {
    use luv_d65::*;
    
    let l_star = luv[0];
    let u_star = luv[1];
    let v_star = luv[2];
    
    // Recover u'v' from u*v*
    let d = if l_star == 0.0 { 0.0 } else { INV_13 / l_star };
    let u = u_star * d + U_N;
    let v = v_star * d + V_N;
    
    // Recover Y from L*
    let y = if l_star <= L_BREAK {
        INV_KAPPA * l_star
    } else {
        let tmp = (l_star + L_OFFSET) * INV_L_SCALE;
        tmp * tmp * tmp
    };
    
    // Recover X, Z from Y and u'v'
    let dd = if v == 0.0 { 0.0 } else { 0.25 / v };
    let x = 9.0 * y * u * dd;
    let z = y * (12.0 - 3.0 * u - 20.0 * v) * dd;
    
    [x, y, z]
}

/// Apply XYZ to L*u*v* conversion in-place.
#[inline]
pub fn apply_xyz_to_luv(xyz: &mut [f32; 3]) {
    *xyz = xyz_to_luv(*xyz);
}

/// Apply L*u*v* to XYZ conversion in-place.
#[inline]
pub fn apply_luv_to_xyz(luv: &mut [f32; 3]) {
    *luv = luv_to_xyz(*luv);
}

/// Apply XYZ to L*u*v* to RGBA buffer.
pub fn apply_xyz_to_luv_rgba(pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let xyz = [chunk[0], chunk[1], chunk[2]];
        let luv = xyz_to_luv(xyz);
        chunk[0] = luv[0];
        chunk[1] = luv[1];
        chunk[2] = luv[2];
        // Alpha unchanged
    }
}

/// Apply L*u*v* to XYZ to RGBA buffer.
pub fn apply_luv_to_xyz_rgba(pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let luv = [chunk[0], chunk[1], chunk[2]];
        let xyz = luv_to_xyz(luv);
        chunk[0] = xyz[0];
        chunk[1] = xyz[1];
        chunk[2] = xyz[2];
        // Alpha unchanged
    }
}

// ============================================================================
// ACES Red Modifier 0.3/0.7 and 1.0
// ============================================================================

/// Constants for ACES Red Modifier 0.3/0.7
mod red_mod_03 {
    pub const SCALE: f32 = 0.85;
    pub const ONE_MINUS_SCALE: f32 = 1.0 - SCALE;  // 0.15
    pub const PIVOT: f32 = 0.03;
    /// 4 / (120 * pi/180) = 4 / 2.0944 ≈ 1.9099
    pub const INV_WIDTH: f32 = 1.9098593171027443;
    pub const NOISE_LIMIT: f32 = 1e-2;
}

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
        // Clamp discriminant to 0 to avoid NaN from sqrt of negative value
        let discriminant = (b * b - 4.0 * a * c).max(0.0);
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
// ACES Red Modifier 0.3/0.7
// ============================================================================

/// ACES Red Modifier 0.3/0.7 forward.
///
/// Earlier ACES version with different scale (0.85) and width (120 degrees).
#[inline]
pub fn aces_red_mod_03_fwd(rgb: &mut [f32; 3]) {
    use red_mod_03::*;
    
    let f_h = calc_hue_weight(rgb[0], rgb[1], rgb[2], INV_WIDTH);
    
    if f_h > 0.0 {
        let f_s = calc_sat_weight(rgb[0], rgb[1], rgb[2], NOISE_LIMIT);
        
        // Preserve hue by scaling green/blue with red
        let red = rgb[0];
        let grn = rgb[1];
        let blu = rgb[2];
        
        let new_red = red + f_h * f_s * (PIVOT - red) * ONE_MINUS_SCALE;
        
        // Restore hue
        if grn >= blu {
            // red >= grn >= blu
            let hue_fac = (grn - blu) / (red - blu).max(1e-10);
            rgb[1] = hue_fac * (new_red - blu) + blu;
        } else {
            // red >= blu >= grn
            let hue_fac = (blu - grn) / (red - grn).max(1e-10);
            rgb[2] = hue_fac * (new_red - grn) + grn;
        }
        
        rgb[0] = new_red;
    }
}

/// ACES Red Modifier 0.3/0.7 inverse.
#[inline]
pub fn aces_red_mod_03_inv(rgb: &mut [f32; 3]) {
    use red_mod_03::*;
    
    let f_h = calc_hue_weight(rgb[0], rgb[1], rgb[2], INV_WIDTH);
    
    if f_h > 0.0 {
        let min_chan = rgb[1].min(rgb[2]);
        let red = rgb[0];
        let grn = rgb[1];
        let blu = rgb[2];
        
        // Quadratic formula: a*x^2 + b*x + c = 0
        let a = f_h * ONE_MINUS_SCALE - 1.0;
        let b = red - f_h * (PIVOT + min_chan) * ONE_MINUS_SCALE;
        let c = f_h * PIVOT * min_chan * ONE_MINUS_SCALE;
        
        let discriminant = b * b - 4.0 * a * c;
        let new_red = (-b - discriminant.sqrt()) / (2.0 * a);
        
        // Restore hue
        if grn >= blu {
            let hue_fac = (grn - blu) / (red - blu).max(1e-10);
            rgb[1] = hue_fac * (new_red - blu) + blu;
        } else {
            let hue_fac = (blu - grn) / (red - grn).max(1e-10);
            rgb[2] = hue_fac * (new_red - grn) + grn;
        }
        
        rgb[0] = new_red;
    }
}

/// Apply ACES Red Modifier 0.3 forward to RGBA buffer.
pub fn apply_aces_red_mod_03_fwd_rgba(pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        aces_red_mod_03_fwd(&mut rgb);
        chunk[0] = rgb[0];
        chunk[1] = rgb[1];
        chunk[2] = rgb[2];
    }
}

/// Apply ACES Red Modifier 0.3 inverse to RGBA buffer.
pub fn apply_aces_red_mod_03_inv_rgba(pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        aces_red_mod_03_inv(&mut rgb);
        chunk[0] = rgb[0];
        chunk[1] = rgb[1];
        chunk[2] = rgb[2];
    }
}

// ============================================================================
// ACES Glow 0.3/0.7 and 1.0
// ============================================================================

/// Constants for ACES Glow 0.3/0.7
mod glow_03 {
    pub const GLOW_GAIN: f32 = 0.075;
    pub const GLOW_MID: f32 = 0.1;
    pub const NOISE_LIMIT: f32 = 1e-2;
}

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
// ACES Glow 0.3/0.7
// ============================================================================

/// ACES Glow 0.3/0.7 forward.
///
/// Earlier ACES version with different gain (0.075) and mid (0.1).
#[inline]
pub fn aces_glow_03_fwd(rgb: &mut [f32; 3]) {
    use glow_03::*;
    
    let yc = rgb_to_yc(rgb[0], rgb[1], rgb[2]);
    let sat = calc_sat_weight(rgb[0], rgb[1], rgb[2], NOISE_LIMIT);
    let s = sigmoid_shaper(sat);
    
    let glow_gain = GLOW_GAIN * s;
    
    // Calculate glow gain output based on YC level
    let glow_gain_out = if yc >= GLOW_MID * 2.0 {
        0.0
    } else if yc <= GLOW_MID * 2.0 / 3.0 {
        glow_gain
    } else {
        glow_gain * (GLOW_MID / yc - 0.5)
    };
    
    // Apply glow (additive)
    let add_glow = 1.0 + glow_gain_out;
    rgb[0] *= add_glow;
    rgb[1] *= add_glow;
    rgb[2] *= add_glow;
}

/// ACES Glow 0.3/0.7 inverse.
#[inline]
pub fn aces_glow_03_inv(rgb: &mut [f32; 3]) {
    use glow_03::*;
    
    let yc = rgb_to_yc(rgb[0], rgb[1], rgb[2]);
    let sat = calc_sat_weight(rgb[0], rgb[1], rgb[2], NOISE_LIMIT);
    let s = sigmoid_shaper(sat);
    
    let glow_gain = GLOW_GAIN * s;
    
    // Calculate glow gain output
    let glow_gain_out = if yc >= GLOW_MID * 2.0 {
        0.0
    } else if yc <= GLOW_MID * 2.0 / 3.0 {
        glow_gain
    } else {
        glow_gain * (GLOW_MID / yc - 0.5)
    };
    
    // Remove glow (inverse of additive)
    let remove_glow = 1.0 / (1.0 + glow_gain_out);
    rgb[0] *= remove_glow;
    rgb[1] *= remove_glow;
    rgb[2] *= remove_glow;
}

/// Apply ACES Glow 0.3 forward to RGBA buffer.
pub fn apply_aces_glow_03_fwd_rgba(pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        aces_glow_03_fwd(&mut rgb);
        chunk[0] = rgb[0];
        chunk[1] = rgb[1];
        chunk[2] = rgb[2];
    }
}

/// Apply ACES Glow 0.3 inverse to RGBA buffer.
pub fn apply_aces_glow_03_inv_rgba(pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        aces_glow_03_inv(&mut rgb);
        chunk[0] = rgb[0];
        chunk[1] = rgb[1];
        chunk[2] = rgb[2];
    }
}

// ============================================================================
// ACES Dark to Dim 1.0
// ============================================================================

/// AP1 luminance coefficients (ACES).
const AP1_Y_R: f32 = 0.27222871678091454;
const AP1_Y_G: f32 = 0.67408176581114831;
const AP1_Y_B: f32 = 0.053689517407937051;

/// ACES Dark to Dim 1.0 gamma values.
mod dark_to_dim_10 {
    /// Forward gamma (dark to dim surround)
    pub const GAMMA_FWD: f32 = 0.9811;
    /// Inverse gamma (dim to dark surround)
    pub const GAMMA_INV: f32 = 1.0192640913260627;
}

/// ACES Dark to Dim 1.0 forward.
///
/// Applies surround correction from dark to dim viewing conditions.
/// Uses AP1 luminance coefficients.
#[inline]
pub fn aces_dark_to_dim_10_fwd(rgb: &mut [f32; 3]) {
    use dark_to_dim_10::GAMMA_FWD;
    
    const MIN_LUM: f32 = 1e-10;
    
    // Calculate luminance assuming AP1 RGB
    let y = (AP1_Y_R * rgb[0] + AP1_Y_G * rgb[1] + AP1_Y_B * rgb[2]).max(MIN_LUM);
    
    // Y^gamma / Y = Y^(gamma-1)
    let y_pow_over_y = y.powf(GAMMA_FWD - 1.0);
    
    rgb[0] *= y_pow_over_y;
    rgb[1] *= y_pow_over_y;
    rgb[2] *= y_pow_over_y;
}

/// ACES Dark to Dim 1.0 inverse (Dim to Dark).
///
/// Applies inverse surround correction from dim to dark viewing conditions.
#[inline]
pub fn aces_dark_to_dim_10_inv(rgb: &mut [f32; 3]) {
    use dark_to_dim_10::GAMMA_INV;
    
    const MIN_LUM: f32 = 1e-10;
    
    // Calculate luminance assuming AP1 RGB
    let y = (AP1_Y_R * rgb[0] + AP1_Y_G * rgb[1] + AP1_Y_B * rgb[2]).max(MIN_LUM);
    
    // Apply inverse gamma
    let y_pow_over_y = y.powf(GAMMA_INV - 1.0);
    
    rgb[0] *= y_pow_over_y;
    rgb[1] *= y_pow_over_y;
    rgb[2] *= y_pow_over_y;
}

/// Apply ACES Dark to Dim 1.0 forward to RGBA buffer.
pub fn apply_aces_dark_to_dim_10_fwd_rgba(pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        aces_dark_to_dim_10_fwd(&mut rgb);
        chunk[0] = rgb[0];
        chunk[1] = rgb[1];
        chunk[2] = rgb[2];
    }
}

/// Apply ACES Dark to Dim 1.0 inverse to RGBA buffer.
pub fn apply_aces_dark_to_dim_10_inv_rgba(pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        aces_dark_to_dim_10_inv(&mut rgb);
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
// LIN_TO_PQ / PQ_TO_LIN (OCIO FixedFunction style)
// ============================================================================

/// ST 2084 PQ constants.
mod st_2084 {
    /// m1 = 0.25 * 2610 / 4096
    pub const M1: f32 = 0.1593017578125;
    /// m2 = 128 * 2523 / 4096
    pub const M2: f32 = 78.84375;
    /// c2 = 32 * 2413 / 4096
    pub const C2: f32 = 18.8515625;
    /// c3 = 32 * 2392 / 4096
    pub const C3: f32 = 18.6875;
    /// c1 = c3 - c2 + 1
    pub const C1: f32 = 0.8359375;
}

/// PQ to Linear (OCIO FixedFunction style).
/// 
/// Input: PQ encoded [0, 1]
/// Output: Linear normalized where 1.0 = 100 nits (nits/100)
/// 
/// Handles negative values by mirroring around zero.
#[inline]
pub fn pq_to_lin(pq: f32) -> f32 {
    use st_2084::*;
    
    let v_abs = pq.abs();
    let x = v_abs.powf(1.0 / M2);
    let nits = ((x - C1).max(0.0) / (C2 - C3 * x)).powf(1.0 / M1);
    
    // Output scale: 1.0 = 10000 nits in ST2084, map to 1.0 = 100 nits
    let result = 100.0 * nits;
    
    result.copysign(pq)
}

/// Linear to PQ (OCIO FixedFunction style).
/// 
/// Input: Linear normalized where 1.0 = 100 nits (nits/100)
/// Output: PQ encoded [0, 1]
/// 
/// Handles negative values by mirroring around zero.
#[inline]
pub fn lin_to_pq(lin: f32) -> f32 {
    use st_2084::*;
    
    // Input is nits/100, convert to [0,1] where 1 = 10000 nits
    let l = (lin * 0.01).abs();
    let y = l.powf(M1);
    let n = ((C1 + C2 * y) / (1.0 + C3 * y)).powf(M2);
    
    n.copysign(lin)
}

/// Apply PQ to Linear to RGB.
#[inline]
pub fn apply_pq_to_lin(rgb: &mut [f32; 3]) {
    rgb[0] = pq_to_lin(rgb[0]);
    rgb[1] = pq_to_lin(rgb[1]);
    rgb[2] = pq_to_lin(rgb[2]);
}

/// Apply Linear to PQ to RGB.
#[inline]
pub fn apply_lin_to_pq(rgb: &mut [f32; 3]) {
    rgb[0] = lin_to_pq(rgb[0]);
    rgb[1] = lin_to_pq(rgb[1]);
    rgb[2] = lin_to_pq(rgb[2]);
}

/// Apply PQ to Linear to RGBA buffer.
pub fn apply_pq_to_lin_rgba(pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        chunk[0] = pq_to_lin(chunk[0]);
        chunk[1] = pq_to_lin(chunk[1]);
        chunk[2] = pq_to_lin(chunk[2]);
    }
}

/// Apply Linear to PQ to RGBA buffer.
pub fn apply_lin_to_pq_rgba(pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        chunk[0] = lin_to_pq(chunk[0]);
        chunk[1] = lin_to_pq(chunk[1]);
        chunk[2] = lin_to_pq(chunk[2]);
    }
}

// ============================================================================
// LIN_TO_GAMMA_LOG / GAMMA_LOG_TO_LIN
// ============================================================================

/// Parameters for Gamma-Log curve.
/// 
/// Reference: OCIO FixedFunctionOpCPU.cpp Renderer_LIN_TO_GAMMA_LOG
/// 
/// The curve has two segments:
/// - Gamma segment (below break): slope * (x + off)^power
/// - Log segment (above break): logSlope * log(linSlope * x + linOff) + logOff
#[derive(Debug, Clone, Copy)]
pub struct GammaLogParams {
    /// Mirror point for negative value handling
    pub mirror: f32,
    /// Break point between gamma and log segments
    pub break_point: f32,
    /// Gamma segment power
    pub gamma_power: f32,
    /// Gamma segment slope
    pub gamma_slope: f32,
    /// Gamma segment offset
    pub gamma_off: f32,
    /// Log base (e.g., 10.0 or e)
    pub log_base: f32,
    /// Log segment slope (before log base conversion)
    pub log_slope: f32,
    /// Log segment offset
    pub log_off: f32,
    /// Linear scaling slope inside log
    pub lin_slope: f32,
    /// Linear offset inside log
    pub lin_off: f32,
    // Precomputed values
    log_slope_baked: f32,  // log_slope / ln(log_base)
    prime_break: f32,      // break point in non-linear domain
    prime_mirror: f32,     // mirror point in non-linear domain
}

impl GammaLogParams {
    /// Create new parameters with precomputed values.
    pub fn new(
        mirror: f32, break_point: f32,
        gamma_power: f32, gamma_slope: f32, gamma_off: f32,
        log_base: f32, log_slope: f32, log_off: f32,
        lin_slope: f32, lin_off: f32,
    ) -> Self {
        let log_slope_baked = log_slope / log_base.ln();
        let prime_break = gamma_slope * (break_point + gamma_off).powf(gamma_power);
        let prime_mirror = gamma_slope * (mirror + gamma_off).powf(gamma_power);
        
        Self {
            mirror,
            break_point,
            gamma_power,
            gamma_slope,
            gamma_off,
            log_base,
            log_slope,
            log_off,
            lin_slope,
            lin_off,
            log_slope_baked,
            prime_break,
            prime_mirror,
        }
    }
}

/// Linear to Gamma-Log curve (forward).
/// 
/// Two-segment curve with gamma below break and log above.
#[inline]
pub fn lin_to_gamma_log(v: f32, params: &GammaLogParams) -> f32 {
    let mirror_in = v - params.mirror;
    let e = mirror_in.abs() + params.mirror;
    
    let e_prime = if e < params.break_point {
        // Gamma segment
        params.gamma_slope * (e + params.gamma_off).powf(params.gamma_power)
    } else {
        // Log segment
        params.log_slope_baked * (params.lin_slope * e + params.lin_off).ln() + params.log_off
    };
    
    e_prime.copysign(mirror_in)
}

/// Gamma-Log to Linear curve (inverse).
/// 
/// Inverse of lin_to_gamma_log.
#[inline]
pub fn gamma_log_to_lin(v: f32, params: &GammaLogParams) -> f32 {
    let mirror_in = v - params.prime_mirror;
    let e_prime = mirror_in.abs() + params.prime_mirror;
    
    let e = if e_prime < params.prime_break {
        // Inverse gamma segment
        (e_prime / params.gamma_slope).powf(1.0 / params.gamma_power) - params.gamma_off
    } else {
        // Inverse log segment
        (((e_prime - params.log_off) / params.log_slope_baked).exp() - params.lin_off) / params.lin_slope
    };
    
    e.copysign(mirror_in)
}

/// Apply Linear to Gamma-Log to RGB.
#[inline]
pub fn apply_lin_to_gamma_log(rgb: &mut [f32; 3], params: &GammaLogParams) {
    rgb[0] = lin_to_gamma_log(rgb[0], params);
    rgb[1] = lin_to_gamma_log(rgb[1], params);
    rgb[2] = lin_to_gamma_log(rgb[2], params);
}

/// Apply Gamma-Log to Linear to RGB.
#[inline]
pub fn apply_gamma_log_to_lin(rgb: &mut [f32; 3], params: &GammaLogParams) {
    rgb[0] = gamma_log_to_lin(rgb[0], params);
    rgb[1] = gamma_log_to_lin(rgb[1], params);
    rgb[2] = gamma_log_to_lin(rgb[2], params);
}

/// Apply Linear to Gamma-Log to RGBA buffer.
pub fn apply_lin_to_gamma_log_rgba(pixels: &mut [f32], params: &GammaLogParams) {
    for chunk in pixels.chunks_exact_mut(4) {
        chunk[0] = lin_to_gamma_log(chunk[0], params);
        chunk[1] = lin_to_gamma_log(chunk[1], params);
        chunk[2] = lin_to_gamma_log(chunk[2], params);
    }
}

/// Apply Gamma-Log to Linear to RGBA buffer.
pub fn apply_gamma_log_to_lin_rgba(pixels: &mut [f32], params: &GammaLogParams) {
    for chunk in pixels.chunks_exact_mut(4) {
        chunk[0] = gamma_log_to_lin(chunk[0], params);
        chunk[1] = gamma_log_to_lin(chunk[1], params);
        chunk[2] = gamma_log_to_lin(chunk[2], params);
    }
}

// ============================================================================
// LIN_TO_DOUBLE_LOG / DOUBLE_LOG_TO_LIN
// ============================================================================

/// Parameters for Double-Log curve.
/// 
/// Reference: OCIO FixedFunctionOpCPU.cpp Renderer_LIN_TO_DOUBLE_LOG
/// 
/// The curve has three segments:
/// - Log segment 1 (below break1)
/// - Linear segment (between break1 and break2)
/// - Log segment 2 (above break2)
#[derive(Debug, Clone, Copy)]
pub struct DoubleLogParams {
    /// Log base
    pub base: f32,
    /// First break point
    pub break1: f32,
    /// Second break point
    pub break2: f32,
    /// Log segment 1: log slope
    pub log1_slope: f32,
    /// Log segment 1: log offset
    pub log1_off: f32,
    /// Log segment 1: linear slope
    pub log1_lin_slope: f32,
    /// Log segment 1: linear offset
    pub log1_lin_off: f32,
    /// Log segment 2: log slope
    pub log2_slope: f32,
    /// Log segment 2: log offset
    pub log2_off: f32,
    /// Log segment 2: linear slope
    pub log2_lin_slope: f32,
    /// Log segment 2: linear offset
    pub log2_lin_off: f32,
    /// Linear segment slope
    pub lin_slope: f32,
    /// Linear segment offset
    pub lin_off: f32,
    // Precomputed values
    log1_slope_baked: f32,
    log2_slope_baked: f32,
    prime_break1: f32,
    prime_break2: f32,
}

impl DoubleLogParams {
    /// Create new parameters with precomputed values.
    pub fn new(
        base: f32,
        break1: f32, break2: f32,
        log1_slope: f32, log1_off: f32, log1_lin_slope: f32, log1_lin_off: f32,
        log2_slope: f32, log2_off: f32, log2_lin_slope: f32, log2_lin_off: f32,
        lin_slope: f32, lin_off: f32,
    ) -> Self {
        let ln_base = base.ln();
        let log1_slope_baked = log1_slope / ln_base;
        let log2_slope_baked = log2_slope / ln_base;
        
        // Calculate break points in non-linear domain
        let prime_break1 = log1_slope_baked * (log1_lin_slope * break1 + log1_lin_off).ln() + log1_off;
        let prime_break2 = log2_slope_baked * (log2_lin_slope * break2 + log2_lin_off).ln() + log2_off;
        
        Self {
            base,
            break1,
            break2,
            log1_slope,
            log1_off,
            log1_lin_slope,
            log1_lin_off,
            log2_slope,
            log2_off,
            log2_lin_slope,
            log2_lin_off,
            lin_slope,
            lin_off,
            log1_slope_baked,
            log2_slope_baked,
            prime_break1,
            prime_break2,
        }
    }
}

/// Linear to Double-Log curve (forward).
/// 
/// Three-segment curve with two log segments and one linear.
#[inline]
pub fn lin_to_double_log(v: f32, params: &DoubleLogParams) -> f32 {
    if v < params.break1 {
        // Log segment 1
        params.log1_slope_baked * (params.log1_lin_slope * v + params.log1_lin_off).ln() + params.log1_off
    } else if v < params.break2 {
        // Linear segment
        params.lin_slope * v + params.lin_off
    } else {
        // Log segment 2
        params.log2_slope_baked * (params.log2_lin_slope * v + params.log2_lin_off).ln() + params.log2_off
    }
}

/// Double-Log to Linear curve (inverse).
/// 
/// Inverse of lin_to_double_log.
#[inline]
pub fn double_log_to_lin(v: f32, params: &DoubleLogParams) -> f32 {
    if v < params.prime_break1 {
        // Inverse log segment 1
        (((v - params.log1_off) / params.log1_slope_baked).exp() - params.log1_lin_off) / params.log1_lin_slope
    } else if v < params.prime_break2 {
        // Inverse linear segment
        (v - params.lin_off) / params.lin_slope
    } else {
        // Inverse log segment 2
        (((v - params.log2_off) / params.log2_slope_baked).exp() - params.log2_lin_off) / params.log2_lin_slope
    }
}

/// Apply Linear to Double-Log to RGB.
#[inline]
pub fn apply_lin_to_double_log(rgb: &mut [f32; 3], params: &DoubleLogParams) {
    rgb[0] = lin_to_double_log(rgb[0], params);
    rgb[1] = lin_to_double_log(rgb[1], params);
    rgb[2] = lin_to_double_log(rgb[2], params);
}

/// Apply Double-Log to Linear to RGB.
#[inline]
pub fn apply_double_log_to_lin(rgb: &mut [f32; 3], params: &DoubleLogParams) {
    rgb[0] = double_log_to_lin(rgb[0], params);
    rgb[1] = double_log_to_lin(rgb[1], params);
    rgb[2] = double_log_to_lin(rgb[2], params);
}

/// Apply Linear to Double-Log to RGBA buffer.
pub fn apply_lin_to_double_log_rgba(pixels: &mut [f32], params: &DoubleLogParams) {
    for chunk in pixels.chunks_exact_mut(4) {
        chunk[0] = lin_to_double_log(chunk[0], params);
        chunk[1] = lin_to_double_log(chunk[1], params);
        chunk[2] = lin_to_double_log(chunk[2], params);
    }
}

/// Apply Double-Log to Linear to RGBA buffer.
pub fn apply_double_log_to_lin_rgba(pixels: &mut [f32], params: &DoubleLogParams) {
    for chunk in pixels.chunks_exact_mut(4) {
        chunk[0] = double_log_to_lin(chunk[0], params);
        chunk[1] = double_log_to_lin(chunk[1], params);
        chunk[2] = double_log_to_lin(chunk[2], params);
    }
}

// ============================================================================
// ACES Gamut Compression 1.3
// ============================================================================

/// Parameters for ACES Gamut Compression 1.3.
/// 
/// Reference: OCIO FixedFunctionOpCPU.cpp Renderer_ACES_GamutComp13
#[derive(Debug, Clone, Copy)]
pub struct GamutComp13Params {
    /// Compression limit for cyan (affects red channel)
    pub lim_cyan: f32,
    /// Compression limit for magenta (affects green channel)
    pub lim_magenta: f32,
    /// Compression limit for yellow (affects blue channel)
    pub lim_yellow: f32,
    /// Threshold for cyan
    pub thr_cyan: f32,
    /// Threshold for magenta
    pub thr_magenta: f32,
    /// Threshold for yellow
    pub thr_yellow: f32,
    /// Power exponent
    pub power: f32,
    // Precomputed scale factors
    scale_cyan: f32,
    scale_magenta: f32,
    scale_yellow: f32,
}

impl GamutComp13Params {
    /// Create new parameters with precomputed scale factors.
    pub fn new(
        lim_cyan: f32, lim_magenta: f32, lim_yellow: f32,
        thr_cyan: f32, thr_magenta: f32, thr_yellow: f32,
        power: f32,
    ) -> Self {
        // Precompute scale factor for y = 1 intersect
        // scale = (lim - thr) / pow(pow((1 - thr) / (lim - thr), -power) - 1, 1/power)
        let calc_scale = |lim: f32, thr: f32, pwr: f32| -> f32 {
            let num = lim - thr;
            let inner = ((1.0 - thr) / num).powf(-pwr) - 1.0;
            num / inner.powf(1.0 / pwr)
        };
        
        Self {
            lim_cyan,
            lim_magenta,
            lim_yellow,
            thr_cyan,
            thr_magenta,
            thr_yellow,
            power,
            scale_cyan: calc_scale(lim_cyan, thr_cyan, power),
            scale_magenta: calc_scale(lim_magenta, thr_magenta, power),
            scale_yellow: calc_scale(lim_yellow, thr_yellow, power),
        }
    }
    
    /// ACES default parameters.
    pub fn aces_default() -> Self {
        Self::new(
            1.147, 1.264, 1.312,  // limits
            0.815, 0.803, 0.880,  // thresholds
            1.2,                   // power
        )
    }
}

/// Compress distance from achromatic.
/// 
/// Parameterized shaper function that compresses values above threshold.
#[inline]
fn gamut_compress(dist: f32, thr: f32, scale: f32, power: f32) -> f32 {
    // Normalize distance outside threshold by scale factor
    let nd = (dist - thr) / scale;
    let p = nd.powf(power);
    
    // Compressed distance
    thr + scale * nd / (1.0 + p).powf(1.0 / power)
}

/// Uncompress distance from achromatic.
/// 
/// Inverse of compress function.
#[inline]
fn gamut_uncompress(dist: f32, thr: f32, scale: f32, power: f32) -> f32 {
    // Avoid singularity
    if dist >= (thr + scale) {
        return dist;
    }
    
    // Normalize distance outside threshold by scale factor
    let nd = (dist - thr) / scale;
    let p = nd.powf(power);
    
    // Uncompressed distance
    thr + scale * (-(p / (p - 1.0))).powf(1.0 / power)
}

/// Apply gamut compression to a single channel.
#[inline]
fn gamut_comp_channel<F>(val: f32, ach: f32, thr: f32, scale: f32, power: f32, f: F) -> f32
where
    F: Fn(f32, f32, f32, f32) -> f32,
{
    // Handle zero achromatic (black)
    if ach == 0.0 {
        return 0.0;
    }
    
    // Distance from the achromatic axis, aka inverse RGB ratios
    let dist = (ach - val) / ach.abs();
    
    // No compression below threshold
    if dist < thr {
        return val;
    }
    
    // Compress/uncompress distance with parameterized shaper function
    let compr_dist = f(dist, thr, scale, power);
    
    // Recalculate RGB from compressed distance and achromatic
    ach - compr_dist * ach.abs()
}

/// ACES Gamut Compression 1.3 forward.
/// 
/// Compresses out-of-gamut colors toward the achromatic axis.
#[inline]
pub fn aces_gamut_comp_13_fwd(rgb: &mut [f32; 3], params: &GamutComp13Params) {
    // Achromatic axis = max(R, G, B)
    let ach = rgb[0].max(rgb[1]).max(rgb[2]);
    
    rgb[0] = gamut_comp_channel(rgb[0], ach, params.thr_cyan, params.scale_cyan, params.power, gamut_compress);
    rgb[1] = gamut_comp_channel(rgb[1], ach, params.thr_magenta, params.scale_magenta, params.power, gamut_compress);
    rgb[2] = gamut_comp_channel(rgb[2], ach, params.thr_yellow, params.scale_yellow, params.power, gamut_compress);
}

/// ACES Gamut Compression 1.3 inverse.
/// 
/// Uncompresses gamut-compressed colors back toward original values.
#[inline]
pub fn aces_gamut_comp_13_inv(rgb: &mut [f32; 3], params: &GamutComp13Params) {
    // Achromatic axis = max(R, G, B)
    let ach = rgb[0].max(rgb[1]).max(rgb[2]);
    
    rgb[0] = gamut_comp_channel(rgb[0], ach, params.thr_cyan, params.scale_cyan, params.power, gamut_uncompress);
    rgb[1] = gamut_comp_channel(rgb[1], ach, params.thr_magenta, params.scale_magenta, params.power, gamut_uncompress);
    rgb[2] = gamut_comp_channel(rgb[2], ach, params.thr_yellow, params.scale_yellow, params.power, gamut_uncompress);
}

/// Apply ACES Gamut Compression 1.3 forward to RGBA buffer.
pub fn apply_aces_gamut_comp_13_fwd_rgba(pixels: &mut [f32], params: &GamutComp13Params) {
    for chunk in pixels.chunks_exact_mut(4) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        aces_gamut_comp_13_fwd(&mut rgb, params);
        chunk[0] = rgb[0];
        chunk[1] = rgb[1];
        chunk[2] = rgb[2];
    }
}

/// Apply ACES Gamut Compression 1.3 inverse to RGBA buffer.
pub fn apply_aces_gamut_comp_13_inv_rgba(pixels: &mut [f32], params: &GamutComp13Params) {
    for chunk in pixels.chunks_exact_mut(4) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        aces_gamut_comp_13_inv(&mut rgb, params);
        chunk[0] = rgb[0];
        chunk[1] = rgb[1];
        chunk[2] = rgb[2];
    }
}

// ============================================================================
// RGB <-> HSY (Hue-Saturation-Luma)
// ============================================================================

/// HSY variant determines saturation calculation method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HsyVariant {
    /// Linear: complex saturation for linear light images
    Lin,
    /// Log: simple saturation gain of 4.0 for log-encoded images
    Log,
    /// Video: simple saturation gain of 1.25 for video/display images
    Vid,
}


/// Convert RGB to HSY (Hue-Saturation-Luma).
/// 
/// Unlike typical HSV, HSY maps magenta to hue=0 (not red).
/// This provides better placement for curve manipulation in UIs.
/// 
/// Reference: OCIO FixedFunctionOpCPU.cpp applyRGBToHSY
#[inline]
pub fn rgb_to_hsy(rgb: [f32; 3], variant: HsyVariant) -> [f32; 3] {
    let (red, grn, blu) = (rgb[0], rgb[1], rgb[2]);
    
    let rgb_min = red.min(grn).min(blu);
    let rgb_max = red.max(grn).max(blu);
    
    // BT.709 luma
    let luma = REC709_LUMA_R * red + REC709_LUMA_G * grn + REC709_LUMA_B * blu;
    
    // Distance from luma (chroma-like)
    let rm = red - luma;
    let gm = grn - luma;
    let bm = blu - luma;
    let dist_rgb = rm.abs() + gm.abs() + bm.abs();
    
    // Saturation calculation varies by variant
    let sat = match variant {
        HsyVariant::Lin => {
            // Complex saturation for linear light
            let sum_rgb = red + grn + blu;
            let k = 0.15;
            let sat_hi = dist_rgb / (0.07 * dist_rgb + 1e-6_f32).max(k + sum_rgb);
            let lo_gain = 5.0;
            let sat_lo = dist_rgb * lo_gain;
            let max_lum = 0.01;
            let min_lum = max_lum * 0.1;
            let alpha = ((luma - min_lum) / (max_lum - min_lum)).clamp(0.0, 1.0);
            (sat_lo + alpha * (sat_hi - sat_lo)) * 1.4
        }
        HsyVariant::Log => dist_rgb * 4.0,
        HsyVariant::Vid => dist_rgb * 1.25,
    };
    
    // Hue: magenta at 0 instead of red
    let mut hue = if rgb_min != rgb_max {
        let delta = rgb_max - rgb_min;
        if red == rgb_max {
            1.0 + (grn - blu) / delta
        } else if grn == rgb_max {
            3.0 + (blu - red) / delta
        } else {
            5.0 + (red - grn) / delta
        }
    } else {
        0.0
    } * (1.0 / 6.0);
    
    // Rotate hue 180 deg for negative luma
    if luma < 0.0 {
        hue += 0.5;
        hue -= hue.floor();
    }
    
    [hue, sat, luma]
}

/// Convert HSY (Hue-Saturation-Luma) to RGB.
/// 
/// Inverse of rgb_to_hsy.
/// 
/// Reference: OCIO FixedFunctionOpCPU.cpp applyHSYToRGB
#[inline]
pub fn hsy_to_rgb(hsy: [f32; 3], variant: HsyVariant) -> [f32; 3] {
    let (h_in, sat, luma) = (hsy[0], hsy[1], hsy[2]);
    
    // Shift hue: magenta at 0 -> red at 0
    let mut hue = h_in - 1.0 / 6.0;
    
    // Rotate hue 180 for negative luma
    if luma < 0.0 {
        hue += 0.5;
    }
    hue = (hue - hue.floor()) * 6.0;
    
    // Calculate base RGB from hue (normalized to luma=1)
    let red = ((hue - 3.0).abs() - 1.0).clamp(0.0, 1.0);
    let grn = (2.0 - (hue - 2.0).abs()).clamp(0.0, 1.0);
    let blu = (2.0 - (hue - 4.0).abs()).clamp(0.0, 1.0);
    
    // Scale to match target luma
    let curr_y = REC709_LUMA_R * red + REC709_LUMA_G * grn + REC709_LUMA_B * blu;
    let scale = if curr_y.abs() > 1e-10 { luma / curr_y } else { 0.0 };
    let (red, grn, blu) = (red * scale, grn * scale, blu * scale);
    
    // Distance from luma
    let dist_rgb = (red - luma).abs() + (grn - luma).abs() + (blu - luma).abs();
    
    // Saturation gain (inverse of forward)
    let gain_s = match variant {
        HsyVariant::Lin => hsy_lin_inv_sat(sat, luma, dist_rgb, red + grn + blu),
        HsyVariant::Log => {
            let curr_sat = dist_rgb * 4.0;
            sat / curr_sat.max(1e-10)
        }
        HsyVariant::Vid => {
            let curr_sat = dist_rgb * 1.25;
            sat / curr_sat.max(1e-10)
        }
    };
    
    // Apply saturation
    [
        luma + (red - luma) * gain_s,
        luma + (grn - luma) * gain_s,
        luma + (blu - luma) * gain_s,
    ]
}

/// Complex saturation inversion for HSY_LIN variant.
#[inline]
fn hsy_lin_inv_sat(sat: f32, luma: f32, dist_rgb: f32, sum_rgb: f32) -> f32 {
    const K: f32 = 0.15;
    const LO_GAIN: f32 = 5.0;
    const MAX_LUM: f32 = 0.01;
    const MIN_LUM: f32 = MAX_LUM * 0.1;
    
    // Undo the 1.4 gain
    let sat = sat / 1.4;
    
    // Calculate blend alpha
    let alpha = ((luma - MIN_LUM) / (MAX_LUM - MIN_LUM)).clamp(0.0, 1.0);
    
    if alpha == 1.0 {
        // Pure high-luma formula
        let tmp = (-sat * sum_rgb + sat * 3.0 * luma + dist_rgb).max(1e-6);
        (sat * (K + 3.0 * luma) / tmp).min(50.0)
    } else if alpha == 0.0 {
        // Pure low-luma formula
        sat / (dist_rgb * LO_GAIN).max(1e-10)
    } else {
        // Blended: solve quadratic equation
        let a = dist_rgb * LO_GAIN * (1.0 - alpha) * (sum_rgb - 3.0 * luma);
        let b = dist_rgb * LO_GAIN * (1.0 - alpha) * (K + 3.0 * luma) 
              + dist_rgb * alpha 
              - sat * (sum_rgb - 3.0 * luma);
        let c = -sat * (K + 3.0 * luma);
        
        let discrim = (b * b - 4.0 * a * c).sqrt();
        let denom = -discrim - b;
        let gain_s = (2.0 * c) / denom;
        
        if gain_s >= 0.0 {
            gain_s
        } else {
            (2.0 * c) / (denom + discrim * 2.0)
        }
    }
}

/// Apply RGB to HSY in-place.
#[inline]
pub fn apply_rgb_to_hsy(rgb: &mut [f32; 3], variant: HsyVariant) {
    let hsy = rgb_to_hsy(*rgb, variant);
    rgb[0] = hsy[0];
    rgb[1] = hsy[1];
    rgb[2] = hsy[2];
}

/// Apply HSY to RGB in-place.
#[inline]
pub fn apply_hsy_to_rgb(hsy: &mut [f32; 3], variant: HsyVariant) {
    let rgb = hsy_to_rgb(*hsy, variant);
    hsy[0] = rgb[0];
    hsy[1] = rgb[1];
    hsy[2] = rgb[2];
}

/// Apply RGB to HSY on RGBA buffer.
pub fn apply_rgb_to_hsy_rgba(pixels: &mut [f32], variant: HsyVariant) {
    for chunk in pixels.chunks_exact_mut(4) {
        let rgb = [chunk[0], chunk[1], chunk[2]];
        let hsy = rgb_to_hsy(rgb, variant);
        chunk[0] = hsy[0];
        chunk[1] = hsy[1];
        chunk[2] = hsy[2];
    }
}

/// Apply HSY to RGB on RGBA buffer.
pub fn apply_hsy_to_rgb_rgba(pixels: &mut [f32], variant: HsyVariant) {
    for chunk in pixels.chunks_exact_mut(4) {
        let hsy = [chunk[0], chunk[1], chunk[2]];
        let rgb = hsy_to_rgb(hsy, variant);
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
    // XYZ <-> L*u*v* tests
    // ========================================================================
    
    #[test]
    fn test_xyz_to_luv_d65_white() {
        // D65 white point: XYZ = (0.95047, 1.0, 1.08883)
        // Should give L*=1.0, u*=0, v*=0 (at white point)
        let xyz = [0.95047, 1.0, 1.08883];
        let luv = xyz_to_luv(xyz);
        
        // L* = 1.16 * 1.0^(1/3) - 0.16 = 1.0
        assert!((luv[0] - 1.0).abs() < 0.001, "L* at D65 white: {}", luv[0]);
        // u*, v* should be near 0 (at reference white)
        assert!(luv[1].abs() < 0.001, "u* at D65 white: {}", luv[1]);
        assert!(luv[2].abs() < 0.001, "v* at D65 white: {}", luv[2]);
    }
    
    #[test]
    fn test_xyz_to_luv_black() {
        // Black: Y = 0
        let xyz = [0.0, 0.0, 0.0];
        let luv = xyz_to_luv(xyz);
        
        assert!(luv[0].abs() < EPSILON, "L* at black: {}", luv[0]);
        assert!(luv[1].abs() < EPSILON, "u* at black: {}", luv[1]);
        assert!(luv[2].abs() < EPSILON, "v* at black: {}", luv[2]);
    }
    
    #[test]
    fn test_xyz_luv_roundtrip() {
        let test_values = [
            [0.0, 0.0, 0.0],      // black
            [0.95047, 1.0, 1.08883],  // D65 white
            [0.5, 0.5, 0.5],
            [0.2, 0.3, 0.1],
            [0.05, 0.05, 0.05],   // near black (linear segment)
        ];
        
        for original in test_values {
            let luv = xyz_to_luv(original);
            let xyz = luv_to_xyz(luv);
            
            assert!(
                (xyz[0] - original[0]).abs() < 1e-5,
                "XYZ->Luv roundtrip X failed for {:?}: got {:?}", original, xyz
            );
            assert!(
                (xyz[1] - original[1]).abs() < 1e-5,
                "XYZ->Luv roundtrip Y failed for {:?}: got {:?}", original, xyz
            );
            assert!(
                (xyz[2] - original[2]).abs() < 1e-5,
                "XYZ->Luv roundtrip Z failed for {:?}: got {:?}", original, xyz
            );
        }
    }
    
    #[test]
    fn test_xyz_luv_ocio_reference() {
        // Test values from OCIO FixedFunctionOpCPU_tests.cpp
        // Input: (3600/4095, 3500/4095, 1900/4095)
        // Output: (61659/65535, 28199/65535, 33176/65535)
        let input = [
            3600.0 / 4095.0,
            3500.0 / 4095.0,
            1900.0 / 4095.0,
        ];
        let expected = [
            61659.0 / 65535.0,
            28199.0 / 65535.0,
            33176.0 / 65535.0,
        ];
        
        let luv = xyz_to_luv(input);
        
        assert!(
            (luv[0] - expected[0]).abs() < 1e-4,
            "L* mismatch: expected {}, got {}", expected[0], luv[0]
        );
        assert!(
            (luv[1] - expected[1]).abs() < 1e-4,
            "u* mismatch: expected {}, got {}", expected[1], luv[1]
        );
        assert!(
            (luv[2] - expected[2]).abs() < 1e-4,
            "v* mismatch: expected {}, got {}", expected[2], luv[2]
        );
    }
    
    #[test]
    fn test_xyz_luv_below_break() {
        // Test value below L* break point (Y = 0.008856451679)
        // Input: (50/4095, 30/4095, 19/4095)
        // Output: (4337/65535, 9090/65535, 926/65535)
        let input = [
            50.0 / 4095.0,
            30.0 / 4095.0,
            19.0 / 4095.0,
        ];
        let expected = [
            4337.0 / 65535.0,
            9090.0 / 65535.0,
            926.0 / 65535.0,
        ];
        
        let luv = xyz_to_luv(input);
        
        assert!(
            (luv[0] - expected[0]).abs() < 1e-4,
            "L* below break: expected {}, got {}", expected[0], luv[0]
        );
        assert!(
            (luv[1] - expected[1]).abs() < 1e-3,
            "u* below break: expected {}, got {}", expected[1], luv[1]
        );
        assert!(
            (luv[2] - expected[2]).abs() < 1e-3,
            "v* below break: expected {}, got {}", expected[2], luv[2]
        );
    }
    
    #[test]
    fn test_xyz_luv_rgba() {
        let mut pixels = [
            0.5, 0.5, 0.5, 1.0,
            0.2, 0.3, 0.1, 0.5,
        ];
        let original = pixels;
        
        apply_xyz_to_luv_rgba(&mut pixels);
        apply_luv_to_xyz_rgba(&mut pixels);
        
        // Alpha unchanged
        assert!((pixels[3] - 1.0).abs() < EPSILON);
        assert!((pixels[7] - 0.5).abs() < EPSILON);
        
        // Roundtrip
        for i in [0, 1, 2, 4, 5, 6] {
            assert!(
                (pixels[i] - original[i]).abs() < 1e-5,
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
    // ACES Red Modifier 0.3 tests
    // ========================================================================
    
    #[test]
    fn test_aces_red_mod_03_identity_non_red() {
        // Non-red colors should be unaffected (hue weight = 0)
        let mut blue = [0.1, 0.2, 0.8];
        let original = blue;
        aces_red_mod_03_fwd(&mut blue);
        
        assert!((blue[0] - original[0]).abs() < EPSILON);
        assert!((blue[1] - original[1]).abs() < EPSILON);
        assert!((blue[2] - original[2]).abs() < EPSILON);
    }
    
    #[test]
    fn test_aces_red_mod_03_affects_red() {
        let mut red = [0.8, 0.1, 0.1];
        let original_red = red[0];
        aces_red_mod_03_fwd(&mut red);
        
        // Red channel should have changed
        assert!(red[0] != original_red, "Red should be modified");
    }
    
    #[test]
    fn test_aces_red_mod_03_roundtrip() {
        let test_values = [
            [0.8, 0.1, 0.1],   // saturated red
            [0.6, 0.2, 0.1],   // orange-ish
            [0.5, 0.3, 0.2],   // warm tone
        ];
        
        for original in test_values {
            let mut rgb = original;
            aces_red_mod_03_fwd(&mut rgb);
            aces_red_mod_03_inv(&mut rgb);
            
            assert!(
                (rgb[0] - original[0]).abs() < 1e-3,
                "RedMod03 roundtrip failed for {:?}: got {:?}", original, rgb
            );
        }
    }
    
    #[test]
    fn test_aces_red_mod_03_rgba() {
        let mut pixels = [
            0.8, 0.1, 0.1, 1.0,
            0.1, 0.8, 0.1, 0.5,
        ];
        
        apply_aces_red_mod_03_fwd_rgba(&mut pixels);
        apply_aces_red_mod_03_inv_rgba(&mut pixels);
        
        // Alpha unchanged
        assert!((pixels[3] - 1.0).abs() < EPSILON);
        assert!((pixels[7] - 0.5).abs() < EPSILON);
    }
    
    // ========================================================================
    // ACES Glow 0.3 tests
    // ========================================================================
    
    #[test]
    fn test_aces_glow_03_identity_bright() {
        // Very bright colors should be unaffected
        let mut bright = [1.0, 1.0, 1.0];
        let original = bright;
        aces_glow_03_fwd(&mut bright);
        
        assert!((bright[0] - original[0]).abs() < EPSILON);
        assert!((bright[1] - original[1]).abs() < EPSILON);
        assert!((bright[2] - original[2]).abs() < EPSILON);
    }
    
    #[test]
    fn test_aces_glow_03_affects_dark_saturated() {
        let mut dark = [0.05, 0.01, 0.01];
        let original_sum = dark[0] + dark[1] + dark[2];
        aces_glow_03_fwd(&mut dark);
        
        let new_sum = dark[0] + dark[1] + dark[2];
        assert!(new_sum > original_sum, "Glow should increase luminance");
    }
    
    #[test]
    fn test_aces_glow_03_roundtrip() {
        let test_values = [
            [0.05, 0.03, 0.02],
            [0.1, 0.05, 0.05],
            [0.3, 0.1, 0.1],
        ];
        
        for original in test_values {
            let mut rgb = original;
            aces_glow_03_fwd(&mut rgb);
            aces_glow_03_inv(&mut rgb);
            
            // Relaxed tolerance - inverse uses current YC, not original
            assert!(
                (rgb[0] - original[0]).abs() < 1e-3,
                "Glow03 roundtrip failed for {:?}: got {:?}", original, rgb
            );
        }
    }
    
    #[test]
    fn test_aces_glow_03_rgba() {
        let mut pixels = [
            0.05, 0.03, 0.02, 1.0,
            1.0, 1.0, 1.0, 0.5,
        ];
        let original = pixels;
        
        apply_aces_glow_03_fwd_rgba(&mut pixels);
        apply_aces_glow_03_inv_rgba(&mut pixels);
        
        // Alpha unchanged
        assert!((pixels[3] - 1.0).abs() < EPSILON);
        assert!((pixels[7] - 0.5).abs() < EPSILON);
        
        // Roundtrip
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
    
    // ========================================================================
    // ACES Dark to Dim 1.0 tests
    // ========================================================================
    
    #[test]
    fn test_aces_dark_to_dim_10_affects_values() {
        let mut rgb = [0.5, 0.3, 0.2];
        let original = rgb;
        aces_dark_to_dim_10_fwd(&mut rgb);
        
        // Values should change (gamma != 1)
        assert!(rgb[0] != original[0] || rgb[1] != original[1] || rgb[2] != original[2]);
    }
    
    #[test]
    fn test_aces_dark_to_dim_10_roundtrip() {
        let test_values = [
            [0.18, 0.18, 0.18],  // 18% grey
            [0.5, 0.3, 0.2],
            [0.1, 0.1, 0.1],
            [0.8, 0.6, 0.4],
            [0.01, 0.02, 0.03],
        ];
        
        for original in test_values {
            let mut rgb = original;
            aces_dark_to_dim_10_fwd(&mut rgb);
            aces_dark_to_dim_10_inv(&mut rgb);
            
            assert!(
                (rgb[0] - original[0]).abs() < 1e-4,
                "Dark2Dim roundtrip failed for {:?}: got {:?}", original, rgb
            );
            assert!(
                (rgb[1] - original[1]).abs() < 1e-4,
                "Dark2Dim roundtrip failed for {:?}: got {:?}", original, rgb
            );
            assert!(
                (rgb[2] - original[2]).abs() < 1e-4,
                "Dark2Dim roundtrip failed for {:?}: got {:?}", original, rgb
            );
        }
    }
    
    #[test]
    fn test_aces_dark_to_dim_10_preserves_ratio() {
        // The transform should preserve the ratio between channels
        let mut rgb = [0.6, 0.3, 0.1];
        let ratio_rg = rgb[0] / rgb[1];
        let ratio_rb = rgb[0] / rgb[2];
        
        aces_dark_to_dim_10_fwd(&mut rgb);
        
        let new_ratio_rg = rgb[0] / rgb[1];
        let new_ratio_rb = rgb[0] / rgb[2];
        
        assert!(
            (new_ratio_rg - ratio_rg).abs() < 1e-5,
            "R/G ratio changed: {} vs {}", ratio_rg, new_ratio_rg
        );
        assert!(
            (new_ratio_rb - ratio_rb).abs() < 1e-5,
            "R/B ratio changed: {} vs {}", ratio_rb, new_ratio_rb
        );
    }
    
    #[test]
    fn test_aces_dark_to_dim_10_rgba() {
        let mut pixels = [
            0.5, 0.3, 0.2, 1.0,
            0.18, 0.18, 0.18, 0.5,
        ];
        let original = pixels;
        
        apply_aces_dark_to_dim_10_fwd_rgba(&mut pixels);
        apply_aces_dark_to_dim_10_inv_rgba(&mut pixels);
        
        // Alpha should be unchanged
        assert!((pixels[3] - 1.0).abs() < EPSILON);
        assert!((pixels[7] - 0.5).abs() < EPSILON);
        
        // Should roundtrip
        assert!((pixels[0] - original[0]).abs() < 1e-4);
        assert!((pixels[4] - original[4]).abs() < 1e-4);
    }
    
    // ========================================================================
    // REC.2100 Surround tests
    // ========================================================================
    
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
    
    // ========================================================================
    // LIN_TO_PQ / PQ_TO_LIN tests
    // ========================================================================
    
    #[test]
    fn test_pq_to_lin_zero() {
        let result = pq_to_lin(0.0);
        assert!(result.abs() < EPSILON, "pq_to_lin(0) = {}", result);
    }
    
    #[test]
    fn test_lin_to_pq_zero() {
        let result = lin_to_pq(0.0);
        assert!(result.abs() < EPSILON, "lin_to_pq(0) = {}", result);
    }
    
    #[test]
    fn test_pq_lin_reference_white() {
        // 100 nits = 1.0 in OCIO normalization
        // PQ(100 nits) ≈ 0.508
        let pq_100 = lin_to_pq(1.0);  // 1.0 = 100 nits
        assert!(
            (pq_100 - 0.508).abs() < 0.01,
            "lin_to_pq(1.0) should be ~0.508, got {}", pq_100
        );
        
        // Inverse
        let lin = pq_to_lin(pq_100);
        assert!(
            (lin - 1.0).abs() < 1e-4,
            "pq_to_lin({}) should be ~1.0, got {}", pq_100, lin
        );
    }
    
    #[test]
    fn test_pq_lin_roundtrip() {
        let test_values = [0.0, 0.01, 0.1, 0.5, 1.0, 2.0, 10.0, 50.0, 100.0];
        
        for &lin in &test_values {
            let pq = lin_to_pq(lin);
            let roundtrip = pq_to_lin(pq);
            
            let tol = if lin == 0.0 { EPSILON } else { lin * 1e-4 + 1e-5 };
            assert!(
                (roundtrip - lin).abs() < tol,
                "PQ roundtrip failed for lin={}: got {}", lin, roundtrip
            );
        }
    }
    
    #[test]
    fn test_pq_lin_negative_mirroring() {
        // OCIO handles negatives by mirroring around zero
        let test_values = [-0.1, -0.5, -1.0, -10.0];
        
        for &lin in &test_values {
            let pq = lin_to_pq(lin);
            let pq_pos = lin_to_pq(-lin);
            
            // pq should be negative, with same magnitude as positive
            assert!(pq < 0.0, "lin_to_pq({}) should be negative, got {}", lin, pq);
            assert!(
                (pq + pq_pos).abs() < EPSILON,
                "lin_to_pq({}) = {} should mirror lin_to_pq({}) = {}",
                lin, pq, -lin, pq_pos
            );
        }
    }
    
    #[test]
    fn test_pq_lin_rgba() {
        let mut pixels = [
            1.0, 0.5, 0.1, 1.0,  // Normal values
            -0.1, 0.0, 2.0, 0.5, // With negative and bright
        ];
        let original = pixels;
        
        apply_lin_to_pq_rgba(&mut pixels);
        apply_pq_to_lin_rgba(&mut pixels);
        
        // Alpha unchanged
        assert!((pixels[3] - 1.0).abs() < EPSILON);
        assert!((pixels[7] - 0.5).abs() < EPSILON);
        
        // Should roundtrip
        for i in 0..3 {
            let tol = if original[i] == 0.0 { EPSILON } else { original[i].abs() * 1e-4 + 1e-4 };
            assert!(
                (pixels[i] - original[i]).abs() < tol,
                "Channel {} roundtrip failed: {} vs {}", i, pixels[i], original[i]
            );
        }
    }
    
    // ========================================================================
    // LIN_TO_GAMMA_LOG / GAMMA_LOG_TO_LIN tests
    // ========================================================================
    
    fn sample_gamma_log_params() -> GammaLogParams {
        // Parameters that form a continuous curve at the break point
        // The key is that at break, both segments must give the same value.
        // gamma at break: slope * (break + off)^power
        // log at break: (log_slope/ln(base)) * ln(lin_slope * break + lin_off) + log_off
        // 
        // Using a simpler curve where both segments connect properly:
        let mirror = 0.0_f32;
        let break_pt = 0.01_f32;
        let gamma_power = 0.5_f32;  // sqrt for gamma segment
        let gamma_slope = 1.0_f32;
        let gamma_off = 0.0_f32;
        let log_base = 10.0_f32;
        let log_slope = 1.0_f32;
        let lin_slope = 1.0_f32;
        let lin_off = 1.0_f32;  // Makes ln(lin_slope * break + lin_off) = ln(1.01) at break
        
        // Calculate log_off for continuity:
        // gamma at break = 1.0 * (0.01 + 0)^0.5 = 0.1
        // log at break = (1/ln10) * ln(1*0.01 + 1) + log_off = 0.00434 + log_off
        // For continuity: log_off = 0.1 - 0.00434 = 0.09566
        let gamma_at_break = gamma_slope * (break_pt + gamma_off).powf(gamma_power);
        let log_slope_baked = log_slope / log_base.ln();
        let log_at_break_without_off = log_slope_baked * (lin_slope * break_pt + lin_off).ln();
        let log_off = gamma_at_break - log_at_break_without_off;
        
        GammaLogParams::new(
            mirror, break_pt,
            gamma_power, gamma_slope, gamma_off,
            log_base, log_slope, log_off,
            lin_slope, lin_off,
        )
    }
    
    #[test]
    fn test_gamma_log_roundtrip() {
        let params = sample_gamma_log_params();
        // Test values in both segments and at the break point
        let test_values = [0.0, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0];
        
        for &v in &test_values {
            let encoded = lin_to_gamma_log(v, &params);
            let decoded = gamma_log_to_lin(encoded, &params);
            
            let tol = if v == 0.0 { 1e-6 } else { v * 1e-4 + 1e-5 };
            assert!(
                (decoded - v).abs() < tol,
                "GammaLog roundtrip failed for {}: encoded={}, decoded={}", v, encoded, decoded
            );
        }
    }
    
    #[test]
    fn test_gamma_log_negative_mirroring() {
        let params = sample_gamma_log_params();
        let test_values = [-0.01, -0.1, -0.5];
        
        for &v in &test_values {
            let encoded = lin_to_gamma_log(v, &params);
            let encoded_pos = lin_to_gamma_log(-v, &params);
            
            // Should be mirrored
            assert!(encoded < 0.0, "lin_to_gamma_log({}) should be negative", v);
            assert!(
                (encoded + encoded_pos).abs() < 1e-5,
                "Mirroring failed: {} vs {}", encoded, encoded_pos
            );
        }
    }
    
    #[test]
    fn test_gamma_log_rgba() {
        let params = sample_gamma_log_params();
        let mut pixels = [
            0.1, 0.05, 0.005, 1.0,
            0.5, 0.2, 0.001, 0.5,
        ];
        let original = pixels;
        
        apply_lin_to_gamma_log_rgba(&mut pixels, &params);
        apply_gamma_log_to_lin_rgba(&mut pixels, &params);
        
        // Alpha unchanged
        assert!((pixels[3] - 1.0).abs() < EPSILON);
        assert!((pixels[7] - 0.5).abs() < EPSILON);
        
        // Roundtrip
        for i in 0..3 {
            assert!(
                (pixels[i] - original[i]).abs() < 1e-4,
                "Channel {} roundtrip failed", i
            );
        }
    }
    
    // ========================================================================
    // LIN_TO_DOUBLE_LOG / DOUBLE_LOG_TO_LIN tests
    // ========================================================================
    
    fn sample_double_log_params() -> DoubleLogParams {
        // Sample parameters for a double-log curve
        DoubleLogParams::new(
            10.0,   // base
            0.01,   // break1
            0.5,    // break2
            0.3,    // log1 slope
            0.1,    // log1 offset
            10.0,   // log1 lin slope
            0.01,   // log1 lin offset
            0.3,    // log2 slope
            0.5,    // log2 offset
            2.0,    // log2 lin slope
            0.1,    // log2 lin offset
            0.8,    // lin slope
            0.15,   // lin offset
        )
    }
    
    #[test]
    fn test_double_log_roundtrip() {
        let params = sample_double_log_params();
        let test_values = [0.005, 0.01, 0.05, 0.1, 0.3, 0.5, 0.8, 1.0];
        
        for &v in &test_values {
            let encoded = lin_to_double_log(v, &params);
            let decoded = double_log_to_lin(encoded, &params);
            
            let tol = v * 1e-4 + 1e-5;
            assert!(
                (decoded - v).abs() < tol,
                "DoubleLog roundtrip failed for {}: encoded={}, decoded={}", v, encoded, decoded
            );
        }
    }
    
    #[test]
    fn test_double_log_segments() {
        let params = sample_double_log_params();
        
        // Test value in each segment
        let v1 = 0.005;  // Below break1 (log segment 1)
        let v2 = 0.2;    // Between breaks (linear segment)
        let v3 = 0.8;    // Above break2 (log segment 2)
        
        let e1 = lin_to_double_log(v1, &params);
        let e2 = lin_to_double_log(v2, &params);
        let e3 = lin_to_double_log(v3, &params);
        
        // All should be finite
        assert!(e1.is_finite());
        assert!(e2.is_finite());
        assert!(e3.is_finite());
        
        // Should be monotonically increasing
        assert!(e1 < e2, "e1={} should be < e2={}", e1, e2);
        assert!(e2 < e3, "e2={} should be < e3={}", e2, e3);
    }
    
    #[test]
    fn test_double_log_rgba() {
        let params = sample_double_log_params();
        let mut pixels = [
            0.1, 0.3, 0.7, 1.0,
            0.01, 0.2, 0.5, 0.5,
        ];
        let original = pixels;
        
        apply_lin_to_double_log_rgba(&mut pixels, &params);
        apply_double_log_to_lin_rgba(&mut pixels, &params);
        
        // Alpha unchanged
        assert!((pixels[3] - 1.0).abs() < EPSILON);
        assert!((pixels[7] - 0.5).abs() < EPSILON);
        
        // Roundtrip
        for i in 0..3 {
            assert!(
                (pixels[i] - original[i]).abs() < 1e-4,
                "Channel {} roundtrip failed", i
            );
        }
    }
    
    // ========================================================================
    // ACES Gamut Compression 1.3 tests
    // ========================================================================
    
    #[test]
    fn test_gamut_comp_13_in_gamut_unchanged() {
        // Colors within gamut (dist < threshold) should be unchanged
        let params = GamutComp13Params::aces_default();
        let mut rgb = [0.5, 0.4, 0.3];  // In-gamut color
        let original = rgb;
        
        aces_gamut_comp_13_fwd(&mut rgb, &params);
        
        // In-gamut colors should pass through unchanged
        assert!((rgb[0] - original[0]).abs() < EPSILON);
        assert!((rgb[1] - original[1]).abs() < EPSILON);
        assert!((rgb[2] - original[2]).abs() < EPSILON);
    }
    
    #[test]
    fn test_gamut_comp_13_achromatic_unchanged() {
        // Achromatic colors (R=G=B) should be unchanged
        let params = GamutComp13Params::aces_default();
        let mut rgb = [0.5, 0.5, 0.5];
        let original = rgb;
        
        aces_gamut_comp_13_fwd(&mut rgb, &params);
        
        assert!((rgb[0] - original[0]).abs() < EPSILON);
        assert!((rgb[1] - original[1]).abs() < EPSILON);
        assert!((rgb[2] - original[2]).abs() < EPSILON);
    }
    
    #[test]
    fn test_gamut_comp_13_black_unchanged() {
        // Black should remain black
        let params = GamutComp13Params::aces_default();
        let mut rgb = [0.0, 0.0, 0.0];
        
        aces_gamut_comp_13_fwd(&mut rgb, &params);
        
        assert!(rgb[0].abs() < EPSILON);
        assert!(rgb[1].abs() < EPSILON);
        assert!(rgb[2].abs() < EPSILON);
    }
    
    #[test]
    fn test_gamut_comp_13_negative_compressed() {
        // Out-of-gamut color (negative component) should be compressed
        let params = GamutComp13Params::aces_default();
        let mut rgb = [1.0, 0.5, -0.2];  // Blue is negative (out of gamut)
        let original = rgb;
        
        aces_gamut_comp_13_fwd(&mut rgb, &params);
        
        // Red unchanged (max), green may change, blue should increase (toward 0)
        assert!((rgb[0] - original[0]).abs() < EPSILON, "Red should be unchanged (achromatic)");
        // Blue should be compressed (less negative or positive)
        assert!(rgb[2] > original[2], "Blue should be compressed toward achromatic: {} vs {}", rgb[2], original[2]);
    }
    
    #[test]
    fn test_gamut_comp_13_roundtrip() {
        let params = GamutComp13Params::aces_default();
        let test_values = [
            [0.5, 0.4, 0.3],           // In gamut
            [1.0, 0.5, -0.1],          // Negative blue
            [0.8, 1.0, -0.05],         // Negative blue, green max
            [1.0, -0.1, 0.5],          // Negative green
            [0.3, 0.3, 0.3],           // Achromatic
        ];
        
        for original in test_values {
            let mut rgb = original;
            aces_gamut_comp_13_fwd(&mut rgb, &params);
            aces_gamut_comp_13_inv(&mut rgb, &params);
            
            assert!(
                (rgb[0] - original[0]).abs() < 1e-4,
                "GamutComp13 roundtrip failed for {:?}: got {:?}", original, rgb
            );
            assert!(
                (rgb[1] - original[1]).abs() < 1e-4,
                "GamutComp13 roundtrip failed for {:?}: got {:?}", original, rgb
            );
            assert!(
                (rgb[2] - original[2]).abs() < 1e-4,
                "GamutComp13 roundtrip failed for {:?}: got {:?}", original, rgb
            );
        }
    }
    
    #[test]
    fn test_gamut_comp_13_rgba() {
        let params = GamutComp13Params::aces_default();
        let mut pixels = [
            1.0, 0.5, -0.1, 1.0,
            0.5, 0.4, 0.3, 0.5,
        ];
        let original = pixels;
        
        apply_aces_gamut_comp_13_fwd_rgba(&mut pixels, &params);
        apply_aces_gamut_comp_13_inv_rgba(&mut pixels, &params);
        
        // Alpha should be unchanged
        assert!((pixels[3] - 1.0).abs() < EPSILON);
        assert!((pixels[7] - 0.5).abs() < EPSILON);
        
        // Should roundtrip
        assert!((pixels[0] - original[0]).abs() < 1e-4);
        assert!((pixels[4] - original[4]).abs() < 1e-4);
    }
    
    #[test]
    fn test_gamut_comp_13_scale_precomputation() {
        // Verify scale factors are computed correctly
        let params = GamutComp13Params::aces_default();
        
        // Scale factors should be positive and reasonable
        assert!(params.scale_cyan > 0.0 && params.scale_cyan < 1.0);
        assert!(params.scale_magenta > 0.0 && params.scale_magenta < 1.0);
        assert!(params.scale_yellow > 0.0 && params.scale_yellow < 1.0);
    }
    
    // ========================================================================
    // RGB <-> HSY tests
    // ========================================================================
    
    #[test]
    fn test_hsy_achromatic() {
        // Achromatic colors (R=G=B) should have saturation=0
        for variant in [HsyVariant::Lin, HsyVariant::Log, HsyVariant::Vid] {
            let rgb = [0.5, 0.5, 0.5];
            let hsy = rgb_to_hsy(rgb, variant);
            
            // Saturation should be 0
            assert!(hsy[1].abs() < EPSILON, "Achromatic sat should be 0 for {:?}, got {}", variant, hsy[1]);
            
            // Luma should be 0.5
            assert!((hsy[2] - 0.5).abs() < EPSILON, "Luma should be 0.5 for {:?}", variant);
        }
    }
    
    #[test]
    fn test_hsy_black() {
        // Black should remain black
        for variant in [HsyVariant::Lin, HsyVariant::Log, HsyVariant::Vid] {
            let rgb = [0.0, 0.0, 0.0];
            let hsy = rgb_to_hsy(rgb, variant);
            
            assert!(hsy[1].abs() < EPSILON, "Black sat should be 0");
            assert!(hsy[2].abs() < EPSILON, "Black luma should be 0");
        }
    }
    
    #[test]
    fn test_hsy_white() {
        // White should have saturation 0
        for variant in [HsyVariant::Lin, HsyVariant::Log, HsyVariant::Vid] {
            let rgb = [1.0, 1.0, 1.0];
            let hsy = rgb_to_hsy(rgb, variant);
            
            assert!(hsy[1].abs() < EPSILON, "White sat should be 0");
            assert!((hsy[2] - 1.0).abs() < EPSILON, "White luma should be 1.0");
        }
    }
    
    #[test]
    fn test_hsy_log_roundtrip() {
        // LOG variant should roundtrip well
        let test_colors = [
            [0.8, 0.4, 0.2],  // Orange-ish
            [0.2, 0.6, 0.4],  // Green-ish
            [0.3, 0.3, 0.8],  // Blue-ish
            [0.5, 0.5, 0.5],  // Grey
            [0.18, 0.18, 0.18], // 18% grey
        ];
        
        for original in test_colors {
            let hsy = rgb_to_hsy(original, HsyVariant::Log);
            let back = hsy_to_rgb(hsy, HsyVariant::Log);
            
            for i in 0..3 {
                assert!(
                    (back[i] - original[i]).abs() < 1e-4,
                    "HSY_LOG roundtrip failed for {:?}: got {:?}", original, back
                );
            }
        }
    }
    
    #[test]
    fn test_hsy_vid_roundtrip() {
        // VID variant should roundtrip well
        let test_colors = [
            [0.8, 0.4, 0.2],
            [0.2, 0.6, 0.4],
            [0.3, 0.3, 0.8],
            [0.5, 0.5, 0.5],
        ];
        
        for original in test_colors {
            let hsy = rgb_to_hsy(original, HsyVariant::Vid);
            let back = hsy_to_rgb(hsy, HsyVariant::Vid);
            
            for i in 0..3 {
                assert!(
                    (back[i] - original[i]).abs() < 1e-4,
                    "HSY_VID roundtrip failed for {:?}: got {:?}", original, back
                );
            }
        }
    }
    
    #[test]
    fn test_hsy_lin_roundtrip() {
        // LIN variant with moderate values
        let test_colors = [
            [0.2, 0.1, 0.05],  // Low light
            [0.5, 0.3, 0.2],   // Mid tones
            [0.8, 0.5, 0.3],   // Highlights
        ];
        
        for original in test_colors {
            let hsy = rgb_to_hsy(original, HsyVariant::Lin);
            let back = hsy_to_rgb(hsy, HsyVariant::Lin);
            
            for i in 0..3 {
                assert!(
                    (back[i] - original[i]).abs() < 1e-3,
                    "HSY_LIN roundtrip failed for {:?}: got {:?}", original, back
                );
            }
        }
    }
    
    #[test]
    fn test_hsy_magenta_hue() {
        // Magenta should have hue near 0 (not red!)
        let magenta = [1.0, 0.0, 1.0];
        let hsy = rgb_to_hsy(magenta, HsyVariant::Log);
        
        // Hue should be near 0 or 1 (wrapped)
        assert!(
            hsy[0] < 0.1 || hsy[0] > 0.9,
            "Magenta hue should be near 0, got {}", hsy[0]
        );
    }
    
    #[test]
    fn test_hsy_red_hue() {
        // Red should have hue ~1/6 (shifted from 0)
        let red = [1.0, 0.0, 0.0];
        let hsy = rgb_to_hsy(red, HsyVariant::Log);
        
        // Red should be around 1/6
        assert!(
            (hsy[0] - 1.0/6.0).abs() < 0.02,
            "Red hue should be ~1/6, got {}", hsy[0]
        );
    }
    
    #[test]
    fn test_hsy_rgba() {
        let mut pixels = [
            0.6, 0.4, 0.2, 1.0,
            0.3, 0.5, 0.7, 0.5,
        ];
        let original = pixels;
        
        apply_rgb_to_hsy_rgba(&mut pixels, HsyVariant::Log);
        apply_hsy_to_rgb_rgba(&mut pixels, HsyVariant::Log);
        
        // Alpha unchanged
        assert!((pixels[3] - 1.0).abs() < EPSILON);
        assert!((pixels[7] - 0.5).abs() < EPSILON);
        
        // Roundtrip
        for i in 0..3 {
            assert!(
                (pixels[i] - original[i]).abs() < 1e-4,
                "RGBA roundtrip failed at {}", i
            );
        }
    }
}
