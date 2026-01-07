//! FFT (Fast Fourier Transform) operations for images.
//!
//! This module provides FFT and inverse FFT operations compatible with
//! OpenImageIO's ImageBufAlgo::fft and ImageBufAlgo::ifft.
//!
//! # Functions
//!
//! - [`fft`] - Forward discrete Fourier transform
//! - [`ifft`] - Inverse discrete Fourier transform
//! - [`complex_to_polar`] - Convert complex (real, imag) to (amplitude, phase)
//! - [`polar_to_complex`] - Convert (amplitude, phase) to (real, imag)

use crate::imagebuf::{ImageBuf, InitializePixels, WrapMode};
use num_complex::Complex;
use rustfft::{FftPlanner, FftDirection};
use std::f32::consts::PI;
use vfx_core::{DataFormat, ImageSpec, Roi3D};

/// Computes the discrete Fourier transform (DFT) of an image.
///
/// Takes a single-channel image and returns a 2-channel float image where
/// channel 0 is the real component and channel 1 is the imaginary component.
///
/// The result is the unitary DFT, scaled by 1/sqrt(npixels).
///
/// # Arguments
///
/// * `src` - Source image (single channel is used)
/// * `roi` - Optional region of interest
///
/// # Returns
///
/// 2-channel float image with real and imaginary components
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::fft;
///
/// let frequency_domain = fft(&spatial_image, None);
/// // frequency_domain has 2 channels: "real" and "imag"
/// ```
pub fn fft(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let width = roi.width() as usize;
    let height = roi.height() as usize;

    // Create output spec with 2 channels (real, imag)
    let mut spec = ImageSpec::new(width as u32, height as u32, 2, DataFormat::F32);
    spec.channel_names = vec!["real".to_string(), "imag".to_string()];

    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    fft_into(&mut dst, src, Some(roi));
    dst
}

/// Computes FFT into an existing buffer.
pub fn fft_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let width = roi.width() as usize;
    let height = roi.height() as usize;

    // Create FFT planners
    let mut planner = FftPlanner::new();
    let fft_row = planner.plan_fft(width, FftDirection::Forward);
    let fft_col = planner.plan_fft(height, FftDirection::Forward);

    // Scale factor for unitary DFT
    let scale = 1.0 / ((width * height) as f32).sqrt();

    // Step 1: Copy source to complex buffer (single channel)
    let mut data: Vec<Complex<f32>> = vec![Complex::new(0.0, 0.0); width * height];
    let mut pixel = [0.0f32; 1];

    for y in 0..height {
        for x in 0..width {
            src.getpixel(
                (roi.xbegin + x as i32) as i32,
                (roi.ybegin + y as i32) as i32,
                roi.zbegin,
                &mut pixel,
                WrapMode::Black,
            );
            data[y * width + x] = Complex::new(pixel[0], 0.0);
        }
    }

    // Step 2: FFT rows
    let mut scratch = vec![Complex::new(0.0, 0.0); fft_row.get_inplace_scratch_len()];
    for y in 0..height {
        let row_start = y * width;
        let row = &mut data[row_start..row_start + width];
        fft_row.process_with_scratch(row, &mut scratch);
    }

    // Step 3: Transpose
    let mut transposed = vec![Complex::new(0.0, 0.0); width * height];
    for y in 0..height {
        for x in 0..width {
            transposed[x * height + y] = data[y * width + x];
        }
    }

    // Step 4: FFT columns (which are now rows after transpose)
    let mut scratch = vec![Complex::new(0.0, 0.0); fft_col.get_inplace_scratch_len()];
    for x in 0..width {
        let col_start = x * height;
        let col = &mut transposed[col_start..col_start + height];
        fft_col.process_with_scratch(col, &mut scratch);
    }

    // Step 5: Transpose back and write to output
    for y in 0..height {
        for x in 0..width {
            let val = transposed[x * height + y] * scale;
            dst.setpixel(x as i32, y as i32, 0, &[val.re, val.im]);
        }
    }
}

/// Computes the inverse discrete Fourier transform.
///
/// Takes a 2-channel complex image (real and imaginary) and returns a
/// single-channel spatial domain image.
///
/// The input should be a 2-channel float image where channel 0 is real
/// and channel 1 is imaginary. The result is scaled by 1/sqrt(npixels).
///
/// # Arguments
///
/// * `src` - Source complex image (2 channels: real, imag)
/// * `roi` - Optional region of interest
///
/// # Returns
///
/// Single-channel float image (real component of spatial domain)
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::{fft, ifft};
///
/// let freq = fft(&image, None);
/// // ... modify frequency domain ...
/// let spatial = ifft(&freq, None);
/// ```
pub fn ifft(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let width = roi.width() as usize;
    let height = roi.height() as usize;

    // Create output spec with 1 channel
    let mut spec = ImageSpec::gray(width as u32, height as u32);
    spec.channel_names = vec!["Y".to_string()];

    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    ifft_into(&mut dst, src, Some(roi));
    dst
}

/// Computes inverse FFT into an existing buffer.
pub fn ifft_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let width = roi.width() as usize;
    let height = roi.height() as usize;

    // Create FFT planners (inverse direction)
    let mut planner = FftPlanner::new();
    let ifft_row = planner.plan_fft(width, FftDirection::Inverse);
    let ifft_col = planner.plan_fft(height, FftDirection::Inverse);

    // Scale factor for unitary DFT
    let scale = 1.0 / ((width * height) as f32).sqrt();

    // Step 1: Copy source complex data
    let mut data: Vec<Complex<f32>> = vec![Complex::new(0.0, 0.0); width * height];
    let mut pixel = [0.0f32; 2];

    for y in 0..height {
        for x in 0..width {
            src.getpixel(
                (roi.xbegin + x as i32) as i32,
                (roi.ybegin + y as i32) as i32,
                roi.zbegin,
                &mut pixel,
                WrapMode::Black,
            );
            data[y * width + x] = Complex::new(pixel[0], pixel[1]);
        }
    }

    // Step 2: IFFT rows
    let mut scratch = vec![Complex::new(0.0, 0.0); ifft_row.get_inplace_scratch_len()];
    for y in 0..height {
        let row_start = y * width;
        let row = &mut data[row_start..row_start + width];
        ifft_row.process_with_scratch(row, &mut scratch);
    }

    // Step 3: Transpose
    let mut transposed = vec![Complex::new(0.0, 0.0); width * height];
    for y in 0..height {
        for x in 0..width {
            transposed[x * height + y] = data[y * width + x];
        }
    }

    // Step 4: IFFT columns (which are now rows after transpose)
    let mut scratch = vec![Complex::new(0.0, 0.0); ifft_col.get_inplace_scratch_len()];
    for x in 0..width {
        let col_start = x * height;
        let col = &mut transposed[col_start..col_start + height];
        ifft_col.process_with_scratch(col, &mut scratch);
    }

    // Step 5: Transpose back and write real component to output
    for y in 0..height {
        for x in 0..width {
            let val = transposed[x * height + y] * scale;
            dst.setpixel(x as i32, y as i32, 0, &[val.re]);
        }
    }
}

/// Converts complex representation to polar representation.
///
/// Input: 2-channel image with (real, imaginary)
/// Output: 2-channel image with (amplitude, phase)
///
/// The phase is in the range [0, 2*PI].
///
/// # Arguments
///
/// * `src` - Source complex image (real, imag)
/// * `roi` - Optional region of interest
///
/// # Returns
///
/// 2-channel image with (amplitude, phase)
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::{fft, complex_to_polar};
///
/// let freq = fft(&image, None);
/// let polar = complex_to_polar(&freq, None);
/// // polar[0] = amplitude, polar[1] = phase
/// ```
pub fn complex_to_polar(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let width = roi.width() as u32;
    let height = roi.height() as u32;

    let mut spec = ImageSpec::new(width, height, 2, DataFormat::F32);
    spec.channel_names = vec!["amplitude".to_string(), "phase".to_string()];

    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    complex_to_polar_into(&mut dst, src, Some(roi));
    dst
}

/// Converts complex to polar into an existing buffer.
pub fn complex_to_polar_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut pixel = [0.0f32; 2];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                let real = pixel[0];
                let imag = pixel[1];

                // amplitude = sqrt(real^2 + imag^2)
                let amplitude = (real * real + imag * imag).sqrt();

                // phase = atan2(imag, real), normalized to [0, 2*PI]
                let mut phase = imag.atan2(real);
                if phase < 0.0 {
                    phase += 2.0 * PI;
                }

                dst.setpixel(
                    x - roi.xbegin,
                    y - roi.ybegin,
                    z - roi.zbegin,
                    &[amplitude, phase],
                );
            }
        }
    }
}

/// Converts polar representation to complex representation.
///
/// Input: 2-channel image with (amplitude, phase)
/// Output: 2-channel image with (real, imaginary)
///
/// # Arguments
///
/// * `src` - Source polar image (amplitude, phase)
/// * `roi` - Optional region of interest
///
/// # Returns
///
/// 2-channel image with (real, imag)
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::{polar_to_complex, ifft};
///
/// // After modifying polar representation
/// let complex = polar_to_complex(&polar, None);
/// let spatial = ifft(&complex, None);
/// ```
pub fn polar_to_complex(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    let roi = roi.unwrap_or_else(|| src.roi());
    let width = roi.width() as u32;
    let height = roi.height() as u32;

    let mut spec = ImageSpec::new(width, height, 2, DataFormat::F32);
    spec.channel_names = vec!["real".to_string(), "imag".to_string()];

    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    polar_to_complex_into(&mut dst, src, Some(roi));
    dst
}

/// Converts polar to complex into an existing buffer.
pub fn polar_to_complex_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let mut pixel = [0.0f32; 2];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                let amplitude = pixel[0];
                let phase = pixel[1];

                // real = amplitude * cos(phase)
                // imag = amplitude * sin(phase)
                let (sin_phase, cos_phase) = phase.sin_cos();
                let real = amplitude * cos_phase;
                let imag = amplitude * sin_phase;

                dst.setpixel(
                    x - roi.xbegin,
                    y - roi.ybegin,
                    z - roi.zbegin,
                    &[real, imag],
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_ifft_roundtrip() {
        // Create a simple test image
        let spec = ImageSpec::gray(8, 8);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        // Fill with a simple pattern
        for y in 0..8 {
            for x in 0..8 {
                let val = ((x + y) % 2) as f32;
                src.setpixel(x, y, 0, &[val]);
            }
        }

        // Forward FFT
        let freq = fft(&src, None);
        assert_eq!(freq.nchannels(), 2);
        assert_eq!(freq.width(), 8);
        assert_eq!(freq.height(), 8);

        // Inverse FFT
        let result = ifft(&freq, None);
        assert_eq!(result.nchannels(), 1);

        // Check roundtrip
        let mut orig = [0.0f32; 1];
        let mut recovered = [0.0f32; 1];

        for y in 0..8 {
            for x in 0..8 {
                src.getpixel(x, y, 0, &mut orig, WrapMode::Black);
                result.getpixel(x, y, 0, &mut recovered, WrapMode::Black);
                assert!(
                    (orig[0] - recovered[0]).abs() < 0.01,
                    "Mismatch at ({}, {}): {} vs {}",
                    x, y, orig[0], recovered[0]
                );
            }
        }
    }

    #[test]
    fn test_polar_complex_roundtrip() {
        // Create complex image
        let mut spec = ImageSpec::new(4, 4, 2, DataFormat::F32);
        spec.channel_names = vec!["real".to_string(), "imag".to_string()];
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        // Fill with complex values
        for y in 0..4 {
            for x in 0..4 {
                let real = (x as f32) * 0.25;
                let imag = (y as f32) * 0.25;
                src.setpixel(x, y, 0, &[real, imag]);
            }
        }

        // Convert to polar and back
        let polar = complex_to_polar(&src, None);
        let recovered = polar_to_complex(&polar, None);

        // Check roundtrip
        let mut orig = [0.0f32; 2];
        let mut result = [0.0f32; 2];

        for y in 0..4 {
            for x in 0..4 {
                src.getpixel(x, y, 0, &mut orig, WrapMode::Black);
                recovered.getpixel(x, y, 0, &mut result, WrapMode::Black);
                assert!(
                    (orig[0] - result[0]).abs() < 0.001,
                    "Real mismatch at ({}, {})",
                    x, y
                );
                assert!(
                    (orig[1] - result[1]).abs() < 0.001,
                    "Imag mismatch at ({}, {})",
                    x, y
                );
            }
        }
    }
}
