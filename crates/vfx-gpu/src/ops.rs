//! GPU-accelerated image operations.

use crate::image::GpuImage;
use crate::backend::{Backend, CpuPrimitives, GpuLimits};
use crate::backend::GpuPrimitives;
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
pub struct ImageProcessor {
    primitives: Box<dyn ImagePrimitives>,
}

trait ImagePrimitives: Send + Sync {
    fn resize(&self, img: &GpuImage, width: u32, height: u32, filter: ResizeFilter) -> GpuResult<GpuImage>;
    fn blur(&self, img: &mut GpuImage, radius: f32) -> GpuResult<()>;
    fn sharpen(&self, img: &mut GpuImage, amount: f32) -> GpuResult<()>;
    fn limits(&self) -> &GpuLimits;
    fn name(&self) -> &'static str;
}

/// CPU implementation of image primitives.
struct CpuImagePrimitives {
    inner: CpuPrimitives,
}

impl ImagePrimitives for CpuImagePrimitives {
    fn resize(&self, img: &GpuImage, new_w: u32, new_h: u32, filter: ResizeFilter) -> GpuResult<GpuImage> {
        use rayon::prelude::*;

        let (sw, sh, c) = (img.width, img.height, img.channels);
        let sx = sw as f32 / new_w as f32;
        let sy = sh as f32 / new_h as f32;

        let mut out = vec![0.0f32; (new_w * new_h * c) as usize];

        out.par_chunks_mut((new_w * c) as usize)
            .enumerate()
            .for_each(|(dy, row)| {
                for dx in 0..new_w as usize {
                    let fx = dx as f32 * sx;
                    let fy = dy as f32 * sy;

                    match filter {
                        ResizeFilter::Nearest => {
                            let x = (fx as usize).min(sw as usize - 1);
                            let y = (fy as usize).min(sh as usize - 1);
                            for ch in 0..c as usize {
                                row[dx * c as usize + ch] = img.data[(y * sw as usize + x) * c as usize + ch];
                            }
                        }
                        _ => {
                            // Bilinear for now
                            let x0 = (fx as usize).min(sw as usize - 1);
                            let y0 = (fy as usize).min(sh as usize - 1);
                            let x1 = (x0 + 1).min(sw as usize - 1);
                            let y1 = (y0 + 1).min(sh as usize - 1);

                            let fx = fx - x0 as f32;
                            let fy = fy - y0 as f32;

                            for ch in 0..c as usize {
                                let idx = |x: usize, y: usize| img.data[(y * sw as usize + x) * c as usize + ch];
                                let c00 = idx(x0, y0);
                                let c10 = idx(x1, y0);
                                let c01 = idx(x0, y1);
                                let c11 = idx(x1, y1);

                                let top = c00 + fx * (c10 - c00);
                                let bot = c01 + fx * (c11 - c01);
                                row[dx * c as usize + ch] = top + fy * (bot - top);
                            }
                        }
                    }
                }
            });

        GpuImage::from_f32(out, new_w, new_h, c)
    }

    fn blur(&self, img: &mut GpuImage, radius: f32) -> GpuResult<()> {
        use rayon::prelude::*;

        let (w, h, c) = (img.width, img.height, img.channels);
        let r = radius.ceil() as i32;
        let sigma = radius / 3.0;

        // Gaussian kernel
        let k_size = (r * 2 + 1) as usize;
        let mut kernel = vec![0.0f32; k_size];
        let mut sum = 0.0;
        for i in 0..k_size {
            let x = (i as i32 - r) as f32;
            let g = (-x * x / (2.0 * sigma * sigma)).exp();
            kernel[i] = g;
            sum += g;
        }
        for k in &mut kernel { *k /= sum; }

        // Horizontal pass
        let mut temp = vec![0.0f32; img.data.len()];
        temp.par_chunks_mut((w * c) as usize)
            .enumerate()
            .for_each(|(y, row)| {
                for x in 0..w as i32 {
                    for ch in 0..c as usize {
                        let mut acc = 0.0;
                        for ki in 0..k_size {
                            let sx = (x + ki as i32 - r).clamp(0, w as i32 - 1) as usize;
                            acc += img.data[(y * w as usize + sx) * c as usize + ch] * kernel[ki];
                        }
                        row[x as usize * c as usize + ch] = acc;
                    }
                }
            });

        // Vertical pass
        img.data.par_chunks_mut((w * c) as usize)
            .enumerate()
            .for_each(|(y, row)| {
                for x in 0..w as usize {
                    for ch in 0..c as usize {
                        let mut acc = 0.0;
                        for ki in 0..k_size {
                            let sy = (y as i32 + ki as i32 - r).clamp(0, h as i32 - 1) as usize;
                            acc += temp[(sy * w as usize + x) * c as usize + ch] * kernel[ki];
                        }
                        row[x * c as usize + ch] = acc;
                    }
                }
            });

        Ok(())
    }

    fn sharpen(&self, img: &mut GpuImage, amount: f32) -> GpuResult<()> {
        use rayon::prelude::*;

        let (w, h, c) = (img.width, img.height, img.channels);
        let original = img.data.clone();

        // Unsharp mask: sharp = original + amount * (original - blur)
        // Using 3x3 blur approximation
        img.data.par_chunks_mut((w * c) as usize)
            .enumerate()
            .for_each(|(y, row)| {
                let y = y as i32;
                for x in 0..w as i32 {
                    for ch in 0..c as usize {
                        let idx = |dx: i32, dy: i32| -> f32 {
                            let sx = (x + dx).clamp(0, w as i32 - 1) as usize;
                            let sy = (y + dy).clamp(0, h as i32 - 1) as usize;
                            original[(sy * w as usize + sx) * c as usize + ch]
                        };

                        let center = idx(0, 0);
                        let blur = (idx(-1, -1) + idx(0, -1) + idx(1, -1) +
                                   idx(-1, 0) + center + idx(1, 0) +
                                   idx(-1, 1) + idx(0, 1) + idx(1, 1)) / 9.0;

                        row[x as usize * c as usize + ch] = center + amount * (center - blur);
                    }
                }
            });

        Ok(())
    }

    fn limits(&self) -> &GpuLimits {
        self.inner.limits()
    }

    fn name(&self) -> &'static str {
        "CPU"
    }
}

impl ImageProcessor {
    /// Create with specified backend.
    pub fn new(backend: Backend) -> GpuResult<Self> {
        match backend {
            Backend::Auto | Backend::Cpu => {
                Ok(Self {
                    primitives: Box::new(CpuImagePrimitives {
                        inner: CpuPrimitives::new(),
                    }),
                })
            }
            Backend::Wgpu => {
                #[cfg(feature = "wgpu")]
                {
                    // TODO: WgpuImagePrimitives
                    Ok(Self {
                        primitives: Box::new(CpuImagePrimitives {
                            inner: CpuPrimitives::new(),
                        }),
                    })
                }
                #[cfg(not(feature = "wgpu"))]
                {
                    Err(crate::GpuError::BackendNotAvailable("wgpu".into()))
                }
            }
        }
    }

    /// Backend name.
    pub fn backend_name(&self) -> &'static str {
        self.primitives.name()
    }

    /// Resize image.
    pub fn resize(&self, img: &GpuImage, width: u32, height: u32, filter: ResizeFilter) -> GpuResult<GpuImage> {
        self.primitives.resize(img, width, height, filter)
    }

    /// Apply Gaussian blur.
    pub fn blur(&self, img: &mut GpuImage, radius: f32) -> GpuResult<()> {
        self.primitives.blur(img, radius)
    }

    /// Apply sharpening.
    pub fn sharpen(&self, img: &mut GpuImage, amount: f32) -> GpuResult<()> {
        self.primitives.sharpen(img, amount)
    }

    /// Resize to half size (useful for mipmap generation).
    pub fn resize_half(&self, img: &GpuImage) -> GpuResult<GpuImage> {
        self.resize(img, img.width / 2, img.height / 2, ResizeFilter::Bilinear)
    }
}
