//! CPU backend using rayon for parallelization.

use rayon::prelude::*;

use super::{GpuLimits, ProcessingBackend, BlendMode};
use super::gpu_primitives::{ImageHandle, GpuPrimitives, AsAny};
use crate::{ComputeError, ComputeResult};

/// CPU image handle - data stored in RAM.
pub struct CpuImage {
    data: Vec<f32>,
    width: u32,
    height: u32,
    channels: u32,
}

impl CpuImage {
    pub fn new(data: Vec<f32>, width: u32, height: u32, channels: u32) -> Self {
        Self { data, width, height, channels }
    }

    pub fn data(&self) -> &[f32] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [f32] {
        &mut self.data
    }
}

impl AsAny for CpuImage {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl ImageHandle for CpuImage {
    fn dimensions(&self) -> (u32, u32, u32) {
        (self.width, self.height, self.channels)
    }
}

/// CPU primitives implementation.
pub struct CpuPrimitives {
    limits: GpuLimits,
}

impl CpuPrimitives {
    pub fn new() -> Self {
        // Get system RAM (fallback to 4GB if detection fails)
        let available = sys_info::mem_info()
            .map(|m| m.avail * 1024)
            .unwrap_or(4 * 1024 * 1024 * 1024);

        Self {
            limits: GpuLimits {
                max_tile_dim: u32::MAX,
                max_buffer_bytes: u64::MAX,
                total_memory: available,
                available_memory: available,
                detected: true,
            },
        }
    }
}

impl Default for CpuPrimitives {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuPrimitives for CpuPrimitives {
    type Handle = CpuImage;

    fn upload(&self, data: &[f32], width: u32, height: u32, channels: u32) -> ComputeResult<Self::Handle> {
        Ok(CpuImage::new(data.to_vec(), width, height, channels))
    }

    fn download(&self, handle: &Self::Handle) -> ComputeResult<Vec<f32>> {
        Ok(handle.data.clone())
    }

    fn allocate(&self, width: u32, height: u32, channels: u32) -> ComputeResult<Self::Handle> {
        let size = (width as usize) * (height as usize) * (channels as usize);
        Ok(CpuImage::new(vec![0.0; size], width, height, channels))
    }

    fn exec_matrix(&self, src: &Self::Handle, dst: &mut Self::Handle, matrix: &[f32; 16]) -> ComputeResult<()> {
        let c = src.channels as usize;

        dst.data.par_chunks_mut(c)
            .zip(src.data.par_chunks(c))
            .for_each(|(out, inp)| {
                let r = inp.get(0).copied().unwrap_or(0.0);
                let g = inp.get(1).copied().unwrap_or(0.0);
                let b = inp.get(2).copied().unwrap_or(0.0);
                let a = inp.get(3).copied().unwrap_or(1.0);

                out[0] = matrix[0] * r + matrix[1] * g + matrix[2] * b + matrix[3] * a;
                out[1] = matrix[4] * r + matrix[5] * g + matrix[6] * b + matrix[7] * a;
                out[2] = matrix[8] * r + matrix[9] * g + matrix[10] * b + matrix[11] * a;
                if c >= 4 {
                    out[3] = matrix[12] * r + matrix[13] * g + matrix[14] * b + matrix[15] * a;
                }
            });

        Ok(())
    }

    fn exec_cdl(&self, src: &Self::Handle, dst: &mut Self::Handle,
                slope: [f32; 3], offset: [f32; 3], power: [f32; 3], sat: f32) -> ComputeResult<()> {
        let c = src.channels as usize;

        dst.data.par_chunks_mut(c)
            .zip(src.data.par_chunks(c))
            .for_each(|(out, inp)| {
                let mut r = (inp[0] * slope[0] + offset[0]).max(0.0).powf(power[0]);
                let mut g = (inp[1] * slope[1] + offset[1]).max(0.0).powf(power[1]);
                let mut b = (inp[2] * slope[2] + offset[2]).max(0.0).powf(power[2]);

                if sat != 1.0 {
                    let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                    r = luma + sat * (r - luma);
                    g = luma + sat * (g - luma);
                    b = luma + sat * (b - luma);
                }

                out[0] = r;
                out[1] = g;
                out[2] = b;
                if c >= 4 {
                    out[3] = inp.get(3).copied().unwrap_or(1.0);
                }
            });

        Ok(())
    }

    fn exec_lut1d(&self, src: &Self::Handle, dst: &mut Self::Handle,
                  lut: &[f32], lut_channels: u32) -> ComputeResult<()> {
        let c = src.channels as usize;
        let lut_size = lut.len() / (lut_channels as usize);
        let scale = (lut_size - 1) as f32;

        dst.data.par_chunks_mut(c)
            .zip(src.data.par_chunks(c))
            .for_each(|(out, inp)| {
                for ch in 0..(c).min(lut_channels as usize) {
                    let v = inp[ch].clamp(0.0, 1.0) * scale;
                    let i0 = (v as usize).min(lut_size - 1);
                    let i1 = (i0 + 1).min(lut_size - 1);
                    let f = v - i0 as f32;

                    let v0 = lut[i0 * (lut_channels as usize) + ch];
                    let v1 = lut[i1 * (lut_channels as usize) + ch];
                    out[ch] = v0 + f * (v1 - v0);
                }
                if c >= 4 && lut_channels < 4 {
                    out[3] = inp.get(3).copied().unwrap_or(1.0);
                }
            });

        Ok(())
    }

    fn exec_lut3d(&self, src: &Self::Handle, dst: &mut Self::Handle,
                  lut: &[f32], size: u32) -> ComputeResult<()> {
        let c = src.channels as usize;
        let s = size as usize;
        let scale = (s - 1) as f32;

        dst.data.par_chunks_mut(c)
            .zip(src.data.par_chunks(c))
            .for_each(|(out, inp)| {
                let r = inp[0].clamp(0.0, 1.0) * scale;
                let g = inp[1].clamp(0.0, 1.0) * scale;
                let b = inp[2].clamp(0.0, 1.0) * scale;

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

                    out[ch] = c0 + fb * (c1 - c0);
                }

                if c >= 4 {
                    out[3] = inp.get(3).copied().unwrap_or(1.0);
                }
            });

        Ok(())
    }

    fn exec_resize(&self, src: &Self::Handle, dst: &mut Self::Handle, _filter: u32) -> ComputeResult<()> {
        let (sw, sh, c) = src.dimensions();
        let (dw, dh, _) = dst.dimensions();

        let sx = sw as f32 / dw as f32;
        let sy = sh as f32 / dh as f32;

        dst.data.par_chunks_mut((dw * c) as usize)
            .enumerate()
            .for_each(|(dy, row)| {
                for dx in 0..dw as usize {
                    let fx = dx as f32 * sx;
                    let fy = dy as f32 * sy;

                    // Bilinear interpolation
                    let x0 = (fx as usize).min(sw as usize - 1);
                    let y0 = (fy as usize).min(sh as usize - 1);
                    let x1 = (x0 + 1).min(sw as usize - 1);
                    let y1 = (y0 + 1).min(sh as usize - 1);

                    let fx = fx - x0 as f32;
                    let fy = fy - y0 as f32;

                    for ch in 0..c as usize {
                        let idx = |x: usize, y: usize| -> f32 {
                            src.data[(y * sw as usize + x) * c as usize + ch]
                        };

                        let c00 = idx(x0, y0);
                        let c10 = idx(x1, y0);
                        let c01 = idx(x0, y1);
                        let c11 = idx(x1, y1);

                        let top = c00 + fx * (c10 - c00);
                        let bot = c01 + fx * (c11 - c01);

                        row[dx * c as usize + ch] = top + fy * (bot - top);
                    }
                }
            });

        Ok(())
    }

    fn exec_blur(&self, src: &Self::Handle, dst: &mut Self::Handle, radius: f32) -> ComputeResult<()> {
        let (w, h, c) = src.dimensions();
        let r = radius.ceil() as i32;
        let sigma = radius / 3.0;

        // Generate Gaussian kernel
        let kernel_size = (r * 2 + 1) as usize;
        let mut kernel = vec![0.0f32; kernel_size];
        let mut sum = 0.0;
        for i in 0..kernel_size {
            let x = (i as i32 - r) as f32;
            let g = (-x * x / (2.0 * sigma * sigma)).exp();
            kernel[i] = g;
            sum += g;
        }
        for k in &mut kernel {
            *k /= sum;
        }

        // Horizontal pass
        let mut temp = vec![0.0f32; src.data.len()];
        temp.par_chunks_mut((w * c) as usize)
            .enumerate()
            .for_each(|(y, row)| {
                for x in 0..w as i32 {
                    for ch in 0..c as usize {
                        let mut acc = 0.0;
                        for ki in 0..kernel_size {
                            let sx = (x + ki as i32 - r).clamp(0, w as i32 - 1) as usize;
                            acc += src.data[(y * w as usize + sx) * c as usize + ch] * kernel[ki];
                        }
                        row[x as usize * c as usize + ch] = acc;
                    }
                }
            });

        // Vertical pass
        dst.data.par_chunks_mut((w * c) as usize)
            .enumerate()
            .for_each(|(y, row)| {
                for x in 0..w as usize {
                    for ch in 0..c as usize {
                        let mut acc = 0.0;
                        for ki in 0..kernel_size {
                            let sy = (y as i32 + ki as i32 - r).clamp(0, h as i32 - 1) as usize;
                            acc += temp[(sy * w as usize + x) * c as usize + ch] * kernel[ki];
                        }
                        row[x * c as usize + ch] = acc;
                    }
                }
            });

        Ok(())
    }

    fn limits(&self) -> &GpuLimits {
        &self.limits
    }

    fn name(&self) -> &'static str {
        "CPU"
    }
}

/// CPU backend wrapper.
pub struct CpuBackend {
    primitives: CpuPrimitives,
}

impl CpuBackend {
    pub fn new() -> Self {
        Self {
            primitives: CpuPrimitives::new(),
        }
    }

    /// Get inner primitives.
    pub fn primitives(&self) -> &CpuPrimitives {
        &self.primitives
    }
}

impl Default for CpuBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessingBackend for CpuBackend {
    fn name(&self) -> &'static str {
        "CPU"
    }

    fn available_memory(&self) -> u64 {
        self.primitives.limits.available_memory
    }

    fn limits(&self) -> &GpuLimits {
        &self.primitives.limits
    }

    fn upload(&self, data: &[f32], width: u32, height: u32, channels: u32) -> ComputeResult<Box<dyn ImageHandle>> {
        let handle = self.primitives.upload(data, width, height, channels)?;
        Ok(Box::new(handle))
    }

    fn download(&self, handle: &dyn ImageHandle) -> ComputeResult<Vec<f32>> {
        let cpu_handle = handle.as_any().downcast_ref::<CpuImage>()
            .ok_or_else(|| ComputeError::OperationFailed("Invalid handle type".into()))?;
        self.primitives.download(cpu_handle)
    }

    fn apply_matrix(&self, handle: &mut dyn ImageHandle, matrix: &[f32; 16]) -> ComputeResult<()> {
        let cpu_handle = handle.as_any_mut().downcast_mut::<CpuImage>()
            .ok_or_else(|| ComputeError::OperationFailed("Invalid handle type".into()))?;

        let mut dst = self.primitives.allocate(cpu_handle.width, cpu_handle.height, cpu_handle.channels)?;
        self.primitives.exec_matrix(cpu_handle, &mut dst, matrix)?;
        *cpu_handle = dst;
        Ok(())
    }

    fn apply_cdl(&self, handle: &mut dyn ImageHandle, slope: [f32; 3], offset: [f32; 3], power: [f32; 3], sat: f32) -> ComputeResult<()> {
        let cpu_handle = handle.as_any_mut().downcast_mut::<CpuImage>()
            .ok_or_else(|| ComputeError::OperationFailed("Invalid handle type".into()))?;

        let mut dst = self.primitives.allocate(cpu_handle.width, cpu_handle.height, cpu_handle.channels)?;
        self.primitives.exec_cdl(cpu_handle, &mut dst, slope, offset, power, sat)?;
        *cpu_handle = dst;
        Ok(())
    }

    fn apply_lut1d(&self, handle: &mut dyn ImageHandle, lut: &[f32], channels: u32) -> ComputeResult<()> {
        let cpu_handle = handle.as_any_mut().downcast_mut::<CpuImage>()
            .ok_or_else(|| ComputeError::OperationFailed("Invalid handle type".into()))?;

        let mut dst = self.primitives.allocate(cpu_handle.width, cpu_handle.height, cpu_handle.channels)?;
        self.primitives.exec_lut1d(cpu_handle, &mut dst, lut, channels)?;
        *cpu_handle = dst;
        Ok(())
    }

    fn apply_lut3d(&self, handle: &mut dyn ImageHandle, lut: &[f32], size: u32) -> ComputeResult<()> {
        let cpu_handle = handle.as_any_mut().downcast_mut::<CpuImage>()
            .ok_or_else(|| ComputeError::OperationFailed("Invalid handle type".into()))?;

        let mut dst = self.primitives.allocate(cpu_handle.width, cpu_handle.height, cpu_handle.channels)?;
        self.primitives.exec_lut3d(cpu_handle, &mut dst, lut, size)?;
        *cpu_handle = dst;
        Ok(())
    }

    fn resize(&self, handle: &dyn ImageHandle, width: u32, height: u32, filter: u32) -> ComputeResult<Box<dyn ImageHandle>> {
        let cpu_handle = handle.as_any().downcast_ref::<CpuImage>()
            .ok_or_else(|| ComputeError::OperationFailed("Invalid handle type".into()))?;

        let mut dst = self.primitives.allocate(width, height, cpu_handle.channels)?;
        self.primitives.exec_resize(cpu_handle, &mut dst, filter)?;
        Ok(Box::new(dst))
    }

    fn blur(&self, handle: &mut dyn ImageHandle, radius: f32) -> ComputeResult<()> {
        let cpu_handle = handle.as_any_mut().downcast_mut::<CpuImage>()
            .ok_or_else(|| ComputeError::OperationFailed("Invalid handle type".into()))?;

        let mut dst = self.primitives.allocate(cpu_handle.width, cpu_handle.height, cpu_handle.channels)?;
        self.primitives.exec_blur(cpu_handle, &mut dst, radius)?;
        *cpu_handle = dst;
        Ok(())
    }

    fn composite_over(&self, fg: &dyn ImageHandle, bg: &mut dyn ImageHandle) -> ComputeResult<()> {
        let fg_img = fg.as_any().downcast_ref::<CpuImage>()
            .ok_or_else(|| ComputeError::OperationFailed("Invalid fg handle".into()))?;
        let bg_img = bg.as_any_mut().downcast_mut::<CpuImage>()
            .ok_or_else(|| ComputeError::OperationFailed("Invalid bg handle".into()))?;

        let (fw, fh, fc) = fg_img.dimensions();
        let (bw, bh, bc) = bg_img.dimensions();
        if fw != bw || fh != bh {
            return Err(ComputeError::OperationFailed("Dimension mismatch".into()));
        }

        // Porter-Duff Over: Fg + Bg * (1 - Fg.a)
        bg_img.data.par_chunks_mut(bc as usize)
            .zip(fg_img.data.par_chunks(fc as usize))
            .for_each(|(bg_px, fg_px)| {
                let fg_a = fg_px.get(3).copied().unwrap_or(1.0);
                let bg_a = bg_px.get(3).copied().unwrap_or(1.0);
                let inv_fg_a = 1.0 - fg_a;

                bg_px[0] = fg_px[0] * fg_a + bg_px[0] * bg_a * inv_fg_a;
                bg_px[1] = fg_px[1] * fg_a + bg_px[1] * bg_a * inv_fg_a;
                bg_px[2] = fg_px[2] * fg_a + bg_px[2] * bg_a * inv_fg_a;
                if bc >= 4 {
                    bg_px[3] = fg_a + bg_a * inv_fg_a;
                }
            });
        Ok(())
    }

    fn blend(&self, fg: &dyn ImageHandle, bg: &mut dyn ImageHandle, mode: BlendMode, opacity: f32) -> ComputeResult<()> {
        let fg_img = fg.as_any().downcast_ref::<CpuImage>()
            .ok_or_else(|| ComputeError::OperationFailed("Invalid fg handle".into()))?;
        let bg_img = bg.as_any_mut().downcast_mut::<CpuImage>()
            .ok_or_else(|| ComputeError::OperationFailed("Invalid bg handle".into()))?;

        let (fw, fh, fc) = fg_img.dimensions();
        let (bw, bh, bc) = bg_img.dimensions();
        if fw != bw || fh != bh {
            return Err(ComputeError::OperationFailed("Dimension mismatch".into()));
        }

        bg_img.data.par_chunks_mut(bc as usize)
            .zip(fg_img.data.par_chunks(fc as usize))
            .for_each(|(bg_px, fg_px)| {
                for ch in 0..3.min(bc as usize) {
                    let a = fg_px[ch];
                    let b = bg_px[ch];
                    let blended = match mode {
                        BlendMode::Normal => a,
                        BlendMode::Multiply => a * b,
                        BlendMode::Screen => 1.0 - (1.0 - a) * (1.0 - b),
                        BlendMode::Add => (a + b).min(1.0),
                        BlendMode::Subtract => (b - a).max(0.0),
                        BlendMode::Overlay => {
                            if b < 0.5 { 2.0 * a * b } else { 1.0 - 2.0 * (1.0 - a) * (1.0 - b) }
                        }
                        BlendMode::SoftLight => {
                            if a < 0.5 { b - (1.0 - 2.0 * a) * b * (1.0 - b) }
                            else { b + (2.0 * a - 1.0) * (((b - 0.5).abs() * 16.0 + 12.0) * b - 3.0) * b }
                        }
                        BlendMode::HardLight => {
                            if a < 0.5 { 2.0 * a * b } else { 1.0 - 2.0 * (1.0 - a) * (1.0 - b) }
                        }
                        BlendMode::Difference => (a - b).abs(),
                    };
                    bg_px[ch] = b + opacity * (blended - b);
                }
            });
        Ok(())
    }

    fn crop(&self, handle: &dyn ImageHandle, x: u32, y: u32, w: u32, h: u32) -> ComputeResult<Box<dyn ImageHandle>> {
        let src = handle.as_any().downcast_ref::<CpuImage>()
            .ok_or_else(|| ComputeError::OperationFailed("Invalid handle".into()))?;
        let (sw, sh, c) = src.dimensions();

        if x + w > sw || y + h > sh {
            return Err(ComputeError::OperationFailed("Crop region out of bounds".into()));
        }

        let mut dst_data = Vec::with_capacity((w * h * c) as usize);
        for row in y..(y + h) {
            let start = ((row * sw + x) * c) as usize;
            let end = start + (w * c) as usize;
            dst_data.extend_from_slice(&src.data[start..end]);
        }
        Ok(Box::new(CpuImage::new(dst_data, w, h, c)))
    }

    fn flip_h(&self, handle: &mut dyn ImageHandle) -> ComputeResult<()> {
        let img = handle.as_any_mut().downcast_mut::<CpuImage>()
            .ok_or_else(|| ComputeError::OperationFailed("Invalid handle".into()))?;
        let (w, _h, c) = img.dimensions();
        let row_size = (w * c) as usize;

        img.data.par_chunks_mut(row_size).for_each(|row| {
            for x in 0..(w / 2) as usize {
                for ch in 0..c as usize {
                    let left = x * c as usize + ch;
                    let right = (w as usize - 1 - x) * c as usize + ch;
                    row.swap(left, right);
                }
            }
        });
        Ok(())
    }

    fn flip_v(&self, handle: &mut dyn ImageHandle) -> ComputeResult<()> {
        let img = handle.as_any_mut().downcast_mut::<CpuImage>()
            .ok_or_else(|| ComputeError::OperationFailed("Invalid handle".into()))?;
        let (w, h, c) = img.dimensions();
        let row_size = (w * c) as usize;

        for y in 0..(h / 2) as usize {
            let top_start = y * row_size;
            let bot_start = (h as usize - 1 - y) * row_size;
            for i in 0..row_size {
                img.data.swap(top_start + i, bot_start + i);
            }
        }
        Ok(())
    }

    fn rotate_90(&self, handle: &dyn ImageHandle, n: u32) -> ComputeResult<Box<dyn ImageHandle>> {
        let src = handle.as_any().downcast_ref::<CpuImage>()
            .ok_or_else(|| ComputeError::OperationFailed("Invalid handle".into()))?;
        let (w, h, c) = src.dimensions();
        let n = n % 4;

        if n == 0 {
            return Ok(Box::new(CpuImage::new(src.data.clone(), w, h, c)));
        }

        let (nw, nh) = if n % 2 == 1 { (h, w) } else { (w, h) };
        let mut dst = vec![0.0f32; (nw * nh * c) as usize];

        for sy in 0..h {
            for sx in 0..w {
                let (dx, dy) = match n {
                    1 => (h - 1 - sy, sx),      // 90° CW
                    2 => (w - 1 - sx, h - 1 - sy), // 180°
                    3 => (sy, w - 1 - sx),      // 270° CW
                    _ => unreachable!(),
                };
                let src_idx = ((sy * w + sx) * c) as usize;
                let dst_idx = ((dy * nw + dx) * c) as usize;
                for ch in 0..c as usize {
                    dst[dst_idx + ch] = src.data[src_idx + ch];
                }
            }
        }
        Ok(Box::new(CpuImage::new(dst, nw, nh, c)))
    }
}
