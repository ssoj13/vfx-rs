//! GPU-accelerated color processing.

use crate::image::GpuImage;
use crate::backend::{Backend, ProcessingBackend, create_backend};
use crate::GpuResult;

/// CDL (Color Decision List) parameters.
#[derive(Debug, Clone, Copy)]
pub struct Cdl {
    pub slope: [f32; 3],
    pub offset: [f32; 3],
    pub power: [f32; 3],
    pub saturation: f32,
}

impl Default for Cdl {
    fn default() -> Self {
        Self {
            slope: [1.0, 1.0, 1.0],
            offset: [0.0, 0.0, 0.0],
            power: [1.0, 1.0, 1.0],
            saturation: 1.0,
        }
    }
}

/// GPU color processor.
///
/// Provides color grading operations using GPU acceleration when available,
/// with automatic fallback to CPU.
pub struct ColorProcessor {
    backend: Box<dyn ProcessingBackend>,
}

impl ColorProcessor {
    /// Create with specified backend.
    pub fn new(backend: Backend) -> GpuResult<Self> {
        Ok(Self {
            backend: create_backend(backend)?,
        })
    }

    /// Backend name.
    pub fn backend_name(&self) -> &'static str {
        self.backend.name()
    }

    /// Available memory in bytes.
    pub fn available_memory(&self) -> u64 {
        self.backend.available_memory()
    }

    /// Upload image to GPU memory.
    pub fn upload(&self, img: &GpuImage) -> GpuResult<Box<dyn crate::backend::ImageHandle>> {
        self.backend.upload(&img.data, img.width, img.height, img.channels)
    }

    /// Download image from GPU memory.
    pub fn download(&self, handle: &dyn crate::backend::ImageHandle, width: u32, height: u32, channels: u32) -> GpuResult<GpuImage> {
        let data = self.backend.download(handle)?;
        GpuImage::from_f32(data, width, height, channels)
    }

    /// Apply 4x4 color matrix.
    pub fn apply_matrix(&self, img: &mut GpuImage, matrix: &[f32; 16]) -> GpuResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.apply_matrix(handle.as_mut(), matrix)?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Apply CDL transform.
    pub fn apply_cdl(&self, img: &mut GpuImage, cdl: &Cdl) -> GpuResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.apply_cdl(handle.as_mut(), cdl.slope, cdl.offset, cdl.power, cdl.saturation)?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Apply 1D LUT.
    pub fn apply_lut1d(&self, img: &mut GpuImage, lut: &[f32], channels: u32) -> GpuResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.apply_lut1d(handle.as_mut(), lut, channels)?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Apply 3D LUT.
    pub fn apply_lut3d(&self, img: &mut GpuImage, lut: &[f32], size: u32) -> GpuResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.apply_lut3d(handle.as_mut(), lut, size)?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Apply exposure adjustment (in stops).
    pub fn apply_exposure(&self, img: &mut GpuImage, stops: f32) -> GpuResult<()> {
        let mult = 2.0f32.powf(stops);
        let matrix = [
            mult, 0.0, 0.0, 0.0,
            0.0, mult, 0.0, 0.0,
            0.0, 0.0, mult, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        self.apply_matrix(img, &matrix)
    }

    /// Apply saturation adjustment.
    pub fn apply_saturation(&self, img: &mut GpuImage, sat: f32) -> GpuResult<()> {
        let cdl = Cdl {
            saturation: sat,
            ..Default::default()
        };
        self.apply_cdl(img, &cdl)
    }
}
