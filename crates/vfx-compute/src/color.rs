//! GPU-accelerated color processing.
//!
//! This module provides color grading and correction operations:
//!
//! - **Matrix transforms** - 4x4 color matrix for color space conversions
//! - **Exposure** - Brightness adjustment in photographic stops
//! - **CDL** - Color Decision List (slope/offset/power/saturation)
//! - **LUT** - 1D and 3D lookup tables for color grading
//! - **Saturation** - Color saturation adjustment
//!
//! # Example
//!
//! ```ignore
//! use vfx_compute::{ColorProcessor, ComputeImage, Backend, Cdl};
//!
//! let proc = ColorProcessor::new(Backend::Auto)?;
//! let mut img = ComputeImage::from_f32(data, 1920, 1080, 3)?;
//!
//! // Apply CDL grade
//! let cdl = Cdl {
//!     slope: [1.1, 1.0, 0.9],
//!     offset: [0.01, 0.0, -0.01],
//!     power: [1.0, 1.0, 1.0],
//!     saturation: 1.2,
//! };
//! proc.apply_cdl(&mut img, &cdl)?;
//!
//! // Apply exposure (+1 stop = 2x brightness)
//! proc.apply_exposure(&mut img, 1.0)?;
//! ```

use crate::image::ComputeImage;
use crate::backend::{Backend, AnyExecutor, create_executor, ColorOp};
use crate::ComputeResult;

/// CDL (Color Decision List) parameters.
///
/// Defines a color correction using the ASC CDL standard:
/// `output = (input × slope + offset)^power × saturation`
///
/// Each channel (RGB) has independent slope, offset, and power values.
/// Saturation is applied globally after the per-channel transforms.
#[derive(Debug, Clone, Copy)]
pub struct Cdl {
    /// Multiplier per channel. Default: [1.0, 1.0, 1.0]
    pub slope: [f32; 3],
    /// Added after slope. Default: [0.0, 0.0, 0.0]
    pub offset: [f32; 3],
    /// Exponent per channel. Default: [1.0, 1.0, 1.0]
    pub power: [f32; 3],
    /// Global saturation. Default: 1.0
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

/// GPU-accelerated color processor.
///
/// Provides color grading operations using GPU compute shaders when available,
/// with automatic fallback to CPU (rayon parallel processing).
///
/// # Backend Selection
///
/// - `Backend::Auto` - Automatically selects best available (CUDA > wgpu > CPU)
/// - `Backend::Cpu` - Force CPU processing with rayon
/// - `Backend::Wgpu` - Force wgpu (Vulkan/Metal/DX12)
/// - `Backend::Cuda` - Force NVIDIA CUDA
pub struct ColorProcessor {
    executor: AnyExecutor,
}

impl ColorProcessor {
    /// Create a new color processor with the specified backend.
    ///
    /// # Arguments
    /// * `backend` - Compute backend to use (Auto, Cpu, Wgpu, or Cuda)
    ///
    /// # Errors
    /// Returns error if the requested backend is not available.
    pub fn new(backend: Backend) -> ComputeResult<Self> {
        Ok(Self {
            executor: create_executor(backend)?,
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
        self.executor.limits().available_memory
    }

    /// Apply a 4x4 color matrix transform.
    ///
    /// The matrix is applied as: `[R', G', B', A']^T = M × [R, G, B, A]^T`
    ///
    /// Matrix layout is column-major (OpenGL style):
    /// ```text
    /// [ m[0]  m[4]  m[8]   m[12] ]   [ R ]   [ R' ]
    /// [ m[1]  m[5]  m[9]   m[13] ] × [ G ] = [ G' ]
    /// [ m[2]  m[6]  m[10]  m[14] ]   [ B ]   [ B' ]
    /// [ m[3]  m[7]  m[11]  m[15] ]   [ A ]   [ A' ]
    /// ```
    pub fn apply_matrix(&self, img: &mut ComputeImage, matrix: &[f32; 16]) -> ComputeResult<()> {
        let op = ColorOp::Matrix(*matrix);
        self.executor.execute_color(img, &op)
    }

    /// Apply CDL (Color Decision List) transform.
    ///
    /// Formula: `output = clamp((input × slope + offset)^power) × saturation`
    pub fn apply_cdl(&self, img: &mut ComputeImage, cdl: &Cdl) -> ComputeResult<()> {
        let op = ColorOp::Cdl {
            slope: cdl.slope,
            offset: cdl.offset,
            power: cdl.power,
            saturation: cdl.saturation,
        };
        self.executor.execute_color(img, &op)
    }

    /// Apply 1D lookup table.
    ///
    /// # Arguments
    /// * `lut` - LUT data as interleaved RGB values `[R0, G0, B0, R1, G1, B1, ...]`
    /// * `channels` - Number of channels in LUT (typically 3)
    ///
    /// The LUT size is inferred from `lut.len() / channels`.
    /// Input values are clamped to [0, 1] before lookup.
    pub fn apply_lut1d(&self, img: &mut ComputeImage, lut: &[f32], channels: u32) -> ComputeResult<()> {
        let op = ColorOp::Lut1d {
            lut: lut.to_vec(),
            channels,
        };
        self.executor.execute_color(img, &op)
    }

    /// Apply 3D lookup table with trilinear interpolation.
    ///
    /// # Arguments
    /// * `lut` - 3D LUT data as `[R, G, B]` triplets in B-G-R order (size³ × 3 values)
    /// * `size` - Cube dimension (e.g., 33 for a 33×33×33 LUT)
    ///
    /// Input RGB values are used as 3D coordinates into the cube.
    pub fn apply_lut3d(&self, img: &mut ComputeImage, lut: &[f32], size: u32) -> ComputeResult<()> {
        let op = ColorOp::Lut3d {
            lut: lut.to_vec(),
            size,
        };
        self.executor.execute_color(img, &op)
    }

    /// Apply exposure adjustment in photographic stops.
    ///
    /// # Arguments
    /// * `stops` - Exposure change in stops. Positive = brighter, negative = darker.
    ///
    /// Each stop doubles (or halves) the brightness:
    /// - `+1.0` = 2× brighter
    /// - `-1.0` = 0.5× darker
    /// - `+2.0` = 4× brighter
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
    /// # Arguments
    /// * `sat` - Saturation multiplier. 0.0 = grayscale, 1.0 = unchanged, >1.0 = more saturated
    pub fn apply_saturation(&self, img: &mut ComputeImage, sat: f32) -> ComputeResult<()> {
        let cdl = Cdl {
            saturation: sat,
            ..Default::default()
        };
        self.apply_cdl(img, &cdl)
    }
}
