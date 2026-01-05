//! LUT types and loaders.

use pyo3::prelude::*;
use pyo3::exceptions::PyIOError;
use std::path::PathBuf;

/// Register lut submodule.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Lut1D>()?;
    m.add_class::<Lut3D>()?;
    
    m.add_function(wrap_pyfunction!(read_cube_1d, m)?)?;
    m.add_function(wrap_pyfunction!(read_cube_3d, m)?)?;
    m.add_function(wrap_pyfunction!(read_cube, m)?)?;
    m.add_function(wrap_pyfunction!(read_clf, m)?)?;
    
    Ok(())
}

/// 1D Look-Up Table.
#[pyclass]
#[derive(Clone)]
pub struct Lut1D {
    pub(crate) inner: vfx_lut::Lut1D,
}

#[pymethods]
impl Lut1D {
    /// Create an identity 1D LUT.
    #[staticmethod]
    #[pyo3(signature = (size=1024))]
    fn identity(size: usize) -> Self {
        Self { inner: vfx_lut::Lut1D::identity(size) }
    }
    
    /// Create a gamma curve LUT.
    #[staticmethod]
    #[pyo3(signature = (size, gamma))]
    fn gamma(size: usize, gamma: f32) -> Self {
        Self { inner: vfx_lut::Lut1D::gamma(size, gamma) }
    }
    
    /// LUT size (number of entries).
    #[getter]
    fn size(&self) -> usize {
        self.inner.size()
    }
    
    /// Apply LUT to a single value.
    fn apply(&self, value: f32) -> f32 {
        self.inner.apply(value)
    }
    
    fn __repr__(&self) -> String {
        format!("Lut1D(size={})", self.size())
    }
}

/// 3D Look-Up Table.
#[pyclass]
#[derive(Clone)]
pub struct Lut3D {
    pub(crate) inner: vfx_lut::Lut3D,
}

#[pymethods]
impl Lut3D {
    /// Create an identity 3D LUT.
    #[staticmethod]
    #[pyo3(signature = (size=33))]
    fn identity(size: usize) -> Self {
        Self { inner: vfx_lut::Lut3D::identity(size) }
    }
    
    /// LUT size (cube dimension).
    #[getter]
    fn size(&self) -> usize {
        self.inner.size
    }
    
    /// Apply LUT to RGB values.
    fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        self.inner.apply(rgb)
    }
    
    fn __repr__(&self) -> String {
        format!("Lut3D(size={})", self.size())
    }
}

/// Read a .cube file as 1D LUT.
#[pyfunction]
fn read_cube_1d(path: PathBuf) -> PyResult<Lut1D> {
    let inner = vfx_lut::read_cube_1d(&path)
        .map_err(|e| PyIOError::new_err(format!("Failed to read CUBE 1D: {}", e)))?;
    Ok(Lut1D { inner })
}

/// Read a .cube file as 3D LUT.
#[pyfunction]
fn read_cube_3d(path: PathBuf) -> PyResult<Lut3D> {
    let inner = vfx_lut::read_cube_3d(&path)
        .map_err(|e| PyIOError::new_err(format!("Failed to read CUBE 3D: {}", e)))?;
    Ok(Lut3D { inner })
}

/// Read a .cube file (auto-detect 1D or 3D).
///
/// Returns either Lut1D or Lut3D based on file contents.
#[pyfunction]
fn read_cube(path: PathBuf) -> PyResult<Lut3D> {
    // Try 3D first (most common use case)
    let inner = vfx_lut::read_cube_3d(&path)
        .map_err(|e| PyIOError::new_err(format!("Failed to read CUBE: {}", e)))?;
    Ok(Lut3D { inner })
}

/// Read a CLF (Common LUT Format) file.
///
/// Returns a ProcessList that can be applied to images.
#[pyfunction]
fn read_clf(path: PathBuf) -> PyResult<ProcessList> {
    let inner = vfx_lut::read_clf(&path)
        .map_err(|e| PyIOError::new_err(format!("Failed to read CLF: {}", e)))?;
    Ok(ProcessList { inner })
}

/// CLF ProcessList - a chain of color operations.
#[pyclass]
pub struct ProcessList {
    pub(crate) inner: vfx_lut::ProcessList,
}

#[pymethods]
impl ProcessList {
    /// Number of operations in the process list.
    #[getter]
    fn len(&self) -> usize {
        self.inner.nodes.len()
    }
    
    fn __repr__(&self) -> String {
        format!("ProcessList({} operations)", self.len())
    }
}
