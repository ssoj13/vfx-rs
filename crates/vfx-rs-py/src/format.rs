//! Format types for Python API.

use pyo3::prelude::*;

/// Bit depth for image I/O operations.
///
/// Use with format-specific writers:
/// ```python
/// from vfx_rs import BitDepth, io
///
/// # Using enum (recommended)
/// io.write_dpx("out.dpx", img, bit_depth=BitDepth.Bit10)
///
/// # Using integer (also works)
/// io.write_dpx("out.dpx", img, bit_depth=10)
/// ```
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitDepth {
    /// 8 bits per channel.
    Bit8 = 8,
    /// 10 bits per channel (DPX film standard).
    Bit10 = 10,
    /// 12 bits per channel (cinema cameras).
    Bit12 = 12,
    /// 16 bits per channel.
    Bit16 = 16,
}

#[pymethods]
impl BitDepth {
    /// Get numeric value.
    #[getter]
    fn value(&self) -> u8 {
        *self as u8
    }

    fn __repr__(&self) -> String {
        format!("BitDepth.Bit{}", self.value())
    }

    fn __int__(&self) -> u8 {
        self.value()
    }
}

impl BitDepth {
    /// Convert from Python int or BitDepth enum.
    pub fn from_py(value: &Bound<'_, PyAny>) -> PyResult<u8> {
        // Try as BitDepth enum first
        if let Ok(bd) = value.extract::<BitDepth>() {
            return Ok(bd.value());
        }
        // Try as integer
        if let Ok(n) = value.extract::<u8>() {
            return Ok(n);
        }
        Err(pyo3::exceptions::PyTypeError::new_err(
            "bit_depth must be BitDepth enum or int (8, 10, 12, 16)"
        ))
    }
}

