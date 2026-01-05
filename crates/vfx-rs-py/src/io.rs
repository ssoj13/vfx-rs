//! Format-specific I/O submodule.

use pyo3::prelude::*;
use pyo3::exceptions::PyIOError;
use std::path::PathBuf;

use crate::Image;

/// Register io submodule.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // EXR
    m.add_function(wrap_pyfunction!(read_exr, m)?)?;
    m.add_function(wrap_pyfunction!(write_exr, m)?)?;
    
    // PNG
    m.add_function(wrap_pyfunction!(read_png, m)?)?;
    m.add_function(wrap_pyfunction!(write_png, m)?)?;
    
    // JPEG
    m.add_function(wrap_pyfunction!(read_jpeg, m)?)?;
    m.add_function(wrap_pyfunction!(write_jpeg, m)?)?;
    
    // DPX
    m.add_function(wrap_pyfunction!(read_dpx, m)?)?;
    m.add_function(wrap_pyfunction!(write_dpx, m)?)?;
    
    // TIFF
    m.add_function(wrap_pyfunction!(read_tiff, m)?)?;
    m.add_function(wrap_pyfunction!(write_tiff, m)?)?;
    
    // HDR
    m.add_function(wrap_pyfunction!(read_hdr, m)?)?;
    m.add_function(wrap_pyfunction!(write_hdr, m)?)?;
    
    Ok(())
}

// ============================================================================
// EXR
// ============================================================================

/// Read an EXR file.
#[pyfunction]
fn read_exr(path: PathBuf) -> PyResult<Image> {
    let data = vfx_io::exr::read(&path)
        .map_err(|e| PyIOError::new_err(format!("EXR read failed: {}", e)))?;
    Ok(Image::from_image_data(data))
}

/// Write an EXR file.
#[pyfunction]
#[pyo3(signature = (path, image, compression=None))]
fn write_exr(path: PathBuf, image: &Image, compression: Option<&str>) -> PyResult<()> {
    // TODO: support compression options
    let _ = compression;
    vfx_io::exr::write(&path, image.as_image_data())
        .map_err(|e| PyIOError::new_err(format!("EXR write failed: {}", e)))?;
    Ok(())
}

// ============================================================================
// PNG
// ============================================================================

/// Read a PNG file.
#[pyfunction]
fn read_png(path: PathBuf) -> PyResult<Image> {
    let data = vfx_io::png::read(&path)
        .map_err(|e| PyIOError::new_err(format!("PNG read failed: {}", e)))?;
    Ok(Image::from_image_data(data))
}

/// Write a PNG file.
#[pyfunction]
#[pyo3(signature = (path, image, compression=None))]
fn write_png(path: PathBuf, image: &Image, compression: Option<u8>) -> PyResult<()> {
    let _ = compression; // TODO: support compression level
    vfx_io::png::write(&path, image.as_image_data())
        .map_err(|e| PyIOError::new_err(format!("PNG write failed: {}", e)))?;
    Ok(())
}

// ============================================================================
// JPEG
// ============================================================================

/// Read a JPEG file.
#[pyfunction]
fn read_jpeg(path: PathBuf) -> PyResult<Image> {
    let data = vfx_io::jpeg::read(&path)
        .map_err(|e| PyIOError::new_err(format!("JPEG read failed: {}", e)))?;
    Ok(Image::from_image_data(data))
}

/// Write a JPEG file.
#[pyfunction]
#[pyo3(signature = (path, image, quality=90))]
fn write_jpeg(path: PathBuf, image: &Image, quality: u8) -> PyResult<()> {
    use vfx_io::jpeg::{JpegWriter, JpegWriterOptions, ColorType};
    use vfx_io::FormatWriter;
    
    let opts = JpegWriterOptions {
        quality,
        color_type: ColorType::Rgb,
    };
    JpegWriter::with_options(opts).write(&path, image.as_image_data())
        .map_err(|e| PyIOError::new_err(format!("JPEG write failed: {}", e)))?;
    Ok(())
}

// ============================================================================
// DPX
// ============================================================================

/// Read a DPX file.
#[pyfunction]
fn read_dpx(path: PathBuf) -> PyResult<Image> {
    let data = vfx_io::dpx::read(&path)
        .map_err(|e| PyIOError::new_err(format!("DPX read failed: {}", e)))?;
    Ok(Image::from_image_data(data))
}

/// Write a DPX file.
#[pyfunction]
#[pyo3(signature = (path, image, bit_depth=10))]
fn write_dpx(path: PathBuf, image: &Image, bit_depth: u8) -> PyResult<()> {
    use vfx_io::dpx::{DpxWriter, DpxWriterOptions, BitDepth};
    use vfx_io::FormatWriter;
    
    let depth = match bit_depth {
        8 => BitDepth::Bit8,
        10 => BitDepth::Bit10,
        12 => BitDepth::Bit12,
        16 => BitDepth::Bit16,
        _ => return Err(PyIOError::new_err("bit_depth must be 8, 10, 12, or 16")),
    };
    
    let opts = DpxWriterOptions {
        bit_depth: depth,
        ..Default::default()
    };
    DpxWriter::with_options(opts).write(&path, image.as_image_data())
        .map_err(|e| PyIOError::new_err(format!("DPX write failed: {}", e)))?;
    Ok(())
}

// ============================================================================
// TIFF
// ============================================================================

/// Read a TIFF file.
#[pyfunction]
fn read_tiff(path: PathBuf) -> PyResult<Image> {
    let data = vfx_io::tiff::read(&path)
        .map_err(|e| PyIOError::new_err(format!("TIFF read failed: {}", e)))?;
    Ok(Image::from_image_data(data))
}

/// Write a TIFF file.
#[pyfunction]
#[pyo3(signature = (path, image))]
fn write_tiff(path: PathBuf, image: &Image) -> PyResult<()> {
    vfx_io::tiff::write(&path, image.as_image_data())
        .map_err(|e| PyIOError::new_err(format!("TIFF write failed: {}", e)))?;
    Ok(())
}

// ============================================================================
// HDR
// ============================================================================

/// Read an HDR (Radiance RGBE) file.
#[pyfunction]
fn read_hdr(path: PathBuf) -> PyResult<Image> {
    let data = vfx_io::hdr::read(&path)
        .map_err(|e| PyIOError::new_err(format!("HDR read failed: {}", e)))?;
    Ok(Image::from_image_data(data))
}

/// Write an HDR (Radiance RGBE) file.
#[pyfunction]
fn write_hdr(path: PathBuf, image: &Image) -> PyResult<()> {
    vfx_io::hdr::write(&path, image.as_image_data())
        .map_err(|e| PyIOError::new_err(format!("HDR write failed: {}", e)))?;
    Ok(())
}
