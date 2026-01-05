//! GPU/CPU processor wrapper.

use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use vfx_compute::{Processor as RustProcessor, Cdl, Backend};

use crate::Image;

/// GPU/CPU image processor.
///
/// Automatically selects GPU if available, falls back to CPU.
///
/// # Example
/// ```python
/// proc = vfx_rs.Processor()
/// proc.exposure(img, 1.5)
/// proc.saturation(img, 1.2)
/// proc.cdl(img, slope=[1.1, 1.0, 0.9])
/// ```
#[pyclass]
pub struct Processor {
    inner: RustProcessor,
}

#[pymethods]
impl Processor {
    /// Create a new processor.
    ///
    /// # Arguments
    /// * `backend` - Optional: "cpu", "wgpu", or None for auto-select
    #[new]
    #[pyo3(signature = (backend=None))]
    fn new(backend: Option<&str>) -> PyResult<Self> {
        let inner = match backend {
            Some("cpu") => RustProcessor::new(Backend::Cpu)
                .map_err(|e| PyRuntimeError::new_err(format!("CPU init failed: {}", e)))?,
            Some("wgpu") | Some("gpu") => RustProcessor::new(Backend::Wgpu)
                .map_err(|e| PyRuntimeError::new_err(format!("GPU init failed: {}", e)))?,
            Some("auto") | None => RustProcessor::auto()
                .map_err(|e| PyRuntimeError::new_err(format!("Processor init failed: {}", e)))?,
            Some(other) => return Err(PyRuntimeError::new_err(
                format!("Unknown backend '{}'. Use 'cpu', 'wgpu', or None", other)
            )),
        };
        Ok(Self { inner })
    }
    
    /// Backend name: "cpu" or "wgpu".
    #[getter]
    fn backend(&self) -> &'static str {
        self.inner.backend_name()
    }
    
    /// Apply exposure adjustment (in stops).
    ///
    /// exposure=1.0 doubles brightness, -1.0 halves it.
    #[pyo3(signature = (image, stops))]
    fn exposure(&self, image: &mut Image, stops: f32) -> PyResult<()> {
        let mut compute_img = vfx_compute::from_image_data(image.as_image_data());
        self.inner.apply_exposure(&mut compute_img, stops)
            .map_err(|e| PyRuntimeError::new_err(format!("Exposure failed: {}", e)))?;
        image.inner = vfx_compute::to_image_data(&compute_img);
        Ok(())
    }
    
    /// Apply saturation adjustment.
    ///
    /// 1.0 = no change, 0.0 = grayscale, 2.0 = double saturation.
    #[pyo3(signature = (image, factor))]
    fn saturation(&self, image: &mut Image, factor: f32) -> PyResult<()> {
        let mut compute_img = vfx_compute::from_image_data(image.as_image_data());
        self.inner.apply_saturation(&mut compute_img, factor)
            .map_err(|e| PyRuntimeError::new_err(format!("Saturation failed: {}", e)))?;
        image.inner = vfx_compute::to_image_data(&compute_img);
        Ok(())
    }
    
    /// Apply contrast adjustment.
    ///
    /// # Arguments
    /// * `factor` - Contrast multiplier (1.0 = no change)
    #[pyo3(signature = (image, factor))]
    fn contrast(&self, image: &mut Image, factor: f32) -> PyResult<()> {
        let mut compute_img = vfx_compute::from_image_data(image.as_image_data());
        self.inner.apply_contrast(&mut compute_img, factor)
            .map_err(|e| PyRuntimeError::new_err(format!("Contrast failed: {}", e)))?;
        image.inner = vfx_compute::to_image_data(&compute_img);
        Ok(())
    }
    
    /// Apply CDL (Color Decision List) grade.
    ///
    /// # Arguments
    /// * `slope` - RGB slope [r, g, b] (default: [1, 1, 1])
    /// * `offset` - RGB offset [r, g, b] (default: [0, 0, 0])
    /// * `power` - RGB power [r, g, b] (default: [1, 1, 1])
    /// * `saturation` - Saturation (default: 1.0)
    #[pyo3(signature = (image, slope=None, offset=None, power=None, saturation=1.0))]
    fn cdl(
        &self,
        image: &mut Image,
        slope: Option<[f32; 3]>,
        offset: Option<[f32; 3]>,
        power: Option<[f32; 3]>,
        saturation: f32,
    ) -> PyResult<()> {
        let cdl = Cdl {
            slope: slope.unwrap_or([1.0, 1.0, 1.0]),
            offset: offset.unwrap_or([0.0, 0.0, 0.0]),
            power: power.unwrap_or([1.0, 1.0, 1.0]),
            saturation,
        };
        
        let mut compute_img = vfx_compute::from_image_data(image.as_image_data());
        self.inner.apply_cdl(&mut compute_img, &cdl)
            .map_err(|e| PyRuntimeError::new_err(format!("CDL failed: {}", e)))?;
        image.inner = vfx_compute::to_image_data(&compute_img);
        Ok(())
    }
    
    fn __repr__(&self) -> String {
        format!("Processor(backend='{}')", self.backend())
    }
}
