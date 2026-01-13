//! GPU-accelerated image operations.
//!
//! This module provides image processing operations:
//!
//! - **Resize** - Scale images with various filter modes (nearest, bilinear, bicubic, Lanczos)
//! - **Blur** - Gaussian blur with configurable radius
//! - **Sharpen** - Unsharp mask sharpening
//! - **Composite** - Porter-Duff alpha compositing
//! - **Blend** - Photoshop-style blend modes (multiply, screen, overlay, etc.)
//! - **Transform** - Flip, rotate, crop operations
//!
//! # Example
//!
//! ```ignore
//! use vfx_compute::{ImageProcessor, ComputeImage, Backend, BlendMode};
//!
//! let proc = ImageProcessor::new(Backend::Auto)?;
//! let mut img = ComputeImage::from_f32(data, 1920, 1080, 3)?;
//!
//! // Apply Gaussian blur
//! proc.blur(&mut img, 2.0)?;
//!
//! // Resize to half
//! let half = proc.resize_half(&img)?;
//!
//! // Blend two images
//! proc.blend(&overlay, &mut img, BlendMode::Screen, 0.5)?;
//! ```

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

/// Interpolation filter for image resizing.
///
/// Quality/speed trade-off (from fastest to highest quality):
/// `Nearest` < `Bilinear` < `Bicubic` < `Lanczos`
#[derive(Debug, Clone, Copy, Default)]
pub enum ResizeFilter {
    /// Nearest-neighbor: Fastest, produces blocky/pixelated results.
    /// Best for pixel art or when speed is critical.
    Nearest = 0,
    
    /// Bilinear interpolation: Good balance of speed and quality.
    /// Default choice for most use cases.
    #[default]
    Bilinear = 1,
    
    /// Bicubic interpolation: Higher quality, smoother gradients.
    /// Good for photographic content.
    Bicubic = 2,
    
    /// Lanczos-3 windowed sinc: Highest quality, sharpest edges.
    /// Best for final output, but slowest.
    Lanczos = 3,
}

/// Photoshop-style blend modes for image compositing.
///
/// These define how foreground and background pixels combine.
/// All modes support an opacity parameter for partial blending.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum BlendMode {
    /// Normal: Foreground replaces background (respects opacity).
    #[default]
    Normal = 0,
    /// Multiply: `fg × bg` - Darkens, blacks stay black.
    Multiply = 1,
    /// Screen: `1 - (1-fg)(1-bg)` - Lightens, whites stay white.
    Screen = 2,
    /// Add: `fg + bg` - Linear dodge, can clip to white.
    Add = 3,
    /// Subtract: `bg - fg` - Can clip to black.
    Subtract = 4,
    /// Overlay: Multiply darks, screen lights.
    Overlay = 5,
    /// Soft Light: Gentle contrast adjustment.
    SoftLight = 6,
    /// Hard Light: Strong contrast, similar to overlay but harsher.
    HardLight = 7,
    /// Difference: `|fg - bg|` - Useful for comparing images.
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

/// GPU-accelerated image processor.
///
/// Provides image operations (resize, blur, composite, blend, transform)
/// using GPU compute shaders when available, with automatic fallback to CPU.
///
/// # Backend Selection
///
/// - `Backend::Auto` - Automatically selects best available (CUDA > wgpu > CPU)
/// - `Backend::Cpu` - Force CPU processing with rayon parallelization
/// - `Backend::Wgpu` - Force wgpu (Vulkan/Metal/DX12)
/// - `Backend::Cuda` - Force NVIDIA CUDA
///
/// # Example
///
/// ```ignore
/// use vfx_compute::{ImageProcessor, ComputeImage, Backend, ResizeFilter};
///
/// let proc = ImageProcessor::new(Backend::Auto)?;
///
/// // Resize with Lanczos filter
/// let resized = proc.resize(&img, 1920, 1080, ResizeFilter::Lanczos)?;
///
/// // Blur and sharpen
/// proc.blur(&mut img, 3.0)?;
/// proc.sharpen(&mut img, 0.5)?;
/// ```
pub struct ImageProcessor {
    executor: AnyExecutor,
}

impl ImageProcessor {
    /// Create a new image processor with the specified backend.
    ///
    /// # Arguments
    /// * `backend` - Compute backend to use
    ///
    /// # Errors
    /// Returns error if the requested backend is not available.
    pub fn new(backend: Backend) -> ComputeResult<Self> {
        Ok(Self {
            executor: AnyExecutor::new(backend)?,
        })
    }

    /// Get the name of the active backend ("cpu", "wgpu", or "cuda").
    #[inline]
    pub fn backend_name(&self) -> &'static str {
        self.executor.name()
    }

    /// Get available GPU/CPU memory in bytes for processing.
    #[inline]
    pub fn available_memory(&self) -> u64 {
        self.executor.available_memory()
    }

    /// Resize image to new dimensions.
    ///
    /// # Arguments
    /// * `img` - Source image
    /// * `width` - Target width in pixels
    /// * `height` - Target height in pixels
    /// * `filter` - Interpolation filter (see [`ResizeFilter`])
    ///
    /// # Returns
    /// A new [`ComputeImage`] with the resized result.
    pub fn resize(&self, img: &ComputeImage, width: u32, height: u32, filter: ResizeFilter) -> ComputeResult<ComputeImage> {
        match &self.executor {
            AnyExecutor::Cpu(e) => {
                let handle = e.gpu().upload(img.data(), img.width, img.height, img.channels)?;
                let mut dst = e.gpu().allocate(width, height, img.channels)?;
                e.gpu().exec_resize(&handle, &mut dst, filter as u32)?;
                let data = e.gpu().download(&dst)?;
                ComputeImage::from_f32(data, width, height, img.channels)
            }
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => {
                let handle = e.gpu().upload(img.data(), img.width, img.height, img.channels)?;
                let mut dst = e.gpu().allocate(width, height, img.channels)?;
                e.gpu().exec_resize(&handle, &mut dst, filter as u32)?;
                let data = e.gpu().download(&dst)?;
                ComputeImage::from_f32(data, width, height, img.channels)
            }
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => {
                let handle = e.gpu().upload(img.data(), img.width, img.height, img.channels)?;
                let mut dst = e.gpu().allocate(width, height, img.channels)?;
                e.gpu().exec_resize(&handle, &mut dst, filter as u32)?;
                let data = e.gpu().download(&dst)?;
                ComputeImage::from_f32(data, width, height, img.channels)
            }
        }
    }

    /// Apply Gaussian blur.
    ///
    /// # Arguments
    /// * `img` - Image to blur (modified in place)
    /// * `radius` - Blur radius in pixels. Larger = more blur.
    pub fn blur(&self, img: &mut ComputeImage, radius: f32) -> ComputeResult<()> {
        match &self.executor {
            AnyExecutor::Cpu(e) => {
                let handle = e.gpu().upload(img.data(), img.width, img.height, img.channels)?;
                let mut dst = e.gpu().allocate(img.width, img.height, img.channels)?;
                e.gpu().exec_blur(&handle, &mut dst, radius)?;
                img.set_data(e.gpu().download(&dst)?)
            }
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => {
                let handle = e.gpu().upload(img.data(), img.width, img.height, img.channels)?;
                let mut dst = e.gpu().allocate(img.width, img.height, img.channels)?;
                e.gpu().exec_blur(&handle, &mut dst, radius)?;
                img.set_data(e.gpu().download(&dst)?)
            }
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => {
                let handle = e.gpu().upload(img.data(), img.width, img.height, img.channels)?;
                let mut dst = e.gpu().allocate(img.width, img.height, img.channels)?;
                e.gpu().exec_blur(&handle, &mut dst, radius)?;
                img.set_data(e.gpu().download(&dst)?);
            }
        }
        Ok(())
    }

    /// Apply sharpening using unsharp mask technique.
    ///
    /// Formula: `sharp = original + amount × (original - blur)`
    ///
    /// # Arguments
    /// * `img` - Image to sharpen (modified in place)
    /// * `amount` - Sharpening strength. 0.0 = no change, 1.0 = strong
    pub fn sharpen(&self, img: &mut ComputeImage, amount: f32) -> ComputeResult<()> {
        // Unsharp mask: sharp = original + amount * (original - blur)
        let original = img.data().to_vec();
        
        // Small blur
        self.blur(img, 1.0)?;
        let blurred = img.take_data();
        
        // Apply unsharp mask
        img.set_data(original.iter()
            .zip(blurred.iter())
            .map(|(o, b)| o + amount * (o - b))
            .collect());
        
        Ok(())
    }

    /// Resize to half size using bilinear filtering.
    ///
    /// Convenient shorthand for `resize(img, w/2, h/2, Bilinear)`.
    /// Useful for mipmap generation or quick thumbnails.
    pub fn resize_half(&self, img: &ComputeImage) -> ComputeResult<ComputeImage> {
        self.resize(img, img.width / 2, img.height / 2, ResizeFilter::Bilinear)
    }

    // === Composite operations ===

    /// Porter-Duff "Over" compositing: foreground over background.
    ///
    /// Standard alpha compositing where foreground is placed over background.
    /// Both images must be RGBA (4 channels) and same dimensions.
    ///
    /// Formula: `result = fg + bg × (1 - fg.alpha)`
    pub fn composite_over(&self, fg: &ComputeImage, bg: &mut ComputeImage) -> ComputeResult<()> {
        match &self.executor {
            AnyExecutor::Cpu(e) => {
                let fg_handle = e.gpu().upload(fg.data(), fg.width, fg.height, fg.channels)?;
                let mut bg_handle = e.gpu().upload(bg.data(), bg.width, bg.height, bg.channels)?;
                e.gpu().exec_composite_over(&fg_handle, &mut bg_handle)?;
                bg.set_data(e.gpu().download(&bg_handle)?)
            }
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => {
                let fg_handle = e.gpu().upload(fg.data(), fg.width, fg.height, fg.channels)?;
                let mut bg_handle = e.gpu().upload(bg.data(), bg.width, bg.height, bg.channels)?;
                e.gpu().exec_composite_over(&fg_handle, &mut bg_handle)?;
                bg.set_data(e.gpu().download(&bg_handle)?)
            }
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => {
                let fg_handle = e.gpu().upload(fg.data(), fg.width, fg.height, fg.channels)?;
                let mut bg_handle = e.gpu().upload(bg.data(), bg.width, bg.height, bg.channels)?;
                e.gpu().exec_composite_over(&fg_handle, &mut bg_handle)?;
                bg.set_data(e.gpu().download(&bg_handle)?);
            }
        }
        Ok(())
    }

    /// Blend foreground onto background with specified mode and opacity.
    ///
    /// # Arguments
    /// * `fg` - Foreground image
    /// * `bg` - Background image (modified in place with result)
    /// * `mode` - Blend mode (see [`BlendMode`])
    /// * `opacity` - Blend opacity 0.0-1.0 (0 = no effect, 1 = full effect)
    pub fn blend(&self, fg: &ComputeImage, bg: &mut ComputeImage, mode: BlendMode, opacity: f32) -> ComputeResult<()> {
        match &self.executor {
            AnyExecutor::Cpu(e) => {
                let fg_handle = e.gpu().upload(fg.data(), fg.width, fg.height, fg.channels)?;
                let mut bg_handle = e.gpu().upload(bg.data(), bg.width, bg.height, bg.channels)?;
                e.gpu().exec_blend(&fg_handle, &mut bg_handle, mode as u32, opacity)?;
                bg.set_data(e.gpu().download(&bg_handle)?)
            }
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => {
                let fg_handle = e.gpu().upload(fg.data(), fg.width, fg.height, fg.channels)?;
                let mut bg_handle = e.gpu().upload(bg.data(), bg.width, bg.height, bg.channels)?;
                e.gpu().exec_blend(&fg_handle, &mut bg_handle, mode as u32, opacity)?;
                bg.set_data(e.gpu().download(&bg_handle)?)
            }
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => {
                let fg_handle = e.gpu().upload(fg.data(), fg.width, fg.height, fg.channels)?;
                let mut bg_handle = e.gpu().upload(bg.data(), bg.width, bg.height, bg.channels)?;
                e.gpu().exec_blend(&fg_handle, &mut bg_handle, mode as u32, opacity)?;
                bg.set_data(e.gpu().download(&bg_handle)?);
            }
        }
        Ok(())
    }

    // === Transform operations ===

    /// Crop a rectangular region from the image.
    ///
    /// # Arguments
    /// * `img` - Source image
    /// * `x`, `y` - Top-left corner of crop region
    /// * `w`, `h` - Width and height of crop region
    ///
    /// # Errors
    /// Returns error if crop region extends beyond image bounds.
    ///
    /// Note: Implemented on CPU (not GPU-accelerated).
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
                .copy_from_slice(&img.data()[src_row..src_row + (w as usize) * c]);
        }
        
        ComputeImage::from_f32(data, w, h, img.channels)
    }

    /// Flip image horizontally (mirror left-right).
    pub fn flip_h(&self, img: &mut ComputeImage) -> ComputeResult<()> {
        match &self.executor {
            AnyExecutor::Cpu(e) => {
                let mut handle = e.gpu().upload(img.data(), img.width, img.height, img.channels)?;
                e.gpu().exec_flip_h(&mut handle)?;
                img.set_data(e.gpu().download(&handle)?)
            }
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => {
                let mut handle = e.gpu().upload(img.data(), img.width, img.height, img.channels)?;
                e.gpu().exec_flip_h(&mut handle)?;
                img.set_data(e.gpu().download(&handle)?)
            }
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => {
                let mut handle = e.gpu().upload(img.data(), img.width, img.height, img.channels)?;
                e.gpu().exec_flip_h(&mut handle)?;
                img.set_data(e.gpu().download(&handle)?);
            }
        }
        Ok(())
    }

    /// Flip image vertically (mirror top-bottom).
    pub fn flip_v(&self, img: &mut ComputeImage) -> ComputeResult<()> {
        match &self.executor {
            AnyExecutor::Cpu(e) => {
                let mut handle = e.gpu().upload(img.data(), img.width, img.height, img.channels)?;
                e.gpu().exec_flip_v(&mut handle)?;
                img.set_data(e.gpu().download(&handle)?)
            }
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => {
                let mut handle = e.gpu().upload(img.data(), img.width, img.height, img.channels)?;
                e.gpu().exec_flip_v(&mut handle)?;
                img.set_data(e.gpu().download(&handle)?)
            }
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => {
                let mut handle = e.gpu().upload(img.data(), img.width, img.height, img.channels)?;
                e.gpu().exec_flip_v(&mut handle)?;
                img.set_data(e.gpu().download(&handle)?);
            }
        }
        Ok(())
    }

    /// Rotate image 90 degrees clockwise, repeated n times.
    ///
    /// # Arguments
    /// * `img` - Source image
    /// * `n` - Number of 90° rotations (1 = 90° CW, 2 = 180°, 3 = 270° CW)
    ///
    /// # Returns
    /// New image with rotated dimensions (width/height swapped for odd n).
    pub fn rotate_90(&self, img: &ComputeImage, n: u32) -> ComputeResult<ComputeImage> {
        match &self.executor {
            AnyExecutor::Cpu(e) => {
                let handle = e.gpu().upload(img.data(), img.width, img.height, img.channels)?;
                let rotated = e.gpu().exec_rotate_90(&handle, n)?;
                let (w, h, c) = rotated.dimensions();
                let data = e.gpu().download(&rotated)?;
                ComputeImage::from_f32(data, w, h, c)
            }
            #[cfg(feature = "wgpu")]
            AnyExecutor::Wgpu(e) => {
                let handle = e.gpu().upload(img.data(), img.width, img.height, img.channels)?;
                let rotated = e.gpu().exec_rotate_90(&handle, n)?;
                let (w, h, c) = rotated.dimensions();
                let data = e.gpu().download(&rotated)?;
                ComputeImage::from_f32(data, w, h, c)
            }
            #[cfg(feature = "cuda")]
            AnyExecutor::Cuda(e) => {
                let handle = e.gpu().upload(img.data(), img.width, img.height, img.channels)?;
                let rotated = e.gpu().exec_rotate_90(&handle, n)?;
                let (w, h, c) = rotated.dimensions();
                let data = e.gpu().download(&rotated)?;
                ComputeImage::from_f32(data, w, h, c)
            }
        }
    }
}
