//! Deep image operations for Python.
//!
//! Provides deep compositing operations for images with multiple samples per pixel.

use pyo3::prelude::*;
use pyo3::exceptions::PyIOError;

use vfx_io::deepdata::DeepData as RustDeepData;
use vfx_io::imagebuf::ImageBuf;
use vfx_io::imagebufalgo::deep as rust_deep;
use vfx_core::TypeDesc as RustTypeDesc;

use crate::Image;

// ============================================================================
// Helper Functions
// ============================================================================

fn image_to_imagebuf(img: &Image) -> ImageBuf {
    ImageBuf::from_image_data(img.as_image_data())
}

fn imagebuf_to_image(buf: &ImageBuf) -> PyResult<Image> {
    let data = buf.to_image_data()
        .map_err(|e| PyIOError::new_err(format!("Conversion failed: {}", e)))?;
    Ok(Image::from_image_data(data))
}

// ============================================================================
// DeepStats Class
// ============================================================================

/// Statistics about a deep image.
#[pyclass]
#[derive(Debug, Clone)]
pub struct DeepStats {
    /// Total number of samples across all pixels
    #[pyo3(get)]
    pub total_samples: u64,
    /// Maximum samples in any single pixel
    #[pyo3(get)]
    pub max_samples_per_pixel: u32,
    /// Average samples per pixel
    #[pyo3(get)]
    pub avg_samples_per_pixel: f64,
    /// Number of pixels with zero samples
    #[pyo3(get)]
    pub empty_pixels: u64,
    /// Number of pixels with multiple samples
    #[pyo3(get)]
    pub multi_sample_pixels: u64,
    /// Minimum Z depth
    #[pyo3(get)]
    pub min_z: f32,
    /// Maximum Z depth
    #[pyo3(get)]
    pub max_z: f32,
}

#[pymethods]
impl DeepStats {
    fn __repr__(&self) -> String {
        format!(
            "DeepStats(total_samples={}, max_per_pixel={}, avg={:.2}, empty={}, multi={}, z=[{:.3}, {:.3}])",
            self.total_samples,
            self.max_samples_per_pixel,
            self.avg_samples_per_pixel,
            self.empty_pixels,
            self.multi_sample_pixels,
            self.min_z,
            self.max_z
        )
    }
}

impl From<rust_deep::DeepStats> for DeepStats {
    fn from(s: rust_deep::DeepStats) -> Self {
        Self {
            total_samples: s.total_samples,
            max_samples_per_pixel: s.max_samples_per_pixel,
            avg_samples_per_pixel: s.avg_samples_per_pixel,
            empty_pixels: s.empty_pixels,
            multi_sample_pixels: s.multi_sample_pixels,
            min_z: s.min_z,
            max_z: s.max_z,
        }
    }
}

// ============================================================================
// DeepData Class
// ============================================================================

/// Deep image data with multiple samples per pixel.
///
/// Used for deep compositing where each pixel can have multiple
/// layers at different Z depths.
///
/// Example:
///     >>> # Create deep data with 100 pixels, 5 channels (RGBA + Z)
///     >>> deep = DeepData(100, ["R", "G", "B", "A", "Z"])
///     >>>
///     >>> # Set samples for first pixel
///     >>> deep.set_samples(0, 2)
///     >>> deep.set_value(0, 0, 0, 1.0)  # R = 1.0 for sample 0
///     >>> deep.set_value(0, 4, 0, 0.5)  # Z = 0.5 for sample 0
#[pyclass]
pub struct DeepData {
    inner: RustDeepData,
}

#[pymethods]
impl DeepData {
    /// Create new DeepData.
    ///
    /// Args:
    ///     pixels: Number of pixels
    ///     channel_names: List of channel names (e.g., ["R", "G", "B", "A", "Z"])
    ///
    /// All channels are created as float type.
    #[new]
    #[pyo3(signature = (pixels, channel_names))]
    pub fn new(pixels: i64, channel_names: Vec<String>) -> Self {
        let nch = channel_names.len();
        let types = vec![RustTypeDesc::FLOAT; nch];
        let names: Vec<&str> = channel_names.iter().map(|s| s.as_str()).collect();
        Self {
            inner: RustDeepData::new(pixels, &types, &names),
        }
    }

    /// Create empty DeepData.
    #[staticmethod]
    pub fn empty() -> Self {
        Self {
            inner: RustDeepData::new_empty(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "DeepData(pixels={}, channels={})",
            self.inner.pixels(),
            self.inner.channels()
        )
    }

    // ========================================================================
    // Basic Properties
    // ========================================================================

    /// Total number of pixels.
    #[getter]
    pub fn pixels(&self) -> i64 {
        self.inner.pixels()
    }

    /// Number of channels.
    #[getter]
    pub fn channels(&self) -> usize {
        self.inner.channels()
    }

    /// Z channel index (-1 if not present).
    #[getter]
    pub fn z_channel(&self) -> i32 {
        self.inner.z_channel()
    }

    /// Alpha channel index (-1 if not present).
    #[getter]
    pub fn a_channel(&self) -> i32 {
        self.inner.a_channel()
    }

    /// Get channel name by index.
    pub fn channel_name(&self, channel: usize) -> String {
        self.inner.channelname(channel)
    }

    /// Get all channel names.
    pub fn channel_names(&self) -> Vec<String> {
        (0..self.inner.channels())
            .map(|c| self.inner.channelname(c))
            .collect()
    }

    /// Size in bytes of one sample (all channels).
    pub fn sample_size(&self) -> usize {
        self.inner.samplesize()
    }

    // ========================================================================
    // Sample Management
    // ========================================================================

    /// Get number of samples for a pixel.
    pub fn samples(&self, pixel: i64) -> u32 {
        self.inner.samples(pixel)
    }

    /// Set number of samples for a pixel.
    pub fn set_samples(&self, pixel: i64, count: u32) {
        self.inner.set_samples(pixel, count);
    }

    /// Get capacity for a pixel.
    pub fn capacity(&self, pixel: i64) -> u32 {
        self.inner.capacity(pixel)
    }

    /// Set capacity for a pixel.
    pub fn set_capacity(&self, pixel: i64, capacity: u32) {
        self.inner.set_capacity(pixel, capacity);
    }

    /// Insert samples at position.
    pub fn insert_samples(&self, pixel: i64, position: usize, count: usize) {
        self.inner.insert_samples(pixel, position, count);
    }

    /// Erase samples at position.
    pub fn erase_samples(&self, pixel: i64, position: usize, count: usize) {
        self.inner.erase_samples(pixel, position, count);
    }

    // ========================================================================
    // Value Access
    // ========================================================================

    /// Get deep value as float.
    ///
    /// Args:
    ///     pixel: Pixel index
    ///     channel: Channel index
    ///     sample: Sample index
    ///
    /// Returns:
    ///     Value as float
    pub fn value(&self, pixel: i64, channel: usize, sample: usize) -> f32 {
        self.inner.deep_value(pixel, channel, sample)
    }

    /// Set deep value as float.
    ///
    /// Args:
    ///     pixel: Pixel index
    ///     channel: Channel index
    ///     sample: Sample index
    ///     value: Value to set
    pub fn set_value(&self, pixel: i64, channel: usize, sample: usize, value: f32) {
        self.inner.set_deep_value_f32(pixel, channel, sample, value);
    }

    /// Get all values for a pixel as nested list.
    ///
    /// Returns:
    ///     List of samples, each sample is list of channel values
    pub fn pixel_values(&self, pixel: i64) -> Vec<Vec<f32>> {
        let nsamps = self.inner.samples(pixel) as usize;
        let nch = self.inner.channels();
        let mut result = Vec::with_capacity(nsamps);

        for s in 0..nsamps {
            let mut sample = Vec::with_capacity(nch);
            for c in 0..nch {
                sample.push(self.inner.deep_value(pixel, c, s));
            }
            result.push(sample);
        }

        result
    }

    // ========================================================================
    // Operations
    // ========================================================================

    /// Sort samples by Z depth.
    pub fn sort(&self, pixel: i64) {
        self.inner.sort(pixel);
    }

    /// Merge overlapping samples.
    pub fn merge_overlaps(&self, pixel: i64) {
        self.inner.merge_overlaps(pixel);
    }

    /// Cull fully occluded samples.
    pub fn occlusion_cull(&self, pixel: i64) {
        self.inner.occlusion_cull(pixel);
    }

    /// Tidy all pixels (sort, merge, cull).
    pub fn tidy(&self) {
        rust_deep::deep_tidy(&self.inner);
    }

    /// Compute statistics.
    pub fn stats(&self) -> DeepStats {
        rust_deep::deep_stats(&self.inner).into()
    }

    /// Clear all data.
    pub fn clear(&self) {
        self.inner.clear();
    }
}

impl DeepData {
    pub fn inner(&self) -> &RustDeepData {
        &self.inner
    }

    pub fn from_rust(inner: RustDeepData) -> Self {
        Self { inner }
    }
}

// ============================================================================
// Deep Image Functions
// ============================================================================

/// Flatten a deep image to a regular RGBA image.
///
/// Composites all samples at each pixel using over blending,
/// ordered by Z depth.
///
/// Args:
///     deep: DeepData to flatten
///     width: Output image width
///     height: Output image height
///
/// Returns:
///     Flattened RGBA image
///
/// Example:
///     >>> flat = flatten_deep(deep_data, 1920, 1080)
#[pyfunction]
pub fn flatten_deep(deep: &DeepData, width: u32, height: u32) -> PyResult<Image> {
    let result = rust_deep::flatten_deep(deep.inner(), width, height);
    imagebuf_to_image(&result)
}

/// Convert a flat image to deep with constant Z.
///
/// Each pixel becomes a single sample at the specified Z depth.
///
/// Args:
///     image: Source flat image
///     z_value: Z depth for all pixels
///
/// Returns:
///     DeepData with one sample per pixel
///
/// Example:
///     >>> deep = deepen(flat_image, 100.0)
#[pyfunction]
pub fn deepen(image: &Image, z_value: f32) -> DeepData {
    let buf = image_to_imagebuf(image);
    let deep = rust_deep::deepen(&buf, z_value);
    DeepData::from_rust(deep)
}

/// Convert a flat image to deep with Z from a depth image.
///
/// Args:
///     image: Source flat image (color/alpha)
///     z_image: Depth image (single channel)
///
/// Returns:
///     DeepData with Z values from depth image
#[pyfunction]
pub fn deepen_with_z(image: &Image, z_image: &Image) -> DeepData {
    let buf = image_to_imagebuf(image);
    let z_buf = image_to_imagebuf(z_image);
    let deep = rust_deep::deepen_with_z(&buf, &z_buf);
    DeepData::from_rust(deep)
}

/// Merge two deep images.
///
/// Combines samples from both images at each pixel, sorting by Z.
///
/// Args:
///     a: First deep image
///     b: Second deep image
///
/// Returns:
///     Merged DeepData
///
/// Example:
///     >>> merged = deep_merge(foreground, background)
#[pyfunction]
pub fn deep_merge(a: &DeepData, b: &DeepData) -> DeepData {
    let result = rust_deep::deep_merge(a.inner(), b.inner());
    DeepData::from_rust(result)
}

/// Apply holdout at a Z depth.
///
/// Removes samples behind (greater Z than) the holdout depth.
///
/// Args:
///     deep: DeepData to modify
///     z: Holdout Z depth
#[pyfunction]
pub fn deep_holdout(deep: &DeepData, z: f32) {
    rust_deep::deep_holdout(deep.inner(), z);
}

/// Apply holdout using a matte deep image.
///
/// Removes samples behind the closest holdout surface at each pixel.
///
/// Args:
///     deep: DeepData to modify
///     holdout: Holdout matte DeepData
#[pyfunction]
pub fn deep_holdout_matte(deep: &DeepData, holdout: &DeepData) {
    rust_deep::deep_holdout_matte(deep.inner(), holdout.inner());
}

/// Tidy a deep image (sort, merge overlaps, cull occluded).
///
/// Args:
///     deep: DeepData to tidy
#[pyfunction]
pub fn deep_tidy(deep: &DeepData) {
    rust_deep::deep_tidy(deep.inner());
}

/// Compute statistics about a deep image.
///
/// Args:
///     deep: DeepData to analyze
///
/// Returns:
///     DeepStats with sample counts, Z range, etc.
#[pyfunction]
pub fn deep_stats(deep: &DeepData) -> DeepStats {
    rust_deep::deep_stats(deep.inner()).into()
}

// ============================================================================
// Module Registration
// ============================================================================

/// Register all deep functions to the module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Classes
    m.add_class::<DeepData>()?;
    m.add_class::<DeepStats>()?;

    // Functions
    m.add_function(wrap_pyfunction!(flatten_deep, m)?)?;
    m.add_function(wrap_pyfunction!(deepen, m)?)?;
    m.add_function(wrap_pyfunction!(deepen_with_z, m)?)?;
    m.add_function(wrap_pyfunction!(deep_merge, m)?)?;
    m.add_function(wrap_pyfunction!(deep_holdout, m)?)?;
    m.add_function(wrap_pyfunction!(deep_holdout_matte, m)?)?;
    m.add_function(wrap_pyfunction!(deep_tidy, m)?)?;
    m.add_function(wrap_pyfunction!(deep_stats, m)?)?;

    Ok(())
}
