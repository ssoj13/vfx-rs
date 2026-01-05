//! Pixel access utilities for ImageBuf.

// This module re-exports items from storage and provides additional utilities
// for pixel manipulation.


/// Helper trait for pixel conversion.
pub trait PixelConvert {
    /// Converts to f32.
    fn to_f32(&self) -> f32;
    /// Converts from f32.
    fn from_f32(v: f32) -> Self;
}

impl PixelConvert for u8 {
    #[inline]
    fn to_f32(&self) -> f32 {
        *self as f32 / 255.0
    }

    #[inline]
    fn from_f32(v: f32) -> Self {
        (v.clamp(0.0, 1.0) * 255.0) as u8
    }
}

impl PixelConvert for u16 {
    #[inline]
    fn to_f32(&self) -> f32 {
        *self as f32 / 65535.0
    }

    #[inline]
    fn from_f32(v: f32) -> Self {
        (v.clamp(0.0, 1.0) * 65535.0) as u16
    }
}

impl PixelConvert for f32 {
    #[inline]
    fn to_f32(&self) -> f32 {
        *self
    }

    #[inline]
    fn from_f32(v: f32) -> Self {
        v
    }
}

impl PixelConvert for u32 {
    #[inline]
    fn to_f32(&self) -> f32 {
        *self as f32
    }

    #[inline]
    fn from_f32(v: f32) -> Self {
        v.max(0.0) as u32
    }
}

/// Clamps a value to [0, 1] range.
#[inline]
pub fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

/// Linear interpolation between two values.
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Bilinear interpolation.
#[inline]
pub fn bilerp(v00: f32, v10: f32, v01: f32, v11: f32, fx: f32, fy: f32) -> f32 {
    let top = lerp(v00, v10, fx);
    let bot = lerp(v01, v11, fx);
    lerp(top, bot, fy)
}

/// Computes cubic interpolation weights.
pub fn cubic_weights(t: f32) -> [f32; 4] {
    let t2 = t * t;
    let t3 = t2 * t;
    [
        -0.5 * t3 + t2 - 0.5 * t,
        1.5 * t3 - 2.5 * t2 + 1.0,
        -1.5 * t3 + 2.0 * t2 + 0.5 * t,
        0.5 * t3 - 0.5 * t2,
    ]
}

/// Catmull-Rom cubic interpolation.
pub fn cubic_interp(v: [f32; 4], t: f32) -> f32 {
    let w = cubic_weights(t);
    v[0] * w[0] + v[1] * w[1] + v[2] * w[2] + v[3] * w[3]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_convert_u8() {
        assert_eq!(255u8.to_f32(), 1.0);
        assert_eq!(0u8.to_f32(), 0.0);
        assert_eq!(u8::from_f32(1.0), 255);
        assert_eq!(u8::from_f32(0.0), 0);
        assert_eq!(u8::from_f32(0.5), 127); // ~127.5 truncated
    }

    #[test]
    fn test_lerp() {
        assert!((lerp(0.0, 1.0, 0.0) - 0.0).abs() < 0.001);
        assert!((lerp(0.0, 1.0, 1.0) - 1.0).abs() < 0.001);
        assert!((lerp(0.0, 1.0, 0.5) - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_bilerp() {
        // All corners same value
        assert!((bilerp(1.0, 1.0, 1.0, 1.0, 0.5, 0.5) - 1.0).abs() < 0.001);

        // Linear gradient
        assert!((bilerp(0.0, 1.0, 0.0, 1.0, 0.5, 0.5) - 0.5).abs() < 0.001);
    }
}
