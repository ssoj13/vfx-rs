//! FFT operations for Python.
//!
//! Provides FFT (Fast Fourier Transform) and inverse FFT operations
//! for frequency domain image processing.

use pyo3::prelude::*;
use pyo3::exceptions::PyIOError;

use vfx_io::imagebuf::ImageBuf;
use vfx_io::imagebufalgo::fft as rust_fft;

use vfx_core::Roi3D as RustRoi3D;

use crate::Image;
use crate::core::Roi3D;

// ============================================================================
// Helper Functions
// ============================================================================

fn image_to_imagebuf(img: &Image) -> ImageBuf {
    ImageBuf::from_image_data(img.as_image_data())
}

fn imagebuf_to_image(buf: &ImageBuf) -> PyResult<Image> {
    let data = buf.to_image_data()
        .map_err(|e| PyIOError::new_err(format!("Conversion failed: {}", e)))?;
    Ok(Image::from_image_data(data))
}

fn py_roi_to_rust(roi: &Roi3D) -> RustRoi3D {
    RustRoi3D {
        xbegin: roi.xbegin,
        xend: roi.xend,
        ybegin: roi.ybegin,
        yend: roi.yend,
        zbegin: roi.zbegin,
        zend: roi.zend,
        chbegin: roi.chbegin,
        chend: roi.chend,
    }
}

fn convert_roi(roi: Option<&Roi3D>) -> Option<RustRoi3D> {
    roi.map(py_roi_to_rust)
}

// ============================================================================
// FFT Functions
// ============================================================================

/// Compute the discrete Fourier transform (DFT) of an image.
///
/// Takes a single-channel image and returns a 2-channel float image
/// where channel 0 is the real component and channel 1 is the imaginary
/// component.
///
/// The result is the unitary DFT, scaled by 1/sqrt(npixels).
///
/// Args:
///     image: Source image (single channel is used)
///     roi: Optional region of interest
///
/// Returns:
///     2-channel float image with real and imaginary components
///
/// Example:
///     >>> freq = fft(grayscale_image)
///     >>> # freq has 2 channels: "real" and "imag"
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn fft(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = rust_fft::fft(&buf, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Compute the inverse discrete Fourier transform.
///
/// Takes a 2-channel complex image (real and imaginary) and returns
/// a single-channel spatial domain image.
///
/// The input should be a 2-channel float image where channel 0 is real
/// and channel 1 is imaginary. The result is scaled by 1/sqrt(npixels).
///
/// Args:
///     image: Source complex image (2 channels: real, imag)
///     roi: Optional region of interest
///
/// Returns:
///     Single-channel float image (real component of spatial domain)
///
/// Example:
///     >>> freq = fft(image)
///     >>> # ... modify frequency domain ...
///     >>> spatial = ifft(freq)
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn ifft(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = rust_fft::ifft(&buf, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Convert complex representation to polar representation.
///
/// Input: 2-channel image with (real, imaginary)
/// Output: 2-channel image with (amplitude, phase)
///
/// The phase is in the range [0, 2*PI].
///
/// Args:
///     image: Source complex image (real, imag)
///     roi: Optional region of interest
///
/// Returns:
///     2-channel image with (amplitude, phase)
///
/// Example:
///     >>> freq = fft(image)
///     >>> polar = complex_to_polar(freq)
///     >>> # polar[0] = amplitude, polar[1] = phase
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn complex_to_polar(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = rust_fft::complex_to_polar(&buf, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Convert polar representation to complex representation.
///
/// Input: 2-channel image with (amplitude, phase)
/// Output: 2-channel image with (real, imaginary)
///
/// Args:
///     image: Source polar image (amplitude, phase)
///     roi: Optional region of interest
///
/// Returns:
///     2-channel image with (real, imag)
///
/// Example:
///     >>> # After modifying polar representation
///     >>> complex_img = polar_to_complex(polar)
///     >>> spatial = ifft(complex_img)
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn polar_to_complex(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = rust_fft::polar_to_complex(&buf, convert_roi(roi));
    imagebuf_to_image(&result)
}

// ============================================================================
// Module Registration
// ============================================================================

/// Register all FFT functions to the module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(fft, m)?)?;
    m.add_function(wrap_pyfunction!(ifft, m)?)?;
    m.add_function(wrap_pyfunction!(complex_to_polar, m)?)?;
    m.add_function(wrap_pyfunction!(polar_to_complex, m)?)?;
    Ok(())
}
