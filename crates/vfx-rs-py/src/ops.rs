//! ImageBufAlgo operations for Python.
//!
//! Provides both standalone functions and Image methods for all image operations.
//! This module exposes OIIO-compatible ImageBufAlgo operations to Python.

use pyo3::prelude::*;
use pyo3::exceptions::{PyValueError, PyIOError};

use vfx_io::imagebuf::{ImageBuf, InitializePixels, WrapMode as RustWrapMode};
use vfx_io::imagebufalgo;
use vfx_io::imagebufalgo::geometry::ResizeFilter as RustResizeFilter;
use vfx_core::{ImageSpec, Roi3D as RustRoi3D, DataFormat as RustDataFormat};

use crate::Image;
use crate::core::Roi3D;

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert Python Image to Rust ImageBuf
fn image_to_imagebuf(img: &Image) -> ImageBuf {
    ImageBuf::from_image_data(img.as_image_data())
}

/// Convert Rust ImageBuf back to Python Image
fn imagebuf_to_image(buf: &ImageBuf) -> PyResult<Image> {
    let data = buf.to_image_data()
        .map_err(|e| PyIOError::new_err(format!("Failed to convert ImageBuf: {}", e)))?;
    Ok(Image::from_image_data(data))
}

/// Convert Python Roi3D to Rust Roi3D
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

/// Convert optional Python Roi3D to Rust Roi3D
fn convert_roi(roi: Option<&Roi3D>) -> Option<RustRoi3D> {
    roi.map(py_roi_to_rust)
}

// ============================================================================
// Wrap Mode enum
// ============================================================================

/// Wrap mode for pixel access outside image bounds.
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapMode {
    /// Return black/zero for out-of-bounds pixels
    Black = 0,
    /// Clamp coordinates to edge pixels
    Clamp = 1,
    /// Periodic/tiling wrap
    Periodic = 2,
    /// Mirror at boundaries
    Mirror = 3,
}

impl From<WrapMode> for RustWrapMode {
    fn from(w: WrapMode) -> Self {
        match w {
            WrapMode::Black => RustWrapMode::Black,
            WrapMode::Clamp => RustWrapMode::Clamp,
            WrapMode::Periodic => RustWrapMode::Periodic,
            WrapMode::Mirror => RustWrapMode::Mirror,
        }
    }
}

// ============================================================================
// Filter type enum
// ============================================================================

/// Filter kernel types for image filtering.
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterType {
    /// Box filter (average)
    Box = 0,
    /// Triangle (bilinear) filter
    Triangle = 1,
    /// Gaussian filter
    Gaussian = 2,
    /// Mitchell-Netravali filter
    Mitchell = 3,
    /// Lanczos 3-lobe filter
    Lanczos3 = 4,
}

// ============================================================================
// Geometry Operations
// ============================================================================

/// Flip an image vertically (top to bottom).
///
/// Args:
///     image: Input image
///     roi: Optional region of interest
///
/// Returns:
///     New flipped image
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn flip(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::flip(&buf, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Flip an image horizontally (left to right).
///
/// Args:
///     image: Input image
///     roi: Optional region of interest
///
/// Returns:
///     New flopped image
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn flop(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::flop(&buf, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Transpose an image (swap x and y axes).
///
/// Args:
///     image: Input image
///     roi: Optional region of interest
///
/// Returns:
///     New transposed image
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn transpose(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::transpose(&buf, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Rotate image 90 degrees clockwise.
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn rotate90(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::rotate90(&buf, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Rotate image 180 degrees.
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn rotate180(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::rotate180(&buf, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Rotate image 270 degrees clockwise (90 counter-clockwise).
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn rotate270(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::rotate270(&buf, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Crop image to specified region.
///
/// Args:
///     image: Input image
///     roi: Region to crop to
///
/// Returns:
///     Cropped image
#[pyfunction]
pub fn crop(image: &Image, roi: &Roi3D) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::crop(&buf, Some(py_roi_to_rust(roi)));
    imagebuf_to_image(&result)
}

/// Cut (crop) a region from an image, zeroing data outside the region.
///
/// Args:
///     image: Input image
///     roi: Region to cut
///
/// Returns:
///     Cut image
#[pyfunction]
pub fn cut(image: &Image, roi: &Roi3D) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::cut(&buf, Some(py_roi_to_rust(roi)));
    imagebuf_to_image(&result)
}

/// Resize filter types.
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeFilter {
    /// Nearest neighbor (fastest, blocky)
    Nearest = 0,
    /// Bilinear interpolation (default, smooth)
    Bilinear = 1,
    /// Bicubic interpolation (sharper)
    Bicubic = 2,
    /// Lanczos 3-lobe filter (highest quality)
    Lanczos3 = 3,
}

impl From<ResizeFilter> for RustResizeFilter {
    fn from(f: ResizeFilter) -> Self {
        match f {
            ResizeFilter::Nearest => RustResizeFilter::Nearest,
            ResizeFilter::Bilinear => RustResizeFilter::Bilinear,
            ResizeFilter::Bicubic => RustResizeFilter::Bicubic,
            ResizeFilter::Lanczos3 => RustResizeFilter::Lanczos3,
        }
    }
}

/// Resize an image to new dimensions.
///
/// Args:
///     image: Input image
///     width: Target width
///     height: Target height
///     filter: Filter type (default: Bilinear)
///     roi: Optional region of interest
///
/// Returns:
///     Resized image
#[pyfunction]
#[pyo3(signature = (image, width, height, filter=None, roi=None))]
pub fn resize(
    image: &Image,
    width: u32,
    height: u32,
    filter: Option<ResizeFilter>,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let rust_filter = filter.map(|f| f.into()).unwrap_or(RustResizeFilter::Bilinear);
    let result = imagebufalgo::resize(&buf, width, height, rust_filter, convert_roi(roi));
    imagebuf_to_image(&result)
}

// ============================================================================
// Arithmetic Operations
// ============================================================================

/// Add two images or add a constant to an image.
///
/// Args:
///     a: First image
///     b: Second image or constant value (float or list of floats)
///     roi: Optional region of interest
///
/// Returns:
///     Result image: a + b
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn add(a: &Image, b: &Bound<'_, PyAny>, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);

    let result = if let Ok(img_b) = b.extract::<Image>() {
        let buf_b = image_to_imagebuf(&img_b);
        imagebufalgo::add(&buf_a, &buf_b, convert_roi(roi))
    } else if let Ok(val) = b.extract::<f32>() {
        imagebufalgo::add(&buf_a, val, convert_roi(roi))
    } else if let Ok(vals) = b.extract::<Vec<f32>>() {
        imagebufalgo::add(&buf_a, vals, convert_roi(roi))
    } else {
        return Err(PyValueError::new_err("b must be Image, float, or list of floats"));
    };

    imagebuf_to_image(&result)
}

/// Subtract two images or subtract a constant from an image.
///
/// Args:
///     a: First image
///     b: Second image or constant value
///     roi: Optional region of interest
///
/// Returns:
///     Result image: a - b
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn sub(a: &Image, b: &Bound<'_, PyAny>, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);

    let result = if let Ok(img_b) = b.extract::<Image>() {
        let buf_b = image_to_imagebuf(&img_b);
        imagebufalgo::sub(&buf_a, &buf_b, convert_roi(roi))
    } else if let Ok(val) = b.extract::<f32>() {
        imagebufalgo::sub(&buf_a, val, convert_roi(roi))
    } else if let Ok(vals) = b.extract::<Vec<f32>>() {
        imagebufalgo::sub(&buf_a, vals, convert_roi(roi))
    } else {
        return Err(PyValueError::new_err("b must be Image, float, or list of floats"));
    };

    imagebuf_to_image(&result)
}

/// Multiply two images or multiply an image by a constant.
///
/// Args:
///     a: First image
///     b: Second image or constant value
///     roi: Optional region of interest
///
/// Returns:
///     Result image: a * b
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn mul(a: &Image, b: &Bound<'_, PyAny>, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);

    let result = if let Ok(img_b) = b.extract::<Image>() {
        let buf_b = image_to_imagebuf(&img_b);
        imagebufalgo::mul(&buf_a, &buf_b, convert_roi(roi))
    } else if let Ok(val) = b.extract::<f32>() {
        imagebufalgo::mul(&buf_a, val, convert_roi(roi))
    } else if let Ok(vals) = b.extract::<Vec<f32>>() {
        imagebufalgo::mul(&buf_a, vals, convert_roi(roi))
    } else {
        return Err(PyValueError::new_err("b must be Image, float, or list of floats"));
    };

    imagebuf_to_image(&result)
}

/// Divide two images or divide an image by a constant.
///
/// Args:
///     a: First image
///     b: Second image or constant value
///     roi: Optional region of interest
///
/// Returns:
///     Result image: a / b
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn div(a: &Image, b: &Bound<'_, PyAny>, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);

    let result = if let Ok(img_b) = b.extract::<Image>() {
        let buf_b = image_to_imagebuf(&img_b);
        imagebufalgo::div(&buf_a, &buf_b, convert_roi(roi))
    } else if let Ok(val) = b.extract::<f32>() {
        imagebufalgo::div(&buf_a, val, convert_roi(roi))
    } else if let Ok(vals) = b.extract::<Vec<f32>>() {
        imagebufalgo::div(&buf_a, vals, convert_roi(roi))
    } else {
        return Err(PyValueError::new_err("b must be Image, float, or list of floats"));
    };

    imagebuf_to_image(&result)
}

/// Multiply-add: a * b + c
///
/// Args:
///     a: First operand
///     b: Second operand (image or constant)
///     c: Third operand (image or constant)
///     roi: Optional region of interest
///
/// Returns:
///     Result image: a * b + c
#[pyfunction]
#[pyo3(signature = (a, b, c, roi=None))]
pub fn mad(a: &Image, b: &Bound<'_, PyAny>, c: &Bound<'_, PyAny>, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);

    // Convert b
    let result = if let Ok(img_b) = b.extract::<Image>() {
        let buf_b = image_to_imagebuf(&img_b);
        if let Ok(img_c) = c.extract::<Image>() {
            let buf_c = image_to_imagebuf(&img_c);
            imagebufalgo::mad(&buf_a, &buf_b, &buf_c, convert_roi(roi))
        } else if let Ok(val_c) = c.extract::<f32>() {
            imagebufalgo::mad(&buf_a, &buf_b, val_c, convert_roi(roi))
        } else if let Ok(vals_c) = c.extract::<Vec<f32>>() {
            imagebufalgo::mad(&buf_a, &buf_b, vals_c, convert_roi(roi))
        } else {
            return Err(PyValueError::new_err("c must be Image, float, or list"));
        }
    } else if let Ok(val_b) = b.extract::<f32>() {
        if let Ok(img_c) = c.extract::<Image>() {
            let buf_c = image_to_imagebuf(&img_c);
            imagebufalgo::mad(&buf_a, val_b, &buf_c, convert_roi(roi))
        } else if let Ok(val_c) = c.extract::<f32>() {
            imagebufalgo::mad(&buf_a, val_b, val_c, convert_roi(roi))
        } else {
            return Err(PyValueError::new_err("c must be Image or float"));
        }
    } else {
        return Err(PyValueError::new_err("b must be Image or float"));
    };

    imagebuf_to_image(&result)
}

/// Compute absolute value of each pixel.
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn abs(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::abs(&buf, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Compute absolute difference: |a - b|
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn absdiff(a: &Image, b: &Bound<'_, PyAny>, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);

    let result = if let Ok(img_b) = b.extract::<Image>() {
        let buf_b = image_to_imagebuf(&img_b);
        imagebufalgo::absdiff(&buf_a, &buf_b, convert_roi(roi))
    } else if let Ok(val) = b.extract::<f32>() {
        imagebufalgo::absdiff(&buf_a, val, convert_roi(roi))
    } else if let Ok(vals) = b.extract::<Vec<f32>>() {
        imagebufalgo::absdiff(&buf_a, vals, convert_roi(roi))
    } else {
        return Err(PyValueError::new_err("b must be Image, float, or list of floats"));
    };

    imagebuf_to_image(&result)
}

/// Raise pixel values to a power.
///
/// Args:
///     image: Input image
///     exponent: Power value(s) per channel
///     roi: Optional region of interest
///
/// Returns:
///     Result image with pixels raised to power
#[pyfunction]
#[pyo3(signature = (image, exponent, roi=None))]
pub fn pow(image: &Image, exponent: &Bound<'_, PyAny>, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);

    let exp_vec = if let Ok(val) = exponent.extract::<f32>() {
        vec![val]
    } else if let Ok(vals) = exponent.extract::<Vec<f32>>() {
        vals
    } else {
        return Err(PyValueError::new_err("exponent must be float or list of floats"));
    };

    let result = imagebufalgo::pow(&buf, &exp_vec, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Clamp pixel values to a range.
///
/// Args:
///     image: Input image
///     min_val: Minimum value(s) per channel
///     max_val: Maximum value(s) per channel
///     roi: Optional region of interest
///
/// Returns:
///     Clamped image
#[pyfunction]
#[pyo3(signature = (image, min_val=0.0, max_val=1.0, roi=None))]
pub fn clamp(image: &Image, min_val: f32, max_val: f32, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::clamp(&buf, &[min_val], &[max_val], convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Invert pixel values: 1 - value.
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn invert(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::invert(&buf, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Composite A over B using alpha.
///
/// Standard Porter-Duff "over" compositing.
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn over(a: &Image, b: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);
    let buf_b = image_to_imagebuf(b);
    let result = imagebufalgo::over(&buf_a, &buf_b, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Pixel-wise maximum.
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn max(a: &Image, b: &Bound<'_, PyAny>, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);

    let result = if let Ok(img_b) = b.extract::<Image>() {
        let buf_b = image_to_imagebuf(&img_b);
        imagebufalgo::max(&buf_a, &buf_b, convert_roi(roi))
    } else if let Ok(val) = b.extract::<f32>() {
        imagebufalgo::max(&buf_a, val, convert_roi(roi))
    } else if let Ok(vals) = b.extract::<Vec<f32>>() {
        imagebufalgo::max(&buf_a, vals, convert_roi(roi))
    } else {
        return Err(PyValueError::new_err("b must be Image, float, or list of floats"));
    };

    imagebuf_to_image(&result)
}

/// Pixel-wise minimum.
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn min(a: &Image, b: &Bound<'_, PyAny>, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);

    let result = if let Ok(img_b) = b.extract::<Image>() {
        let buf_b = image_to_imagebuf(&img_b);
        imagebufalgo::min(&buf_a, &buf_b, convert_roi(roi))
    } else if let Ok(val) = b.extract::<f32>() {
        imagebufalgo::min(&buf_a, val, convert_roi(roi))
    } else if let Ok(vals) = b.extract::<Vec<f32>>() {
        imagebufalgo::min(&buf_a, vals, convert_roi(roi))
    } else {
        return Err(PyValueError::new_err("b must be Image, float, or list of floats"));
    };

    imagebuf_to_image(&result)
}

// ============================================================================
// Filter Operations
// ============================================================================

/// Apply median filter.
///
/// Args:
///     image: Input image
///     size: Filter size (default 3)
///     roi: Optional region of interest
///
/// Returns:
///     Filtered image
#[pyfunction]
#[pyo3(signature = (image, size=3, roi=None))]
pub fn median(image: &Image, size: u32, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::median(&buf, size, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Apply Gaussian blur.
///
/// Args:
///     image: Input image
///     sigma: Blur sigma (standard deviation)
///     roi: Optional region of interest
///
/// Returns:
///     Blurred image
#[pyfunction]
#[pyo3(signature = (image, sigma=1.0, roi=None))]
pub fn blur(image: &Image, sigma: f32, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::blur(&buf, sigma, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Apply box blur (average filter).
///
/// Args:
///     image: Input image
///     size: Filter size (default 3)
///     roi: Optional region of interest
///
/// Returns:
///     Blurred image
#[pyfunction]
#[pyo3(signature = (image, size=3, roi=None))]
pub fn box_blur(image: &Image, size: u32, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::box_blur(&buf, size, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Apply unsharp mask sharpening.
///
/// Args:
///     image: Input image
///     sigma: Blur sigma for mask
///     amount: Sharpening strength (typically 0.5-2.0)
///     threshold: Edge threshold (0-1)
///     roi: Optional region of interest
///
/// Returns:
///     Sharpened image
#[pyfunction]
#[pyo3(signature = (image, sigma=1.0, amount=1.0, threshold=0.0, roi=None))]
pub fn unsharp_mask(
    image: &Image,
    sigma: f32,
    amount: f32,
    threshold: f32,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::unsharp_mask(&buf, sigma, amount, threshold, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Apply sharpening filter.
///
/// Args:
///     image: Input image
///     amount: Sharpening strength
///     roi: Optional region of interest
///
/// Returns:
///     Sharpened image
#[pyfunction]
#[pyo3(signature = (image, amount=1.0, roi=None))]
pub fn sharpen(image: &Image, amount: f32, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::sharpen(&buf, amount, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Dilate (grow) bright regions.
///
/// Args:
///     image: Input image
///     size: Structuring element size (default 3)
///     roi: Optional region of interest
///
/// Returns:
///     Dilated image
#[pyfunction]
#[pyo3(signature = (image, size=3, roi=None))]
pub fn dilate(image: &Image, size: u32, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::dilate(&buf, size, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Erode (shrink) bright regions.
///
/// Args:
///     image: Input image
///     size: Structuring element size (default 3)
///     roi: Optional region of interest
///
/// Returns:
///     Eroded image
#[pyfunction]
#[pyo3(signature = (image, size=3, roi=None))]
pub fn erode(image: &Image, size: u32, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::erode(&buf, size, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Morphological opening (erode then dilate).
#[pyfunction]
#[pyo3(signature = (image, size=3, roi=None))]
pub fn morph_open(image: &Image, size: u32, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::morph_open(&buf, size, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Morphological closing (dilate then erode).
#[pyfunction]
#[pyo3(signature = (image, size=3, roi=None))]
pub fn morph_close(image: &Image, size: u32, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::morph_close(&buf, size, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Apply Laplacian edge detection.
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn laplacian(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::laplacian(&buf, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Apply Sobel edge detection.
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn sobel(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::sobel(&buf, convert_roi(roi));
    imagebuf_to_image(&result)
}

// ============================================================================
// Color Operations
// ============================================================================

/// Premultiply RGB by alpha.
///
/// Converts from straight alpha to premultiplied alpha.
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn premult(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::premult(&buf, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Unpremultiply (divide RGB by alpha).
///
/// Converts from premultiplied alpha to straight alpha.
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn unpremult(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::unpremult(&buf, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Adjust color saturation.
///
/// Args:
///     image: Input image
///     scale: Saturation scale (0=grayscale, 1=unchanged, >1=more saturated)
///     firstchannel: First RGB channel index (default 0)
///     roi: Optional region of interest
///
/// Returns:
///     Saturation-adjusted image
#[pyfunction]
#[pyo3(signature = (image, scale, firstchannel=0, roi=None))]
pub fn saturate(
    image: &Image,
    scale: f32,
    firstchannel: usize,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::saturate(&buf, scale, firstchannel, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Apply contrast remap.
///
/// Args:
///     image: Input image
///     black: Input black point
///     white: Input white point
///     min_val: Output minimum
///     max_val: Output maximum
///     scontrast: Sigmoidal contrast (1.0=linear)
///     sthresh: Sigmoidal threshold (pivot, default 0.5)
///     roi: Optional region of interest
///
/// Returns:
///     Contrast-adjusted image
#[pyfunction]
#[pyo3(signature = (image, black=0.0, white=1.0, min_val=0.0, max_val=1.0, scontrast=1.0, sthresh=0.5, roi=None))]
pub fn contrast_remap(
    image: &Image,
    black: f32,
    white: f32,
    min_val: f32,
    max_val: f32,
    scontrast: f32,
    sthresh: f32,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::contrast_remap(
        &buf, black, white, min_val, max_val, scontrast, sthresh, convert_roi(roi)
    );
    imagebuf_to_image(&result)
}

/// Color map name enum.
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMapName {
    /// Inferno (perceptually uniform, dark-to-bright)
    Inferno = 0,
    /// Viridis (perceptually uniform, blue-green-yellow)
    Viridis = 1,
    /// Turbo (rainbow-like, perceptually improved)
    Turbo = 2,
    /// Magma (perceptually uniform, dark purple to yellow)
    Magma = 3,
    /// Plasma (perceptually uniform, purple to yellow)
    Plasma = 4,
    /// Blue-Red diverging
    BlueRed = 5,
    /// Heat (black-red-yellow-white)
    Heat = 6,
    /// Spectrum/Rainbow
    Spectrum = 7,
}

impl From<ColorMapName> for imagebufalgo::ColorMapName {
    fn from(c: ColorMapName) -> Self {
        match c {
            ColorMapName::Inferno => imagebufalgo::ColorMapName::Inferno,
            ColorMapName::Viridis => imagebufalgo::ColorMapName::Viridis,
            ColorMapName::Turbo => imagebufalgo::ColorMapName::Turbo,
            ColorMapName::Magma => imagebufalgo::ColorMapName::Magma,
            ColorMapName::Plasma => imagebufalgo::ColorMapName::Plasma,
            ColorMapName::BlueRed => imagebufalgo::ColorMapName::BlueRed,
            ColorMapName::Heat => imagebufalgo::ColorMapName::Heat,
            ColorMapName::Spectrum => imagebufalgo::ColorMapName::Spectrum,
        }
    }
}

/// Apply a color map to a single-channel image.
///
/// Args:
///     image: Input image
///     srcchannel: Source channel (-1 for luminance)
///     map_name: Color map to apply
///     roi: Optional region of interest
///
/// Returns:
///     RGB color-mapped image
#[pyfunction]
#[pyo3(signature = (image, srcchannel=-1, map_name=ColorMapName::Inferno, roi=None))]
pub fn color_map(
    image: &Image,
    srcchannel: i32,
    map_name: ColorMapName,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::color_map(&buf, srcchannel, map_name.into(), convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Apply 4x4 color matrix transformation.
///
/// Args:
///     image: Input image
///     matrix: 16-element color matrix (row-major)
///     unpremult: Unpremultiply before, repremultiply after
///     roi: Optional region of interest
///
/// Returns:
///     Transformed image
#[pyfunction]
#[pyo3(signature = (image, matrix, unpremult=false, roi=None))]
pub fn colormatrixtransform(
    image: &Image,
    matrix: Vec<f32>,
    unpremult: bool,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    if matrix.len() != 16 {
        return Err(PyValueError::new_err("matrix must have 16 elements"));
    }

    let buf = image_to_imagebuf(image);
    let mut mat = [0.0f32; 16];
    mat.copy_from_slice(&matrix);
    let result = imagebufalgo::colormatrixtransform(&buf, &mat, unpremult, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Compress dynamic range for HDR processing.
///
/// Maps [0, infinity) to [0, 1) using logarithmic compression.
#[pyfunction]
#[pyo3(signature = (image, use_luma=false, roi=None))]
pub fn rangecompress(image: &Image, use_luma: bool, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::rangecompress(&buf, use_luma, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Expand dynamic range (inverse of rangecompress).
///
/// Maps [0, 1) back to [0, infinity).
#[pyfunction]
#[pyo3(signature = (image, use_luma=false, roi=None))]
pub fn rangeexpand(image: &Image, use_luma: bool, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::rangeexpand(&buf, use_luma, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Convert sRGB to linear RGB.
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn srgb_to_linear(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::srgb_to_linear(&buf, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Convert linear RGB to sRGB.
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn linear_to_srgb(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::linear_to_srgb(&buf, convert_roi(roi));
    imagebuf_to_image(&result)
}

// ============================================================================
// Compositing / Blend Modes
// ============================================================================

/// Porter-Duff "under" compositing: A under B.
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn under(a: &Image, b: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);
    let buf_b = image_to_imagebuf(b);
    let result = imagebufalgo::under(&buf_a, &buf_b, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Porter-Duff "in" compositing: A masked by B's alpha.
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn in_op(a: &Image, b: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);
    let buf_b = image_to_imagebuf(b);
    let result = imagebufalgo::in_op(&buf_a, &buf_b, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Porter-Duff "out" compositing: A masked by (1 - B's alpha).
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn out(a: &Image, b: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);
    let buf_b = image_to_imagebuf(b);
    let result = imagebufalgo::out(&buf_a, &buf_b, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Porter-Duff "atop" compositing.
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn atop(a: &Image, b: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);
    let buf_b = image_to_imagebuf(b);
    let result = imagebufalgo::atop(&buf_a, &buf_b, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Porter-Duff "xor" compositing.
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn xor(a: &Image, b: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);
    let buf_b = image_to_imagebuf(b);
    let result = imagebufalgo::xor(&buf_a, &buf_b, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Screen blend mode: 1 - (1-A) * (1-B).
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn screen(a: &Image, b: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);
    let buf_b = image_to_imagebuf(b);
    let result = imagebufalgo::screen(&buf_a, &buf_b, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Multiply blend mode: A * B.
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn multiply(a: &Image, b: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);
    let buf_b = image_to_imagebuf(b);
    let result = imagebufalgo::multiply(&buf_a, &buf_b, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Overlay blend mode.
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn overlay(a: &Image, b: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);
    let buf_b = image_to_imagebuf(b);
    let result = imagebufalgo::overlay(&buf_a, &buf_b, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Hard light blend mode.
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn hardlight(a: &Image, b: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);
    let buf_b = image_to_imagebuf(b);
    let result = imagebufalgo::hardlight(&buf_a, &buf_b, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Soft light blend mode.
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn softlight(a: &Image, b: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);
    let buf_b = image_to_imagebuf(b);
    let result = imagebufalgo::softlight(&buf_a, &buf_b, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Difference blend mode: |A - B|.
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn difference(a: &Image, b: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);
    let buf_b = image_to_imagebuf(b);
    let result = imagebufalgo::difference(&buf_a, &buf_b, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Exclusion blend mode: A + B - 2*A*B.
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn exclusion(a: &Image, b: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);
    let buf_b = image_to_imagebuf(b);
    let result = imagebufalgo::exclusion(&buf_a, &buf_b, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Color dodge blend mode.
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn colordodge(a: &Image, b: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);
    let buf_b = image_to_imagebuf(b);
    let result = imagebufalgo::colordodge(&buf_a, &buf_b, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Color burn blend mode.
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn colorburn(a: &Image, b: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);
    let buf_b = image_to_imagebuf(b);
    let result = imagebufalgo::colorburn(&buf_a, &buf_b, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Additive blend: A + B.
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn add_blend(a: &Image, b: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);
    let buf_b = image_to_imagebuf(b);
    let result = imagebufalgo::add_blend(&buf_a, &buf_b, convert_roi(roi));
    imagebuf_to_image(&result)
}

// ============================================================================
// Pattern Generation
// ============================================================================

/// Create a zero-filled image.
///
/// Args:
///     width: Image width
///     height: Image height
///     channels: Number of channels (default 4)
///
/// Returns:
///     Black image
#[pyfunction]
#[pyo3(signature = (width, height, channels=4))]
pub fn zero(width: u32, height: u32, channels: u32) -> PyResult<Image> {
    let spec = ImageSpec::new(width, height, channels as u8, RustDataFormat::F32);
    let buf = ImageBuf::new(spec, InitializePixels::Yes);
    imagebuf_to_image(&buf)
}

/// Fill an image with a constant color.
///
/// Args:
///     width: Image width
///     height: Image height
///     color: Fill color as list of channel values
///
/// Returns:
///     Filled image
#[pyfunction]
#[pyo3(signature = (width, height, color))]
pub fn fill(width: u32, height: u32, color: Vec<f32>) -> PyResult<Image> {
    let channels = color.len() as i32;
    let roi = RustRoi3D::new_2d_with_channels(0, width as i32, 0, height as i32, 0, channels);
    let buf = imagebufalgo::fill(&color, roi);
    imagebuf_to_image(&buf)
}

/// Create a checkerboard pattern.
///
/// Args:
///     width: Image width
///     height: Image height
///     check_width: Width of each check
///     check_height: Height of each check
///     color1: First color
///     color2: Second color
///
/// Returns:
///     Checkerboard image
#[pyfunction]
#[pyo3(signature = (width, height, check_width=32, check_height=32, color1=None, color2=None))]
pub fn checker(
    width: u32,
    height: u32,
    check_width: u32,
    check_height: u32,
    color1: Option<Vec<f32>>,
    color2: Option<Vec<f32>>,
) -> PyResult<Image> {
    let c1 = color1.unwrap_or_else(|| vec![0.0, 0.0, 0.0, 1.0]);
    let c2 = color2.unwrap_or_else(|| vec![1.0, 1.0, 1.0, 1.0]);
    let channels = c1.len().max(c2.len()) as i32;

    let roi = RustRoi3D::new_2d_with_channels(0, width as i32, 0, height as i32, 0, channels);
    let buf = imagebufalgo::checker(
        check_width as i32,
        check_height as i32,
        1,  // check_depth
        &c1,
        &c2,
        (0, 0, 0),  // offset
        roi,
    );
    imagebuf_to_image(&buf)
}

/// Generate noise image.
///
/// Args:
///     width: Image width
///     height: Image height
///     channels: Number of channels
///     noise_type: Type of noise ("uniform", "gaussian", "salt", "blue")
///     a: First parameter (mean for gaussian, min for uniform, portion for salt)
///     b: Second parameter (stddev for gaussian, max for uniform)
///     mono: If True, all channels get the same noise value
///     seed: Random seed
///
/// Returns:
///     Noise image
#[pyfunction]
#[pyo3(signature = (width, height, channels=4, noise_type="gaussian", a=0.5, b=0.1, mono=false, seed=0))]
pub fn noise(
    width: u32,
    height: u32,
    channels: u32,
    noise_type: &str,
    a: f32,
    b: f32,
    mono: bool,
    seed: u32,
) -> PyResult<Image> {
    use imagebufalgo::patterns::NoiseType;

    let ntype = match noise_type.to_lowercase().as_str() {
        "uniform" => NoiseType::Uniform,
        "gaussian" => NoiseType::Gaussian,
        "salt" | "salt_pepper" => NoiseType::Salt,
        "blue" => NoiseType::Blue,
        _ => return Err(PyValueError::new_err(
            format!("Unknown noise type: {}. Use: uniform, gaussian, salt, blue", noise_type)
        )),
    };

    let roi = RustRoi3D::new_2d_with_channels(
        0, width as i32,
        0, height as i32,
        0, channels as i32,
    );

    let buf = imagebufalgo::noise(ntype, a, b, mono, seed, roi);
    imagebuf_to_image(&buf)
}

// ============================================================================
// Channel Operations
// ============================================================================

/// Extract a single channel from an image.
///
/// Args:
///     image: Input image
///     channel: Channel index to extract
///     roi: Optional region of interest
///
/// Returns:
///     Single-channel image
#[pyfunction]
#[pyo3(signature = (image, channel, roi=None))]
pub fn extract_channel(image: &Image, channel: usize, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::extract_channel(&buf, channel, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Shuffle/reorder channels.
///
/// Args:
///     image: Input image
///     channel_order: New channel order as list of indices (-1 for fill value)
///     fill_values: Values for fill channels (default: [0.0])
///     roi: Optional region of interest
///
/// Returns:
///     Image with reordered channels
#[pyfunction]
#[pyo3(signature = (image, channel_order, fill_values=None, roi=None))]
pub fn channels(image: &Image, channel_order: Vec<i32>, fill_values: Option<Vec<f32>>, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let fv = fill_values.unwrap_or_else(|| vec![0.0]);
    let result = imagebufalgo::channels(&buf, &channel_order, &fv, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Append channels from one image to another.
#[pyfunction]
#[pyo3(signature = (a, b, roi=None))]
pub fn channel_append(a: &Image, b: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);
    let buf_b = image_to_imagebuf(b);
    let result = imagebufalgo::channel_append(&buf_a, &buf_b, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Sum all channels into a single-channel image.
///
/// Args:
///     image: Input image
///     weights: Optional weights per channel (default: equal weights)
///     roi: Optional region of interest
#[pyfunction]
#[pyo3(signature = (image, weights=None, roi=None))]
pub fn channel_sum(image: &Image, weights: Option<Vec<f32>>, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let w = weights.unwrap_or_default();
    let result = imagebufalgo::channel_sum(&buf, &w, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Flatten a multi-layer/multi-channel image to RGB(A).
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn flatten(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::channel_flatten(&buf, convert_roi(roi));
    imagebuf_to_image(&result)
}

// ============================================================================
// FFT Operations
// ============================================================================

/// Forward FFT (Fast Fourier Transform).
///
/// Returns complex result as 2-channel image (real, imaginary).
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn fft(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::fft(&buf, convert_roi(roi))
        .map_err(|e| PyIOError::new_err(format!("FFT failed: {}", e)))?;
    imagebuf_to_image(&result)
}

/// Inverse FFT.
///
/// Input should be 2-channel complex image (real, imaginary).
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn ifft(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::ifft(&buf, convert_roi(roi))
        .map_err(|e| PyIOError::new_err(format!("IFFT failed: {}", e)))?;
    imagebuf_to_image(&result)
}

/// Shift FFT result so DC is centered.
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn fft_shift(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::fft_shift(&buf, convert_roi(roi))
        .map_err(|e| PyIOError::new_err(format!("FFT shift failed: {}", e)))?;
    imagebuf_to_image(&result)
}

/// Inverse FFT shift (undo fft_shift).
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn ifft_shift(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::ifft_shift(&buf, convert_roi(roi))
        .map_err(|e| PyIOError::new_err(format!("FFT shift failed: {}", e)))?;
    imagebuf_to_image(&result)
}

/// Convert polar (magnitude, phase) to complex (real, imag).
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn polar_to_complex(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::polar_to_complex(&buf, convert_roi(roi))
        .map_err(|e| PyIOError::new_err(format!("Polar to complex failed: {}", e)))?;
    imagebuf_to_image(&result)
}

/// Convert complex (real, imag) to polar (magnitude, phase).
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn complex_to_polar(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::complex_to_polar(&buf, convert_roi(roi))
        .map_err(|e| PyIOError::new_err(format!("Complex to polar failed: {}", e)))?;
    imagebuf_to_image(&result)
}

// ============================================================================
// Module Registration
// ============================================================================

/// Register all ops functions to the module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Enums
    m.add_class::<WrapMode>()?;
    m.add_class::<FilterType>()?;
    m.add_class::<ResizeFilter>()?;
    m.add_class::<ColorMapName>()?;

    // Geometry
    m.add_function(wrap_pyfunction!(flip, m)?)?;
    m.add_function(wrap_pyfunction!(flop, m)?)?;
    m.add_function(wrap_pyfunction!(transpose, m)?)?;
    m.add_function(wrap_pyfunction!(rotate90, m)?)?;
    m.add_function(wrap_pyfunction!(rotate180, m)?)?;
    m.add_function(wrap_pyfunction!(rotate270, m)?)?;
    m.add_function(wrap_pyfunction!(crop, m)?)?;
    m.add_function(wrap_pyfunction!(cut, m)?)?;
    m.add_function(wrap_pyfunction!(resize, m)?)?;

    // Arithmetic
    m.add_function(wrap_pyfunction!(add, m)?)?;
    m.add_function(wrap_pyfunction!(sub, m)?)?;
    m.add_function(wrap_pyfunction!(mul, m)?)?;
    m.add_function(wrap_pyfunction!(div, m)?)?;
    m.add_function(wrap_pyfunction!(mad, m)?)?;
    m.add_function(wrap_pyfunction!(abs, m)?)?;
    m.add_function(wrap_pyfunction!(absdiff, m)?)?;
    m.add_function(wrap_pyfunction!(pow, m)?)?;
    m.add_function(wrap_pyfunction!(clamp, m)?)?;
    m.add_function(wrap_pyfunction!(invert, m)?)?;
    m.add_function(wrap_pyfunction!(over, m)?)?;
    m.add_function(wrap_pyfunction!(max, m)?)?;
    m.add_function(wrap_pyfunction!(min, m)?)?;

    // Filters
    m.add_function(wrap_pyfunction!(median, m)?)?;
    m.add_function(wrap_pyfunction!(blur, m)?)?;
    m.add_function(wrap_pyfunction!(box_blur, m)?)?;
    m.add_function(wrap_pyfunction!(unsharp_mask, m)?)?;
    m.add_function(wrap_pyfunction!(sharpen, m)?)?;
    m.add_function(wrap_pyfunction!(dilate, m)?)?;
    m.add_function(wrap_pyfunction!(erode, m)?)?;
    m.add_function(wrap_pyfunction!(morph_open, m)?)?;
    m.add_function(wrap_pyfunction!(morph_close, m)?)?;
    m.add_function(wrap_pyfunction!(laplacian, m)?)?;
    m.add_function(wrap_pyfunction!(sobel, m)?)?;

    // Color
    m.add_function(wrap_pyfunction!(premult, m)?)?;
    m.add_function(wrap_pyfunction!(unpremult, m)?)?;
    m.add_function(wrap_pyfunction!(saturate, m)?)?;
    m.add_function(wrap_pyfunction!(contrast_remap, m)?)?;
    m.add_function(wrap_pyfunction!(color_map, m)?)?;
    m.add_function(wrap_pyfunction!(colormatrixtransform, m)?)?;
    m.add_function(wrap_pyfunction!(rangecompress, m)?)?;
    m.add_function(wrap_pyfunction!(rangeexpand, m)?)?;
    m.add_function(wrap_pyfunction!(srgb_to_linear, m)?)?;
    m.add_function(wrap_pyfunction!(linear_to_srgb, m)?)?;

    // Compositing / Blend modes
    m.add_function(wrap_pyfunction!(under, m)?)?;
    m.add_function(wrap_pyfunction!(in_op, m)?)?;
    m.add_function(wrap_pyfunction!(out, m)?)?;
    m.add_function(wrap_pyfunction!(atop, m)?)?;
    m.add_function(wrap_pyfunction!(xor, m)?)?;
    m.add_function(wrap_pyfunction!(screen, m)?)?;
    m.add_function(wrap_pyfunction!(multiply, m)?)?;
    m.add_function(wrap_pyfunction!(overlay, m)?)?;
    m.add_function(wrap_pyfunction!(hardlight, m)?)?;
    m.add_function(wrap_pyfunction!(softlight, m)?)?;
    m.add_function(wrap_pyfunction!(difference, m)?)?;
    m.add_function(wrap_pyfunction!(exclusion, m)?)?;
    m.add_function(wrap_pyfunction!(colordodge, m)?)?;
    m.add_function(wrap_pyfunction!(colorburn, m)?)?;
    m.add_function(wrap_pyfunction!(add_blend, m)?)?;

    // Patterns
    m.add_function(wrap_pyfunction!(zero, m)?)?;
    m.add_function(wrap_pyfunction!(fill, m)?)?;
    m.add_function(wrap_pyfunction!(checker, m)?)?;
    m.add_function(wrap_pyfunction!(noise, m)?)?;

    // Channels
    m.add_function(wrap_pyfunction!(extract_channel, m)?)?;
    m.add_function(wrap_pyfunction!(channels, m)?)?;
    m.add_function(wrap_pyfunction!(channel_append, m)?)?;
    m.add_function(wrap_pyfunction!(channel_sum, m)?)?;
    m.add_function(wrap_pyfunction!(flatten, m)?)?;

    // FFT
    m.add_function(wrap_pyfunction!(fft, m)?)?;
    m.add_function(wrap_pyfunction!(ifft, m)?)?;
    m.add_function(wrap_pyfunction!(fft_shift, m)?)?;
    m.add_function(wrap_pyfunction!(ifft_shift, m)?)?;
    m.add_function(wrap_pyfunction!(polar_to_complex, m)?)?;
    m.add_function(wrap_pyfunction!(complex_to_polar, m)?)?;

    Ok(())
}
