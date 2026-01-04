//! GPU-accelerated image operations.

use crate::image::ComputeImage;
use crate::backend::{Backend, ProcessingBackend, create_backend};
pub use crate::backend::BlendMode;
use crate::ComputeResult;

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

    /// Resize image.
    pub fn resize(&self, img: &ComputeImage, width: u32, height: u32, filter: ResizeFilter) -> ComputeResult<ComputeImage> {
        let handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        let resized = self.backend.resize(handle.as_ref(), width, height, filter as u32)?;
        let data = self.backend.download(resized.as_ref())?;
        ComputeImage::from_f32(data, width, height, img.channels)
    }

    /// Apply Gaussian blur.
    pub fn blur(&self, img: &mut ComputeImage, radius: f32) -> ComputeResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.blur(handle.as_mut(), radius)?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Apply sharpening (unsharp mask).
    pub fn sharpen(&self, img: &mut ComputeImage, amount: f32) -> ComputeResult<()> {
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
    pub fn resize_half(&self, img: &ComputeImage) -> ComputeResult<ComputeImage> {
        self.resize(img, img.width / 2, img.height / 2, ResizeFilter::Bilinear)
    }

    // === Composite operations ===

    /// Porter-Duff Over: foreground over background.
    pub fn composite_over(&self, fg: &ComputeImage, bg: &mut ComputeImage) -> ComputeResult<()> {
        let fg_handle = self.backend.upload(&fg.data, fg.width, fg.height, fg.channels)?;
        let mut bg_handle = self.backend.upload(&bg.data, bg.width, bg.height, bg.channels)?;
        self.backend.composite_over(fg_handle.as_ref(), bg_handle.as_mut())?;
        bg.data = self.backend.download(bg_handle.as_ref())?;
        Ok(())
    }

    /// Blend with mode and opacity.
    pub fn blend(&self, fg: &ComputeImage, bg: &mut ComputeImage, mode: BlendMode, opacity: f32) -> ComputeResult<()> {
        let fg_handle = self.backend.upload(&fg.data, fg.width, fg.height, fg.channels)?;
        let mut bg_handle = self.backend.upload(&bg.data, bg.width, bg.height, bg.channels)?;
        self.backend.blend(fg_handle.as_ref(), bg_handle.as_mut(), mode, opacity)?;
        bg.data = self.backend.download(bg_handle.as_ref())?;
        Ok(())
    }

    // === Transform operations ===

    /// Crop region from image.
    pub fn crop(&self, img: &ComputeImage, x: u32, y: u32, w: u32, h: u32) -> ComputeResult<ComputeImage> {
        let handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        let cropped = self.backend.crop(handle.as_ref(), x, y, w, h)?;
        let data = self.backend.download(cropped.as_ref())?;
        ComputeImage::from_f32(data, w, h, img.channels)
    }

    /// Flip horizontal.
    pub fn flip_h(&self, img: &mut ComputeImage) -> ComputeResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.flip_h(handle.as_mut())?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Flip vertical.
    pub fn flip_v(&self, img: &mut ComputeImage) -> ComputeResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.flip_v(handle.as_mut())?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Rotate 90 degrees clockwise (n times).
    pub fn rotate_90(&self, img: &ComputeImage, n: u32) -> ComputeResult<ComputeImage> {
        let handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        let rotated = self.backend.rotate_90(handle.as_ref(), n)?;
        let (w, h, c) = rotated.dimensions();
        let data = self.backend.download(rotated.as_ref())?;
        ComputeImage::from_f32(data, w, h, c)
    }
}
