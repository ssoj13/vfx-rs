//! GPU-accelerated image operations.
//!
//! Provides a unified API for image operations using TiledExecutor.

use crate::image::ComputeImage;
use crate::backend::{
    Backend, TiledExecutor,
    CpuPrimitives, GpuPrimitives, ImageHandle,
};
#[cfg(feature = "wgpu")]
use crate::backend::WgpuPrimitives;
#[cfg(feature = "cuda")]
use crate::backend::CudaPrimitives;

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

/// Blend modes for compositing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum BlendMode {
    #[default]
    Normal = 0,
    Multiply = 1,
    Screen = 2,
    Add = 3,
    Subtract = 4,
    Overlay = 5,
    SoftLight = 6,
    HardLight = 7,
    Difference = 8,
}

/// Unified executor that wraps any backend.
enum AnyExecutor {
    Cpu(TiledExecutor<CpuPrimitives>),
    #[cfg(feature = "wgpu")]
    Wgpu(TiledExecutor<WgpuPrimitives>),
    #[cfg(feature = "cuda")]
    Cuda(TiledExecutor<CudaPrimitives>),
}

impl AnyExecutor {
    fn new(backend: Backend) -> ComputeResult<Self> {
        match backend {
            Backend::Cpu => {
                let cpu = CpuPrimitives::new();
                Ok(Self::Cpu(TiledExecutor::new(cpu)))
            }
            #[cfg(feature = "wgpu")]
            Backend::Wgpu => {
                let wgpu = WgpuPrimitives::new()?;
                Ok(Self::Wgpu(TiledExecutor::new(wgpu)))
            }
            #[cfg(feature = "cuda")]
            Backend::Cuda => {
                let cuda = CudaPrimitives::new()?;
                Ok(Self::Cuda(TiledExecutor::new(cuda)))
            }
            Backend::Auto => {
                // Try CUDA first, then wgpu, then CPU
                #[cfg(feature = "cuda")]
                if CudaPrimitives::is_available() {
                    return Self::new(Backend::Cuda);
                }
                #[cfg(feature = "wgpu")]
                if WgpuPrimitives::is_available() {
                    return Self::new(Backend::Wgpu);
                }
                Self::new(Backend::Cpu)
            }
            #[allow(unreachable_patterns)]
            _ => Self::new(Backend::Cpu),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Cpu(e) => e.name(),
            #[cfg(feature = "wgpu")]
            Self::Wgpu(e) => e.name(),
            #[cfg(feature = "cuda")]
            Self::Cuda(e) => e.name(),
        }
    }

    fn available_memory(&self) -> u64 {
        match self {
            Self::Cpu(e) => e.limits().available_memory,
            #[cfg(feature = "wgpu")]
            Self::Wgpu(e) => e.limits().available_memory,
            #[cfg(feature = "cuda")]
            Self::Cuda(e) => e.limits().available_memory,
        }
    }
}

/// GPU image processor.
///
/// Provides image operations using GPU acceleration when available,
/// with automatic fallback to CPU.
pub struct ImageProcessor {
    executor: AnyExecutor,
}

impl ImageProcessor {
    /// Create with specified backend.
    pub fn new(backend: Backend) -> ComputeResult<Self> {
        Ok(Self {
            executor: AnyExecutor::new(backend)?,
        })
    }

    /// Backend name.
    pub fn backend_name(&self) -> &'static str {
        self.executor.name()
    }

    /// Available memory in bytes.
    pub fn available_memory(&self) -> u64 {
        self.executor.available_memory()
    }

    /// Resize image.
    pub fn resize(&self, img: &ComputeImage, width: u32, height: u32, filter: ResizeFilter) -> ComputeResult<ComputeImage> {
        match &self.executor {
            AnyExecutor::Cpu(e) => {
                let handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                let mut dst = e.gpu().allocate(width, height, img.channels)?;
                e.gpu().exec_resize(&handle, &mut dst, filter as u32)?;
                let data = e.gpu().download(&dst)?;
                ComputeImage::from_f32(data, width, height, img.channels)
            }
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => {
                let handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                let mut dst = e.gpu().allocate(width, height, img.channels)?;
                e.gpu().exec_resize(&handle, &mut dst, filter as u32)?;
                let data = e.gpu().download(&dst)?;
                ComputeImage::from_f32(data, width, height, img.channels)
            }
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => {
                let handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                let mut dst = e.gpu().allocate(width, height, img.channels)?;
                e.gpu().exec_resize(&handle, &mut dst, filter as u32)?;
                let data = e.gpu().download(&dst)?;
                ComputeImage::from_f32(data, width, height, img.channels)
            }
        }
    }

    /// Apply Gaussian blur.
    pub fn blur(&self, img: &mut ComputeImage, radius: f32) -> ComputeResult<()> {
        match &self.executor {
            AnyExecutor::Cpu(e) => {
                let handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                let mut dst = e.gpu().allocate(img.width, img.height, img.channels)?;
                e.gpu().exec_blur(&handle, &mut dst, radius)?;
                img.data = e.gpu().download(&dst)?;
            }
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => {
                let handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                let mut dst = e.gpu().allocate(img.width, img.height, img.channels)?;
                e.gpu().exec_blur(&handle, &mut dst, radius)?;
                img.data = e.gpu().download(&dst)?;
            }
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => {
                let handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                let mut dst = e.gpu().allocate(img.width, img.height, img.channels)?;
                e.gpu().exec_blur(&handle, &mut dst, radius)?;
                img.data = e.gpu().download(&dst)?;
            }
        }
        Ok(())
    }

    /// Apply sharpening (unsharp mask).
    pub fn sharpen(&self, img: &mut ComputeImage, amount: f32) -> ComputeResult<()> {
        // Unsharp mask: sharp = original + amount * (original - blur)
        let original = img.data.clone();
        
        // Small blur
        self.blur(img, 1.0)?;
        let blurred = std::mem::take(&mut img.data);
        
        // Apply unsharp mask
        img.data = original.iter()
            .zip(blurred.iter())
            .map(|(o, b)| o + amount * (o - b))
            .collect();
        
        Ok(())
    }

    /// Resize to half size (useful for mipmap generation).
    pub fn resize_half(&self, img: &ComputeImage) -> ComputeResult<ComputeImage> {
        self.resize(img, img.width / 2, img.height / 2, ResizeFilter::Bilinear)
    }

    // === Composite operations ===

    /// Porter-Duff Over: foreground over background.
    pub fn composite_over(&self, fg: &ComputeImage, bg: &mut ComputeImage) -> ComputeResult<()> {
        match &self.executor {
            AnyExecutor::Cpu(e) => {
                let fg_handle = e.gpu().upload(&fg.data, fg.width, fg.height, fg.channels)?;
                let mut bg_handle = e.gpu().upload(&bg.data, bg.width, bg.height, bg.channels)?;
                e.gpu().exec_composite_over(&fg_handle, &mut bg_handle)?;
                bg.data = e.gpu().download(&bg_handle)?;
            }
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => {
                let fg_handle = e.gpu().upload(&fg.data, fg.width, fg.height, fg.channels)?;
                let mut bg_handle = e.gpu().upload(&bg.data, bg.width, bg.height, bg.channels)?;
                e.gpu().exec_composite_over(&fg_handle, &mut bg_handle)?;
                bg.data = e.gpu().download(&bg_handle)?;
            }
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => {
                let fg_handle = e.gpu().upload(&fg.data, fg.width, fg.height, fg.channels)?;
                let mut bg_handle = e.gpu().upload(&bg.data, bg.width, bg.height, bg.channels)?;
                e.gpu().exec_composite_over(&fg_handle, &mut bg_handle)?;
                bg.data = e.gpu().download(&bg_handle)?;
            }
        }
        Ok(())
    }

    /// Blend with mode and opacity.
    pub fn blend(&self, fg: &ComputeImage, bg: &mut ComputeImage, mode: BlendMode, opacity: f32) -> ComputeResult<()> {
        match &self.executor {
            AnyExecutor::Cpu(e) => {
                let fg_handle = e.gpu().upload(&fg.data, fg.width, fg.height, fg.channels)?;
                let mut bg_handle = e.gpu().upload(&bg.data, bg.width, bg.height, bg.channels)?;
                e.gpu().exec_blend(&fg_handle, &mut bg_handle, mode as u32, opacity)?;
                bg.data = e.gpu().download(&bg_handle)?;
            }
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => {
                let fg_handle = e.gpu().upload(&fg.data, fg.width, fg.height, fg.channels)?;
                let mut bg_handle = e.gpu().upload(&bg.data, bg.width, bg.height, bg.channels)?;
                e.gpu().exec_blend(&fg_handle, &mut bg_handle, mode as u32, opacity)?;
                bg.data = e.gpu().download(&bg_handle)?;
            }
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => {
                let fg_handle = e.gpu().upload(&fg.data, fg.width, fg.height, fg.channels)?;
                let mut bg_handle = e.gpu().upload(&bg.data, bg.width, bg.height, bg.channels)?;
                e.gpu().exec_blend(&fg_handle, &mut bg_handle, mode as u32, opacity)?;
                bg.data = e.gpu().download(&bg_handle)?;
            }
        }
        Ok(())
    }

    // === Transform operations ===

    /// Crop region from image.
    /// 
    /// Note: This is implemented on CPU as GpuPrimitives doesn't have exec_crop.
    pub fn crop(&self, img: &ComputeImage, x: u32, y: u32, w: u32, h: u32) -> ComputeResult<ComputeImage> {
        let src_w = img.width as usize;
        let src_h = img.height as usize;
        let c = img.channels as usize;
        
        // Bounds check
        if x as usize + w as usize > src_w || y as usize + h as usize > src_h {
            return Err(crate::ComputeError::InvalidDimensions(w, h));
        }
        
        let mut data = vec![0.0f32; (w as usize) * (h as usize) * c];
        
        for row in 0..h as usize {
            let src_row = (y as usize + row) * src_w * c + (x as usize) * c;
            let dst_row = row * (w as usize) * c;
            data[dst_row..dst_row + (w as usize) * c]
                .copy_from_slice(&img.data[src_row..src_row + (w as usize) * c]);
        }
        
        ComputeImage::from_f32(data, w, h, img.channels)
    }

    /// Flip horizontal.
    pub fn flip_h(&self, img: &mut ComputeImage) -> ComputeResult<()> {
        match &self.executor {
            AnyExecutor::Cpu(e) => {
                let mut handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                e.gpu().exec_flip_h(&mut handle)?;
                img.data = e.gpu().download(&handle)?;
            }
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => {
                let mut handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                e.gpu().exec_flip_h(&mut handle)?;
                img.data = e.gpu().download(&handle)?;
            }
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => {
                let mut handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                e.gpu().exec_flip_h(&mut handle)?;
                img.data = e.gpu().download(&handle)?;
            }
        }
        Ok(())
    }

    /// Flip vertical.
    pub fn flip_v(&self, img: &mut ComputeImage) -> ComputeResult<()> {
        match &self.executor {
            AnyExecutor::Cpu(e) => {
                let mut handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                e.gpu().exec_flip_v(&mut handle)?;
                img.data = e.gpu().download(&handle)?;
            }
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => {
                let mut handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                e.gpu().exec_flip_v(&mut handle)?;
                img.data = e.gpu().download(&handle)?;
            }
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => {
                let mut handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                e.gpu().exec_flip_v(&mut handle)?;
                img.data = e.gpu().download(&handle)?;
            }
        }
        Ok(())
    }

    /// Rotate 90 degrees clockwise (n times).
    pub fn rotate_90(&self, img: &ComputeImage, n: u32) -> ComputeResult<ComputeImage> {
        match &self.executor {
            AnyExecutor::Cpu(e) => {
                let handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                let rotated = e.gpu().exec_rotate_90(&handle, n)?;
                let (w, h, c) = rotated.dimensions();
                let data = e.gpu().download(&rotated)?;
                ComputeImage::from_f32(data, w, h, c)
            }
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => {
                let handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                let rotated = e.gpu().exec_rotate_90(&handle, n)?;
                let (w, h, c) = rotated.dimensions();
                let data = e.gpu().download(&rotated)?;
                ComputeImage::from_f32(data, w, h, c)
            }
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => {
                let handle = e.gpu().upload(&img.data, img.width, img.height, img.channels)?;
                let rotated = e.gpu().exec_rotate_90(&handle, n)?;
                let (w, h, c) = rotated.dimensions();
                let data = e.gpu().download(&rotated)?;
                ComputeImage::from_f32(data, w, h, c)
            }
        }
    }
}
