//! GradingTone types: GradingRGBMSW, GradingTone, RGBMChannel.
//!
//! Reference: OCIO include/OpenColorIO/OpenColorTransforms.h

use crate::GradingStyle;

/// Channel index for RGBM operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum RGBMChannel {
    /// Red channel.
    R = 0,
    /// Green channel.
    G = 1,
    /// Blue channel.
    B = 2,
    /// Master (applies to all RGB).
    M = 3,
}

impl RGBMChannel {
    /// All channels: R, G, B, M.
    pub const ALL: [RGBMChannel; 4] = [
        RGBMChannel::R,
        RGBMChannel::G,
        RGBMChannel::B,
        RGBMChannel::M,
    ];

    /// RGB channels only (no Master).
    pub const RGB: [RGBMChannel; 3] = [RGBMChannel::R, RGBMChannel::G, RGBMChannel::B];
}

/// RGBM + Start/Width parameters for a tonal zone.
///
/// Used for blacks, shadows, midtones, highlights, whites.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradingRGBMSW {
    /// Red channel adjustment (1.0 = no change).
    pub red: f64,
    /// Green channel adjustment (1.0 = no change).
    pub green: f64,
    /// Blue channel adjustment (1.0 = no change).
    pub blue: f64,
    /// Master (all channels) adjustment (1.0 = no change).
    pub master: f64,
    /// Start position of the zone.
    pub start: f64,
    /// Width/pivot of the zone.
    pub width: f64,
}

impl GradingRGBMSW {
    /// Create with all RGBM at unity (no effect).
    pub fn identity(start: f64, width: f64) -> Self {
        Self {
            red: 1.0,
            green: 1.0,
            blue: 1.0,
            master: 1.0,
            start,
            width,
        }
    }

    /// Create with all values specified.
    pub fn new(
        red: f64,
        green: f64,
        blue: f64,
        master: f64,
        start: f64,
        width: f64,
    ) -> Self {
        Self { red, green, blue, master, start, width }
    }

    /// Check if RGBM values are at identity (1.0).
    pub fn is_identity(&self) -> bool {
        self.red == 1.0 && self.green == 1.0 && self.blue == 1.0 && self.master == 1.0
    }

    /// Get value for a specific channel.
    #[inline]
    pub fn get(&self, channel: RGBMChannel) -> f64 {
        match channel {
            RGBMChannel::R => self.red,
            RGBMChannel::G => self.green,
            RGBMChannel::B => self.blue,
            RGBMChannel::M => self.master,
        }
    }

    /// Get value as f32, clamped to valid range.
    #[inline]
    pub fn get_clamped(&self, channel: RGBMChannel, min: f32, max: f32) -> f32 {
        (self.get(channel) as f32).clamp(min, max)
    }
}

impl Default for GradingRGBMSW {
    fn default() -> Self {
        Self::identity(0.0, 1.0)
    }
}

/// GradingTone parameters for zone-based tonal control.
///
/// Five zones: blacks, shadows, midtones, highlights, whites.
/// Plus s-contrast for overall contrast adjustment.
#[derive(Debug, Clone, PartialEq)]
pub struct GradingTone {
    /// Blacks control (toe region).
    pub blacks: GradingRGBMSW,
    /// Shadows control.
    pub shadows: GradingRGBMSW,
    /// Midtones control.
    pub midtones: GradingRGBMSW,
    /// Highlights control.
    pub highlights: GradingRGBMSW,
    /// Whites control (shoulder region).
    pub whites: GradingRGBMSW,
    /// S-Contrast (1.0 = no change).
    pub s_contrast: f64,
}

impl GradingTone {
    /// Create with default parameters for the given style.
    pub fn new(style: GradingStyle) -> Self {
        match style {
            GradingStyle::Log => Self::log_defaults(),
            GradingStyle::Linear => Self::linear_defaults(),
            GradingStyle::Video => Self::video_defaults(),
        }
    }

    /// LOG style defaults (from OCIO).
    /// 
    /// Designed for footage that is already in log space.
    pub fn log_defaults() -> Self {
        Self {
            // start=0.4, width=0.4 (width controls the transition zone)
            blacks: GradingRGBMSW::identity(0.4, 0.4),
            // start=0.5, width=0.0 (width is pivot point for shadows)
            shadows: GradingRGBMSW::identity(0.5, 0.0),
            // start=0.4 (center), width=0.6 (range of effect)
            midtones: GradingRGBMSW::identity(0.4, 0.6),
            // start=0.3, width=1.0 (width is pivot for highlights)
            highlights: GradingRGBMSW::identity(0.3, 1.0),
            // start=0.4, width=0.5
            whites: GradingRGBMSW::identity(0.4, 0.5),
            s_contrast: 1.0,
        }
    }

    /// LINEAR style defaults (from OCIO).
    ///
    /// For scene-linear footage. Internally converts to log for grading.
    pub fn linear_defaults() -> Self {
        Self {
            // Different ranges for linear space
            blacks: GradingRGBMSW::identity(0.0, 4.0),
            shadows: GradingRGBMSW::identity(2.0, -7.0),
            midtones: GradingRGBMSW::identity(0.0, 8.0),
            highlights: GradingRGBMSW::identity(-2.0, 9.0),
            whites: GradingRGBMSW::identity(0.0, 8.0),
            s_contrast: 1.0,
        }
    }

    /// VIDEO style defaults (from OCIO).
    ///
    /// For gamma-encoded video footage.
    pub fn video_defaults() -> Self {
        Self {
            blacks: GradingRGBMSW::identity(0.4, 0.4),
            shadows: GradingRGBMSW::identity(0.6, 0.0),
            midtones: GradingRGBMSW::identity(0.4, 0.7),
            highlights: GradingRGBMSW::identity(0.2, 1.0),
            whites: GradingRGBMSW::identity(0.5, 0.5),
            s_contrast: 1.0,
        }
    }

    /// Check if this is an identity transform (no effect).
    pub fn is_identity(&self) -> bool {
        self.blacks.is_identity()
            && self.shadows.is_identity()
            && self.midtones.is_identity()
            && self.highlights.is_identity()
            && self.whites.is_identity()
            && self.s_contrast == 1.0
    }
}

impl Default for GradingTone {
    fn default() -> Self {
        Self::log_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity() {
        let tone = GradingTone::new(GradingStyle::Log);
        assert!(tone.is_identity());

        let lin = GradingTone::new(GradingStyle::Linear);
        assert!(lin.is_identity());

        let vid = GradingTone::new(GradingStyle::Video);
        assert!(vid.is_identity());
    }

    #[test]
    fn test_channel_get() {
        let zone = GradingRGBMSW::new(1.2, 1.3, 1.4, 1.5, 0.5, 0.6);
        assert_eq!(zone.get(RGBMChannel::R), 1.2);
        assert_eq!(zone.get(RGBMChannel::G), 1.3);
        assert_eq!(zone.get(RGBMChannel::B), 1.4);
        assert_eq!(zone.get(RGBMChannel::M), 1.5);
    }
}
