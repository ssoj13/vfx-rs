//! GPU-accelerated color processing.

use crate::image::ComputeImage;
use crate::backend::{Backend, AnyExecutor, create_executor, ColorOp};
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
    executor: AnyExecutor,
}

impl ColorProcessor {
    /// Create with specified backend.
    pub fn new(backend: Backend) -> ComputeResult<Self> {
        Ok(Self {
            executor: create_executor(backend)?,
        })
    }

    /// Backend name.
    pub fn backend_name(&self) -> &'static str {
        self.executor.name()
    }

    /// Available memory in bytes.
    pub fn available_memory(&self) -> u64 {
        self.executor.limits().available_memory
    }

    /// Apply 4x4 color matrix.
    pub fn apply_matrix(&self, img: &mut ComputeImage, matrix: &[f32; 16]) -> ComputeResult<()> {
        let op = ColorOp::Matrix(*matrix);
        self.executor.execute_color(img, &op)
    }

    /// Apply CDL transform.
    pub fn apply_cdl(&self, img: &mut ComputeImage, cdl: &Cdl) -> ComputeResult<()> {
        let op = ColorOp::Cdl {
            slope: cdl.slope,
            offset: cdl.offset,
            power: cdl.power,
            saturation: cdl.saturation,
        };
        self.executor.execute_color(img, &op)
    }

    /// Apply 1D LUT.
    pub fn apply_lut1d(&self, img: &mut ComputeImage, lut: &[f32], channels: u32) -> ComputeResult<()> {
        let op = ColorOp::Lut1d {
            lut: lut.to_vec(),
            channels,
        };
        self.executor.execute_color(img, &op)
    }

    /// Apply 3D LUT.
    pub fn apply_lut3d(&self, img: &mut ComputeImage, lut: &[f32], size: u32) -> ComputeResult<()> {
        let op = ColorOp::Lut3d {
            lut: lut.to_vec(),
            size,
        };
        self.executor.execute_color(img, &op)
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
