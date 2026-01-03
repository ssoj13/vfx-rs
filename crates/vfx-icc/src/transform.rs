//! Color transforms between ICC profiles.

use crate::{IccError, IccResult, Intent, Profile};
use lcms2::{Flags, Transform as LcmsTransform};

/// A color transform between two ICC profiles.
///
/// Converts pixel data from the source color space to the destination
/// color space using the specified rendering intent.
///
/// # Example
///
/// ```rust
/// use vfx_icc::{Profile, Transform, Intent};
///
/// let srgb = Profile::srgb();
/// let aces = Profile::aces_ap1();
///
/// let transform = Transform::new(&srgb, &aces, Intent::Perceptual).unwrap();
///
/// let mut pixels = [[0.5f32, 0.3, 0.2]];
/// transform.apply(&mut pixels);
/// ```
pub struct Transform {
    inner: LcmsTransform<[f32; 3], [f32; 3]>,
}

impl Transform {
    /// Creates a new transform between two profiles.
    ///
    /// Uses 32-bit float RGB format for both input and output.
    ///
    /// # Arguments
    ///
    /// * `source` - Source color profile
    /// * `dest` - Destination color profile
    /// * `intent` - Rendering intent
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_icc::{Profile, Transform, Intent};
    ///
    /// let camera = Profile::srgb();
    /// let display = Profile::display_p3();
    ///
    /// let transform = Transform::new(&camera, &display, Intent::Perceptual).unwrap();
    /// ```
    pub fn new(source: &Profile, dest: &Profile, intent: Intent) -> IccResult<Self> {
        let inner = LcmsTransform::new(
            &source.inner,
            lcms2::PixelFormat::RGB_FLT,
            &dest.inner,
            lcms2::PixelFormat::RGB_FLT,
            intent.into(),
        )
        .map_err(|e| IccError::TransformFailed(e.to_string()))?;

        Ok(Self { inner })
    }

    /// Creates a thread-safe transform without caching.
    ///
    /// This transform can be shared between threads safely.
    ///
    /// # Arguments
    ///
    /// * `source` - Source color profile
    /// * `dest` - Destination color profile
    /// * `intent` - Rendering intent
    pub fn new_uncached(source: &Profile, dest: &Profile, intent: Intent) -> IccResult<Self> {
        let inner = LcmsTransform::new_flags(
            &source.inner,
            lcms2::PixelFormat::RGB_FLT,
            &dest.inner,
            lcms2::PixelFormat::RGB_FLT,
            intent.into(),
            Flags::NO_CACHE,
        )
        .map_err(|e| IccError::TransformFailed(e.to_string()))?;

        Ok(Self { inner })
    }

    /// Applies the transform to RGB pixels in-place.
    ///
    /// # Arguments
    ///
    /// * `pixels` - Array of RGB pixels [f32; 3]
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_icc::{Profile, Transform, Intent};
    ///
    /// let transform = Transform::new(
    ///     &Profile::srgb(),
    ///     &Profile::aces_ap1(),
    ///     Intent::Perceptual,
    /// ).unwrap();
    ///
    /// let mut pixels = [[0.5f32, 0.3, 0.2], [0.8, 0.6, 0.4]];
    /// transform.apply(&mut pixels);
    /// ```
    pub fn apply(&self, pixels: &mut [[f32; 3]]) {
        self.inner.transform_in_place(pixels);
    }

    /// Applies the transform to a single RGB pixel in-place.
    ///
    /// # Arguments
    ///
    /// * `rgb` - RGB pixel as [f32; 3]
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_icc::{Profile, Transform, Intent};
    ///
    /// let transform = Transform::new(
    ///     &Profile::srgb(),
    ///     &Profile::aces_ap1(),
    ///     Intent::Perceptual,
    /// ).unwrap();
    ///
    /// let mut pixel = [0.5f32, 0.3, 0.2];
    /// transform.apply_pixel(&mut pixel);
    /// ```
    pub fn apply_pixel(&self, rgb: &mut [f32; 3]) {
        let pixels = std::slice::from_mut(rgb);
        self.inner.transform_in_place(pixels);
    }

    /// Transforms pixels from source to destination buffer.
    ///
    /// # Arguments
    ///
    /// * `source` - Source pixels
    /// * `dest` - Destination buffer (must be same length as source)
    pub fn transform(&self, source: &[[f32; 3]], dest: &mut [[f32; 3]]) {
        assert_eq!(source.len(), dest.len(), "source and dest must have same length");
        self.inner.transform_pixels(source, dest);
    }

    /// Applies the transform to a flat f32 buffer.
    ///
    /// Buffer must contain RGB triplets (length divisible by 3).
    ///
    /// # Arguments
    ///
    /// * `data` - Flat RGB data as f32 values
    pub fn apply_buffer(&self, data: &mut [f32]) {
        assert!(data.len() % 3 == 0, "buffer length must be divisible by 3");
        
        // Safe cast: [f32; 3] has same layout as 3 contiguous f32s
        let pixels: &mut [[f32; 3]] = unsafe {
            std::slice::from_raw_parts_mut(
                data.as_mut_ptr() as *mut [f32; 3],
                data.len() / 3,
            )
        };
        
        self.inner.transform_in_place(pixels);
    }
}

impl std::fmt::Debug for Transform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transform").finish_non_exhaustive()
    }
}

/// Convenience function to convert RGB from one profile to another.
///
/// Creates a temporary transform and applies it.
/// For repeated conversions, prefer creating a [`Transform`] once.
///
/// # Example
///
/// ```rust
/// use vfx_icc::{Profile, Intent, convert_rgb};
///
/// let mut pixels = [[0.5f32, 0.3, 0.2]];
/// convert_rgb(&mut pixels, &Profile::srgb(), &Profile::aces_ap1(), Intent::Perceptual).unwrap();
/// ```
pub fn convert_rgb(
    pixels: &mut [[f32; 3]],
    source: &Profile,
    dest: &Profile,
    intent: Intent,
) -> IccResult<()> {
    let transform = Transform::new(source, dest, intent)?;
    transform.apply(pixels);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srgb_to_aces() {
        let srgb = Profile::srgb();
        let aces = Profile::aces_ap1();

        let transform = Transform::new(&srgb, &aces, Intent::Perceptual).unwrap();

        let mut pixels = [[0.5f32, 0.5, 0.5]];
        transform.apply(&mut pixels);

        // Values should change (ACES has different primaries)
        // Mid-gray should remain approximately neutral
        assert!((pixels[0][0] - pixels[0][1]).abs() < 0.1);
        assert!((pixels[0][1] - pixels[0][2]).abs() < 0.1);
    }

    #[test]
    fn test_identity() {
        let srgb = Profile::srgb();

        let transform = Transform::new(&srgb, &srgb, Intent::Perceptual).unwrap();

        let original = [0.5f32, 0.3, 0.2];
        let mut pixels = [original];
        transform.apply(&mut pixels);

        // Should be unchanged
        assert!((pixels[0][0] - original[0]).abs() < 0.01);
        assert!((pixels[0][1] - original[1]).abs() < 0.01);
        assert!((pixels[0][2] - original[2]).abs() < 0.01);
    }

    #[test]
    fn test_pixel() {
        let transform = Transform::new(
            &Profile::srgb(),
            &Profile::linear_srgb(),
            Intent::Perceptual,
        )
        .unwrap();

        let mut pixel = [0.5f32, 0.5, 0.5];
        transform.apply_pixel(&mut pixel);

        // Linearized values should be lower (gamma removed)
        assert!(pixel[0] < 0.5);
    }

    #[test]
    fn test_convert_rgb() {
        let mut pixels = [[0.5f32, 0.3, 0.2]];
        convert_rgb(
            &mut pixels,
            &Profile::srgb(),
            &Profile::display_p3(),
            Intent::RelativeColorimetric,
        )
        .unwrap();

        // P3 has wider gamut, so sRGB colors should map slightly differently
        assert!(pixels[0][0] > 0.0 && pixels[0][0] < 1.0);
    }

    #[test]
    fn test_buffer() {
        let transform = Transform::new(
            &Profile::srgb(),
            &Profile::linear_srgb(),
            Intent::Perceptual,
        )
        .unwrap();

        let mut buffer = [0.5f32, 0.5, 0.5, 0.8, 0.8, 0.8];
        transform.apply_buffer(&mut buffer);

        // Linearized values should be lower
        assert!(buffer[0] < 0.5);
    }
}
