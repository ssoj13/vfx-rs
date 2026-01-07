//! Image statistics and analysis for Python.
//!
//! Provides comprehensive statistics, comparison, and analysis functions.

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;

use vfx_io::imagebuf::ImageBuf;
use vfx_io::imagebufalgo::stats as rust_stats;
use vfx_core::Roi3D as RustRoi3D;

use crate::Image;
use crate::core::Roi3D;

// ============================================================================
// Helper Functions
// ============================================================================

fn image_to_imagebuf(img: &Image) -> ImageBuf {
    ImageBuf::from_image_data(img.as_image_data())
}

fn py_roi_to_rust(roi: &Roi3D) -> RustRoi3D {
    RustRoi3D {
        xbegin: roi.xbegin,
        xend: roi.xend,
        ybegin: roi.ybegin,
        yend: roi.yend,
        zbegin: roi.zbegin,
        zend: roi.zend,
        chbegin: roi.chbegin,
        chend: roi.chend,
    }
}

fn convert_roi(roi: Option<&Roi3D>) -> Option<RustRoi3D> {
    roi.map(py_roi_to_rust)
}

// ============================================================================
// PixelStats Class
// ============================================================================

/// Per-channel pixel statistics.
///
/// Contains min, max, average, standard deviation, and special value counts
/// for each channel in an image.
///
/// Attributes:
///     min: Minimum value per channel
///     max: Maximum value per channel
///     avg: Average value per channel
///     stddev: Standard deviation per channel
///     nan_count: Count of NaN values per channel
///     inf_count: Count of infinite values per channel
///     finite_count: Count of finite values per channel
#[pyclass]
#[derive(Debug, Clone)]
pub struct PixelStats {
    #[pyo3(get)]
    pub min: Vec<f32>,
    #[pyo3(get)]
    pub max: Vec<f32>,
    #[pyo3(get)]
    pub avg: Vec<f32>,
    #[pyo3(get)]
    pub stddev: Vec<f32>,
    #[pyo3(get)]
    pub nan_count: Vec<u64>,
    #[pyo3(get)]
    pub inf_count: Vec<u64>,
    #[pyo3(get)]
    pub finite_count: Vec<u64>,
}

#[pymethods]
impl PixelStats {
    fn __repr__(&self) -> String {
        format!(
            "PixelStats(min={:?}, max={:?}, avg={:?}, stddev={:?})",
            self.min, self.max, self.avg, self.stddev
        )
    }

    /// Number of channels.
    #[getter]
    fn nchannels(&self) -> usize {
        self.min.len()
    }

    /// Check if any channel has NaN values.
    fn has_nan(&self) -> bool {
        self.nan_count.iter().any(|&c| c > 0)
    }

    /// Check if any channel has infinite values.
    fn has_inf(&self) -> bool {
        self.inf_count.iter().any(|&c| c > 0)
    }

    /// Total count of NaN values across all channels.
    fn total_nan(&self) -> u64 {
        self.nan_count.iter().sum()
    }

    /// Total count of infinite values across all channels.
    fn total_inf(&self) -> u64 {
        self.inf_count.iter().sum()
    }
}

impl From<rust_stats::PixelStats> for PixelStats {
    fn from(s: rust_stats::PixelStats) -> Self {
        Self {
            min: s.min,
            max: s.max,
            avg: s.avg,
            stddev: s.stddev,
            nan_count: s.nan_count,
            inf_count: s.inf_count,
            finite_count: s.finite_count,
        }
    }
}

// ============================================================================
// CompareResults Class
// ============================================================================

/// Results from comparing two images.
///
/// Contains error metrics and failure/warning counts.
///
/// Attributes:
///     mean_error: Mean absolute error
///     rms_error: Root mean square error
///     psnr: Peak signal-to-noise ratio (dB)
///     max_error: Maximum absolute error
///     max_x: X coordinate of max error pixel
///     max_y: Y coordinate of max error pixel
///     max_z: Z coordinate of max error pixel
///     max_c: Channel index of max error
///     nwarn: Number of warning-level differences
///     nfail: Number of failure-level differences
///     error: Whether an error occurred
#[pyclass]
#[derive(Debug, Clone)]
pub struct CompareResults {
    #[pyo3(get)]
    pub mean_error: f64,
    #[pyo3(get)]
    pub rms_error: f64,
    #[pyo3(get)]
    pub psnr: f64,
    #[pyo3(get)]
    pub max_error: f64,
    #[pyo3(get)]
    pub max_x: i32,
    #[pyo3(get)]
    pub max_y: i32,
    #[pyo3(get)]
    pub max_z: i32,
    #[pyo3(get)]
    pub max_c: i32,
    #[pyo3(get)]
    pub nwarn: u64,
    #[pyo3(get)]
    pub nfail: u64,
    #[pyo3(get)]
    pub error: bool,
}

#[pymethods]
impl CompareResults {
    fn __repr__(&self) -> String {
        format!(
            "CompareResults(mean_error={:.6}, rms_error={:.6}, psnr={:.2} dB, max_error={:.6}, nfail={}, nwarn={})",
            self.mean_error, self.rms_error, self.psnr, self.max_error, self.nfail, self.nwarn
        )
    }

    /// Check if images are identical (zero error).
    fn is_identical(&self) -> bool {
        self.max_error == 0.0
    }

    /// Check if comparison passed (no failures).
    fn passed(&self) -> bool {
        self.nfail == 0 && !self.error
    }
}

impl From<rust_stats::CompareResults> for CompareResults {
    fn from(r: rust_stats::CompareResults) -> Self {
        Self {
            mean_error: r.mean_error,
            rms_error: r.rms_error,
            psnr: r.psnr,
            max_error: r.max_error,
            max_x: r.max_x,
            max_y: r.max_y,
            max_z: r.max_z,
            max_c: r.max_c,
            nwarn: r.nwarn,
            nfail: r.nfail,
            error: r.error,
        }
    }
}

// ============================================================================
// Histogram Class
// ============================================================================

/// Histogram data for a single channel.
///
/// Attributes:
///     bins: Histogram bin counts
///     min: Minimum value represented
///     max: Maximum value represented
///     nbins: Number of bins
#[pyclass]
#[derive(Debug, Clone)]
pub struct Histogram {
    #[pyo3(get)]
    pub bins: Vec<u64>,
    #[pyo3(get)]
    pub min: f32,
    #[pyo3(get)]
    pub max: f32,
    #[pyo3(get)]
    pub nbins: usize,
}

#[pymethods]
impl Histogram {
    fn __repr__(&self) -> String {
        format!(
            "Histogram(nbins={}, min={}, max={}, total={})",
            self.nbins, self.min, self.max, self.total()
        )
    }

    /// Total count of samples in histogram.
    fn total(&self) -> u64 {
        self.bins.iter().sum()
    }

    /// Get normalized histogram (probabilities).
    fn normalized(&self) -> Vec<f64> {
        let total = self.total() as f64;
        if total > 0.0 {
            self.bins.iter().map(|&c| c as f64 / total).collect()
        } else {
            vec![0.0; self.nbins]
        }
    }

    /// Get cumulative histogram.
    fn cumulative(&self) -> Vec<u64> {
        let mut cum = Vec::with_capacity(self.nbins);
        let mut sum = 0u64;
        for &count in &self.bins {
            sum += count;
            cum.push(sum);
        }
        cum
    }

    /// Get normalized cumulative histogram (CDF).
    fn cdf(&self) -> Vec<f64> {
        let total = self.total() as f64;
        if total > 0.0 {
            let mut sum = 0.0;
            self.bins.iter().map(|&c| {
                sum += c as f64 / total;
                sum
            }).collect()
        } else {
            vec![0.0; self.nbins]
        }
    }

    /// Get the bin index for a value.
    fn bin_for_value(&self, val: f32) -> usize {
        if val <= self.min {
            0
        } else if val >= self.max {
            self.nbins - 1
        } else {
            let t = (val - self.min) / (self.max - self.min);
            let bin = (t * self.nbins as f32) as usize;
            bin.min(self.nbins - 1)
        }
    }

    /// Get the value at center of a bin.
    fn value_for_bin(&self, bin: usize) -> f32 {
        let bin = bin.min(self.nbins - 1);
        let t = (bin as f32 + 0.5) / self.nbins as f32;
        self.min + t * (self.max - self.min)
    }
}

impl From<rust_stats::Histogram> for Histogram {
    fn from(h: rust_stats::Histogram) -> Self {
        Self {
            bins: h.bins,
            min: h.min,
            max: h.max,
            nbins: h.nbins,
        }
    }
}

// ============================================================================
// RangeCheckResult Class
// ============================================================================

/// Result of a color range check.
///
/// Attributes:
///     low_count: Pixels below low threshold
///     high_count: Pixels above high threshold
///     in_range_count: Pixels within range
#[pyclass]
#[derive(Debug, Clone)]
pub struct RangeCheckResult {
    #[pyo3(get)]
    pub low_count: u64,
    #[pyo3(get)]
    pub high_count: u64,
    #[pyo3(get)]
    pub in_range_count: u64,
}

#[pymethods]
impl RangeCheckResult {
    fn __repr__(&self) -> String {
        format!(
            "RangeCheckResult(low={}, in_range={}, high={})",
            self.low_count, self.in_range_count, self.high_count
        )
    }

    /// Total pixel count.
    fn total(&self) -> u64 {
        self.low_count + self.high_count + self.in_range_count
    }

    /// Percentage of pixels in range.
    fn in_range_percent(&self) -> f64 {
        let total = self.total();
        if total > 0 {
            self.in_range_count as f64 / total as f64 * 100.0
        } else {
            0.0
        }
    }

    /// Check if all pixels are in range.
    fn all_in_range(&self) -> bool {
        self.low_count == 0 && self.high_count == 0
    }
}

impl From<rust_stats::RangeCheckResult> for RangeCheckResult {
    fn from(r: rust_stats::RangeCheckResult) -> Self {
        Self {
            low_count: r.low_count,
            high_count: r.high_count,
            in_range_count: r.in_range_count,
        }
    }
}

// ============================================================================
// Statistics Functions
// ============================================================================

/// Compute comprehensive pixel statistics for an image.
///
/// Args:
///     image: Input image
///     roi: Optional region of interest
///
/// Returns:
///     PixelStats with min, max, avg, stddev per channel
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn compute_pixel_stats(image: &Image, roi: Option<&Roi3D>) -> PixelStats {
    let buf = image_to_imagebuf(image);
    let stats = rust_stats::compute_pixel_stats(&buf, convert_roi(roi));
    stats.into()
}

/// Numerically compare two images.
///
/// Args:
///     a: First image
///     b: Second image
///     fail_thresh: Threshold for "failure" (default 0.001)
///     warn_thresh: Threshold for "warning" (default 0.0001)
///     roi: Optional region of interest
///
/// Returns:
///     CompareResults with error metrics
#[pyfunction]
#[pyo3(signature = (a, b, fail_thresh=0.001, warn_thresh=0.0001, roi=None))]
pub fn compare(
    a: &Image,
    b: &Image,
    fail_thresh: f32,
    warn_thresh: f32,
    roi: Option<&Roi3D>,
) -> CompareResults {
    let buf_a = image_to_imagebuf(a);
    let buf_b = image_to_imagebuf(b);
    let results = rust_stats::compare(&buf_a, &buf_b, fail_thresh, warn_thresh, convert_roi(roi));
    results.into()
}

/// Compare two images with relative error thresholds.
///
/// Args:
///     a: First image
///     b: Second image
///     fail_thresh: Absolute threshold for failure
///     warn_thresh: Absolute threshold for warning
///     fail_relative: Relative threshold for failure (fraction of mean)
///     warn_relative: Relative threshold for warning (fraction of mean)
///     roi: Optional region of interest
///
/// Returns:
///     CompareResults with error metrics
#[pyfunction]
#[pyo3(signature = (a, b, fail_thresh=0.001, warn_thresh=0.0001, fail_relative=0.0, warn_relative=0.0, roi=None))]
pub fn compare_relative(
    a: &Image,
    b: &Image,
    fail_thresh: f32,
    warn_thresh: f32,
    fail_relative: f32,
    warn_relative: f32,
    roi: Option<&Roi3D>,
) -> CompareResults {
    let buf_a = image_to_imagebuf(a);
    let buf_b = image_to_imagebuf(b);
    let results = rust_stats::compare_relative(
        &buf_a, &buf_b,
        fail_thresh, warn_thresh,
        fail_relative, warn_relative,
        convert_roi(roi),
    );
    results.into()
}

// ============================================================================
// Property Checks
// ============================================================================

/// Check if an image is a constant (solid) color.
///
/// Args:
///     image: Input image
///     threshold: Tolerance for considering values equal (default 0.001)
///     roi: Optional region of interest
///
/// Returns:
///     True if all pixels have the same color (within threshold)
#[pyfunction]
#[pyo3(signature = (image, threshold=0.001, roi=None))]
pub fn is_constant_color(image: &Image, threshold: f32, roi: Option<&Roi3D>) -> bool {
    let buf = image_to_imagebuf(image);
    rust_stats::is_constant_color(&buf, threshold, None, convert_roi(roi))
}

/// Get the constant color of an image if it is solid.
///
/// Args:
///     image: Input image
///     threshold: Tolerance for considering values equal
///     roi: Optional region of interest
///
/// Returns:
///     List of channel values if constant, None otherwise
#[pyfunction]
#[pyo3(signature = (image, threshold=0.001, roi=None))]
pub fn get_constant_color(image: &Image, threshold: f32, roi: Option<&Roi3D>) -> Option<Vec<f32>> {
    let buf = image_to_imagebuf(image);
    let nch = buf.nchannels() as usize;
    let mut color = vec![0.0f32; nch];

    if rust_stats::is_constant_color(&buf, threshold, Some(&mut color), convert_roi(roi)) {
        Some(color)
    } else {
        None
    }
}

/// Check if a specific channel is constant.
///
/// Args:
///     image: Input image
///     channel: Channel index to check
///     value: Expected constant value
///     threshold: Tolerance (default 0.001)
///     roi: Optional region of interest
///
/// Returns:
///     True if the channel is constant
#[pyfunction]
#[pyo3(signature = (image, channel, value, threshold=0.001, roi=None))]
pub fn is_constant_channel(
    image: &Image,
    channel: usize,
    value: f32,
    threshold: f32,
    roi: Option<&Roi3D>,
) -> bool {
    let buf = image_to_imagebuf(image);
    rust_stats::is_constant_channel(&buf, channel, value, threshold, convert_roi(roi))
}

/// Check if an image is monochrome (grayscale).
///
/// Args:
///     image: Input image
///     threshold: Tolerance (default 0.001)
///     roi: Optional region of interest
///
/// Returns:
///     True if all channels have the same value per pixel
#[pyfunction]
#[pyo3(signature = (image, threshold=0.001, roi=None))]
pub fn is_monochrome(image: &Image, threshold: f32, roi: Option<&Roi3D>) -> bool {
    let buf = image_to_imagebuf(image);
    rust_stats::is_monochrome(&buf, threshold, convert_roi(roi))
}

// ============================================================================
// Histogram
// ============================================================================

/// Compute histogram for a single channel.
///
/// Args:
///     image: Input image
///     channel: Channel index (default 0)
///     nbins: Number of bins (default 256)
///     min_val: Minimum value (default 0.0)
///     max_val: Maximum value (default 1.0)
///     roi: Optional region of interest
///
/// Returns:
///     Histogram object
#[pyfunction]
#[pyo3(signature = (image, channel=0, nbins=256, min_val=0.0, max_val=1.0, roi=None))]
pub fn histogram(
    image: &Image,
    channel: usize,
    nbins: usize,
    min_val: f32,
    max_val: f32,
    roi: Option<&Roi3D>,
) -> Histogram {
    let buf = image_to_imagebuf(image);
    let hist = rust_stats::histogram(&buf, channel, nbins, min_val, max_val, convert_roi(roi));
    hist.into()
}

/// Compute histograms for all channels.
///
/// Args:
///     image: Input image
///     nbins: Number of bins (default 256)
///     min_val: Minimum value (default 0.0)
///     max_val: Maximum value (default 1.0)
///     roi: Optional region of interest
///
/// Returns:
///     List of Histogram objects, one per channel
#[pyfunction]
#[pyo3(signature = (image, nbins=256, min_val=0.0, max_val=1.0, roi=None))]
pub fn histogram_all(
    image: &Image,
    nbins: usize,
    min_val: f32,
    max_val: f32,
    roi: Option<&Roi3D>,
) -> Vec<Histogram> {
    let buf = image_to_imagebuf(image);
    let nch = buf.nchannels() as usize;
    let roi_rust = convert_roi(roi);

    (0..nch)
        .map(|ch| rust_stats::histogram(&buf, ch, nbins, min_val, max_val, roi_rust).into())
        .collect()
}

// ============================================================================
// Channel Operations
// ============================================================================

/// Compute maximum value across all channels for each pixel.
///
/// Args:
///     image: Input image
///     roi: Optional region of interest
///
/// Returns:
///     Single-channel image with max values
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn maxchan(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = rust_stats::maxchan(&buf, convert_roi(roi));
    let data = result.to_image_data()
        .map_err(|e| PyValueError::new_err(format!("Failed to convert: {}", e)))?;
    Ok(Image::from_image_data(data))
}

/// Compute minimum value across all channels for each pixel.
///
/// Args:
///     image: Input image
///     roi: Optional region of interest
///
/// Returns:
///     Single-channel image with min values
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn minchan(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let result = rust_stats::minchan(&buf, convert_roi(roi));
    let data = result.to_image_data()
        .map_err(|e| PyValueError::new_err(format!("Failed to convert: {}", e)))?;
    Ok(Image::from_image_data(data))
}

// ============================================================================
// Range and Color Analysis
// ============================================================================

/// Check how many pixels have values outside a given range.
///
/// Args:
///     image: Input image
///     low: Low threshold per channel
///     high: High threshold per channel
///     roi: Optional region of interest
///
/// Returns:
///     RangeCheckResult with counts
#[pyfunction]
#[pyo3(signature = (image, low, high, roi=None))]
pub fn color_range_check(
    image: &Image,
    low: Vec<f32>,
    high: Vec<f32>,
    roi: Option<&Roi3D>,
) -> RangeCheckResult {
    let buf = image_to_imagebuf(image);
    let result = rust_stats::color_range_check(&buf, &low, &high, convert_roi(roi));
    result.into()
}


// ============================================================================
// Module Registration
// ============================================================================

/// Register all stats functions to the module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Classes
    m.add_class::<PixelStats>()?;
    m.add_class::<CompareResults>()?;
    m.add_class::<Histogram>()?;
    m.add_class::<RangeCheckResult>()?;

    // Statistics
    m.add_function(wrap_pyfunction!(compute_pixel_stats, m)?)?;
    m.add_function(wrap_pyfunction!(compare, m)?)?;
    m.add_function(wrap_pyfunction!(compare_relative, m)?)?;

    // Property checks
    m.add_function(wrap_pyfunction!(is_constant_color, m)?)?;
    m.add_function(wrap_pyfunction!(get_constant_color, m)?)?;
    m.add_function(wrap_pyfunction!(is_constant_channel, m)?)?;
    m.add_function(wrap_pyfunction!(is_monochrome, m)?)?;

    // Histogram
    m.add_function(wrap_pyfunction!(histogram, m)?)?;
    m.add_function(wrap_pyfunction!(histogram_all, m)?)?;

    // Channel operations
    m.add_function(wrap_pyfunction!(maxchan, m)?)?;
    m.add_function(wrap_pyfunction!(minchan, m)?)?;

    // Range and color analysis
    m.add_function(wrap_pyfunction!(color_range_check, m)?)?;

    Ok(())
}
