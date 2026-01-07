//! ImageBufAlgo operations for Python.
//!
//! Provides both standalone functions and Image methods for all image operations.
//! This module exposes OIIO-compatible ImageBufAlgo operations to Python.

use pyo3::prelude::*;
use pyo3::exceptions::{PyValueError, PyIOError};

use vfx_io::imagebuf::{ImageBuf, InitializePixels, WrapMode as RustWrapMode};
use vfx_io::imagebufalgo;
use vfx_io::imagebufalgo::geometry::ResizeFilter as RustResizeFilter;
use vfx_io::imagebufalgo::demosaic::{BayerPattern as RustBayerPattern, DemosaicAlgorithm as RustDemosaicAlgorithm};
use vfx_io::imagebufalgo::texture::{MipmapFilter as RustMipmapFilter, MipmapOptions as RustMipmapOptions};
use vfx_io::imagebufalgo::fillholes::FillHolesOptions as RustFillHolesOptions;
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
// Bayer Pattern enum
// ============================================================================

/// Bayer pattern arrangement for demosaicing.
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BayerPattern {
    /// Red-Green / Green-Blue (most common)
    RGGB = 0,
    /// Blue-Green / Green-Red
    BGGR = 1,
    /// Green-Red / Blue-Green
    GRBG = 2,
    /// Green-Blue / Red-Green
    GBRG = 3,
}

impl From<BayerPattern> for RustBayerPattern {
    fn from(p: BayerPattern) -> Self {
        match p {
            BayerPattern::RGGB => RustBayerPattern::RGGB,
            BayerPattern::BGGR => RustBayerPattern::BGGR,
            BayerPattern::GRBG => RustBayerPattern::GRBG,
            BayerPattern::GBRG => RustBayerPattern::GBRG,
        }
    }
}

/// Demosaicing algorithm.
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemosaicAlgorithm {
    /// Simple bilinear interpolation. Fast but lower quality.
    Bilinear = 0,
    /// Variable Number of Gradients. Good balance of speed and quality.
    VNG = 1,
    /// Adaptive Homogeneity-Directed. Highest quality, slower.
    AHD = 2,
}

impl From<DemosaicAlgorithm> for RustDemosaicAlgorithm {
    fn from(a: DemosaicAlgorithm) -> Self {
        match a {
            DemosaicAlgorithm::Bilinear => RustDemosaicAlgorithm::Bilinear,
            DemosaicAlgorithm::VNG => RustDemosaicAlgorithm::VNG,
            DemosaicAlgorithm::AHD => RustDemosaicAlgorithm::AHD,
        }
    }
}

// ============================================================================
// Mipmap types
// ============================================================================

/// Mipmap filter for texture generation.
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MipmapFilter {
    /// Box filter (simple averaging) - fastest.
    Box = 0,
    /// Bilinear filter - good balance.
    Bilinear = 1,
    /// Lanczos filter - highest quality.
    Lanczos = 2,
    /// Kaiser filter - sharp results.
    Kaiser = 3,
}

impl From<MipmapFilter> for RustMipmapFilter {
    fn from(f: MipmapFilter) -> Self {
        match f {
            MipmapFilter::Box => RustMipmapFilter::Box,
            MipmapFilter::Bilinear => RustMipmapFilter::Bilinear,
            MipmapFilter::Lanczos => RustMipmapFilter::Lanczos,
            MipmapFilter::Kaiser => RustMipmapFilter::Kaiser,
        }
    }
}

/// Options for mipmap generation.
#[pyclass]
#[derive(Debug, Clone)]
pub struct MipmapOptions {
    #[pyo3(get, set)]
    pub filter: MipmapFilter,
    #[pyo3(get, set)]
    pub srgb: bool,
    #[pyo3(get, set)]
    pub premultiply_alpha: bool,
    #[pyo3(get, set)]
    pub wrap: WrapMode,
}

#[pymethods]
impl MipmapOptions {
    #[new]
    #[pyo3(signature = (filter=None, srgb=false, premultiply_alpha=true, wrap=None))]
    fn new(
        filter: Option<MipmapFilter>,
        srgb: bool,
        premultiply_alpha: bool,
        wrap: Option<WrapMode>,
    ) -> Self {
        Self {
            filter: filter.unwrap_or(MipmapFilter::Bilinear),
            srgb,
            premultiply_alpha,
            wrap: wrap.unwrap_or(WrapMode::Clamp),
        }
    }
}

impl From<&MipmapOptions> for RustMipmapOptions {
    fn from(o: &MipmapOptions) -> Self {
        RustMipmapOptions {
            filter: o.filter.into(),
            srgb: o.srgb,
            premultiply_alpha: o.premultiply_alpha,
            wrap: o.wrap.into(),
        }
    }
}

// ============================================================================
// Hole filling types
// ============================================================================

/// Options for push-pull hole filling.
#[pyclass]
#[derive(Debug, Clone)]
pub struct FillHolesOptions {
    #[pyo3(get, set)]
    pub alpha_channel: i32,
    #[pyo3(get, set)]
    pub alpha_threshold: f32,
    #[pyo3(get, set)]
    pub dilate: bool,
    #[pyo3(get, set)]
    pub max_levels: u32,
}

#[pymethods]
impl FillHolesOptions {
    #[new]
    #[pyo3(signature = (alpha_channel=-1, alpha_threshold=0.001, dilate=true, max_levels=0))]
    fn new(alpha_channel: i32, alpha_threshold: f32, dilate: bool, max_levels: u32) -> Self {
        Self {
            alpha_channel,
            alpha_threshold,
            dilate,
            max_levels,
        }
    }
}

impl From<&FillHolesOptions> for RustFillHolesOptions {
    fn from(o: &FillHolesOptions) -> Self {
        RustFillHolesOptions {
            alpha_channel: o.alpha_channel,
            alpha_threshold: o.alpha_threshold,
            dilate: o.dilate,
            max_levels: o.max_levels,
        }
    }
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

/// Rotate an image by an arbitrary angle.
///
/// Args:
///     image: Input image
///     angle: Rotation angle in radians (positive = counter-clockwise)
///     filter: Filter type (default: Bilinear)
///     roi: Optional region of interest
///
/// Returns:
///     Rotated image
#[pyfunction]
#[pyo3(signature = (image, angle, filter=None, roi=None))]
pub fn rotate(
    image: &Image,
    angle: f32,
    filter: Option<ResizeFilter>,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let rust_filter = filter.map(|f| f.into()).unwrap_or(RustResizeFilter::Bilinear);
    let result = imagebufalgo::rotate(&buf, angle, rust_filter, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Fit an image into target dimensions while preserving aspect ratio.
///
/// Args:
///     image: Input image
///     width: Target width
///     height: Target height
///     filter: Filter type (default: Bilinear)
///     roi: Optional region of interest
///
/// Returns:
///     Fitted image (may be smaller than target to preserve aspect ratio)
#[pyfunction]
#[pyo3(signature = (image, width, height, filter=None, roi=None))]
pub fn fit(
    image: &Image,
    width: u32,
    height: u32,
    filter: Option<ResizeFilter>,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let rust_filter = filter.map(|f| f.into()).unwrap_or(RustResizeFilter::Bilinear);
    let result = imagebufalgo::fit(&buf, width, height, rust_filter, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Fast resize using nearest neighbor (point) sampling.
///
/// Args:
///     image: Input image
///     width: Target width
///     height: Target height
///     roi: Optional region of interest
///
/// Returns:
///     Resampled image
#[pyfunction]
#[pyo3(signature = (image, width, height, roi=None))]
pub fn resample(
    image: &Image,
    width: u32,
    height: u32,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::resample(&buf, width, height, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Circularly shift an image (wrap around edges).
///
/// Args:
///     image: Input image
///     xshift: Horizontal shift (positive = right)
///     yshift: Vertical shift (positive = down)
///     zshift: Depth shift (default: 0)
///     roi: Optional region of interest
///
/// Returns:
///     Shifted image with wrap-around
#[pyfunction]
#[pyo3(signature = (image, xshift, yshift, zshift=0, roi=None))]
pub fn circular_shift(
    image: &Image,
    xshift: i32,
    yshift: i32,
    zshift: i32,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::circular_shift(&buf, xshift, yshift, zshift, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Reorient an image based on EXIF orientation value.
///
/// Applies the appropriate transformations (flip, rotate) to make the image
/// upright based on the specified orientation tag.
///
/// Args:
///     image: Source image
///     orientation: EXIF orientation value (1-8). Values:
///         1 = Normal
///         2 = Flip horizontal
///         3 = Rotate 180
///         4 = Flip vertical
///         5 = Transpose (flip + rotate90)
///         6 = Rotate 90 CW
///         7 = Transverse (flip + rotate270)
///         8 = Rotate 270 CW
///
/// Returns:
///     Reoriented image
///
/// Example:
///     >>> # Manually specify orientation
///     >>> fixed = reorient(img, 6)  # Rotate 90 CW
#[pyfunction]
#[pyo3(signature = (image, orientation))]
pub fn reorient(image: &Image, orientation: u8) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::reorient(&buf, orientation);
    imagebuf_to_image(&result)
}

/// Automatically reorient an image using its embedded EXIF orientation.
///
/// Reads the "Orientation" metadata from the image and applies the appropriate
/// transforms to make it upright. This is the recommended function for
/// automatically fixing image orientation from cameras and phones.
///
/// Args:
///     image: Source image with orientation metadata
///
/// Returns:
///     Reoriented image (orientation = 1)
///
/// Example:
///     >>> photo = vfx_rs.read("photo.jpg")
///     >>> oriented = reorient_auto(photo)
#[pyfunction]
#[pyo3(signature = (image,))]
pub fn reorient_auto(image: &Image) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::reorient_auto(&buf);
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

/// Compute pixel-wise maximum: max(a, b)
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

/// Compute pixel-wise minimum: min(a, b)
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

/// Normalize RGB vectors to unit length.
///
/// Treats each pixel's RGB channels as a 3D vector and normalizes it to unit length.
/// Useful for normalizing normal maps and direction vectors.
///
/// Args:
///     image: Source image (must have 3 or 4 channels)
///     in_center: Value to subtract before normalizing (default 0.0)
///     out_center: Value to add after normalizing (default 0.0)
///     scale: Scale factor for normalized vector (default 1.0)
///     roi: Optional region of interest
///
/// Returns:
///     Normalized image
///
/// Example:
///     >>> # For normal maps stored in 0-1 range:
///     >>> normalized = normalize(normals, in_center=0.5, out_center=0.5, scale=1.0)
#[pyfunction]
#[pyo3(signature = (image, in_center=0.0, out_center=0.0, scale=1.0, roi=None))]
pub fn normalize(
    image: &Image,
    in_center: f32,
    out_center: f32,
    scale: f32,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::normalize(&buf, in_center, out_center, scale, convert_roi(roi));
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
// Demosaic Operations
// ============================================================================

/// Demosaic a Bayer pattern image to RGB.
///
/// Converts raw single-channel Bayer sensor data to a full RGB image.
///
/// Args:
///     image: Single-channel Bayer pattern image
///     pattern: Bayer pattern arrangement (RGGB, BGGR, GRBG, GBRG)
///     algorithm: Demosaicing algorithm (Bilinear, VNG, AHD)
///
/// Returns:
///     3-channel RGB image
///
/// Example:
///     >>> raw = vfx_rs.read("raw_bayer.exr")
///     >>> rgb = demosaic(raw, BayerPattern.RGGB, DemosaicAlgorithm.VNG)
#[pyfunction]
#[pyo3(signature = (image, pattern=None, algorithm=None))]
pub fn demosaic(
    image: &Image,
    pattern: Option<BayerPattern>,
    algorithm: Option<DemosaicAlgorithm>,
) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let p = pattern.unwrap_or(BayerPattern::RGGB).into();
    let a = algorithm.unwrap_or(DemosaicAlgorithm::VNG).into();
    let result = imagebufalgo::demosaic(&buf, p, a);
    imagebuf_to_image(&result)
}

// ============================================================================
// Texture / Mipmap Operations
// ============================================================================

/// Generate a complete mipmap chain from the source image.
///
/// Returns a list of images, starting with level 0 (copy of source)
/// down to the smallest level (typically 1x1).
///
/// Args:
///     image: Source image
///     options: Mipmap generation options (optional)
///
/// Returns:
///     List of mipmap levels
///
/// Example:
///     >>> mipmaps = make_texture(img, MipmapOptions(filter=MipmapFilter.Lanczos))
///     >>> print(f"Generated {len(mipmaps)} mip levels")
#[pyfunction]
#[pyo3(signature = (image, options=None))]
pub fn make_texture(image: &Image, options: Option<&MipmapOptions>) -> PyResult<Vec<Image>> {
    let buf = image_to_imagebuf(image);
    let opts = options.map(|o| o.into()).unwrap_or_default();
    let mipmaps = imagebufalgo::make_texture(&buf, &opts);
    mipmaps.iter().map(imagebuf_to_image).collect()
}

/// Generate a single mip level by downsampling the source.
///
/// Args:
///     image: Source image
///     level: Mip level to generate (0 = source, 1 = half, etc.)
///     options: Mipmap generation options (optional)
///
/// Returns:
///     Downsampled image at the specified level
#[pyfunction]
#[pyo3(signature = (image, level, options=None))]
pub fn make_mip_level(image: &Image, level: u32, options: Option<&MipmapOptions>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let opts = options.map(|o| o.into()).unwrap_or_default();
    let result = imagebufalgo::make_mip_level(&buf, level, &opts);
    imagebuf_to_image(&result)
}

/// Calculate the number of mip levels for given dimensions.
///
/// Args:
///     width: Image width
///     height: Image height
///
/// Returns:
///     Number of mip levels (including level 0)
#[pyfunction]
pub fn mip_level_count(width: u32, height: u32) -> u32 {
    imagebufalgo::mip_level_count(width, height)
}

/// Calculate dimensions at a specific mip level.
///
/// Args:
///     width: Original image width
///     height: Original image height
///     level: Mip level
///
/// Returns:
///     Tuple of (width, height) at that level
#[pyfunction]
pub fn mip_dimensions(width: u32, height: u32, level: u32) -> (u32, u32) {
    imagebufalgo::mip_dimensions(width, height, level)
}

// ============================================================================
// Hole Filling Operations
// ============================================================================

/// Fill holes in alpha channel using push-pull algorithm.
///
/// Fills transparent (alpha = 0) regions with colors interpolated from
/// neighboring pixels using a multi-resolution pyramid approach.
///
/// Args:
///     image: Input RGBA image with holes (alpha = 0)
///     options: Hole filling options (optional)
///
/// Returns:
///     Image with holes filled
#[pyfunction]
#[pyo3(signature = (image, options=None))]
pub fn fillholes_pushpull(image: &Image, options: Option<&FillHolesOptions>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let opts = options.map(|o| o.into()).unwrap_or_default();
    let result = imagebufalgo::fillholes_pushpull(&buf, &opts);
    imagebuf_to_image(&result)
}

/// Check if an image has any holes (transparent pixels).
///
/// Args:
///     image: Input image with alpha channel
///     options: Hole detection options (optional)
///
/// Returns:
///     True if any holes are found
#[pyfunction]
#[pyo3(signature = (image, options=None))]
pub fn has_holes(image: &Image, options: Option<&FillHolesOptions>) -> bool {
    let buf = image_to_imagebuf(image);
    let opts = options.map(|o| o.into()).unwrap_or_default();
    imagebufalgo::has_holes(&buf, &opts)
}

/// Count hole pixels in an image.
///
/// Args:
///     image: Input image with alpha channel
///     options: Hole detection options (optional)
///
/// Returns:
///     Number of hole pixels
#[pyfunction]
#[pyo3(signature = (image, options=None))]
pub fn count_holes(image: &Image, options: Option<&FillHolesOptions>) -> usize {
    let buf = image_to_imagebuf(image);
    let opts = options.map(|o| o.into()).unwrap_or_default();
    imagebufalgo::count_holes(&buf, &opts)
}

/// Create a filter kernel by name.
///
/// Generates various filter kernels for use with convolution operations.
///
/// Supported kernels:
/// - "box" - Box/average filter
/// - "gaussian" - Gaussian blur kernel
/// - "triangle" - Triangle/tent filter (linear interpolation)
/// - "laplacian" - Laplacian edge detection
/// - "binomial" - Binomial filter (approximates Gaussian)
/// - "sharpen" - Simple sharpening kernel
///
/// Args:
///     name: Kernel name (case insensitive)
///     width: Kernel width
///     height: Kernel height
///     param: Kernel parameter (e.g., sigma for Gaussian, default 1.0)
///
/// Returns:
///     Kernel data as flat list of floats
///
/// Example:
///     >>> gaussian = make_kernel("gaussian", 5, 5)
///     >>> laplacian = make_kernel("laplacian", 3, 3)
#[pyfunction]
#[pyo3(signature = (name, width, height, param=1.0))]
pub fn make_kernel(name: &str, width: u32, height: u32, param: f32) -> PyResult<Vec<f32>> {
    imagebufalgo::make_kernel_from_name(name, width, height, param)
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err(
            format!("Unknown kernel type: '{}'", name)
        ))
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
// OCIO Color Transforms
// ============================================================================

/// Apply a named color transform.
///
/// Supports common transform names like:
/// - "srgb_to_linear", "linear_to_srgb"
/// - "aces_to_acescg", "acescg_to_aces"
/// - Generic patterns: "X_to_Y" or "X2Y"
///
/// Args:
///     image: Source image
///     name: Transform name (e.g., "srgb_to_linear")
///     inverse: Apply inverse transform
///     unpremult: Unpremultiply before transform, repremultiply after
///     roi: Optional region of interest
///
/// Returns:
///     Transformed image
#[pyfunction]
#[pyo3(signature = (image, name, inverse=false, unpremult=false, roi=None))]
pub fn ocionamedtransform(
    image: &Image,
    name: &str,
    inverse: bool,
    unpremult: bool,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::ocionamedtransform(&buf, name, inverse, unpremult, None, convert_roi(roi));
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

/// Z-depth compositing (zover).
///
/// Composites two images based on their Z-depth channels. The pixel with the
/// smaller Z value (closer to camera) is composited "over" the further one.
///
/// Both images should have an alpha channel and ideally a Z channel. Images
/// with RGBAZ format (5 channels) work best.
///
/// Args:
///     a: First image
///     b: Second image
///     z_zeroisinf: If True, treat Z=0 as infinity (far away). Default False.
///     roi: Optional region of interest
///
/// Returns:
///     Z-composited image
///
/// Example:
///     >>> fg = vfx_rs.read("fg.exr")  # Has RGBA + Z
///     >>> bg = vfx_rs.read("bg.exr")  # Has RGBA + Z
///     >>> composite = zover(fg, bg)
#[pyfunction]
#[pyo3(signature = (a, b, z_zeroisinf=false, roi=None))]
pub fn zover(a: &Image, b: &Image, z_zeroisinf: bool, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf_a = image_to_imagebuf(a);
    let buf_b = image_to_imagebuf(b);
    let result = imagebufalgo::zover(&buf_a, &buf_b, z_zeroisinf, convert_roi(roi));
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
// Statistics / Color Analysis
// ============================================================================

/// Count pixels matching specific colors.
///
/// For each color in the list, counts how many pixels match that color
/// within the specified epsilon tolerance.
///
/// Args:
///     image: Source image
///     colors: List of colors (flat list: [r1,g1,b1,a1, r2,g2,b2,a2, ...])
///     epsilon: Tolerance per channel (default [0.001] for each)
///     roi: Optional region of interest
///
/// Returns:
///     List of counts, one per color
///
/// Example:
///     >>> # Count red and blue pixels in an RGBA image
///     >>> counts = color_count(img,
///     ...     colors=[1.0, 0.0, 0.0, 1.0,   # red
///     ...             0.0, 0.0, 1.0, 1.0],  # blue
///     ...     epsilon=[0.01, 0.01, 0.01, 0.01])
///     >>> print(f"Red: {counts[0]}, Blue: {counts[1]}")
#[pyfunction]
#[pyo3(signature = (image, colors, epsilon=None, roi=None))]
pub fn color_count(
    image: &Image,
    colors: Vec<f32>,
    epsilon: Option<Vec<f32>>,
    roi: Option<&Roi3D>,
) -> PyResult<Vec<u64>> {
    let buf = image_to_imagebuf(image);
    let eps = epsilon.unwrap_or_else(|| vec![0.001]);
    Ok(imagebufalgo::color_count(&buf, &colors, &eps, convert_roi(roi)))
}

/// Count unique colors in an image.
///
/// Returns the number of distinct pixel values in the image.
/// Colors are quantized to 16-bit precision to handle floating point comparison.
///
/// Args:
///     image: Source image
///     roi: Optional region of interest
///
/// Returns:
///     Number of unique colors
///
/// Example:
///     >>> n = unique_color_count(img)
///     >>> print(f"Image has {n} unique colors")
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn unique_color_count(image: &Image, roi: Option<&Roi3D>) -> PyResult<usize> {
    let buf = image_to_imagebuf(image);
    Ok(imagebufalgo::unique_color_count(&buf, convert_roi(roi)))
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

/// Flatten a multi-channel image to single channel by averaging.
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn flatten(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = imagebufalgo::flatten(&buf, convert_roi(roi));
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
    m.add_class::<BayerPattern>()?;
    m.add_class::<DemosaicAlgorithm>()?;
    m.add_class::<MipmapFilter>()?;
    m.add_class::<MipmapOptions>()?;
    m.add_class::<FillHolesOptions>()?;

    // Geometry
    m.add_function(wrap_pyfunction!(flip, m)?)?;
    m.add_function(wrap_pyfunction!(flop, m)?)?;
    m.add_function(wrap_pyfunction!(transpose, m)?)?;
    m.add_function(wrap_pyfunction!(rotate90, m)?)?;
    m.add_function(wrap_pyfunction!(rotate180, m)?)?;
    m.add_function(wrap_pyfunction!(rotate270, m)?)?;
    m.add_function(wrap_pyfunction!(rotate, m)?)?;
    m.add_function(wrap_pyfunction!(crop, m)?)?;
    m.add_function(wrap_pyfunction!(cut, m)?)?;
    m.add_function(wrap_pyfunction!(resize, m)?)?;
    m.add_function(wrap_pyfunction!(resample, m)?)?;
    m.add_function(wrap_pyfunction!(fit, m)?)?;
    m.add_function(wrap_pyfunction!(circular_shift, m)?)?;
    m.add_function(wrap_pyfunction!(reorient, m)?)?;
    m.add_function(wrap_pyfunction!(reorient_auto, m)?)?;

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
    m.add_function(wrap_pyfunction!(normalize, m)?)?;
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
    m.add_function(wrap_pyfunction!(make_kernel, m)?)?;

    // Demosaic
    m.add_function(wrap_pyfunction!(demosaic, m)?)?;

    // Texture / Mipmaps
    m.add_function(wrap_pyfunction!(make_texture, m)?)?;
    m.add_function(wrap_pyfunction!(make_mip_level, m)?)?;
    m.add_function(wrap_pyfunction!(mip_level_count, m)?)?;
    m.add_function(wrap_pyfunction!(mip_dimensions, m)?)?;

    // Hole filling
    m.add_function(wrap_pyfunction!(fillholes_pushpull, m)?)?;
    m.add_function(wrap_pyfunction!(has_holes, m)?)?;
    m.add_function(wrap_pyfunction!(count_holes, m)?)?;

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

    // OCIO Color Transforms
    m.add_function(wrap_pyfunction!(ocionamedtransform, m)?)?;

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
    m.add_function(wrap_pyfunction!(zover, m)?)?;

    // Patterns
    m.add_function(wrap_pyfunction!(zero, m)?)?;
    m.add_function(wrap_pyfunction!(fill, m)?)?;
    m.add_function(wrap_pyfunction!(checker, m)?)?;
    m.add_function(wrap_pyfunction!(noise, m)?)?;

    // Statistics / Color analysis
    m.add_function(wrap_pyfunction!(color_count, m)?)?;
    m.add_function(wrap_pyfunction!(unique_color_count, m)?)?;

    // Channels
    m.add_function(wrap_pyfunction!(extract_channel, m)?)?;
    m.add_function(wrap_pyfunction!(channels, m)?)?;
    m.add_function(wrap_pyfunction!(channel_append, m)?)?;
    m.add_function(wrap_pyfunction!(channel_sum, m)?)?;
    m.add_function(wrap_pyfunction!(flatten, m)?)?;

    Ok(())
}
