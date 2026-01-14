//! Grading Primary operations (lift/gamma/gain style color grading).
//!
//! Reference: OCIO GradingPrimaryOpCPU.cpp
//!
//! Provides professional color grading controls including:
//! - Brightness/Offset
//! - Contrast
//! - Gamma
//! - Lift/Gain
//! - Exposure
//! - Saturation
//!
//! Three styles are supported:
//! - Log: For log-encoded footage
//! - Linear: For scene-linear footage
//! - Video: For video/display-referred footage

use vfx_core::{REC709_LUMA_B, REC709_LUMA_G, REC709_LUMA_R};

/// Minimum value to prevent division by zero in contrast/exposure/slope.
const MIN_DIVISOR: f32 = 1e-6;

/// Grading style determines the math used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GradingStyle {
    /// Logarithmic (for log footage).
    #[default]
    Log,
    /// Linear (scene-referred).
    Linear,
    /// Video (display-referred).
    Video,
}

/// RGBM (Red, Green, Blue, Master) control.
/// Master applies equally to all channels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradingRGBM {
    /// Red channel adjustment.
    pub red: f32,
    /// Green channel adjustment.
    pub green: f32,
    /// Blue channel adjustment.
    pub blue: f32,
    /// Master (applies to all channels).
    pub master: f32,
}

impl GradingRGBM {
    /// Create with all zeros.
    pub fn zero() -> Self {
        Self { red: 0.0, green: 0.0, blue: 0.0, master: 0.0 }
    }

    /// Create with all ones.
    pub fn one() -> Self {
        Self { red: 1.0, green: 1.0, blue: 1.0, master: 1.0 }
    }

    /// Create with uniform value.
    pub fn uniform(v: f32) -> Self {
        Self { red: v, green: v, blue: v, master: v }
    }

    /// Get effective RGB values (channel + master for additive ops).
    #[inline]
    pub fn rgb_add(&self) -> [f32; 3] {
        [
            self.red + self.master,
            self.green + self.master,
            self.blue + self.master,
        ]
    }

    /// Get effective RGB values (channel * master for multiplicative ops).
    #[inline]
    pub fn rgb_mul(&self) -> [f32; 3] {
        [
            self.red * self.master,
            self.green * self.master,
            self.blue * self.master,
        ]
    }
}

impl Default for GradingRGBM {
    fn default() -> Self {
        Self::zero()
    }
}

/// Grading Primary parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct GradingPrimary {
    /// Grading style (Log, Linear, Video).
    pub style: GradingStyle,
    
    /// Brightness adjustment (additive, for Log style).
    pub brightness: GradingRGBM,
    /// Contrast adjustment (multiplicative around pivot).
    pub contrast: GradingRGBM,
    /// Gamma adjustment (power function).
    pub gamma: GradingRGBM,
    /// Offset adjustment (additive, for Linear/Video styles).
    pub offset: GradingRGBM,
    /// Exposure adjustment (multiplicative, for Linear style).
    pub exposure: GradingRGBM,
    /// Lift adjustment (shadow control for Video style).
    pub lift: GradingRGBM,
    /// Gain adjustment (highlight control for Video style).
    pub gain: GradingRGBM,
    
    /// Saturation (1.0 = no change).
    pub saturation: f32,
    /// Pivot for contrast adjustment.
    pub pivot: f32,
    /// Black pivot for gamma.
    pub pivot_black: f32,
    /// White pivot for gamma.
    pub pivot_white: f32,
    /// Black clamp value.
    pub clamp_black: f32,
    /// White clamp value.
    pub clamp_white: f32,
}

impl GradingPrimary {
    /// Create identity grading for the given style.
    pub fn identity(style: GradingStyle) -> Self {
        let pivot = match style {
            GradingStyle::Log => -0.2,
            GradingStyle::Linear | GradingStyle::Video => 0.18,
        };
        
        Self {
            style,
            brightness: GradingRGBM::zero(),
            contrast: GradingRGBM::one(),
            gamma: GradingRGBM::one(),
            offset: GradingRGBM::zero(),
            exposure: GradingRGBM::zero(),
            lift: GradingRGBM::zero(),
            gain: GradingRGBM::one(),
            saturation: 1.0,
            pivot,
            pivot_black: 0.0,
            pivot_white: 1.0,
            clamp_black: f32::NEG_INFINITY,
            clamp_white: f32::INFINITY,
        }
    }

    /// Check if this is an identity operation.
    pub fn is_identity(&self) -> bool {
        self.saturation == 1.0
            && self.brightness == GradingRGBM::zero()
            && self.contrast == GradingRGBM::one()
            && self.gamma == GradingRGBM::one()
            && self.offset == GradingRGBM::zero()
            && self.exposure == GradingRGBM::zero()
            && self.lift == GradingRGBM::zero()
            && self.gain == GradingRGBM::one()
    }

    /// Apply forward grading.
    #[inline]
    pub fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        match self.style {
            GradingStyle::Log => self.apply_log(rgb),
            GradingStyle::Linear => self.apply_linear(rgb),
            GradingStyle::Video => self.apply_video(rgb),
        }
    }

    /// Apply inverse grading.
    #[inline]
    pub fn apply_inverse(&self, rgb: [f32; 3]) -> [f32; 3] {
        match self.style {
            GradingStyle::Log => self.apply_log_inv(rgb),
            GradingStyle::Linear => self.apply_linear_inv(rgb),
            GradingStyle::Video => self.apply_video_inv(rgb),
        }
    }

    // ========================================================================
    // Log style
    // ========================================================================

    /// Log style forward.
    /// out = in + brightness
    /// out = (out - pivot) * contrast + pivot
    /// out = pow(abs(out - pivotBlack) / range, gamma) * sign * range + pivotBlack
    /// out = luma + saturation * (out - luma)
    /// out = clamp(out, clampBlack, clampWhite)
    fn apply_log(&self, rgb: [f32; 3]) -> [f32; 3] {
        let brightness = self.compute_brightness();
        let contrast = self.compute_contrast();
        let gamma = self.compute_gamma();
        let actual_pivot = self.compute_pivot();
        
        let mut out = rgb;
        
        // Brightness
        out[0] += brightness[0];
        out[1] += brightness[1];
        out[2] += brightness[2];
        
        // Contrast around pivot
        out[0] = (out[0] - actual_pivot) * contrast[0] + actual_pivot;
        out[1] = (out[1] - actual_pivot) * contrast[1] + actual_pivot;
        out[2] = (out[2] - actual_pivot) * contrast[2] + actual_pivot;
        
        // Gamma
        if !self.is_gamma_identity() {
            let range = self.pivot_white - self.pivot_black;
            out = [
                self.apply_gamma_channel(out[0], gamma[0], range),
                self.apply_gamma_channel(out[1], gamma[1], range),
                self.apply_gamma_channel(out[2], gamma[2], range),
            ];
        }
        
        // Saturation
        out = self.apply_saturation(out);
        
        // Clamp
        self.apply_clamp(out)
    }

    /// Log style inverse.
    fn apply_log_inv(&self, rgb: [f32; 3]) -> [f32; 3] {
        let brightness = self.compute_brightness();
        let contrast = self.compute_contrast();
        let gamma = self.compute_gamma();
        let actual_pivot = self.compute_pivot();
        
        let mut out = rgb;
        
        // Clamp
        out = self.apply_clamp(out);
        
        // Inverse saturation
        out = self.apply_saturation_inv(out);
        
        // Inverse gamma (clamp to avoid division by zero)
        if !self.is_gamma_identity() {
            let range = self.pivot_white - self.pivot_black;
            let inv_gamma = [
                1.0 / gamma[0].abs().max(MIN_DIVISOR),
                1.0 / gamma[1].abs().max(MIN_DIVISOR),
                1.0 / gamma[2].abs().max(MIN_DIVISOR),
            ];
            out = [
                self.apply_gamma_channel(out[0], inv_gamma[0], range),
                self.apply_gamma_channel(out[1], inv_gamma[1], range),
                self.apply_gamma_channel(out[2], inv_gamma[2], range),
            ];
        }
        
        // Inverse contrast (clamp to avoid division by zero)
        let inv_contrast = [
            1.0 / contrast[0].abs().max(MIN_DIVISOR),
            1.0 / contrast[1].abs().max(MIN_DIVISOR),
            1.0 / contrast[2].abs().max(MIN_DIVISOR),
        ];
        out[0] = (out[0] - actual_pivot) * inv_contrast[0] + actual_pivot;
        out[1] = (out[1] - actual_pivot) * inv_contrast[1] + actual_pivot;
        out[2] = (out[2] - actual_pivot) * inv_contrast[2] + actual_pivot;
        
        // Inverse brightness
        out[0] -= brightness[0];
        out[1] -= brightness[1];
        out[2] -= brightness[2];
        
        out
    }

    // ========================================================================
    // Linear style
    // ========================================================================

    /// Linear style forward.
    /// out = (in + offset) * pow(2, exposure)
    /// out = pow(abs(out / pivot), contrast) * sign(out) * pivot
    /// out = luma + saturation * (out - luma)
    fn apply_linear(&self, rgb: [f32; 3]) -> [f32; 3] {
        let offset = self.compute_offset();
        let exposure = self.compute_exposure();
        let contrast = self.compute_contrast();
        let actual_pivot = self.compute_pivot();
        
        let mut out = rgb;
        
        // Offset
        out[0] += offset[0];
        out[1] += offset[1];
        out[2] += offset[2];
        
        // Exposure
        out[0] *= exposure[0];
        out[1] *= exposure[1];
        out[2] *= exposure[2];
        
        // Contrast
        if !self.is_contrast_identity() {
            out[0] = (out[0] / actual_pivot).abs().powf(contrast[0]) * out[0].signum() * actual_pivot;
            out[1] = (out[1] / actual_pivot).abs().powf(contrast[1]) * out[1].signum() * actual_pivot;
            out[2] = (out[2] / actual_pivot).abs().powf(contrast[2]) * out[2].signum() * actual_pivot;
        }
        
        // Saturation
        out = self.apply_saturation(out);
        
        // Clamp
        self.apply_clamp(out)
    }

    /// Linear style inverse.
    fn apply_linear_inv(&self, rgb: [f32; 3]) -> [f32; 3] {
        let offset = self.compute_offset();
        let exposure = self.compute_exposure();
        let contrast = self.compute_contrast();
        let actual_pivot = self.compute_pivot();
        
        let mut out = rgb;
        
        // Clamp
        out = self.apply_clamp(out);
        
        // Inverse saturation
        out = self.apply_saturation_inv(out);
        
        // Inverse contrast (clamp to avoid division by zero)
        if !self.is_contrast_identity() {
            let inv_contrast = [
                1.0 / contrast[0].abs().max(MIN_DIVISOR),
                1.0 / contrast[1].abs().max(MIN_DIVISOR),
                1.0 / contrast[2].abs().max(MIN_DIVISOR),
            ];
            out[0] = (out[0] / actual_pivot).abs().powf(inv_contrast[0]) * out[0].signum() * actual_pivot;
            out[1] = (out[1] / actual_pivot).abs().powf(inv_contrast[1]) * out[1].signum() * actual_pivot;
            out[2] = (out[2] / actual_pivot).abs().powf(inv_contrast[2]) * out[2].signum() * actual_pivot;
        }
        
        // Inverse exposure (clamp to avoid division by zero)
        let inv_exposure = [
            1.0 / exposure[0].abs().max(MIN_DIVISOR),
            1.0 / exposure[1].abs().max(MIN_DIVISOR),
            1.0 / exposure[2].abs().max(MIN_DIVISOR),
        ];
        out[0] *= inv_exposure[0];
        out[1] *= inv_exposure[1];
        out[2] *= inv_exposure[2];
        
        // Inverse offset
        out[0] -= offset[0];
        out[1] -= offset[1];
        out[2] -= offset[2];
        
        out
    }

    // ========================================================================
    // Video style
    // ========================================================================

    /// Video style forward.
    /// out = in + (lift + offset)
    /// out = (out - pivotBlack) * slope + pivotBlack
    /// out = pow(abs(out - pivotBlack) / range, gamma) * sign * range + pivotBlack
    /// out = luma + saturation * (out - luma)
    fn apply_video(&self, rgb: [f32; 3]) -> [f32; 3] {
        let offset = self.compute_video_offset();
        let slope = self.compute_slope();
        let gamma = self.compute_gamma();
        
        let mut out = rgb;
        
        // Offset (lift + offset combined)
        out[0] += offset[0];
        out[1] += offset[1];
        out[2] += offset[2];
        
        // Slope (contrast around black pivot)
        out[0] = (out[0] - self.pivot_black) * slope[0] + self.pivot_black;
        out[1] = (out[1] - self.pivot_black) * slope[1] + self.pivot_black;
        out[2] = (out[2] - self.pivot_black) * slope[2] + self.pivot_black;
        
        // Gamma
        if !self.is_gamma_identity() {
            let range = self.pivot_white - self.pivot_black;
            out = [
                self.apply_gamma_channel(out[0], gamma[0], range),
                self.apply_gamma_channel(out[1], gamma[1], range),
                self.apply_gamma_channel(out[2], gamma[2], range),
            ];
        }
        
        // Saturation
        out = self.apply_saturation(out);
        
        // Clamp
        self.apply_clamp(out)
    }

    /// Video style inverse.
    fn apply_video_inv(&self, rgb: [f32; 3]) -> [f32; 3] {
        let offset = self.compute_video_offset();
        let slope = self.compute_slope();
        let gamma = self.compute_gamma();
        
        let mut out = rgb;
        
        // Clamp
        out = self.apply_clamp(out);
        
        // Inverse saturation
        out = self.apply_saturation_inv(out);
        
        // Inverse gamma (clamp to avoid division by zero)
        if !self.is_gamma_identity() {
            let range = self.pivot_white - self.pivot_black;
            let inv_gamma = [
                1.0 / gamma[0].abs().max(MIN_DIVISOR),
                1.0 / gamma[1].abs().max(MIN_DIVISOR),
                1.0 / gamma[2].abs().max(MIN_DIVISOR),
            ];
            out = [
                self.apply_gamma_channel(out[0], inv_gamma[0], range),
                self.apply_gamma_channel(out[1], inv_gamma[1], range),
                self.apply_gamma_channel(out[2], inv_gamma[2], range),
            ];
        }
        
        // Inverse slope (clamp to avoid division by zero)
        let inv_slope = [
            1.0 / slope[0].abs().max(MIN_DIVISOR),
            1.0 / slope[1].abs().max(MIN_DIVISOR),
            1.0 / slope[2].abs().max(MIN_DIVISOR),
        ];
        out[0] = (out[0] - self.pivot_black) * inv_slope[0] + self.pivot_black;
        out[1] = (out[1] - self.pivot_black) * inv_slope[1] + self.pivot_black;
        out[2] = (out[2] - self.pivot_black) * inv_slope[2] + self.pivot_black;
        
        // Inverse offset
        out[0] -= offset[0];
        out[1] -= offset[1];
        out[2] -= offset[2];
        
        out
    }

    // ========================================================================
    // Helper methods
    // ========================================================================

    fn compute_brightness(&self) -> [f32; 3] {
        self.brightness.rgb_add()
    }

    fn compute_contrast(&self) -> [f32; 3] {
        self.contrast.rgb_mul()
    }

    fn compute_gamma(&self) -> [f32; 3] {
        self.gamma.rgb_mul()
    }

    fn compute_offset(&self) -> [f32; 3] {
        self.offset.rgb_add()
    }

    fn compute_exposure(&self) -> [f32; 3] {
        let exp = self.exposure.rgb_add();
        [2.0_f32.powf(exp[0]), 2.0_f32.powf(exp[1]), 2.0_f32.powf(exp[2])]
    }

    fn compute_video_offset(&self) -> [f32; 3] {
        let lift = self.lift.rgb_add();
        let offset = self.offset.rgb_add();
        [lift[0] + offset[0], lift[1] + offset[1], lift[2] + offset[2]]
    }

    fn compute_slope(&self) -> [f32; 3] {
        // For video: slope combines gain and contrast
        let gain = self.gain.rgb_mul();
        let contrast = self.contrast.rgb_mul();
        [gain[0] * contrast[0], gain[1] * contrast[1], gain[2] * contrast[2]]
    }

    fn compute_pivot(&self) -> f32 {
        // For log style, pivot is already the log value
        // For linear/video, use pivot directly
        self.pivot
    }

    fn is_gamma_identity(&self) -> bool {
        let g = self.compute_gamma();
        (g[0] - 1.0).abs() < 1e-6 && (g[1] - 1.0).abs() < 1e-6 && (g[2] - 1.0).abs() < 1e-6
    }

    fn is_contrast_identity(&self) -> bool {
        let c = self.compute_contrast();
        (c[0] - 1.0).abs() < 1e-6 && (c[1] - 1.0).abs() < 1e-6 && (c[2] - 1.0).abs() < 1e-6
    }

    #[inline]
    fn apply_gamma_channel(&self, val: f32, gamma: f32, range: f32) -> f32 {
        let shifted = val - self.pivot_black;
        let sign = shifted.signum();
        // Clamp range to avoid division by zero
        let safe_range = range.abs().max(MIN_DIVISOR);
        let normalized = shifted.abs() / safe_range;
        normalized.powf(gamma) * sign * safe_range + self.pivot_black
    }

    #[inline]
    fn apply_saturation(&self, rgb: [f32; 3]) -> [f32; 3] {
        if (self.saturation - 1.0).abs() < 1e-6 {
            return rgb;
        }
        
        let lum = rgb[0] * REC709_LUMA_R + rgb[1] * REC709_LUMA_G + rgb[2] * REC709_LUMA_B;
        [
            lum + self.saturation * (rgb[0] - lum),
            lum + self.saturation * (rgb[1] - lum),
            lum + self.saturation * (rgb[2] - lum),
        ]
    }

    #[inline]
    fn apply_saturation_inv(&self, rgb: [f32; 3]) -> [f32; 3] {
        if self.saturation == 0.0 || (self.saturation - 1.0).abs() < 1e-6 {
            return rgb;
        }
        
        let inv_sat = 1.0 / self.saturation;
        let lum = rgb[0] * REC709_LUMA_R + rgb[1] * REC709_LUMA_G + rgb[2] * REC709_LUMA_B;
        [
            lum + inv_sat * (rgb[0] - lum),
            lum + inv_sat * (rgb[1] - lum),
            lum + inv_sat * (rgb[2] - lum),
        ]
    }

    #[inline]
    fn apply_clamp(&self, rgb: [f32; 3]) -> [f32; 3] {
        [
            rgb[0].clamp(self.clamp_black, self.clamp_white),
            rgb[1].clamp(self.clamp_black, self.clamp_white),
            rgb[2].clamp(self.clamp_black, self.clamp_white),
        ]
    }
}

impl Default for GradingPrimary {
    fn default() -> Self {
        Self::identity(GradingStyle::Log)
    }
}

/// Apply grading primary to image buffer in-place.
pub fn apply_grading_primary_inplace(buffer: &mut [f32], gp: &GradingPrimary) {
    if gp.is_identity() {
        return;
    }

    for chunk in buffer.chunks_exact_mut(3) {
        let rgb = [chunk[0], chunk[1], chunk[2]];
        let result = gp.apply(rgb);
        chunk[0] = result[0];
        chunk[1] = result[1];
        chunk[2] = result[2];
    }
}

/// Apply grading primary to RGBA buffer in-place.
pub fn apply_grading_primary_rgba_inplace(buffer: &mut [f32], gp: &GradingPrimary) {
    if gp.is_identity() {
        return;
    }

    for chunk in buffer.chunks_exact_mut(4) {
        let rgb = [chunk[0], chunk[1], chunk[2]];
        let result = gp.apply(rgb);
        chunk[0] = result[0];
        chunk[1] = result[1];
        chunk[2] = result[2];
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    #[test]
    fn identity() {
        let gp = GradingPrimary::identity(GradingStyle::Log);
        assert!(gp.is_identity());

        let rgb = [0.5, 0.3, 0.7];
        let result = gp.apply(rgb);
        assert!((result[0] - rgb[0]).abs() < EPSILON);
        assert!((result[1] - rgb[1]).abs() < EPSILON);
        assert!((result[2] - rgb[2]).abs() < EPSILON);
    }

    #[test]
    fn log_brightness() {
        let mut gp = GradingPrimary::identity(GradingStyle::Log);
        gp.brightness.master = 0.1; // Add 0.1 to all channels
        gp.clamp_black = f32::NEG_INFINITY; // Disable clamping for test
        gp.clamp_white = f32::INFINITY;

        let rgb = [0.3, 0.3, 0.3];
        let result = gp.apply(rgb);
        // Brightness adds 0.1, then contrast (1.0) keeps it same
        // 0.3 + 0.1 = 0.4, then contrast around pivot -0.2:
        // (0.4 - (-0.2)) * 1.0 + (-0.2) = 0.4
        assert!((result[0] - 0.4).abs() < EPSILON, "got {}", result[0]);
        assert!((result[1] - 0.4).abs() < EPSILON, "got {}", result[1]);
        assert!((result[2] - 0.4).abs() < EPSILON, "got {}", result[2]);
    }

    #[test]
    fn log_roundtrip() {
        let mut gp = GradingPrimary::identity(GradingStyle::Log);
        gp.brightness = GradingRGBM { red: 0.05, green: 0.0, blue: -0.05, master: 0.02 };
        gp.contrast = GradingRGBM { red: 1.1, green: 1.0, blue: 0.9, master: 1.0 };
        gp.saturation = 1.1;

        let rgb = [0.3, 0.5, 0.4];
        let forward = gp.apply(rgb);
        let inverse = gp.apply_inverse(forward);

        assert!((inverse[0] - rgb[0]).abs() < 0.01, "R: {} vs {}", inverse[0], rgb[0]);
        assert!((inverse[1] - rgb[1]).abs() < 0.01, "G: {} vs {}", inverse[1], rgb[1]);
        assert!((inverse[2] - rgb[2]).abs() < 0.01, "B: {} vs {}", inverse[2], rgb[2]);
    }

    #[test]
    fn linear_roundtrip() {
        let mut gp = GradingPrimary::identity(GradingStyle::Linear);
        gp.offset = GradingRGBM::uniform(0.01);
        gp.exposure = GradingRGBM::uniform(0.5);
        gp.contrast = GradingRGBM { red: 1.1, green: 1.0, blue: 0.95, master: 1.0 };
        gp.saturation = 0.9;

        let rgb = [0.3, 0.5, 0.4];
        let forward = gp.apply(rgb);
        let inverse = gp.apply_inverse(forward);

        assert!((inverse[0] - rgb[0]).abs() < 0.01, "R: {} vs {}", inverse[0], rgb[0]);
        assert!((inverse[1] - rgb[1]).abs() < 0.01, "G: {} vs {}", inverse[1], rgb[1]);
        assert!((inverse[2] - rgb[2]).abs() < 0.01, "B: {} vs {}", inverse[2], rgb[2]);
    }

    #[test]
    fn video_roundtrip() {
        let mut gp = GradingPrimary::identity(GradingStyle::Video);
        gp.lift = GradingRGBM::uniform(0.02);
        gp.gain = GradingRGBM { red: 1.1, green: 1.0, blue: 0.95, master: 1.0 };
        gp.gamma = GradingRGBM { red: 0.95, green: 1.0, blue: 1.05, master: 1.0 };
        gp.saturation = 1.05;

        let rgb = [0.3, 0.5, 0.4];
        let forward = gp.apply(rgb);
        let inverse = gp.apply_inverse(forward);

        assert!((inverse[0] - rgb[0]).abs() < 0.01, "R: {} vs {}", inverse[0], rgb[0]);
        assert!((inverse[1] - rgb[1]).abs() < 0.01, "G: {} vs {}", inverse[1], rgb[1]);
        assert!((inverse[2] - rgb[2]).abs() < 0.01, "B: {} vs {}", inverse[2], rgb[2]);
    }

    #[test]
    fn saturation_desaturate() {
        let mut gp = GradingPrimary::identity(GradingStyle::Log);
        gp.saturation = 0.0;

        let rgb = [1.0, 0.0, 0.0]; // Pure red
        let result = gp.apply(rgb);

        // Should be grey (luminance of red)
        let expected_lum = REC709_LUMA_R;
        assert!((result[0] - expected_lum).abs() < EPSILON);
        assert!((result[1] - expected_lum).abs() < EPSILON);
        assert!((result[2] - expected_lum).abs() < EPSILON);
    }

    #[test]
    fn rgbm_add() {
        let rgbm = GradingRGBM { red: 0.1, green: 0.2, blue: 0.3, master: 0.05 };
        let rgb = rgbm.rgb_add();
        assert!((rgb[0] - 0.15).abs() < EPSILON);
        assert!((rgb[1] - 0.25).abs() < EPSILON);
        assert!((rgb[2] - 0.35).abs() < EPSILON);
    }

    #[test]
    fn rgbm_mul() {
        let rgbm = GradingRGBM { red: 1.0, green: 1.5, blue: 0.8, master: 2.0 };
        let rgb = rgbm.rgb_mul();
        assert!((rgb[0] - 2.0).abs() < EPSILON);
        assert!((rgb[1] - 3.0).abs() < EPSILON);
        assert!((rgb[2] - 1.6).abs() < EPSILON);
    }
}
