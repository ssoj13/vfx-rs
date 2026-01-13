//! ASC CDL (Color Decision List) operations.
//!
//! Re-exports `vfx_color::cdl::Cdl` and adds buffer processing utilities.
//!
//! # Example
//!
//! ```rust
//! use vfx_ops::Cdl;
//!
//! let cdl = Cdl::new()
//!     .with_slope([1.1, 1.0, 0.9])
//!     .with_saturation(1.2);
//!
//! let mut pixel = [0.5, 0.5, 0.5];
//! cdl.apply(&mut pixel);
//! ```

// Re-export canonical Cdl from vfx-color
pub use vfx_color::cdl::Cdl;

/// Apply CDL to an RGB buffer in-place.
///
/// Buffer must contain RGB triplets (length divisible by 3).
pub fn apply_cdl_inplace(buffer: &mut [f32], cdl: &Cdl) {
    if cdl.is_identity() {
        return;
    }

    for chunk in buffer.chunks_exact_mut(3) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        cdl.apply(&mut rgb);
        chunk[0] = rgb[0];
        chunk[1] = rgb[1];
        chunk[2] = rgb[2];
    }
}

/// Apply CDL to an RGBA buffer in-place (alpha unchanged).
///
/// Buffer must contain RGBA quads (length divisible by 4).
pub fn apply_cdl_rgba_inplace(buffer: &mut [f32], cdl: &Cdl) {
    if cdl.is_identity() {
        return;
    }

    for chunk in buffer.chunks_exact_mut(4) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        cdl.apply(&mut rgb);
        chunk[0] = rgb[0];
        chunk[1] = rgb[1];
        chunk[2] = rgb[2];
        // alpha unchanged
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    #[test]
    fn test_identity() {
        let cdl = Cdl::new();
        assert!(cdl.is_identity());

        let mut pixel = [0.5, 0.3, 0.7];
        let original = pixel;
        cdl.apply(&mut pixel);

        assert!((pixel[0] - original[0]).abs() < EPSILON);
        assert!((pixel[1] - original[1]).abs() < EPSILON);
        assert!((pixel[2] - original[2]).abs() < EPSILON);
    }

    #[test]
    fn test_buffer_processing() {
        let cdl = Cdl::new().with_slope([2.0, 2.0, 2.0]);

        let mut buffer = [0.25, 0.25, 0.25, 0.5, 0.5, 0.5];
        apply_cdl_inplace(&mut buffer, &cdl);

        assert!((buffer[0] - 0.5).abs() < EPSILON);
        assert!((buffer[3] - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_rgba_buffer() {
        let cdl = Cdl::new().with_slope([2.0, 2.0, 2.0]);

        let mut buffer = [0.25, 0.25, 0.25, 0.8]; // RGBA
        apply_cdl_rgba_inplace(&mut buffer, &cdl);

        assert!((buffer[0] - 0.5).abs() < EPSILON);
        assert!((buffer[3] - 0.8).abs() < EPSILON); // alpha unchanged
    }
}
