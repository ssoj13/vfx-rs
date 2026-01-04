//! Color processor with caching.
//!
//! The [`ColorProcessor`] provides efficient color transformations with
//! optional caching of intermediate results and optimized pipelines.
//!
//! # Features
//!
//! - Pipeline application with matrix concatenation optimization
//! - Batch processing for image data
//! - Statistics collection (optional)
//!
//! # Example
//!
//! ```rust
//! use vfx_color::{ColorProcessor, Pipeline};
//! use vfx_color::transfer::srgb;
//!
//! let mut proc = ColorProcessor::new();
//!
//! let pipeline = Pipeline::new()
//!     .transfer_in(srgb::eotf)
//!     .transfer_out(srgb::oetf);
//!
//! // Single pixel
//! let rgb = [0.5, 0.3, 0.2];
//! let result = proc.apply(&pipeline, rgb);
//!
//! // Batch processing
//! let pixels = vec![[0.5, 0.3, 0.2], [0.8, 0.6, 0.4]];
//! let results = proc.apply_batch(&pipeline, &pixels);
//! ```

use crate::{Pipeline, TransformOp};
use vfx_math::Mat3;

#[cfg(feature = "gpu")]
use vfx_compute::{Processor as GpuProcessor, ComputeImage, Backend};

/// Color processor for efficient color transformations.
///
/// Provides optimized pipeline execution with optional caching.
/// Thread-safe for parallel processing.
///
/// # Optimization
///
/// The processor can optimize pipelines by:
/// - Concatenating consecutive matrix operations
/// - Merging scale/offset operations
/// - Pre-computing transfer function LUTs
///
/// # Example
///
/// ```rust
/// use vfx_color::{ColorProcessor, Pipeline};
/// use vfx_color::primaries::{SRGB, REC2020, rgb_to_xyz_matrix, xyz_to_rgb_matrix};
///
/// let mut proc = ColorProcessor::new();
///
/// // Two matrices will be concatenated into one
/// let pipeline = Pipeline::new()
///     .matrix(rgb_to_xyz_matrix(&SRGB))
///     .matrix(xyz_to_rgb_matrix(&REC2020));
///
/// let optimized = proc.optimize(&pipeline);
/// assert_eq!(optimized.len(), 1); // Single matrix
/// ```
/// Color processor for efficient color transformations.
#[derive(Default)]
pub struct ColorProcessor {
    /// Enable pipeline optimization.
    optimize: bool,
    /// Collect processing statistics.
    collect_stats: bool,
    /// Number of pixels processed.
    pixels_processed: u64,
    /// GPU processor (when gpu feature enabled).
    #[cfg(feature = "gpu")]
    gpu: Option<GpuProcessor>,
}

impl std::fmt::Debug for ColorProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("ColorProcessor");
        d.field("optimize", &self.optimize)
            .field("collect_stats", &self.collect_stats)
            .field("pixels_processed", &self.pixels_processed);
        #[cfg(feature = "gpu")]
        d.field("has_gpu", &self.gpu.is_some());
        d.finish()
    }
}

impl ColorProcessor {
    /// Creates a new color processor.
    pub fn new() -> Self {
        Self {
            optimize: true,
            collect_stats: false,
            pixels_processed: 0,
            #[cfg(feature = "gpu")]
            gpu: None,
        }
    }

    /// Creates a color processor with GPU acceleration.
    ///
    /// Uses GPU for supported operations (matrix, LUT), falls back to CPU for others.
    #[cfg(feature = "gpu")]
    pub fn with_gpu() -> Self {
        Self {
            optimize: true,
            collect_stats: false,
            pixels_processed: 0,
            gpu: GpuProcessor::auto().ok(),
        }
    }

    /// Creates a color processor with specified backend.
    #[cfg(feature = "gpu")]
    pub fn with_backend(backend: Backend) -> Self {
        Self {
            optimize: true,
            collect_stats: false,
            pixels_processed: 0,
            gpu: GpuProcessor::new(backend).ok(),
        }
    }

    /// Returns true if GPU acceleration is available.
    #[cfg(feature = "gpu")]
    pub fn has_gpu(&self) -> bool {
        self.gpu.is_some()
    }

    /// Returns the GPU backend name if available.
    #[cfg(feature = "gpu")]
    pub fn gpu_backend(&self) -> Option<&'static str> {
        self.gpu.as_ref().map(|g| g.backend_name())
    }

    /// Enables or disables pipeline optimization.
    ///
    /// When enabled, consecutive matrix operations are concatenated,
    /// and scale/offset operations are merged.
    pub fn with_optimization(mut self, enable: bool) -> Self {
        self.optimize = enable;
        self
    }

    /// Enables or disables statistics collection.
    pub fn with_stats(mut self, enable: bool) -> Self {
        self.collect_stats = enable;
        self
    }

    /// Returns the number of pixels processed.
    pub fn pixels_processed(&self) -> u64 {
        self.pixels_processed
    }

    /// Resets the statistics.
    pub fn reset_stats(&mut self) {
        self.pixels_processed = 0;
    }

    /// Applies a pipeline to a single RGB value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_color::{ColorProcessor, Pipeline};
    /// use vfx_color::transfer::srgb;
    ///
    /// let mut proc = ColorProcessor::new();
    /// let pipeline = Pipeline::new()
    ///     .transfer_in(srgb::eotf)
    ///     .transfer_out(srgb::oetf);
    ///
    /// let result = proc.apply(&pipeline, [0.5, 0.3, 0.2]);
    /// ```
    pub fn apply(&mut self, pipeline: &Pipeline, rgb: [f32; 3]) -> [f32; 3] {
        if self.collect_stats {
            self.pixels_processed += 1;
        }
        pipeline.apply(rgb)
    }

    /// Applies a pipeline to a batch of RGB values.
    ///
    /// More efficient than calling `apply` repeatedly.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_color::{ColorProcessor, Pipeline};
    /// use vfx_color::transfer::gamma;
    ///
    /// let mut proc = ColorProcessor::new();
    /// let pipeline = Pipeline::new()
    ///     .transfer_in(|v| gamma::gamma_eotf(v, 2.2));
    ///
    /// let pixels = vec![
    ///     [0.5, 0.3, 0.2],
    ///     [0.8, 0.6, 0.4],
    ///     [1.0, 1.0, 1.0],
    /// ];
    /// let results = proc.apply_batch(&pipeline, &pixels);
    /// assert_eq!(results.len(), 3);
    /// ```
    pub fn apply_batch(&mut self, pipeline: &Pipeline, pixels: &[[f32; 3]]) -> Vec<[f32; 3]> {
        if self.collect_stats {
            self.pixels_processed += pixels.len() as u64;
        }
        pixels.iter().map(|&rgb| pipeline.apply(rgb)).collect()
    }

    /// Applies a pipeline to a mutable slice of RGB values in-place.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_color::{ColorProcessor, Pipeline};
    ///
    /// let mut proc = ColorProcessor::new();
    /// let pipeline = Pipeline::new()
    ///     .scale([2.0, 2.0, 2.0])
    ///     .clamp_01();
    ///
    /// let mut pixels = vec![[0.3, 0.4, 0.5], [0.6, 0.7, 0.8]];
    /// proc.apply_in_place(&pipeline, &mut pixels);
    /// ```
    pub fn apply_in_place(&mut self, pipeline: &Pipeline, pixels: &mut [[f32; 3]]) {
        if self.collect_stats {
            self.pixels_processed += pixels.len() as u64;
        }
        for pixel in pixels.iter_mut() {
            *pixel = pipeline.apply(*pixel);
        }
    }

    /// Applies a pipeline to a flat f32 buffer (RGB interleaved).
    ///
    /// When GPU feature is enabled and GPU is available, uses GPU acceleration
    /// for supported operations (Matrix, Lut3D). Falls back to CPU for others.
    ///
    /// # Arguments
    /// * `pipeline` - The color pipeline to apply
    /// * `data` - Flat f32 buffer (RGB interleaved: [R,G,B,R,G,B,...])
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    ///
    /// # Example
    ///
    /// ```ignore
    /// use vfx_color::{ColorProcessor, Pipeline};
    ///
    /// let mut proc = ColorProcessor::with_gpu();
    /// let pipeline = Pipeline::new().scale([2.0, 2.0, 2.0]);
    ///
    /// let mut buffer = vec![0.5f32; 1920 * 1080 * 3];
    /// proc.apply_buffer(&pipeline, &mut buffer, 1920, 1080);
    /// ```
    #[cfg(feature = "gpu")]
    pub fn apply_buffer(&mut self, pipeline: &Pipeline, data: &mut [f32], width: u32, height: u32) {
        let pixel_count = (width * height) as usize;
        if self.collect_stats {
            self.pixels_processed += pixel_count as u64;
        }

        // Try GPU for supported ops, fallback to CPU
        if let Some(ref gpu) = self.gpu {
            for op in pipeline.ops() {
                match op {
                    TransformOp::Matrix(m) => {
                        // Convert Mat3 to 4x4 matrix for GPU
                        let m4x4 = mat3_to_mat4x4(m);
                        if let Ok(mut img) = ComputeImage::from_f32(data.to_vec(), width, height, 3) {
                            if gpu.apply_matrix(&mut img, &m4x4).is_ok() {
                                data.copy_from_slice(img.data());
                                continue;
                            }
                        }
                        // Fallback to CPU
                        apply_matrix_cpu(data, m);
                    }
                    TransformOp::Lut3D(lut) => {
                        // GPU 3D LUT - flatten Vec<[f32; 3]> to &[f32]
                        let flat_lut: Vec<f32> = lut.data.iter().flat_map(|c| c.iter().copied()).collect();
                        if let Ok(mut img) = ComputeImage::from_f32(data.to_vec(), width, height, 3) {
                            if gpu.apply_lut3d(&mut img, &flat_lut, lut.size as u32).is_ok() {
                                data.copy_from_slice(img.data());
                                continue;
                            }
                        }
                        // Fallback to CPU
                        apply_lut3d_cpu(data, lut);
                    }
                    TransformOp::Scale(s) => {
                        // GPU scale via matrix
                        let m4x4 = [
                            s[0], 0.0, 0.0, 0.0,
                            0.0, s[1], 0.0, 0.0,
                            0.0, 0.0, s[2], 0.0,
                            0.0, 0.0, 0.0, 1.0,
                        ];
                        if let Ok(mut img) = ComputeImage::from_f32(data.to_vec(), width, height, 3) {
                            if gpu.apply_matrix(&mut img, &m4x4).is_ok() {
                                data.copy_from_slice(img.data());
                                continue;
                            }
                        }
                        apply_scale_cpu(data, s);
                    }
                    // CPU-only operations
                    _ => apply_op_cpu(data, op),
                }
            }
        } else {
            // No GPU, use CPU for everything
            for op in pipeline.ops() {
                apply_op_cpu(data, op);
            }
        }
    }

    /// Applies a pipeline to a flat f32 buffer (CPU only).
    #[cfg(not(feature = "gpu"))]
    pub fn apply_buffer(&mut self, pipeline: &Pipeline, data: &mut [f32], width: u32, height: u32) {
        let pixel_count = (width * height) as usize;
        if self.collect_stats {
            self.pixels_processed += pixel_count as u64;
        }
        for op in pipeline.ops() {
            apply_op_cpu(data, op);
        }
    }

    /// Optimizes a pipeline by merging operations.
    ///
    /// Optimizations performed:
    /// - Consecutive matrices are multiplied together
    /// - Consecutive scales are multiplied
    /// - Consecutive offsets are added
    /// - Scale followed by offset is preserved (ASC-CDL order)
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_color::{ColorProcessor, Pipeline};
    /// use vfx_color::primaries::{SRGB, ACES_AP1, rgb_to_xyz_matrix, xyz_to_rgb_matrix};
    ///
    /// let proc = ColorProcessor::new();
    ///
    /// // Three matrices -> one matrix
    /// let pipeline = Pipeline::new()
    ///     .matrix(rgb_to_xyz_matrix(&SRGB))
    ///     .matrix(xyz_to_rgb_matrix(&ACES_AP1))
    ///     .matrix(rgb_to_xyz_matrix(&ACES_AP1));
    ///
    /// let optimized = proc.optimize(&pipeline);
    /// assert_eq!(optimized.len(), 1);
    /// ```
    pub fn optimize(&self, pipeline: &Pipeline) -> Pipeline {
        if !self.optimize || pipeline.is_empty() {
            return pipeline.clone();
        }

        let mut result = Pipeline::with_capacity(pipeline.len());
        let mut pending_matrix: Option<Mat3> = None;
        let mut pending_scale: Option<[f32; 3]> = None;
        let mut pending_offset: Option<[f32; 3]> = None;

        for op in pipeline.ops() {
            match op {
                TransformOp::Matrix(m) => {
                    // Flush pending scale/offset
                    if let Some(s) = pending_scale.take() {
                        result = result.scale(s);
                    }
                    if let Some(o) = pending_offset.take() {
                        result = result.offset(o);
                    }
                    
                    // Accumulate matrices
                    pending_matrix = Some(match pending_matrix {
                        Some(prev) => *m * prev,
                        None => *m,
                    });
                }
                TransformOp::Scale(s) => {
                    // Flush pending matrix
                    if let Some(m) = pending_matrix.take() {
                        result = result.matrix(m);
                    }
                    
                    // Accumulate scales
                    pending_scale = Some(match pending_scale {
                        Some(prev) => [prev[0] * s[0], prev[1] * s[1], prev[2] * s[2]],
                        None => *s,
                    });
                }
                TransformOp::Offset(o) => {
                    // Flush pending matrix
                    if let Some(m) = pending_matrix.take() {
                        result = result.matrix(m);
                    }
                    
                    // Flush pending scale first (scale before offset = CDL order)
                    if let Some(s) = pending_scale.take() {
                        result = result.scale(s);
                    }
                    
                    // Accumulate offsets
                    pending_offset = Some(match pending_offset {
                        Some(prev) => [prev[0] + o[0], prev[1] + o[1], prev[2] + o[2]],
                        None => *o,
                    });
                }
                other => {
                    // Flush all pending
                    if let Some(m) = pending_matrix.take() {
                        result = result.matrix(m);
                    }
                    if let Some(s) = pending_scale.take() {
                        result = result.scale(s);
                    }
                    if let Some(o) = pending_offset.take() {
                        result = result.offset(o);
                    }
                    result = result.push(other.clone());
                }
            }
        }

        // Flush remaining pending operations
        if let Some(m) = pending_matrix {
            result = result.matrix(m);
        }
        if let Some(s) = pending_scale {
            result = result.scale(s);
        }
        if let Some(o) = pending_offset {
            result = result.offset(o);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vfx_primaries::{SRGB, REC2020, rgb_to_xyz_matrix, xyz_to_rgb_matrix};
    use vfx_transfer::srgb;

    #[test]
    fn test_apply_single() {
        let mut proc = ColorProcessor::new();
        let pipeline = Pipeline::new()
            .transfer_in(srgb::eotf)
            .transfer_out(srgb::oetf);
        
        let rgb = [0.5, 0.3, 0.2];
        let result = proc.apply(&pipeline, rgb);
        
        assert!((result[0] - rgb[0]).abs() < 0.001);
    }

    #[test]
    fn test_apply_batch() {
        let mut proc = ColorProcessor::new().with_stats(true);
        let pipeline = Pipeline::new().scale([2.0, 2.0, 2.0]);
        
        let pixels = vec![[0.1, 0.2, 0.3], [0.4, 0.5, 0.6]];
        let results = proc.apply_batch(&pipeline, &pixels);
        
        assert_eq!(results.len(), 2);
        assert!((results[0][0] - 0.2).abs() < 0.001);
        assert!((results[1][0] - 0.8).abs() < 0.001);
        assert_eq!(proc.pixels_processed(), 2);
    }

    #[test]
    fn test_apply_in_place() {
        let mut proc = ColorProcessor::new();
        let pipeline = Pipeline::new().offset([0.1, 0.1, 0.1]);
        
        let mut pixels = vec![[0.1, 0.2, 0.3]];
        proc.apply_in_place(&pipeline, &mut pixels);
        
        assert!((pixels[0][0] - 0.2).abs() < 0.001);
        assert!((pixels[0][1] - 0.3).abs() < 0.001);
    }

    #[test]
    fn test_optimize_matrices() {
        let proc = ColorProcessor::new();
        
        let pipeline = Pipeline::new()
            .matrix(rgb_to_xyz_matrix(&SRGB))
            .matrix(xyz_to_rgb_matrix(&REC2020));
        
        let optimized = proc.optimize(&pipeline);
        assert_eq!(optimized.len(), 1);
        
        // Results should be the same
        let rgb = [0.5, 0.3, 0.2];
        let r1 = pipeline.apply(rgb);
        let r2 = optimized.apply(rgb);
        
        assert!((r1[0] - r2[0]).abs() < 0.0001);
        assert!((r1[1] - r2[1]).abs() < 0.0001);
        assert!((r1[2] - r2[2]).abs() < 0.0001);
    }

    #[test]
    fn test_optimize_scales() {
        let proc = ColorProcessor::new();
        
        let pipeline = Pipeline::new()
            .scale([2.0, 2.0, 2.0])
            .scale([0.5, 0.5, 0.5]);
        
        let optimized = proc.optimize(&pipeline);
        assert_eq!(optimized.len(), 1);
        
        // 2.0 * 0.5 = 1.0 (identity)
        let rgb = [0.5, 0.3, 0.2];
        let result = optimized.apply(rgb);
        
        assert!((result[0] - rgb[0]).abs() < 0.001);
    }

    #[test]
    fn test_stats() {
        let mut proc = ColorProcessor::new().with_stats(true);
        let pipeline = Pipeline::new();
        
        proc.apply(&pipeline, [0.5, 0.5, 0.5]);
        proc.apply(&pipeline, [0.5, 0.5, 0.5]);
        proc.apply_batch(&pipeline, &[[0.5, 0.5, 0.5]; 10]);
        
        assert_eq!(proc.pixels_processed(), 12);
        
        proc.reset_stats();
        assert_eq!(proc.pixels_processed(), 0);
    }
}

// =============================================================================
// Helper functions for buffer processing
// =============================================================================

/// Convert Mat3 to 4x4 matrix for GPU (column-major).
/// Mat3.m is row-major [row][col], GPU expects column-major.
#[cfg(feature = "gpu")]
fn mat3_to_mat4x4(m: &Mat3) -> [f32; 16] {
    // m.m[row][col] -> column-major 4x4
    [
        m.m[0][0], m.m[1][0], m.m[2][0], 0.0,  // col 0
        m.m[0][1], m.m[1][1], m.m[2][1], 0.0,  // col 1
        m.m[0][2], m.m[1][2], m.m[2][2], 0.0,  // col 2
        0.0, 0.0, 0.0, 1.0,                    // col 3
    ]
}

/// Apply matrix to buffer (CPU fallback).
/// Mat3.m is row-major: result[i] = sum(m[i][j] * input[j])
fn apply_matrix_cpu(data: &mut [f32], m: &Mat3) {
    for chunk in data.chunks_exact_mut(3) {
        let r = chunk[0];
        let g = chunk[1];
        let b = chunk[2];
        chunk[0] = m.m[0][0] * r + m.m[0][1] * g + m.m[0][2] * b;
        chunk[1] = m.m[1][0] * r + m.m[1][1] * g + m.m[1][2] * b;
        chunk[2] = m.m[2][0] * r + m.m[2][1] * g + m.m[2][2] * b;
    }
}

/// Apply 3D LUT to buffer (CPU fallback).
fn apply_lut3d_cpu(data: &mut [f32], lut: &vfx_lut::Lut3D) {
    for chunk in data.chunks_exact_mut(3) {
        let rgb = [chunk[0], chunk[1], chunk[2]];
        let result = lut.apply(rgb);
        chunk[0] = result[0];
        chunk[1] = result[1];
        chunk[2] = result[2];
    }
}

/// Apply scale to buffer (CPU).
fn apply_scale_cpu(data: &mut [f32], s: &[f32; 3]) {
    for chunk in data.chunks_exact_mut(3) {
        chunk[0] *= s[0];
        chunk[1] *= s[1];
        chunk[2] *= s[2];
    }
}

/// Apply any operation to buffer (CPU).
fn apply_op_cpu(data: &mut [f32], op: &TransformOp) {
    match op {
        TransformOp::TransferIn(f) | TransformOp::TransferOut(f) => {
            for v in data.iter_mut() {
                *v = f(*v);
            }
        }
        TransformOp::Matrix(m) => apply_matrix_cpu(data, m),
        TransformOp::Lut1D(lut) => {
            for chunk in data.chunks_exact_mut(3) {
                let rgb = [chunk[0], chunk[1], chunk[2]];
                let result = lut.apply_rgb(rgb);
                chunk[0] = result[0];
                chunk[1] = result[1];
                chunk[2] = result[2];
            }
        }
        TransformOp::Lut3D(lut) => apply_lut3d_cpu(data, lut),
        TransformOp::Scale(s) => apply_scale_cpu(data, s),
        TransformOp::Offset(o) => {
            for chunk in data.chunks_exact_mut(3) {
                chunk[0] += o[0];
                chunk[1] += o[1];
                chunk[2] += o[2];
            }
        }
        TransformOp::Clamp { min, max } => {
            for chunk in data.chunks_exact_mut(3) {
                chunk[0] = chunk[0].clamp(min[0], max[0]);
                chunk[1] = chunk[1].clamp(min[1], max[1]);
                chunk[2] = chunk[2].clamp(min[2], max[2]);
            }
        }
    }
}