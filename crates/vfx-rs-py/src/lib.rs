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
mod format;
mod layered;
mod core;
mod ops;
mod stats;
mod ocio;
mod deep;
#[cfg(feature = "viewer")]
mod viewer;

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

/// Read a multi-layer EXR file.
///
/// Returns a LayeredImage with all layers and channels preserved.
///
/// # Example
/// ```python
/// layered = vfx_rs.read_layered("render.exr")
/// beauty = layered["beauty"]
/// depth = layered["depth"]
/// ```
#[pyfunction]
#[pyo3(signature = (path))]
fn read_layered(path: PathBuf) -> PyResult<layered::LayeredImage> {
    let data = vfx_io::exr::read_layers(&path)
        .map_err(|e| PyIOError::new_err(format!("Failed to read {}: {}", path.display(), e)))?;
    Ok(layered::LayeredImage { inner: data })
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
    m.add_class::<format::BitDepth>()?;

    // Layered image types
    layered::register(m)?;

    // Top-level I/O
    m.add_function(wrap_pyfunction!(read, m)?)?;
    m.add_function(wrap_pyfunction!(read_layered, m)?)?;
    m.add_function(wrap_pyfunction!(write, m)?)?;

    // Submodules
    let io_module = PyModule::new(m.py(), "io")?;
    io::register(&io_module)?;
    m.add_submodule(&io_module)?;

    let lut_module = PyModule::new(m.py(), "lut")?;
    lut::register(&lut_module)?;
    m.add_submodule(&lut_module)?;

    // Core types submodule (TypeDesc, ImageSpec, Roi3D)
    let core_module = PyModule::new(m.py(), "core")?;
    core::register(&core_module)?;
    m.add_submodule(&core_module)?;

    // Operations submodule (ImageBufAlgo)
    let ops_module = PyModule::new(m.py(), "ops")?;
    ops::register(&ops_module)?;
    m.add_submodule(&ops_module)?;

    // Statistics submodule
    let stats_module = PyModule::new(m.py(), "stats")?;
    stats::register(&stats_module)?;
    m.add_submodule(&stats_module)?;

    // OCIO color management submodule
    let ocio_module = PyModule::new(m.py(), "ocio")?;
    ocio::register(&ocio_module)?;
    m.add_submodule(&ocio_module)?;

    // Deep compositing submodule
    let deep_module = PyModule::new(m.py(), "deep")?;
    deep::register(&deep_module)?;
    m.add_submodule(&deep_module)?;

    // Also register core types at top level for convenience
    m.add_class::<core::TypeDesc>()?;
    m.add_class::<core::ImageSpec>()?;
    m.add_class::<core::Roi3D>()?;
    m.add_class::<core::DataFormat>()?;
    m.add_class::<core::BaseType>()?;
    m.add_class::<core::Aggregate>()?;

    // Viewer (optional)
    #[cfg(feature = "viewer")]
    {
        m.add_function(wrap_pyfunction!(viewer::view, m)?)?;
    }

    Ok(())
}
