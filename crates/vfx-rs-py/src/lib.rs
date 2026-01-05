//! Python bindings for vfx-rs.
//!
//! Provides high-performance VFX image processing via PyO3.

use pyo3::prelude::*;
use pyo3::exceptions::PyIOError;
use std::path::PathBuf;

mod image;
mod io;
mod processor;
mod lut;

pub use image::Image;
pub use processor::Processor;

/// Read an image from file.
///
/// Supports: EXR, PNG, JPEG, TIFF, DPX, HDR, WebP, AVIF, JP2
///
/// # Example
/// ```python
/// img = vfx_rs.read("input.exr")
/// ```
#[pyfunction]
#[pyo3(signature = (path))]
fn read(path: PathBuf) -> PyResult<Image> {
    let data = vfx_io::read(&path)
        .map_err(|e| PyIOError::new_err(format!("Failed to read {}: {}", path.display(), e)))?;
    Ok(Image::from_image_data(data))
}

/// Write an image to file.
///
/// Format is auto-detected from extension.
///
/// # Example
/// ```python
/// vfx_rs.write("output.exr", img)
/// vfx_rs.write("output.png", img)
/// ```
#[pyfunction]
#[pyo3(signature = (path, image))]
fn write(path: PathBuf, image: &Image) -> PyResult<()> {
    vfx_io::write(&path, image.as_image_data())
        .map_err(|e| PyIOError::new_err(format!("Failed to write {}: {}", path.display(), e)))?;
    Ok(())
}

/// vfx_rs - High-performance VFX image processing
#[pymodule]
fn vfx_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Core types
    m.add_class::<Image>()?;
    m.add_class::<Processor>()?;
    
    // Top-level I/O
    m.add_function(wrap_pyfunction!(read, m)?)?;
    m.add_function(wrap_pyfunction!(write, m)?)?;
    
    // Submodules
    let io_module = PyModule::new(m.py(), "io")?;
    io::register(&io_module)?;
    m.add_submodule(&io_module)?;
    
    let lut_module = PyModule::new(m.py(), "lut")?;
    lut::register(&lut_module)?;
    m.add_submodule(&lut_module)?;
    
    Ok(())
}
