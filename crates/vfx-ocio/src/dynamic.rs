//! Dynamic properties for runtime parameter adjustment.
//!
//! Allows modifying processor parameters (exposure, contrast, gamma)
//! without rebuilding the entire transform chain.
//!
//! # Example
//!
//! ```ignore
//! use vfx_ocio::{Config, DynamicProcessor};
//!
//! let config = Config::from_file("config.ocio")?;
//! let processor = config.processor("ACEScg", "sRGB")?;
//!
//! // Create dynamic wrapper
//! let mut dynamic = DynamicProcessor::new(processor);
//!
//! // Adjust exposure in real-time
//! dynamic.set_exposure(1.5);
//! dynamic.set_contrast(1.1);
//! dynamic.set_gamma(1.0);
//!
//! // Apply with dynamic adjustments
//! dynamic.apply_rgb(&mut pixels);
//! ```

use vfx_core::pixel::{REC709_LUMA_R, REC709_LUMA_G, REC709_LUMA_B};

use crate::processor::Processor;

/// Dynamic property types that can be adjusted at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DynamicPropertyType {
    /// Exposure adjustment (stops).
    Exposure,
    /// Contrast adjustment (multiplier around pivot).
    Contrast,
    /// Gamma adjustment (power).
    Gamma,
    /// Saturation adjustment (multiplier).
    Saturation,
}

/// A processor wrapper that supports runtime parameter adjustment.
///
/// This wraps a base processor and applies additional exposure/contrast/gamma
/// adjustments that can be modified without rebuilding the transform chain.
#[derive(Debug)]
pub struct DynamicProcessor {
    /// Base processor (immutable).
    base: Processor,
    /// Exposure adjustment in stops (0 = no change).
    exposure: f32,
    /// Contrast multiplier (1.0 = no change).
    contrast: f32,
    /// Gamma power (1.0 = no change).
    gamma: f32,
    /// Saturation multiplier (1.0 = no change).
    saturation: f32,
    /// Pivot point for contrast (typically 0.18 for scene-linear).
    pivot: f32,
    /// Whether to apply adjustments before or after base transform.
    apply_before: bool,
}

impl DynamicProcessor {
    /// Creates a new dynamic processor wrapping the given base processor.
    pub fn new(base: Processor) -> Self {
        Self {
            base,
            exposure: 0.0,
            contrast: 1.0,
            gamma: 1.0,
            saturation: 1.0,
            pivot: 0.18,
            apply_before: false,
        }
    }

    /// Sets exposure adjustment in stops.
    ///
    /// - 0.0 = no change
    /// - 1.0 = +1 stop (2x brighter)
    /// - -1.0 = -1 stop (2x darker)
    pub fn set_exposure(&mut self, stops: f32) {
        self.exposure = stops;
    }

    /// Returns current exposure adjustment.
    pub fn exposure(&self) -> f32 {
        self.exposure
    }

    /// Sets contrast multiplier.
    ///
    /// - 1.0 = no change
    /// - >1.0 = increased contrast
    /// - <1.0 = decreased contrast
    pub fn set_contrast(&mut self, contrast: f32) {
        self.contrast = contrast;
    }

    /// Returns current contrast multiplier.
    pub fn contrast(&self) -> f32 {
        self.contrast
    }

    /// Sets gamma (power) adjustment.
    ///
    /// - 1.0 = no change
    /// - <1.0 = brighter midtones
    /// - >1.0 = darker midtones
    pub fn set_gamma(&mut self, gamma: f32) {
        self.gamma = gamma.max(0.01); // Prevent division by zero
    }

    /// Returns current gamma value.
    pub fn gamma(&self) -> f32 {
        self.gamma
    }

    /// Sets saturation multiplier.
    ///
    /// - 1.0 = no change
    /// - 0.0 = grayscale
    /// - >1.0 = increased saturation
    pub fn set_saturation(&mut self, saturation: f32) {
        self.saturation = saturation.max(0.0);
    }

    /// Returns current saturation multiplier.
    pub fn saturation(&self) -> f32 {
        self.saturation
    }

    /// Sets the contrast pivot point.
    ///
    /// Default is 0.18 (18% gray in scene-linear).
    pub fn set_pivot(&mut self, pivot: f32) {
        self.pivot = pivot;
    }

    /// Returns current pivot point.
    pub fn pivot(&self) -> f32 {
        self.pivot
    }

    /// Sets whether to apply dynamic adjustments before or after base transform.
    ///
    /// - `true`: Apply exposure/contrast/gamma BEFORE base transform
    /// - `false`: Apply AFTER base transform (default)
    pub fn set_apply_before(&mut self, before: bool) {
        self.apply_before = before;
    }

    /// Returns whether adjustments are applied before base transform.
    pub fn apply_before(&self) -> bool {
        self.apply_before
    }

    /// Resets all dynamic properties to their default values.
    pub fn reset(&mut self) {
        self.exposure = 0.0;
        self.contrast = 1.0;
        self.gamma = 1.0;
        self.saturation = 1.0;
    }

    /// Returns true if any dynamic property is modified from default.
    pub fn has_adjustments(&self) -> bool {
        self.exposure.abs() > 1e-6
            || (self.contrast - 1.0).abs() > 1e-6
            || (self.gamma - 1.0).abs() > 1e-6
            || (self.saturation - 1.0).abs() > 1e-6
    }

    /// Applies the dynamic adjustments to a single RGB pixel.
    #[inline]
    fn apply_dynamic(&self, pixel: &mut [f32; 3]) {
        // Exposure (2^stops)
        if self.exposure.abs() > 1e-6 {
            let mult = 2.0_f32.powf(self.exposure);
            pixel[0] *= mult;
            pixel[1] *= mult;
            pixel[2] *= mult;
        }

        // Contrast (around pivot)
        if (self.contrast - 1.0).abs() > 1e-6 {
            pixel[0] = self.pivot + (pixel[0] - self.pivot) * self.contrast;
            pixel[1] = self.pivot + (pixel[1] - self.pivot) * self.contrast;
            pixel[2] = self.pivot + (pixel[2] - self.pivot) * self.contrast;
        }

        // Gamma
        if (self.gamma - 1.0).abs() > 1e-6 {
            let inv_gamma = 1.0 / self.gamma;
            pixel[0] = pixel[0].max(0.0).powf(inv_gamma);
            pixel[1] = pixel[1].max(0.0).powf(inv_gamma);
            pixel[2] = pixel[2].max(0.0).powf(inv_gamma);
        }

        // Saturation
        if (self.saturation - 1.0).abs() > 1e-6 {
            let luma = REC709_LUMA_R * pixel[0] + REC709_LUMA_G * pixel[1] + REC709_LUMA_B * pixel[2];
            pixel[0] = luma + (pixel[0] - luma) * self.saturation;
            pixel[1] = luma + (pixel[1] - luma) * self.saturation;
            pixel[2] = luma + (pixel[2] - luma) * self.saturation;
        }
    }

    /// Applies the processor with dynamic adjustments to RGB pixel data.
    pub fn apply_rgb(&self, pixels: &mut [[f32; 3]]) {
        if self.apply_before {
            // Dynamic first, then base
            if self.has_adjustments() {
                for pixel in pixels.iter_mut() {
                    self.apply_dynamic(pixel);
                }
            }
            self.base.apply_rgb(pixels);
        } else {
            // Base first, then dynamic
            self.base.apply_rgb(pixels);
            if self.has_adjustments() {
                for pixel in pixels.iter_mut() {
                    self.apply_dynamic(pixel);
                }
            }
        }
    }

    /// Applies the processor with dynamic adjustments to RGBA pixel data.
    pub fn apply_rgba(&self, pixels: &mut [[f32; 4]]) {
        if self.apply_before {
            // Dynamic first, then base
            if self.has_adjustments() {
                for pixel in pixels.iter_mut() {
                    let mut rgb = [pixel[0], pixel[1], pixel[2]];
                    self.apply_dynamic(&mut rgb);
                    pixel[0] = rgb[0];
                    pixel[1] = rgb[1];
                    pixel[2] = rgb[2];
                }
            }
            self.base.apply_rgba(pixels);
        } else {
            // Base first, then dynamic
            self.base.apply_rgba(pixels);
            if self.has_adjustments() {
                for pixel in pixels.iter_mut() {
                    let mut rgb = [pixel[0], pixel[1], pixel[2]];
                    self.apply_dynamic(&mut rgb);
                    pixel[0] = rgb[0];
                    pixel[1] = rgb[1];
                    pixel[2] = rgb[2];
                }
            }
        }
    }

    /// Returns a reference to the base processor.
    pub fn base(&self) -> &Processor {
        &self.base
    }

    /// Consumes this and returns the base processor.
    pub fn into_base(self) -> Processor {
        self.base
    }
}

/// Builder for configuring dynamic processor settings.
#[derive(Debug, Clone)]
pub struct DynamicProcessorBuilder {
    exposure: f32,
    contrast: f32,
    gamma: f32,
    saturation: f32,
    pivot: f32,
    apply_before: bool,
}

impl Default for DynamicProcessorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicProcessorBuilder {
    /// Creates a new builder with default values.
    pub fn new() -> Self {
        Self {
            exposure: 0.0,
            contrast: 1.0,
            gamma: 1.0,
            saturation: 1.0,
            pivot: 0.18,
            apply_before: false,
        }
    }

    /// Sets initial exposure.
    pub fn exposure(mut self, stops: f32) -> Self {
        self.exposure = stops;
        self
    }

    /// Sets initial contrast.
    pub fn contrast(mut self, contrast: f32) -> Self {
        self.contrast = contrast;
        self
    }

    /// Sets initial gamma.
    pub fn gamma(mut self, gamma: f32) -> Self {
        self.gamma = gamma.max(0.01);
        self
    }

    /// Sets initial saturation.
    pub fn saturation(mut self, saturation: f32) -> Self {
        self.saturation = saturation.max(0.0);
        self
    }

    /// Sets pivot point.
    pub fn pivot(mut self, pivot: f32) -> Self {
        self.pivot = pivot;
        self
    }

    /// Sets whether to apply before base transform.
    pub fn apply_before(mut self, before: bool) -> Self {
        self.apply_before = before;
        self
    }

    /// Builds the dynamic processor with the given base.
    pub fn build(self, base: Processor) -> DynamicProcessor {
        DynamicProcessor {
            base,
            exposure: self.exposure,
            contrast: self.contrast,
            gamma: self.gamma,
            saturation: self.saturation,
            pivot: self.pivot,
            apply_before: self.apply_before,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transform::{Transform, MatrixTransform, TransformDirection};

    fn identity_processor() -> Processor {
        let identity = Transform::Matrix(MatrixTransform {
            matrix: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
            offset: [0.0, 0.0, 0.0, 0.0],
            direction: TransformDirection::Forward,
        });
        Processor::from_transform(&identity, TransformDirection::Forward).unwrap()
    }

    #[test]
    fn exposure_adjustment() {
        let proc = identity_processor();
        let mut dynamic = DynamicProcessor::new(proc);

        // +1 stop should double values
        dynamic.set_exposure(1.0);

        let mut pixels = [[0.5_f32, 0.5, 0.5]];
        dynamic.apply_rgb(&mut pixels);

        assert!((pixels[0][0] - 1.0).abs() < 0.001);
        assert!((pixels[0][1] - 1.0).abs() < 0.001);
        assert!((pixels[0][2] - 1.0).abs() < 0.001);
    }

    #[test]
    fn contrast_adjustment() {
        let proc = identity_processor();
        let mut dynamic = DynamicProcessor::new(proc);
        dynamic.set_pivot(0.5);
        dynamic.set_contrast(2.0);

        // 0.5 at pivot should stay 0.5
        let mut pixels = [[0.5_f32, 0.5, 0.5]];
        dynamic.apply_rgb(&mut pixels);
        assert!((pixels[0][0] - 0.5).abs() < 0.001);

        // 0.75 should be pushed further from pivot
        // (0.75 - 0.5) * 2.0 + 0.5 = 1.0
        let mut pixels = [[0.75_f32, 0.75, 0.75]];
        dynamic.apply_rgb(&mut pixels);
        assert!((pixels[0][0] - 1.0).abs() < 0.001);
    }

    #[test]
    fn gamma_adjustment() {
        let proc = identity_processor();
        let mut dynamic = DynamicProcessor::new(proc);
        dynamic.set_gamma(2.2);

        // 0.5^(1/2.2) ≈ 0.73
        let mut pixels = [[0.5_f32, 0.5, 0.5]];
        dynamic.apply_rgb(&mut pixels);
        assert!((pixels[0][0] - 0.5_f32.powf(1.0/2.2)).abs() < 0.001);
    }

    #[test]
    fn saturation_adjustment() {
        let proc = identity_processor();
        let mut dynamic = DynamicProcessor::new(proc);
        dynamic.set_saturation(0.0);

        // Full desaturation should produce grayscale
        let mut pixels = [[1.0_f32, 0.0, 0.0]];
        dynamic.apply_rgb(&mut pixels);

        // All channels should be equal (luma value of pure red)
        let luma = REC709_LUMA_R;
        assert!((pixels[0][0] - luma).abs() < 0.001);
        assert!((pixels[0][1] - luma).abs() < 0.001);
        assert!((pixels[0][2] - luma).abs() < 0.001);
    }

    #[test]
    fn no_adjustments() {
        let proc = identity_processor();
        let dynamic = DynamicProcessor::new(proc);

        assert!(!dynamic.has_adjustments());

        let mut pixels = [[0.5_f32, 0.3, 0.1]];
        let original = pixels.clone();
        dynamic.apply_rgb(&mut pixels);

        assert!((pixels[0][0] - original[0][0]).abs() < 0.0001);
        assert!((pixels[0][1] - original[0][1]).abs() < 0.0001);
        assert!((pixels[0][2] - original[0][2]).abs() < 0.0001);
    }

    #[test]
    fn reset_adjustments() {
        let proc = identity_processor();
        let mut dynamic = DynamicProcessor::new(proc);

        dynamic.set_exposure(2.0);
        dynamic.set_contrast(1.5);
        dynamic.set_gamma(2.2);
        dynamic.set_saturation(0.5);

        assert!(dynamic.has_adjustments());

        dynamic.reset();

        assert!(!dynamic.has_adjustments());
        assert!((dynamic.exposure() - 0.0).abs() < 0.0001);
        assert!((dynamic.contrast() - 1.0).abs() < 0.0001);
        assert!((dynamic.gamma() - 1.0).abs() < 0.0001);
        assert!((dynamic.saturation() - 1.0).abs() < 0.0001);
    }

    #[test]
    fn builder_pattern() {
        let proc = identity_processor();
        let dynamic = DynamicProcessorBuilder::new()
            .exposure(1.0)
            .contrast(1.2)
            .gamma(2.2)
            .saturation(0.9)
            .pivot(0.18)
            .build(proc);

        assert!((dynamic.exposure() - 1.0).abs() < 0.0001);
        assert!((dynamic.contrast() - 1.2).abs() < 0.0001);
        assert!((dynamic.gamma() - 2.2).abs() < 0.0001);
        assert!((dynamic.saturation() - 0.9).abs() < 0.0001);
    }

    #[test]
    fn apply_before_vs_after() {
        let proc = identity_processor();

        // With identity base, order shouldn't matter for pure exposure
        let mut dynamic_before = DynamicProcessor::new(identity_processor());
        dynamic_before.set_exposure(1.0);
        dynamic_before.set_apply_before(true);

        let mut dynamic_after = DynamicProcessor::new(proc);
        dynamic_after.set_exposure(1.0);
        dynamic_after.set_apply_before(false);

        let mut pixels_before = [[0.5_f32, 0.5, 0.5]];
        let mut pixels_after = [[0.5_f32, 0.5, 0.5]];

        dynamic_before.apply_rgb(&mut pixels_before);
        dynamic_after.apply_rgb(&mut pixels_after);

        // With identity, both should be the same
        assert!((pixels_before[0][0] - pixels_after[0][0]).abs() < 0.0001);
    }
}
