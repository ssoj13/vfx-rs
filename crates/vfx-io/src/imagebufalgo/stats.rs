//! Image statistics and analysis functions for ImageBuf.
//!
//! This module provides functions for computing image statistics:
//! - [`compute_pixel_stats`] - Compute min/max/avg/stddev per channel
//! - [`compare`] - Numerically compare two images
//! - [`is_constant_color`] - Check if image is solid color
//! - [`is_constant_channel`] - Check if channel is constant
//! - [`is_monochrome`] - Check if image is grayscale
//! - [`histogram`] - Compute histogram for a channel
//! - [`maxchan`] / [`minchan`] - Max/min across all channels
//! - [`color_range_check`] - Check if colors are within range

use crate::imagebuf::{ImageBuf, WrapMode};
use vfx_core::Roi3D;

// ============================================================================
// Pixel Statistics
// ============================================================================

/// Statistics computed per-channel for an image.
#[derive(Debug, Clone, Default)]
pub struct PixelStats {
    /// Minimum value per channel.
    pub min: Vec<f32>,
    /// Maximum value per channel.
    pub max: Vec<f32>,
    /// Average value per channel.
    pub avg: Vec<f32>,
    /// Standard deviation per channel.
    pub stddev: Vec<f32>,
    /// Count of NaN values per channel.
    pub nan_count: Vec<u64>,
    /// Count of infinite values per channel.
    pub inf_count: Vec<u64>,
    /// Count of finite values per channel.
    pub finite_count: Vec<u64>,
}

impl PixelStats {
    /// Creates new pixel stats with given number of channels.
    pub fn new(nchannels: usize) -> Self {
        Self {
            min: vec![f32::MAX; nchannels],
            max: vec![f32::MIN; nchannels],
            avg: vec![0.0; nchannels],
            stddev: vec![0.0; nchannels],
            nan_count: vec![0; nchannels],
            inf_count: vec![0; nchannels],
            finite_count: vec![0; nchannels],
        }
    }

    /// Merges another PixelStats into this one.
    pub fn merge(&mut self, other: &PixelStats) {
        for c in 0..self.min.len().min(other.min.len()) {
            self.min[c] = self.min[c].min(other.min[c]);
            self.max[c] = self.max[c].max(other.max[c]);
            self.nan_count[c] += other.nan_count[c];
            self.inf_count[c] += other.inf_count[c];
            self.finite_count[c] += other.finite_count[c];
        }
        // Note: avg and stddev require recomputation after merge
    }
}

/// Compute comprehensive pixel statistics for an image.
///
/// # Arguments
///
/// * `src` - Source image
/// * `roi` - Optional region of interest
///
/// # Returns
///
/// PixelStats containing min, max, avg, stddev for each channel,
/// plus counts of NaN, infinite, and finite values.
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::stats::compute_pixel_stats;
///
/// let stats = compute_pixel_stats(&image, None);
/// println!("Min: {:?}, Max: {:?}", stats.min, stats.max);
/// println!("Avg: {:?}, Stddev: {:?}", stats.avg, stats.stddev);
/// ```
pub fn compute_pixel_stats(src: &ImageBuf, roi: Option<Roi3D>) -> PixelStats {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;

    let mut stats = PixelStats::new(nch);
    let mut sum = vec![0.0f64; nch];
    let mut sum2 = vec![0.0f64; nch];

    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                for c in 0..nch {
                    let v = pixel[c];

                    if v.is_nan() {
                        stats.nan_count[c] += 1;
                    } else if v.is_infinite() {
                        stats.inf_count[c] += 1;
                    } else {
                        stats.min[c] = stats.min[c].min(v);
                        stats.max[c] = stats.max[c].max(v);
                        sum[c] += v as f64;
                        sum2[c] += (v as f64) * (v as f64);
                        stats.finite_count[c] += 1;
                    }
                }
            }
        }
    }

    // Compute average and stddev
    for c in 0..nch {
        let n = stats.finite_count[c] as f64;
        if n > 0.0 {
            stats.avg[c] = (sum[c] / n) as f32;
            let variance = (sum2[c] / n) - (sum[c] / n).powi(2);
            stats.stddev[c] = variance.max(0.0).sqrt() as f32;
        }
    }

    stats
}

// ============================================================================
// Image Comparison
// ============================================================================

/// Results from comparing two images.
#[derive(Debug, Clone, Default)]
pub struct CompareResults {
    /// Mean error across all compared pixels.
    pub mean_error: f64,
    /// Root mean square error.
    pub rms_error: f64,
    /// Peak signal-to-noise ratio (in dB).
    pub psnr: f64,
    /// Maximum absolute error.
    pub max_error: f64,
    /// X coordinate of pixel with maximum error.
    pub max_x: i32,
    /// Y coordinate of pixel with maximum error.
    pub max_y: i32,
    /// Z coordinate of pixel with maximum error.
    pub max_z: i32,
    /// Channel index of maximum error.
    pub max_c: i32,
    /// Number of warning-level differences.
    pub nwarn: u64,
    /// Number of failure-level differences.
    pub nfail: u64,
    /// Whether an error occurred during comparison.
    pub error: bool,
}

/// Numerically compare two images.
///
/// # Arguments
///
/// * `a` - First image
/// * `b` - Second image
/// * `fail_thresh` - Threshold for "failure" (absolute difference)
/// * `warn_thresh` - Threshold for "warning" (absolute difference)
/// * `roi` - Optional region of interest
///
/// # Returns
///
/// CompareResults containing error metrics and failure/warning counts.
pub fn compare(
    a: &ImageBuf,
    b: &ImageBuf,
    fail_thresh: f32,
    warn_thresh: f32,
    roi: Option<Roi3D>,
) -> CompareResults {
    compare_relative(a, b, fail_thresh, warn_thresh, 0.0, 0.0, roi)
}

/// Compare two images with relative error thresholds.
///
/// # Arguments
///
/// * `a` - First image
/// * `b` - Second image
/// * `fail_thresh` - Absolute threshold for "failure"
/// * `warn_thresh` - Absolute threshold for "warning"
/// * `fail_relative` - Relative threshold for failure (as fraction of mean)
/// * `warn_relative` - Relative threshold for warning (as fraction of mean)
/// * `roi` - Optional region of interest
pub fn compare_relative(
    a: &ImageBuf,
    b: &ImageBuf,
    fail_thresh: f32,
    warn_thresh: f32,
    fail_relative: f32,
    warn_relative: f32,
    roi: Option<Roi3D>,
) -> CompareResults {
    let roi = roi.unwrap_or_else(|| a.roi().union(&b.roi()));
    let nch = a.nchannels().max(b.nchannels()) as usize;

    let mut results = CompareResults::default();

    let mut pixel_a = vec![0.0f32; nch];
    let mut pixel_b = vec![0.0f32; nch];
    let mut sum_error = 0.0f64;
    let mut sum_sq_error = 0.0f64;
    let mut count = 0u64;

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                a.getpixel(x, y, z, &mut pixel_a, WrapMode::Black);
                b.getpixel(x, y, z, &mut pixel_b, WrapMode::Black);

                for c in 0..nch {
                    let va = pixel_a[c];
                    let vb = pixel_b[c];
                    let diff = (va - vb).abs();

                    sum_error += diff as f64;
                    sum_sq_error += (diff as f64).powi(2);
                    count += 1;

                    // Check for maximum error
                    if diff as f64 > results.max_error {
                        results.max_error = diff as f64;
                        results.max_x = x;
                        results.max_y = y;
                        results.max_z = z;
                        results.max_c = c as i32;
                    }

                    // Compute thresholds (combine absolute and relative)
                    let mean = (va.abs() + vb.abs()) * 0.5;
                    let fail_t = fail_thresh.max(mean * fail_relative);
                    let warn_t = warn_thresh.max(mean * warn_relative);

                    if diff > fail_t {
                        results.nfail += 1;
                    } else if diff > warn_t {
                        results.nwarn += 1;
                    }
                }
            }
        }
    }

    if count > 0 {
        results.mean_error = sum_error / count as f64;
        results.rms_error = (sum_sq_error / count as f64).sqrt();

        // PSNR: 20 * log10(MAX / RMSE)
        // Assuming normalized [0,1] data, MAX = 1.0
        if results.rms_error > 0.0 {
            results.psnr = 20.0 * (1.0 / results.rms_error).log10();
        } else {
            results.psnr = f64::INFINITY;
        }
    }

    results
}

// ============================================================================
// Image Property Checks
// ============================================================================

/// Check if an image is a constant (solid) color.
///
/// # Arguments
///
/// * `src` - Source image
/// * `threshold` - Tolerance for considering values equal
/// * `color` - Optional output buffer to receive the constant color
/// * `roi` - Optional region of interest
///
/// # Returns
///
/// True if all pixels in the image have the same color (within threshold),
/// optionally storing that color in the provided buffer.
pub fn is_constant_color(
    src: &ImageBuf,
    threshold: f32,
    color: Option<&mut [f32]>,
    roi: Option<Roi3D>,
) -> bool {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;

    let mut first_pixel = vec![0.0f32; nch];
    let mut pixel = vec![0.0f32; nch];
    let mut first = true;

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                if first {
                    first_pixel.copy_from_slice(&pixel);
                    first = false;
                } else {
                    // Compare to first pixel
                    for c in 0..nch {
                        if (pixel[c] - first_pixel[c]).abs() > threshold {
                            return false;
                        }
                    }
                }
            }
        }
    }

    // Store the constant color if requested
    if let Some(out_color) = color {
        let len = out_color.len().min(nch);
        out_color[..len].copy_from_slice(&first_pixel[..len]);
    }

    true
}

/// Check if a specific channel is constant.
///
/// # Arguments
///
/// * `src` - Source image
/// * `channel` - Channel index to check
/// * `val` - Expected constant value
/// * `threshold` - Tolerance for considering values equal
/// * `roi` - Optional region of interest
pub fn is_constant_channel(
    src: &ImageBuf,
    channel: usize,
    val: f32,
    threshold: f32,
    roi: Option<Roi3D>,
) -> bool {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;

    if channel >= nch {
        return false;
    }

    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                if (pixel[channel] - val).abs() > threshold {
                    return false;
                }
            }
        }
    }

    true
}

/// Check if an image is monochrome (all channels have the same value per pixel).
///
/// # Arguments
///
/// * `src` - Source image
/// * `threshold` - Tolerance for considering values equal
/// * `roi` - Optional region of interest
pub fn is_monochrome(src: &ImageBuf, threshold: f32, roi: Option<Roi3D>) -> bool {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;

    if nch < 2 {
        return true; // Single channel is trivially monochrome
    }

    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                // Compare all channels to the first
                let first = pixel[0];
                for c in 1..nch {
                    if (pixel[c] - first).abs() > threshold {
                        return false;
                    }
                }
            }
        }
    }

    true
}

// ============================================================================
// Histogram
// ============================================================================

/// Histogram data for a single channel.
#[derive(Debug, Clone)]
pub struct Histogram {
    /// Histogram bins.
    pub bins: Vec<u64>,
    /// Minimum value represented.
    pub min: f32,
    /// Maximum value represented.
    pub max: f32,
    /// Number of bins.
    pub nbins: usize,
}

impl Histogram {
    /// Creates a new histogram.
    pub fn new(nbins: usize, min: f32, max: f32) -> Self {
        Self {
            bins: vec![0; nbins],
            min,
            max,
            nbins,
        }
    }

    /// Gets the bin index for a value.
    pub fn bin_for_value(&self, val: f32) -> usize {
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

    /// Adds a value to the histogram.
    pub fn add(&mut self, val: f32) {
        if val.is_finite() {
            let bin = self.bin_for_value(val);
            self.bins[bin] += 1;
        }
    }

    /// Returns the total count of samples.
    pub fn total(&self) -> u64 {
        self.bins.iter().sum()
    }

    /// Returns normalized histogram (probabilities).
    pub fn normalized(&self) -> Vec<f64> {
        let total = self.total() as f64;
        if total > 0.0 {
            self.bins.iter().map(|&c| c as f64 / total).collect()
        } else {
            vec![0.0; self.nbins]
        }
    }
}

/// Compute histogram for a single channel.
///
/// # Arguments
///
/// * `src` - Source image
/// * `channel` - Channel index to histogram
/// * `nbins` - Number of histogram bins
/// * `min` - Minimum value (values below are clamped)
/// * `max` - Maximum value (values above are clamped)
/// * `roi` - Optional region of interest
pub fn histogram(
    src: &ImageBuf,
    channel: usize,
    nbins: usize,
    min: f32,
    max: f32,
    roi: Option<Roi3D>,
) -> Histogram {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;

    let mut hist = Histogram::new(nbins, min, max);

    if channel >= nch {
        return hist;
    }

    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);
                hist.add(pixel[channel]);
            }
        }
    }

    hist
}

// ============================================================================
// Channel Min/Max Operations
// ============================================================================

/// Compute the maximum value across all channels for each pixel.
///
/// Returns a single-channel image where each pixel is the maximum
/// of all channels from the source.
pub fn maxchan(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    use crate::imagebuf::InitializePixels;
    use vfx_core::ImageSpec;

    let roi = roi.unwrap_or_else(|| src.roi());

    let spec = ImageSpec::new(
        roi.width() as u32,
        roi.height() as u32,
        1,
        vfx_core::DataFormat::F32,
    );

    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    maxchan_into(&mut dst, src, Some(roi));
    dst
}

/// Compute maximum channel value into existing destination.
pub fn maxchan_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;

    let mut pixel = vec![0.0f32; nch];
    let mut out_pixel = [0.0f32];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                let max_val = pixel.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                out_pixel[0] = max_val;

                dst.setpixel(x, y, z, &out_pixel);
            }
        }
    }
}

/// Compute the minimum value across all channels for each pixel.
///
/// Returns a single-channel image where each pixel is the minimum
/// of all channels from the source.
pub fn minchan(src: &ImageBuf, roi: Option<Roi3D>) -> ImageBuf {
    use crate::imagebuf::InitializePixels;
    use vfx_core::ImageSpec;

    let roi = roi.unwrap_or_else(|| src.roi());

    let spec = ImageSpec::new(
        roi.width() as u32,
        roi.height() as u32,
        1,
        vfx_core::DataFormat::F32,
    );

    let mut dst = ImageBuf::new(spec, InitializePixels::No);
    minchan_into(&mut dst, src, Some(roi));
    dst
}

/// Compute minimum channel value into existing destination.
pub fn minchan_into(dst: &mut ImageBuf, src: &ImageBuf, roi: Option<Roi3D>) {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;

    let mut pixel = vec![0.0f32; nch];
    let mut out_pixel = [0.0f32];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                let min_val = pixel.iter().copied().fold(f32::INFINITY, f32::min);
                out_pixel[0] = min_val;

                dst.setpixel(x, y, z, &out_pixel);
            }
        }
    }
}

// ============================================================================
// Color Range Check
// ============================================================================

/// Result of a color range check.
#[derive(Debug, Clone, Default)]
pub struct RangeCheckResult {
    /// Number of pixels with values below the low threshold.
    pub low_count: u64,
    /// Number of pixels with values above the high threshold.
    pub high_count: u64,
    /// Number of pixels within range.
    pub in_range_count: u64,
}

/// Check how many pixels have values outside a given range.
///
/// # Arguments
///
/// * `src` - Source image
/// * `low` - Low threshold per channel
/// * `high` - High threshold per channel
/// * `roi` - Optional region of interest
pub fn color_range_check(
    src: &ImageBuf,
    low: &[f32],
    high: &[f32],
    roi: Option<Roi3D>,
) -> RangeCheckResult {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;

    let mut result = RangeCheckResult::default();
    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                let mut is_low = false;
                let mut is_high = false;

                for c in 0..nch {
                    let lo = low.get(c).copied().unwrap_or(f32::NEG_INFINITY);
                    let hi = high.get(c).copied().unwrap_or(f32::INFINITY);

                    if pixel[c] < lo {
                        is_low = true;
                    }
                    if pixel[c] > hi {
                        is_high = true;
                    }
                }

                if is_low {
                    result.low_count += 1;
                } else if is_high {
                    result.high_count += 1;
                } else {
                    result.in_range_count += 1;
                }
            }
        }
    }

    result
}

// ============================================================================
// Color Counting
// ============================================================================

/// Count pixels matching specific colors.
///
/// For each color in the list, counts how many pixels in the image match
/// that color within the specified epsilon tolerance.
///
/// # Arguments
/// * `src` - Source image
/// * `colors` - List of colors to count (flattened: [r1,g1,b1,a1,r2,g2,b2,a2,...])
/// * `epsilon` - Tolerance per channel for color matching
/// * `roi` - Region of interest (or None for full image)
///
/// # Returns
/// Vector with count for each color
///
/// # Example
/// ```ignore
/// use vfx_io::imagebuf::ImageBuf;
/// use vfx_io::imagebufalgo::color_count;
///
/// let img = ImageBuf::read("image.exr").unwrap();
/// // Count red and green pixels
/// let colors = vec![1.0, 0.0, 0.0, 1.0,  // red
///                   0.0, 1.0, 0.0, 1.0]; // green
/// let epsilon = vec![0.01, 0.01, 0.01, 0.01];
/// let counts = color_count(&img, &colors, &epsilon, None);
/// println!("Red pixels: {}, Green pixels: {}", counts[0], counts[1]);
/// ```
pub fn color_count(src: &ImageBuf, colors: &[f32], epsilon: &[f32], roi: Option<Roi3D>) -> Vec<u64> {
    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;

    // Calculate number of colors
    if colors.is_empty() || nch == 0 {
        return Vec::new();
    }
    let ncolors = colors.len() / nch;

    // Extend epsilon to nch if needed
    let mut eps = vec![0.001f32; nch];
    for (i, &e) in epsilon.iter().enumerate() {
        if i < nch {
            eps[i] = e;
        }
    }
    // Fill remaining with last epsilon or default
    if !epsilon.is_empty() {
        let last = *epsilon.last().unwrap();
        for e in eps.iter_mut().skip(epsilon.len()) {
            *e = last;
        }
    }

    let mut counts = vec![0u64; ncolors];
    let mut pixel = vec![0.0f32; nch];

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                // Check each color
                for col in 0..ncolors {
                    let color_offset = col * nch;
                    let mut matches = true;

                    for c in 0..nch {
                        if (pixel[c] - colors[color_offset + c]).abs() > eps[c] {
                            matches = false;
                            break;
                        }
                    }

                    if matches {
                        counts[col] += 1;
                    }
                }
            }
        }
    }

    counts
}

/// Count unique colors in an image.
///
/// Returns the number of distinct pixel values in the image.
/// For images with many colors, this may require significant memory.
///
/// # Arguments
/// * `src` - Source image
/// * `roi` - Region of interest (or None for full image)
///
/// # Returns
/// Number of unique colors found
pub fn unique_color_count(src: &ImageBuf, roi: Option<Roi3D>) -> usize {
    use std::collections::HashSet;
    use std::hash::{Hash, Hasher};

    let roi = roi.unwrap_or_else(|| src.roi());
    let nch = src.nchannels() as usize;

    // Use a wrapper for hashing floats (quantized to avoid floating point issues)
    #[derive(Clone, Eq, PartialEq)]
    struct QuantizedColor(Vec<i64>);

    impl Hash for QuantizedColor {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.0.hash(state);
        }
    }

    let mut unique = HashSet::new();
    let mut pixel = vec![0.0f32; nch];

    // Quantize to avoid floating point comparison issues (1/65536 precision)
    let scale = 65536.0;

    for z in roi.zbegin..roi.zend {
        for y in roi.ybegin..roi.yend {
            for x in roi.xbegin..roi.xend {
                src.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                let quantized: Vec<i64> = pixel.iter()
                    .map(|&v| (v * scale).round() as i64)
                    .collect();

                unique.insert(QuantizedColor(quantized));
            }
        }
    }

    unique.len()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imagebuf::InitializePixels;
    use vfx_core::ImageSpec;

    #[test]
    fn test_compute_pixel_stats() {
        let spec = ImageSpec::gray(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        // Fill with values 0 to 99
        for y in 0..10 {
            for x in 0..10 {
                let val = (y * 10 + x) as f32;
                src.setpixel(x, y, 0, &[val]);
            }
        }

        let stats = compute_pixel_stats(&src, None);

        assert!((stats.min[0] - 0.0).abs() < 0.001);
        assert!((stats.max[0] - 99.0).abs() < 0.001);
        assert!((stats.avg[0] - 49.5).abs() < 0.001);
        assert_eq!(stats.finite_count[0], 100);
    }

    #[test]
    fn test_compare() {
        let spec = ImageSpec::gray(10, 10);
        let mut a = ImageBuf::new(spec.clone(), InitializePixels::No);
        let mut b = ImageBuf::new(spec, InitializePixels::No);

        // Fill with same values
        for y in 0..10 {
            for x in 0..10 {
                let val = (y * 10 + x) as f32 / 100.0;
                a.setpixel(x, y, 0, &[val]);
                b.setpixel(x, y, 0, &[val]);
            }
        }

        let results = compare(&a, &b, 0.001, 0.0001, None);
        assert!((results.max_error).abs() < 0.001);
        assert_eq!(results.nfail, 0);
    }

    #[test]
    fn test_compare_different() {
        let spec = ImageSpec::gray(10, 10);
        let mut a = ImageBuf::new(spec.clone(), InitializePixels::No);
        let mut b = ImageBuf::new(spec, InitializePixels::No);

        // Fill with different values
        for y in 0..10 {
            for x in 0..10 {
                a.setpixel(x, y, 0, &[0.0]);
                b.setpixel(x, y, 0, &[1.0]);
            }
        }

        let results = compare(&a, &b, 0.5, 0.1, None);
        assert!((results.max_error - 1.0).abs() < 0.001);
        assert_eq!(results.nfail, 100);
    }

    #[test]
    fn test_is_constant_color() {
        let spec = ImageSpec::rgb(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        // Fill with constant color
        for y in 0..10 {
            for x in 0..10 {
                src.setpixel(x, y, 0, &[0.5, 0.3, 0.1]);
            }
        }

        assert!(is_constant_color(&src, 0.001, None, None));

        // Now change one pixel
        src.setpixel(5, 5, 0, &[1.0, 0.0, 0.0]);
        assert!(!is_constant_color(&src, 0.001, None, None));
    }

    #[test]
    fn test_is_monochrome() {
        let spec = ImageSpec::rgb(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        // Fill with grayscale values
        for y in 0..10 {
            for x in 0..10 {
                let val = (y * 10 + x) as f32 / 100.0;
                src.setpixel(x, y, 0, &[val, val, val]);
            }
        }

        assert!(is_monochrome(&src, 0.001, None));

        // Now make one pixel colored
        src.setpixel(5, 5, 0, &[1.0, 0.0, 0.0]);
        assert!(!is_monochrome(&src, 0.001, None));
    }

    #[test]
    fn test_histogram() {
        let spec = ImageSpec::gray(100, 1);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        // Fill with values 0.0 to 0.99
        for x in 0..100 {
            src.setpixel(x, 0, 0, &[x as f32 / 100.0]);
        }

        let hist = histogram(&src, 0, 10, 0.0, 1.0, None);

        // Each bin should have approximately 10 samples
        for bin in &hist.bins {
            assert!(*bin >= 9 && *bin <= 11);
        }
        assert_eq!(hist.total(), 100);
    }

    #[test]
    fn test_maxchan() {
        let spec = ImageSpec::rgb(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        src.setpixel(5, 5, 0, &[0.2, 0.8, 0.5]);

        let result = maxchan(&src, None);

        let mut pixel = [0.0f32];
        result.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_minchan() {
        let spec = ImageSpec::rgb(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        src.setpixel(5, 5, 0, &[0.2, 0.8, 0.5]);

        let result = minchan(&src, None);

        let mut pixel = [0.0f32];
        result.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 0.2).abs() < 0.001);
    }

    #[test]
    fn test_color_range_check() {
        let spec = ImageSpec::gray(10, 1);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        // 3 pixels below, 4 in range, 3 above
        for x in 0..10 {
            let val = x as f32 / 10.0;
            src.setpixel(x, 0, 0, &[val]);
        }

        let result = color_range_check(&src, &[0.3], &[0.6], None);

        assert_eq!(result.low_count, 3);  // 0.0, 0.1, 0.2
        assert_eq!(result.in_range_count, 4);  // 0.3, 0.4, 0.5, 0.6
        assert_eq!(result.high_count, 3);  // 0.7, 0.8, 0.9
    }
}
