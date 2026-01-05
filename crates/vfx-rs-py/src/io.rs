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
///
/// Args:
///     path: Output file path
///     image: Image to write
///     compression: Compression method: "none", "rle", "zip" (default), "piz", "dwaa", "dwab"
///     use_half: Write as half-float (f16) instead of f32, reduces size by 50%
#[pyfunction]
#[pyo3(signature = (path, image, compression=None, use_half=false))]
fn write_exr(path: PathBuf, image: &Image, compression: Option<&str>, use_half: bool) -> PyResult<()> {
    use vfx_io::exr::{ExrWriter, ExrWriterOptions, Compression};
    use vfx_io::FormatWriter;
    
    let comp = match compression.map(|s| s.to_lowercase()).as_deref() {
        Some("none") => Compression::None,
        Some("rle") => Compression::Rle,
        Some("zip") | None => Compression::Zip,
        Some("piz") => Compression::Piz,
        Some("dwaa") => Compression::Dwaa,
        Some("dwab") => Compression::Dwab,
        Some(other) => return Err(PyIOError::new_err(
            format!("Unknown EXR compression: '{}'. Use: none, rle, zip, piz, dwaa, dwab", other)
        )),
    };
    
    let opts = ExrWriterOptions {
        compression: comp,
        use_half,
        ..Default::default()
    };
    
    ExrWriter::with_options(opts).write(&path, image.as_image_data())
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
///
/// Args:
///     path: Output file path
///     image: Image to write
///     compression: Compression level: "fast", "default", "best" (or 0-2)
///     bit_depth: Bits per channel: 8 (default) or 16
#[pyfunction]
#[pyo3(signature = (path, image, compression=None, bit_depth=None))]
fn write_png(path: PathBuf, image: &Image, compression: Option<&Bound<'_, PyAny>>, bit_depth: Option<u8>) -> PyResult<()> {
    use vfx_io::png::{PngWriter, PngWriterOptions, CompressionLevel, BitDepth};
    use vfx_io::FormatWriter;
    
    // Parse compression: string or int
    let comp = match compression {
        None => CompressionLevel::Default,
        Some(v) => {
            if let Ok(s) = v.extract::<String>() {
                match s.to_lowercase().as_str() {
                    "fast" | "0" => CompressionLevel::Fast,
                    "default" | "1" => CompressionLevel::Default,
                    "best" | "2" => CompressionLevel::Best,
                    other => return Err(PyIOError::new_err(
                        format!("Unknown PNG compression: '{}'. Use: fast, default, best", other)
                    )),
                }
            } else if let Ok(n) = v.extract::<u8>() {
                match n {
                    0 => CompressionLevel::Fast,
                    1 => CompressionLevel::Default,
                    2 => CompressionLevel::Best,
                    _ => return Err(PyIOError::new_err("PNG compression must be 0-2")),
                }
            } else {
                return Err(PyIOError::new_err("compression must be string or int"));
            }
        }
    };
    
    let depth = match bit_depth {
        Some(8) | None => BitDepth::Eight,
        Some(16) => BitDepth::Sixteen,
        Some(n) => return Err(PyIOError::new_err(format!("PNG bit_depth must be 8 or 16, got {}", n))),
    };
    
    let opts = PngWriterOptions {
        compression: comp,
        bit_depth: depth,
        ..Default::default()
    };
    
    PngWriter::with_options(opts).write(&path, image.as_image_data())
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
///
/// Args:
///     path: Output file path
///     image: Image to write
///     bit_depth: BitDepth.Bit8/10/12/16 or int (default: 10)
///
/// Example:
///     io.write_dpx("out.dpx", img, bit_depth=BitDepth.Bit10)
///     io.write_dpx("out.dpx", img, bit_depth=10)  # also works
#[pyfunction]
#[pyo3(signature = (path, image, bit_depth=None))]
fn write_dpx(path: PathBuf, image: &Image, bit_depth: Option<&Bound<'_, PyAny>>) -> PyResult<()> {
    use vfx_io::dpx::{DpxWriter, DpxWriterOptions, BitDepth};
    use vfx_io::FormatWriter;
    
    let bits = match bit_depth {
        Some(v) => crate::format::BitDepth::from_py(v)?,
        None => 10, // DPX default
    };
    
    let depth = match bits {
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
