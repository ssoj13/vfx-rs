//! Warp (geometric transformation) operations for Python.
//!
//! Provides image warping using transformation matrices and ST maps.

use pyo3::prelude::*;
use pyo3::exceptions::PyIOError;

use vfx_io::imagebuf::ImageBuf;
use vfx_io::imagebufalgo::warp as rust_warp;
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

fn wrap_from_str(wrap: &str) -> rust_warp::WarpWrap {
    rust_warp::WarpWrap::from(wrap)
}

// ============================================================================
// Python Wrap Mode Enum
// ============================================================================

/// Wrap mode for warping operations.
#[pyclass]
#[derive(Debug, Clone)]
pub enum WarpWrap {
    /// Return black for out-of-bounds coordinates
    Black,
    /// Clamp coordinates to image bounds
    Clamp,
    /// Tile/repeat the image periodically
    Periodic,
    /// Mirror at edges
    Mirror,
}

impl From<WarpWrap> for rust_warp::WarpWrap {
    fn from(w: WarpWrap) -> Self {
        match w {
            WarpWrap::Black => rust_warp::WarpWrap::Black,
            WarpWrap::Clamp => rust_warp::WarpWrap::Clamp,
            WarpWrap::Periodic => rust_warp::WarpWrap::Periodic,
            WarpWrap::Mirror => rust_warp::WarpWrap::Mirror,
        }
    }
}

// ============================================================================
// Warp Functions
// ============================================================================

/// Warp an image using a 3x3 transformation matrix.
///
/// The matrix transforms destination pixel coordinates to source pixel coordinates.
/// Uses bilinear interpolation for sampling.
///
/// Args:
///     image: Source image to warp
///     matrix: 3x3 transformation matrix as list of 9 floats (row-major)
///     wrap: Wrap mode - "black", "clamp", "periodic", or "mirror"
///     roi: Optional output region
///
/// Returns:
///     Warped image
///
/// Matrix Format:
///     [a, b, tx, c, d, ty, px, py, 1]
///     For destination (x, y), source is:
///     w = px*x + py*y + 1
///     src_x = (a*x + b*y + tx) / w
///     src_y = (c*x + d*y + ty) / w
///
/// Example:
///     >>> # Scale by 2x
///     >>> scale_2x = [2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 1.0]
///     >>> warped = warp(img, scale_2x, "clamp")
#[pyfunction]
#[pyo3(signature = (image, matrix, wrap="black", roi=None))]
pub fn warp(
    image: &Image,
    matrix: [f32; 9],
    wrap: &str,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = rust_warp::warp(&buf, &matrix, wrap_from_str(wrap), convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Warp an image using per-pixel ST coordinates (like Nuke's STMap).
///
/// Each pixel in stbuf provides normalized (0-1) coordinates specifying
/// where to sample from the source image.
///
/// Args:
///     image: Source image to sample from
///     stbuf: ST coordinate image (at least 2 channels for S and T)
///     chan_s: Channel index for S coordinate (default 0)
///     chan_t: Channel index for T coordinate (default 1)
///     flip_s: Mirror S coordinate horizontally
///     flip_t: Mirror T coordinate vertically
///     wrap: Wrap mode - "black", "clamp", "periodic", or "mirror"
///     roi: Optional output region
///
/// Returns:
///     Warped image
///
/// ST Coordinates:
///     S=0, T=0 -> top-left of source
///     S=1, T=1 -> bottom-right of source
///
/// Example:
///     >>> # stmap contains UV coordinates
///     >>> warped = st_warp(source, stmap)
#[pyfunction]
#[pyo3(signature = (image, stbuf, chan_s=0, chan_t=1, flip_s=false, flip_t=false, wrap="black", roi=None))]
pub fn st_warp(
    image: &Image,
    stbuf: &Image,
    chan_s: usize,
    chan_t: usize,
    flip_s: bool,
    flip_t: bool,
    wrap: &str,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let src_buf = image_to_imagebuf(image);
    let st_buf = image_to_imagebuf(stbuf);
    let result = rust_warp::st_warp(
        &src_buf,
        &st_buf,
        chan_s,
        chan_t,
        flip_s,
        flip_t,
        wrap_from_str(wrap),
        convert_roi(roi),
    );
    imagebuf_to_image(&result)
}

// ============================================================================
// Matrix Helper Functions
// ============================================================================

/// Create an identity matrix (no transformation).
///
/// Returns:
///     Identity matrix [1,0,0, 0,1,0, 0,0,1]
#[pyfunction]
pub fn matrix_identity() -> [f32; 9] {
    rust_warp::matrix_identity()
}

/// Create a translation matrix.
///
/// Args:
///     tx: X translation
///     ty: Y translation
///
/// Returns:
///     Translation matrix
#[pyfunction]
pub fn matrix_translate(tx: f32, ty: f32) -> [f32; 9] {
    rust_warp::matrix_translate(tx, ty)
}

/// Create a scale matrix.
///
/// Args:
///     sx: X scale factor
///     sy: Y scale factor
///
/// Returns:
///     Scale matrix
#[pyfunction]
pub fn matrix_scale(sx: f32, sy: f32) -> [f32; 9] {
    rust_warp::matrix_scale(sx, sy)
}

/// Create a rotation matrix.
///
/// Args:
///     angle: Rotation angle in radians
///
/// Returns:
///     Rotation matrix
#[pyfunction]
pub fn matrix_rotate(angle: f32) -> [f32; 9] {
    rust_warp::matrix_rotate(angle)
}

/// Create a shear matrix.
///
/// Args:
///     shx: X shear factor
///     shy: Y shear factor
///
/// Returns:
///     Shear matrix
#[pyfunction]
pub fn matrix_shear(shx: f32, shy: f32) -> [f32; 9] {
    rust_warp::matrix_shear(shx, shy)
}

/// Multiply two 3x3 matrices.
///
/// Args:
///     a: First matrix
///     b: Second matrix
///
/// Returns:
///     Result of a * b
#[pyfunction]
pub fn matrix_multiply(a: [f32; 9], b: [f32; 9]) -> [f32; 9] {
    rust_warp::matrix_multiply(&a, &b)
}

/// Invert a 3x3 matrix.
///
/// Args:
///     m: Matrix to invert
///
/// Returns:
///     Inverted matrix, or None if matrix is singular
#[pyfunction]
pub fn matrix_invert(m: [f32; 9]) -> Option<[f32; 9]> {
    rust_warp::matrix_invert(&m)
}

// ============================================================================
// Module Registration
// ============================================================================

/// Register all warp functions to the module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Classes
    m.add_class::<WarpWrap>()?;

    // Warp functions
    m.add_function(wrap_pyfunction!(warp, m)?)?;
    m.add_function(wrap_pyfunction!(st_warp, m)?)?;

    // Matrix helpers
    m.add_function(wrap_pyfunction!(matrix_identity, m)?)?;
    m.add_function(wrap_pyfunction!(matrix_translate, m)?)?;
    m.add_function(wrap_pyfunction!(matrix_scale, m)?)?;
    m.add_function(wrap_pyfunction!(matrix_rotate, m)?)?;
    m.add_function(wrap_pyfunction!(matrix_shear, m)?)?;
    m.add_function(wrap_pyfunction!(matrix_multiply, m)?)?;
    m.add_function(wrap_pyfunction!(matrix_invert, m)?)?;

    Ok(())
}
