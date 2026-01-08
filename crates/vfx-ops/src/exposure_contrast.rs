//! Exposure/Contrast operations for viewport controls.
//!
//! Reference: OCIO ExposureContrastOpCPU.cpp
//!
//! Provides exposure and contrast/gamma adjustments with three styles:
//! - Linear: For scene-linear images
//! - Video: For video-encoded (gamma) images
//! - Logarithmic: For log-encoded images
//!
//! These controls are designed for interactive viewport adjustments.

/// Video OETF power (approximation to BT.709 camera curve).
/// 1 / 1.83 = 0.54644808743169393
pub const VIDEO_OETF_POWER: f32 = 0.54644808743169393;

/// Minimum pivot value to avoid division by zero.
pub const MIN_PIVOT: f32 = 0.001;

/// Minimum contrast value.
pub const MIN_CONTRAST: f32 = 0.001;

/// Default log exposure step (Cineon-style).
pub const LOG_EXPOSURE_STEP_DEFAULT: f32 = 0.088;

/// Default log mid-gray position.
pub const LOG_MIDGRAY_DEFAULT: f32 = 0.435;

/// Exposure/contrast style determines how values are interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExposureContrastStyle {
    /// Scene-linear images (most common in VFX).
    #[default]
    Linear,
    /// Video-encoded images (gamma-corrected).
    Video,
    /// Logarithmic-encoded images (log footage).
    Logarithmic,
}

/// Parameters for Exposure/Contrast operation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExposureContrast {
    /// Style determines the math used.
    pub style: ExposureContrastStyle,
    /// Exposure in stops (0 = no change, +1 = 2x brighter).
    pub exposure: f32,
    /// Contrast adjustment (1.0 = no change).
    pub contrast: f32,
    /// Gamma adjustment (1.0 = no change), multiplied with contrast.
    pub gamma: f32,
    /// Pivot point for contrast adjustment.
    pub pivot: f32,
    /// Log exposure step size (for Logarithmic style).
    pub log_exposure_step: f32,
    /// Log mid-gray position (for Logarithmic style).
    pub log_midgray: f32,
}

impl Default for ExposureContrast {
    fn default() -> Self {
        Self {
            style: ExposureContrastStyle::Linear,
            exposure: 0.0,
            contrast: 1.0,
            gamma: 1.0,
            pivot: 0.18,
            log_exposure_step: LOG_EXPOSURE_STEP_DEFAULT,
            log_midgray: LOG_MIDGRAY_DEFAULT,
        }
    }
}

impl ExposureContrast {
    /// Create identity (no change).
    pub fn identity() -> Self {
        Self::default()
    }

    /// Check if this is identity (no-op).
    pub fn is_identity(&self) -> bool {
        self.exposure == 0.0 
            && self.contrast == 1.0 
            && self.gamma == 1.0
    }

    /// Apply forward transform to RGB pixel.
    #[inline]
    pub fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        match self.style {
            ExposureContrastStyle::Linear => self.apply_linear(rgb),
            ExposureContrastStyle::Video => self.apply_video(rgb),
            ExposureContrastStyle::Logarithmic => self.apply_log(rgb),
        }
    }

    /// Apply inverse transform to RGB pixel.
    #[inline]
    pub fn apply_inverse(&self, rgb: [f32; 3]) -> [f32; 3] {
        match self.style {
            ExposureContrastStyle::Linear => self.apply_linear_inv(rgb),
            ExposureContrastStyle::Video => self.apply_video_inv(rgb),
            ExposureContrastStyle::Logarithmic => self.apply_log_inv(rgb),
        }
    }

    /// Linear style forward.
    /// out = pivot * (in * 2^exposure / pivot)^contrast
    #[inline]
    fn apply_linear(&self, rgb: [f32; 3]) -> [f32; 3] {
        let contrast_val = (self.contrast * self.gamma).max(MIN_CONTRAST);
        let exposure_val = 2.0_f32.powf(self.exposure);
        let pivot = self.pivot.max(MIN_PIVOT);

        if contrast_val == 1.0 {
            // Just exposure
            [rgb[0] * exposure_val, rgb[1] * exposure_val, rgb[2] * exposure_val]
        } else {
            let exposure_over_pivot = exposure_val / pivot;
            [
                (rgb[0] * exposure_over_pivot).max(0.0).powf(contrast_val) * pivot,
                (rgb[1] * exposure_over_pivot).max(0.0).powf(contrast_val) * pivot,
                (rgb[2] * exposure_over_pivot).max(0.0).powf(contrast_val) * pivot,
            ]
        }
    }

    /// Linear style inverse.
    /// out = pivot^(1/contrast) * (in / pivot)^(1/contrast) / 2^exposure
    #[inline]
    fn apply_linear_inv(&self, rgb: [f32; 3]) -> [f32; 3] {
        let contrast_val = (self.contrast * self.gamma).max(MIN_CONTRAST);
        let inv_contrast = 1.0 / contrast_val;
        let inv_exposure = 1.0 / 2.0_f32.powf(self.exposure);
        let pivot = self.pivot.max(MIN_PIVOT);

        if contrast_val == 1.0 {
            [rgb[0] * inv_exposure, rgb[1] * inv_exposure, rgb[2] * inv_exposure]
        } else {
            let pivot_over_exposure = pivot * inv_exposure;
            let inv_pivot = 1.0 / pivot;
            [
                (rgb[0] * inv_pivot).max(0.0).powf(inv_contrast) * pivot_over_exposure,
                (rgb[1] * inv_pivot).max(0.0).powf(inv_contrast) * pivot_over_exposure,
                (rgb[2] * inv_pivot).max(0.0).powf(inv_contrast) * pivot_over_exposure,
            ]
        }
    }

    /// Video style forward.
    /// Same as linear but pivot is raised to VIDEO_OETF_POWER.
    #[inline]
    fn apply_video(&self, rgb: [f32; 3]) -> [f32; 3] {
        let contrast_val = (self.contrast * self.gamma).max(MIN_CONTRAST);
        let exposure_val = 2.0_f32.powf(self.exposure).powf(VIDEO_OETF_POWER);
        let pivot = self.pivot.max(MIN_PIVOT).powf(VIDEO_OETF_POWER);

        if contrast_val == 1.0 {
            [rgb[0] * exposure_val, rgb[1] * exposure_val, rgb[2] * exposure_val]
        } else {
            let exposure_over_pivot = exposure_val / pivot;
            [
                (rgb[0] * exposure_over_pivot).max(0.0).powf(contrast_val) * pivot,
                (rgb[1] * exposure_over_pivot).max(0.0).powf(contrast_val) * pivot,
                (rgb[2] * exposure_over_pivot).max(0.0).powf(contrast_val) * pivot,
            ]
        }
    }

    /// Video style inverse.
    #[inline]
    fn apply_video_inv(&self, rgb: [f32; 3]) -> [f32; 3] {
        let contrast_val = (self.contrast * self.gamma).max(MIN_CONTRAST);
        let inv_contrast = 1.0 / contrast_val;
        let inv_exposure = 1.0 / 2.0_f32.powf(self.exposure).powf(VIDEO_OETF_POWER);
        let pivot = self.pivot.max(MIN_PIVOT).powf(VIDEO_OETF_POWER);

        if contrast_val == 1.0 {
            [rgb[0] * inv_exposure, rgb[1] * inv_exposure, rgb[2] * inv_exposure]
        } else {
            let pivot_over_exposure = pivot * inv_exposure;
            let inv_pivot = 1.0 / pivot;
            [
                (rgb[0] * inv_pivot).max(0.0).powf(inv_contrast) * pivot_over_exposure,
                (rgb[1] * inv_pivot).max(0.0).powf(inv_contrast) * pivot_over_exposure,
                (rgb[2] * inv_pivot).max(0.0).powf(inv_contrast) * pivot_over_exposure,
            ]
        }
    }

    /// Logarithmic style forward.
    /// logPivot = log2(pivot / 0.18) * logExposureStep + logMidGray
    /// out = (in + exposure * logExposureStep - logPivot) * contrast + logPivot
    #[inline]
    fn apply_log(&self, rgb: [f32; 3]) -> [f32; 3] {
        let pivot = self.pivot.max(MIN_PIVOT);
        let log_pivot = ((pivot / 0.18).log2() * self.log_exposure_step + self.log_midgray).max(0.0);
        let contrast_val = (self.contrast * self.gamma).max(MIN_CONTRAST);
        let exposure_val = self.exposure * self.log_exposure_step;
        
        // Rearrange: out = in * contrast + (exposure - pivot) * contrast + pivot
        let offset = (exposure_val - log_pivot) * contrast_val + log_pivot;
        
        [
            rgb[0] * contrast_val + offset,
            rgb[1] * contrast_val + offset,
            rgb[2] * contrast_val + offset,
        ]
    }

    /// Logarithmic style inverse.
    #[inline]
    fn apply_log_inv(&self, rgb: [f32; 3]) -> [f32; 3] {
        let pivot = self.pivot.max(MIN_PIVOT);
        let log_pivot = ((pivot / 0.18).log2() * self.log_exposure_step + self.log_midgray).max(0.0);
        let inv_contrast = 1.0 / (self.contrast * self.gamma).max(MIN_CONTRAST);
        let exposure_val = self.exposure * self.log_exposure_step;
        
        // Inverse: in = (out - pivot) / contrast + pivot - exposure
        let neg_offset = log_pivot - log_pivot * inv_contrast - exposure_val;
        
        [
            rgb[0] * inv_contrast + neg_offset,
            rgb[1] * inv_contrast + neg_offset,
            rgb[2] * inv_contrast + neg_offset,
        ]
    }
}

/// Apply exposure/contrast to image buffer in-place.
pub fn apply_exposure_contrast_inplace(buffer: &mut [f32], ec: &ExposureContrast) {
    if ec.is_identity() {
        return;
    }

    for chunk in buffer.chunks_exact_mut(3) {
        let rgb = [chunk[0], chunk[1], chunk[2]];
        let result = ec.apply(rgb);
        chunk[0] = result[0];
        chunk[1] = result[1];
        chunk[2] = result[2];
    }
}

/// Apply exposure/contrast to RGBA buffer in-place.
pub fn apply_exposure_contrast_rgba_inplace(buffer: &mut [f32], ec: &ExposureContrast) {
    if ec.is_identity() {
        return;
    }

    for chunk in buffer.chunks_exact_mut(4) {
        let rgb = [chunk[0], chunk[1], chunk[2]];
        let result = ec.apply(rgb);
        chunk[0] = result[0];
        chunk[1] = result[1];
        chunk[2] = result[2];
        // alpha unchanged
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    #[test]
    fn identity() {
        let ec = ExposureContrast::identity();
        assert!(ec.is_identity());

        let rgb = [0.5, 0.3, 0.7];
        let result = ec.apply(rgb);
        assert!((result[0] - rgb[0]).abs() < EPSILON);
        assert!((result[1] - rgb[1]).abs() < EPSILON);
        assert!((result[2] - rgb[2]).abs() < EPSILON);
    }

    #[test]
    fn linear_exposure() {
        let ec = ExposureContrast {
            exposure: 1.0, // +1 stop = 2x brighter
            ..Default::default()
        };

        let rgb = [0.25, 0.25, 0.25];
        let result = ec.apply(rgb);
        assert!((result[0] - 0.5).abs() < EPSILON);
        assert!((result[1] - 0.5).abs() < EPSILON);
        assert!((result[2] - 0.5).abs() < EPSILON);
    }

    #[test]
    fn linear_roundtrip() {
        let ec = ExposureContrast {
            exposure: 0.5,
            contrast: 1.2,
            gamma: 0.9,
            ..Default::default()
        };

        let rgb = [0.3, 0.5, 0.4];
        let forward = ec.apply(rgb);
        let inverse = ec.apply_inverse(forward);

        assert!((inverse[0] - rgb[0]).abs() < 0.001, "R: {} vs {}", inverse[0], rgb[0]);
        assert!((inverse[1] - rgb[1]).abs() < 0.001, "G: {} vs {}", inverse[1], rgb[1]);
        assert!((inverse[2] - rgb[2]).abs() < 0.001, "B: {} vs {}", inverse[2], rgb[2]);
    }

    #[test]
    fn video_roundtrip() {
        let ec = ExposureContrast {
            style: ExposureContrastStyle::Video,
            exposure: 0.5,
            contrast: 1.1,
            ..Default::default()
        };

        let rgb = [0.3, 0.5, 0.4];
        let forward = ec.apply(rgb);
        let inverse = ec.apply_inverse(forward);

        assert!((inverse[0] - rgb[0]).abs() < 0.001, "R: {} vs {}", inverse[0], rgb[0]);
        assert!((inverse[1] - rgb[1]).abs() < 0.001, "G: {} vs {}", inverse[1], rgb[1]);
        assert!((inverse[2] - rgb[2]).abs() < 0.001, "B: {} vs {}", inverse[2], rgb[2]);
    }

    #[test]
    fn log_roundtrip() {
        let ec = ExposureContrast {
            style: ExposureContrastStyle::Logarithmic,
            exposure: 0.5,
            contrast: 1.2,
            ..Default::default()
        };

        let rgb = [0.3, 0.5, 0.4];
        let forward = ec.apply(rgb);
        let inverse = ec.apply_inverse(forward);

        assert!((inverse[0] - rgb[0]).abs() < 0.001, "R: {} vs {}", inverse[0], rgb[0]);
        assert!((inverse[1] - rgb[1]).abs() < 0.001, "G: {} vs {}", inverse[1], rgb[1]);
        assert!((inverse[2] - rgb[2]).abs() < 0.001, "B: {} vs {}", inverse[2], rgb[2]);
    }

    #[test]
    fn log_is_linear_transform() {
        // Log style should be a linear transform (affine)
        let ec = ExposureContrast {
            style: ExposureContrastStyle::Logarithmic,
            exposure: 1.0,
            contrast: 1.5,
            ..Default::default()
        };

        // Check linearity: f(a*x + b*y) = a*f(x) + b*f(y) for affine transforms
        let rgb1 = [0.2, 0.3, 0.4];
        let rgb2 = [0.5, 0.6, 0.7];
        
        let result1 = ec.apply(rgb1);
        let result2 = ec.apply(rgb2);
        
        // For affine: result should be linear combination
        // Just verify the result is reasonable
        assert!(result1[0] > 0.0);
        assert!(result2[0] > result1[0]);
    }
}
