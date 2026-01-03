//! Image filtering and convolution operations.
//!
//! Provides common filters like blur, sharpen, and edge detection.
//!
//! # Kernels
//!
//! - [`Kernel::box_blur`] - Simple average blur
//! - [`Kernel::gaussian`] - Gaussian blur (smooth)
//! - [`Kernel::sharpen`] - Unsharp masking
//! - [`Kernel::edge_detect`] - Sobel/Laplacian edges
//!
//! # Example
//!
//! ```rust
//! use vfx_ops::filter::{convolve, Kernel};
//!
//! let src = vec![0.5f32; 16 * 16 * 3];
//! let kernel = Kernel::gaussian(3, 1.0);
//! let blurred = convolve(&src, 16, 16, 3, &kernel).unwrap();
//! ```

use crate::{OpsError, OpsResult};

/// Convolution kernel for image filtering.
#[derive(Debug, Clone)]
pub struct Kernel {
    /// Kernel weights.
    pub data: Vec<f32>,
    /// Kernel width (must be odd).
    pub width: usize,
    /// Kernel height (must be odd).
    pub height: usize,
}

impl Kernel {
    /// Creates a new kernel from data.
    ///
    /// Width and height must be odd numbers.
    pub fn new(data: Vec<f32>, width: usize, height: usize) -> OpsResult<Self> {
        if width % 2 == 0 || height % 2 == 0 {
            return Err(OpsError::InvalidParameter(
                "kernel dimensions must be odd".into(),
            ));
        }
        if data.len() != width * height {
            return Err(OpsError::InvalidParameter(format!(
                "kernel data size {} doesn't match {}x{}",
                data.len(),
                width,
                height
            )));
        }
        Ok(Self { data, width, height })
    }

    /// Creates a box blur kernel (simple average).
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_ops::filter::Kernel;
    ///
    /// let k = Kernel::box_blur(3);
    /// assert_eq!(k.width, 3);
    /// assert_eq!(k.height, 3);
    /// ```
    pub fn box_blur(size: usize) -> Self {
        let size = if size % 2 == 0 { size + 1 } else { size };
        let count = size * size;
        let weight = 1.0 / count as f32;
        Self {
            data: vec![weight; count],
            width: size,
            height: size,
        }
    }

    /// Creates a Gaussian blur kernel.
    ///
    /// # Arguments
    ///
    /// * `size` - Kernel size (will be made odd)
    /// * `sigma` - Standard deviation (blur amount)
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_ops::filter::Kernel;
    ///
    /// let k = Kernel::gaussian(5, 1.5);
    /// assert_eq!(k.width, 5);
    /// ```
    pub fn gaussian(size: usize, sigma: f32) -> Self {
        let size = if size % 2 == 0 { size + 1 } else { size };
        let half = (size / 2) as i32;
        let sigma2 = 2.0 * sigma * sigma;

        let mut data = Vec::with_capacity(size * size);
        let mut sum = 0.0f32;

        for y in -half..=half {
            for x in -half..=half {
                let d = (x * x + y * y) as f32;
                let w = (-d / sigma2).exp();
                data.push(w);
                sum += w;
            }
        }

        // Normalize
        for w in &mut data {
            *w /= sum;
        }

        Self { data, width: size, height: size }
    }

    /// Creates a sharpening kernel.
    ///
    /// # Arguments
    ///
    /// * `amount` - Sharpening strength (0.5-2.0 typical)
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_ops::filter::Kernel;
    ///
    /// let k = Kernel::sharpen(1.0);
    /// assert_eq!(k.width, 3);
    /// ```
    pub fn sharpen(amount: f32) -> Self {
        let center = 1.0 + 4.0 * amount;
        Self {
            data: vec![
                0.0, -amount, 0.0,
                -amount, center, -amount,
                0.0, -amount, 0.0,
            ],
            width: 3,
            height: 3,
        }
    }

    /// Creates an edge detection kernel (Laplacian).
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_ops::filter::Kernel;
    ///
    /// let k = Kernel::edge_detect();
    /// assert_eq!(k.width, 3);
    /// ```
    pub fn edge_detect() -> Self {
        Self {
            data: vec![
                0.0, -1.0, 0.0,
                -1.0, 4.0, -1.0,
                0.0, -1.0, 0.0,
            ],
            width: 3,
            height: 3,
        }
    }

    /// Creates an emboss kernel.
    pub fn emboss() -> Self {
        Self {
            data: vec![
                -2.0, -1.0, 0.0,
                -1.0, 1.0, 1.0,
                0.0, 1.0, 2.0,
            ],
            width: 3,
            height: 3,
        }
    }

    /// Returns the kernel radius (half-size).
    #[inline]
    pub fn radius(&self) -> (usize, usize) {
        (self.width / 2, self.height / 2)
    }
}

/// Applies convolution filter to image.
///
/// # Arguments
///
/// * `src` - Source pixel data
/// * `width` - Image width
/// * `height` - Image height
/// * `channels` - Number of channels (3 or 4)
/// * `kernel` - Convolution kernel
///
/// # Returns
///
/// Filtered image as Vec<f32>.
///
/// # Example
///
/// ```rust
/// use vfx_ops::filter::{convolve, Kernel};
///
/// let src = vec![0.5f32; 8 * 8 * 3];
/// let kernel = Kernel::box_blur(3);
/// let result = convolve(&src, 8, 8, 3, &kernel).unwrap();
/// assert_eq!(result.len(), 8 * 8 * 3);
/// ```
pub fn convolve(
    src: &[f32],
    width: usize,
    height: usize,
    channels: usize,
    kernel: &Kernel,
) -> OpsResult<Vec<f32>> {
    let expected = width * height * channels;
    if src.len() != expected {
        return Err(OpsError::InvalidDimensions(format!(
            "expected {} pixels, got {}",
            expected,
            src.len()
        )));
    }

    let mut dst = vec![0.0f32; expected];
    let (rx, ry) = kernel.radius();

    for y in 0..height {
        for x in 0..width {
            let mut sums = vec![0.0f32; channels];

            for ky in 0..kernel.height {
                for kx in 0..kernel.width {
                    // Source coordinates with edge clamping
                    let sx = (x as isize + kx as isize - rx as isize)
                        .max(0)
                        .min(width as isize - 1) as usize;
                    let sy = (y as isize + ky as isize - ry as isize)
                        .max(0)
                        .min(height as isize - 1) as usize;

                    let src_idx = (sy * width + sx) * channels;
                    let kw = kernel.data[ky * kernel.width + kx];

                    for c in 0..channels {
                        sums[c] += src[src_idx + c] * kw;
                    }
                }
            }

            let dst_idx = (y * width + x) * channels;
            for c in 0..channels {
                dst[dst_idx + c] = sums[c];
            }
        }
    }

    Ok(dst)
}

/// Fast box blur using sliding window (separable).
///
/// More efficient than general convolution for box blur.
///
/// # Example
///
/// ```rust
/// use vfx_ops::filter::box_blur;
///
/// let src = vec![0.5f32; 16 * 16 * 4];
/// let result = box_blur(&src, 16, 16, 4, 3).unwrap();
/// ```
pub fn box_blur(
    src: &[f32],
    width: usize,
    height: usize,
    channels: usize,
    radius: usize,
) -> OpsResult<Vec<f32>> {
    let expected = width * height * channels;
    if src.len() != expected {
        return Err(OpsError::InvalidDimensions(format!(
            "expected {} pixels, got {}",
            expected,
            src.len()
        )));
    }

    // Two-pass separable blur
    let temp = blur_horizontal(src, width, height, channels, radius);
    let result = blur_vertical(&temp, width, height, channels, radius);

    Ok(result)
}

/// Horizontal blur pass.
fn blur_horizontal(
    src: &[f32],
    width: usize,
    height: usize,
    channels: usize,
    radius: usize,
) -> Vec<f32> {
    let mut dst = vec![0.0f32; width * height * channels];
    let kernel_size = 2 * radius + 1;
    let inv_size = 1.0 / kernel_size as f32;

    for y in 0..height {
        for c in 0..channels {
            // Initialize sum for first pixel
            let mut sum = 0.0f32;
            for kx in 0..=radius {
                let sx = kx.min(width - 1);
                sum += src[(y * width + sx) * channels + c];
            }
            // Add extra from left edge clamping
            sum += src[y * width * channels + c] * radius as f32;

            for x in 0..width {
                dst[(y * width + x) * channels + c] = sum * inv_size;

                // Slide window
                let left = (x as isize - radius as isize).max(0) as usize;
                let right = (x + radius + 1).min(width - 1);

                sum -= src[(y * width + left) * channels + c];
                sum += src[(y * width + right) * channels + c];
            }
        }
    }

    dst
}

/// Vertical blur pass.
fn blur_vertical(
    src: &[f32],
    width: usize,
    height: usize,
    channels: usize,
    radius: usize,
) -> Vec<f32> {
    let mut dst = vec![0.0f32; width * height * channels];
    let kernel_size = 2 * radius + 1;
    let inv_size = 1.0 / kernel_size as f32;

    for x in 0..width {
        for c in 0..channels {
            // Initialize sum for first pixel
            let mut sum = 0.0f32;
            for ky in 0..=radius {
                let sy = ky.min(height - 1);
                sum += src[(sy * width + x) * channels + c];
            }
            sum += src[x * channels + c] * radius as f32;

            for y in 0..height {
                dst[(y * width + x) * channels + c] = sum * inv_size;

                // Slide window
                let top = (y as isize - radius as isize).max(0) as usize;
                let bottom = (y + radius + 1).min(height - 1);

                sum -= src[(top * width + x) * channels + c];
                sum += src[(bottom * width + x) * channels + c];
            }
        }
    }

    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_box() {
        let k = Kernel::box_blur(3);
        assert_eq!(k.width, 3);
        assert_eq!(k.height, 3);
        assert_eq!(k.data.len(), 9);

        // All weights should be equal
        let w = 1.0 / 9.0;
        for v in &k.data {
            assert!((*v - w).abs() < 0.001);
        }
    }

    #[test]
    fn test_kernel_gaussian() {
        let k = Kernel::gaussian(5, 1.0);
        assert_eq!(k.width, 5);
        assert_eq!(k.height, 5);

        // Sum should be 1.0
        let sum: f32 = k.data.iter().sum();
        assert!((sum - 1.0).abs() < 0.001);

        // Center should be highest
        let center = k.data[12]; // Middle of 5x5
        assert!(center > k.data[0]); // Corner
    }

    #[test]
    fn test_kernel_sharpen() {
        let k = Kernel::sharpen(1.0);
        assert_eq!(k.width, 3);

        // Sum should be 1.0 (preserves brightness)
        let sum: f32 = k.data.iter().sum();
        assert!((sum - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_convolve_identity() {
        // Constant image stays constant after box blur
        let src = vec![0.5f32; 8 * 8 * 3];
        let kernel = Kernel::box_blur(3);
        let result = convolve(&src, 8, 8, 3, &kernel).unwrap();

        for v in result {
            assert!((v - 0.5).abs() < 0.01);
        }
    }

    #[test]
    fn test_box_blur_fast() {
        let src = vec![0.5f32; 16 * 16 * 4];
        let result = box_blur(&src, 16, 16, 4, 2).unwrap();
        assert_eq!(result.len(), src.len());

        // Constant image stays constant
        for v in result {
            assert!((v - 0.5).abs() < 0.01);
        }
    }
}
