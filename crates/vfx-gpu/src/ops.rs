//! GPU-accelerated image operations.

use crate::image::GpuImage;
use crate::backend::{Backend, ProcessingBackend, create_backend};
use crate::GpuResult;

/// Resize filter modes.
#[derive(Debug, Clone, Copy, Default)]
pub enum ResizeFilter {
    /// Nearest-neighbor (fast, blocky).
    Nearest = 0,
    /// Bilinear interpolation.
    #[default]
    Bilinear = 1,
    /// Bicubic interpolation.
    Bicubic = 2,
    /// Lanczos3 (slow, sharp).
    Lanczos = 3,
}

/// GPU image processor.
///
/// Provides image operations using GPU acceleration when available,
/// with automatic fallback to CPU.
pub struct ImageProcessor {
    backend: Box<dyn ProcessingBackend>,
}

impl ImageProcessor {
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

    /// Resize image.
    pub fn resize(&self, img: &GpuImage, width: u32, height: u32, filter: ResizeFilter) -> GpuResult<GpuImage> {
        let handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        let resized = self.backend.resize(handle.as_ref(), width, height, filter as u32)?;
        let data = self.backend.download(resized.as_ref())?;
        GpuImage::from_f32(data, width, height, img.channels)
    }

    /// Apply Gaussian blur.
    pub fn blur(&self, img: &mut GpuImage, radius: f32) -> GpuResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.blur(handle.as_mut(), radius)?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Apply sharpening (unsharp mask).
    pub fn sharpen(&self, img: &mut GpuImage, amount: f32) -> GpuResult<()> {
        // Unsharp mask: sharp = original + amount * (original - blur)
        let original = img.data.clone();
        
        // Small blur
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.blur(handle.as_mut(), 1.0)?;
        let blurred = self.backend.download(handle.as_ref())?;
        
        // Apply unsharp mask
        for i in 0..img.data.len() {
            img.data[i] = original[i] + amount * (original[i] - blurred[i]);
        }
        
        Ok(())
    }

    /// Resize to half size (useful for mipmap generation).
    pub fn resize_half(&self, img: &GpuImage) -> GpuResult<GpuImage> {
        self.resize(img, img.width / 2, img.height / 2, ResizeFilter::Bilinear)
    }
}
