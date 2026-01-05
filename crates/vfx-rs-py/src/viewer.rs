//! Python bindings for the image viewer.

use pyo3::prelude::*;
use std::path::PathBuf;

/// Open the image viewer.
///
/// Opens a GPU-accelerated image viewer with OCIO color management.
/// Supports EXR, HDR, PNG, JPEG, TIFF, DPX and other formats.
///
/// # Arguments
///
/// * `path` - Optional path to image file. If None, opens empty viewer
///            or loads last viewed file from persistence.
///
/// # Example
///
/// ```python
/// import vfx_rs
///
/// # Open viewer with image
/// vfx_rs.view("render.exr")
///
/// # Open empty viewer (file dialog on double-click)
/// vfx_rs.view()
/// ```
///
/// # Returns
///
/// Exit code (0 = success, 1 = error)
#[pyfunction]
#[pyo3(signature = (path=None))]
pub fn view(path: Option<PathBuf>) -> i32 {
    let config = vfx_view::ViewerConfig::default();
    vfx_view::run_opt(path, config)
}
