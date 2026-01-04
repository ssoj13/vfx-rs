//! GPU-accelerated color processing.

use crate::image::ComputeImage;
use crate::backend::{Backend, ProcessingBackend, create_backend};
use crate::ComputeResult;

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
    pub fn new(backend: Backend) -> ComputeResult<Self> {
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
    pub fn upload(&self, img: &ComputeImage) -> ComputeResult<Box<dyn crate::backend::ImageHandle>> {
        self.backend.upload(&img.data, img.width, img.height, img.channels)
    }

    /// Download image from GPU memory.
    pub fn download(&self, handle: &dyn crate::backend::ImageHandle, width: u32, height: u32, channels: u32) -> ComputeResult<ComputeImage> {
        let data = self.backend.download(handle)?;
        ComputeImage::from_f32(data, width, height, channels)
    }

    /// Apply 4x4 color matrix.
    pub fn apply_matrix(&self, img: &mut ComputeImage, matrix: &[f32; 16]) -> ComputeResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.apply_matrix(handle.as_mut(), matrix)?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Apply CDL transform.
    pub fn apply_cdl(&self, img: &mut ComputeImage, cdl: &Cdl) -> ComputeResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.apply_cdl(handle.as_mut(), cdl.slope, cdl.offset, cdl.power, cdl.saturation)?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Apply 1D LUT.
    pub fn apply_lut1d(&self, img: &mut ComputeImage, lut: &[f32], channels: u32) -> ComputeResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.apply_lut1d(handle.as_mut(), lut, channels)?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Apply 3D LUT.
    pub fn apply_lut3d(&self, img: &mut ComputeImage, lut: &[f32], size: u32) -> ComputeResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.apply_lut3d(handle.as_mut(), lut, size)?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Apply exposure adjustment (in stops).
    pub fn apply_exposure(&self, img: &mut ComputeImage, stops: f32) -> ComputeResult<()> {
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
    pub fn apply_saturation(&self, img: &mut ComputeImage, sat: f32) -> ComputeResult<()> {
        let cdl = Cdl {
            saturation: sat,
            ..Default::default()
        };
        self.apply_cdl(img, &cdl)
    }
}
