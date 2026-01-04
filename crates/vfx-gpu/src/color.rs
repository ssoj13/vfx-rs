//! GPU-accelerated color processing.

use crate::image::GpuImage;
use crate::backend::{Backend, CpuPrimitives, GpuLimits};
use crate::backend::gpu_primitives::GpuPrimitives;
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
pub struct ColorProcessor {
    primitives: Box<dyn ColorPrimitives>,
}

trait ColorPrimitives: Send + Sync {
    fn apply_matrix(&self, img: &mut GpuImage, matrix: &[f32; 16]) -> GpuResult<()>;
    fn apply_cdl(&self, img: &mut GpuImage, cdl: &Cdl) -> GpuResult<()>;
    fn apply_lut1d(&self, img: &mut GpuImage, lut: &[f32], channels: u32) -> GpuResult<()>;
    fn apply_lut3d(&self, img: &mut GpuImage, lut: &[f32], size: u32) -> GpuResult<()>;
    fn limits(&self) -> &GpuLimits;
    fn name(&self) -> &'static str;
}

/// CPU implementation of color primitives.
struct CpuColorPrimitives {
    inner: CpuPrimitives,
}

impl ColorPrimitives for CpuColorPrimitives {
    fn apply_matrix(&self, img: &mut GpuImage, matrix: &[f32; 16]) -> GpuResult<()> {
        use rayon::prelude::*;
        let c = img.channels as usize;
        
        img.data.par_chunks_mut(c)
            .for_each(|px| {
                let r = px.get(0).copied().unwrap_or(0.0);
                let g = px.get(1).copied().unwrap_or(0.0);
                let b = px.get(2).copied().unwrap_or(0.0);
                let a = px.get(3).copied().unwrap_or(1.0);
                
                px[0] = matrix[0] * r + matrix[1] * g + matrix[2] * b + matrix[3] * a;
                px[1] = matrix[4] * r + matrix[5] * g + matrix[6] * b + matrix[7] * a;
                px[2] = matrix[8] * r + matrix[9] * g + matrix[10] * b + matrix[11] * a;
                if c >= 4 {
                    px[3] = matrix[12] * r + matrix[13] * g + matrix[14] * b + matrix[15] * a;
                }
            });
        Ok(())
    }
    
    fn apply_cdl(&self, img: &mut GpuImage, cdl: &Cdl) -> GpuResult<()> {
        use rayon::prelude::*;
        let c = img.channels as usize;
        
        img.data.par_chunks_mut(c)
            .for_each(|px| {
                let mut r = (px[0] * cdl.slope[0] + cdl.offset[0]).max(0.0).powf(cdl.power[0]);
                let mut g = (px[1] * cdl.slope[1] + cdl.offset[1]).max(0.0).powf(cdl.power[1]);
                let mut b = (px[2] * cdl.slope[2] + cdl.offset[2]).max(0.0).powf(cdl.power[2]);
                
                if cdl.saturation != 1.0 {
                    let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                    r = luma + cdl.saturation * (r - luma);
                    g = luma + cdl.saturation * (g - luma);
                    b = luma + cdl.saturation * (b - luma);
                }
                
                px[0] = r;
                px[1] = g;
                px[2] = b;
            });
        Ok(())
    }
    
    fn apply_lut1d(&self, img: &mut GpuImage, lut: &[f32], lut_ch: u32) -> GpuResult<()> {
        use rayon::prelude::*;
        let c = img.channels as usize;
        let lut_size = lut.len() / (lut_ch as usize);
        let scale = (lut_size - 1) as f32;
        
        img.data.par_chunks_mut(c)
            .for_each(|px| {
                for ch in 0..c.min(lut_ch as usize) {
                    let v = px[ch].clamp(0.0, 1.0) * scale;
                    let i0 = (v as usize).min(lut_size - 1);
                    let i1 = (i0 + 1).min(lut_size - 1);
                    let f = v - i0 as f32;
                    
                    let v0 = lut[i0 * (lut_ch as usize) + ch];
                    let v1 = lut[i1 * (lut_ch as usize) + ch];
                    px[ch] = v0 + f * (v1 - v0);
                }
            });
        Ok(())
    }
    
    fn apply_lut3d(&self, img: &mut GpuImage, lut: &[f32], size: u32) -> GpuResult<()> {
        use rayon::prelude::*;
        let c = img.channels as usize;
        let s = size as usize;
        let scale = (s - 1) as f32;
        
        img.data.par_chunks_mut(c)
            .for_each(|px| {
                let r = px[0].clamp(0.0, 1.0) * scale;
                let g = px[1].clamp(0.0, 1.0) * scale;
                let b = px[2].clamp(0.0, 1.0) * scale;
                
                let r0 = (r as usize).min(s - 1);
                let g0 = (g as usize).min(s - 1);
                let b0 = (b as usize).min(s - 1);
                let r1 = (r0 + 1).min(s - 1);
                let g1 = (g0 + 1).min(s - 1);
                let b1 = (b0 + 1).min(s - 1);
                
                let fr = r - r0 as f32;
                let fg = g - g0 as f32;
                let fb = b - b0 as f32;
                
                let idx = |ri: usize, gi: usize, bi: usize, ch: usize| -> f32 {
                    lut[(bi * s * s + gi * s + ri) * 3 + ch]
                };
                
                for ch in 0..3 {
                    let c000 = idx(r0, g0, b0, ch);
                    let c100 = idx(r1, g0, b0, ch);
                    let c010 = idx(r0, g1, b0, ch);
                    let c110 = idx(r1, g1, b0, ch);
                    let c001 = idx(r0, g0, b1, ch);
                    let c101 = idx(r1, g0, b1, ch);
                    let c011 = idx(r0, g1, b1, ch);
                    let c111 = idx(r1, g1, b1, ch);
                    
                    let c00 = c000 + fr * (c100 - c000);
                    let c10 = c010 + fr * (c110 - c010);
                    let c01 = c001 + fr * (c101 - c001);
                    let c11 = c011 + fr * (c111 - c011);
                    
                    let c0 = c00 + fg * (c10 - c00);
                    let c1 = c01 + fg * (c11 - c01);
                    
                    px[ch] = c0 + fb * (c1 - c0);
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

impl ColorProcessor {
    /// Create with specified backend.
    pub fn new(backend: Backend) -> GpuResult<Self> {
        match backend {
            Backend::Auto | Backend::Cpu => {
                Ok(Self {
                    primitives: Box::new(CpuColorPrimitives {
                        inner: CpuPrimitives::new(),
                    }),
                })
            }
            Backend::Wgpu => {
                #[cfg(feature = "wgpu")]
                {
                    // TODO: implement WgpuColorPrimitives
                    Ok(Self {
                        primitives: Box::new(CpuColorPrimitives {
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
    
    /// Apply 4x4 color matrix.
    pub fn apply_matrix(&self, img: &mut GpuImage, matrix: &[f32; 16]) -> GpuResult<()> {
        self.primitives.apply_matrix(img, matrix)
    }
    
    /// Apply CDL transform.
    pub fn apply_cdl(&self, img: &mut GpuImage, cdl: &Cdl) -> GpuResult<()> {
        self.primitives.apply_cdl(img, cdl)
    }
    
    /// Apply 1D LUT.
    pub fn apply_lut1d(&self, img: &mut GpuImage, lut: &[f32], channels: u32) -> GpuResult<()> {
        self.primitives.apply_lut1d(img, lut, channels)
    }
    
    /// Apply 3D LUT.
    pub fn apply_lut3d(&self, img: &mut GpuImage, lut: &[f32], size: u32) -> GpuResult<()> {
        self.primitives.apply_lut3d(img, lut, size)
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
