//! Unified compute processor combining color and image operations.

use crate::image::ComputeImage;
use crate::backend::{Backend, ProcessingBackend, create_backend};
use crate::color::Cdl;
use crate::ops::ResizeFilter;
use crate::ComputeResult;

/// Unified compute processor.
///
/// Combines color grading and image processing operations with automatic
/// backend selection (GPU when available, CPU fallback).
///
/// # Example
/// ```ignore
/// use vfx_compute::{Processor, Backend, ComputeImage};
///
/// let proc = Processor::auto()?;
/// let mut img = ComputeImage::from_f32(data, 1920, 1080, 3)?;
///
/// // Color operations
/// proc.apply_exposure(&mut img, 1.5)?;
/// proc.apply_saturation(&mut img, 1.2)?;
///
/// // Image operations  
/// let resized = proc.resize(&img, 960, 540, ResizeFilter::Bilinear)?;
/// ```
pub struct Processor {
    backend: Box<dyn ProcessingBackend>,
}

impl Processor {
    /// Create with specified backend.
    pub fn new(backend: Backend) -> ComputeResult<Self> {
        Ok(Self {
            backend: create_backend(backend)?,
        })
    }

    /// Create with auto-selected backend (GPU if available, else CPU).
    pub fn auto() -> ComputeResult<Self> {
        Self::new(Backend::Auto)
    }

    /// Create with CPU backend.
    pub fn cpu() -> ComputeResult<Self> {
        Self::new(Backend::Cpu)
    }

    /// Create with GPU backend (requires wgpu feature).
    #[cfg(feature = "wgpu")]
    pub fn gpu() -> ComputeResult<Self> {
        Self::new(Backend::Wgpu)
    }

    // =========================================================================
    // Info
    // =========================================================================

    /// Backend name ("cpu" or "wgpu").
    pub fn backend_name(&self) -> &'static str {
        self.backend.name()
    }

    /// Available memory in bytes.
    pub fn available_memory(&self) -> u64 {
        self.backend.available_memory()
    }

    /// Check if using GPU backend.
    pub fn is_gpu(&self) -> bool {
        self.backend.name() == "wgpu"
    }

    // =========================================================================
    // Color Operations
    // =========================================================================

    /// Apply 4x4 color matrix transform.
    pub fn apply_matrix(&self, img: &mut ComputeImage, matrix: &[f32; 16]) -> ComputeResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.apply_matrix(handle.as_mut(), matrix)?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Apply CDL (Color Decision List) transform.
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

    /// Apply 3D LUT with trilinear interpolation.
    pub fn apply_lut3d(&self, img: &mut ComputeImage, lut: &[f32], size: u32) -> ComputeResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.apply_lut3d(handle.as_mut(), lut, size)?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Apply exposure adjustment (in stops).
    /// 
    /// +1.0 = 2x brighter, -1.0 = 2x darker.
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
    ///
    /// 1.0 = no change, 0.0 = grayscale, 2.0 = double saturation.
    pub fn apply_saturation(&self, img: &mut ComputeImage, sat: f32) -> ComputeResult<()> {
        let cdl = Cdl {
            saturation: sat,
            ..Default::default()
        };
        self.apply_cdl(img, &cdl)
    }

    /// Apply contrast adjustment.
    ///
    /// 1.0 = no change, 0.5 = less contrast, 2.0 = more contrast.
    pub fn apply_contrast(&self, img: &mut ComputeImage, contrast: f32) -> ComputeResult<()> {
        let offset = 0.5 * (1.0 - contrast);
        let matrix = [
            contrast, 0.0, 0.0, offset,
            0.0, contrast, 0.0, offset,
            0.0, 0.0, contrast, offset,
            0.0, 0.0, 0.0, 1.0,
        ];
        self.apply_matrix(img, &matrix)
    }

    // =========================================================================
    // Image Operations
    // =========================================================================

    /// Resize image with specified filter.
    pub fn resize(&self, img: &ComputeImage, width: u32, height: u32, filter: ResizeFilter) -> ComputeResult<ComputeImage> {
        let handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        let resized = self.backend.resize(handle.as_ref(), width, height, filter as u32)?;
        let data = self.backend.download(resized.as_ref())?;
        ComputeImage::from_f32(data, width, height, img.channels)
    }

    /// Resize to half dimensions (useful for mipmap generation).
    pub fn resize_half(&self, img: &ComputeImage) -> ComputeResult<ComputeImage> {
        self.resize(img, img.width / 2, img.height / 2, ResizeFilter::Bilinear)
    }

    /// Apply Gaussian blur.
    pub fn blur(&self, img: &mut ComputeImage, radius: f32) -> ComputeResult<()> {
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.blur(handle.as_mut(), radius)?;
        img.data = self.backend.download(handle.as_ref())?;
        Ok(())
    }

    /// Apply sharpening (unsharp mask).
    ///
    /// Amount 1.0 = moderate sharpening.
    pub fn sharpen(&self, img: &mut ComputeImage, amount: f32) -> ComputeResult<()> {
        let original = img.data.clone();
        
        // Small blur
        let mut handle = self.backend.upload(&img.data, img.width, img.height, img.channels)?;
        self.backend.blur(handle.as_mut(), 1.0)?;
        let blurred = self.backend.download(handle.as_ref())?;
        
        // Unsharp mask: sharp = original + amount * (original - blur)
        for i in 0..img.data.len() {
            img.data[i] = original[i] + amount * (original[i] - blurred[i]);
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_auto() {
        let proc = Processor::auto().unwrap();
        println!("Backend: {}", proc.backend_name());
        // CPU backend may report 0 on some systems
        let _mem = proc.available_memory();
    }

    #[test]
    fn test_processor_exposure() {
        let proc = Processor::cpu().unwrap();
        let mut img = ComputeImage::from_f32(vec![0.5, 0.5, 0.5], 1, 1, 3).unwrap();
        
        proc.apply_exposure(&mut img, 1.0).unwrap();
        
        // +1 stop = 2x brightness
        assert!((img.data()[0] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_processor_contrast() {
        let proc = Processor::cpu().unwrap();
        let mut img = ComputeImage::from_f32(vec![0.5, 0.5, 0.5], 1, 1, 3).unwrap();
        
        // No change at contrast=1.0
        proc.apply_contrast(&mut img, 1.0).unwrap();
        assert!((img.data()[0] - 0.5).abs() < 1e-5);
    }
}
