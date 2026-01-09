//! Deep EXR data structures and I/O.
//!
//! Copied and adapted from custom exrs library for standalone use.

use crate::{IoError, IoResult};
use half::f16;
use std::path::Path;

// ============================================================================
// DeepChannelData - sample data for a single channel
// ============================================================================

/// Sample data for a single channel in a deep image.
#[derive(Debug, Clone, PartialEq)]
pub enum DeepChannelData {
    /// 16-bit float samples.
    F16(Vec<f16>),
    /// 32-bit float samples.
    F32(Vec<f32>),
    /// 32-bit unsigned integer samples.
    U32(Vec<u32>),
}

impl DeepChannelData {
    /// Number of samples in this channel.
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            DeepChannelData::F16(v) => v.len(),
            DeepChannelData::F32(v) => v.len(),
            DeepChannelData::U32(v) => v.len(),
        }
    }

    /// Check if channel is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get sample type of this channel.
    pub fn sample_type(&self) -> SampleType {
        match self {
            DeepChannelData::F16(_) => SampleType::F16,
            DeepChannelData::F32(_) => SampleType::F32,
            DeepChannelData::U32(_) => SampleType::U32,
        }
    }

    /// Get f16 value at index.
    #[inline]
    pub fn get_f16(&self, index: usize) -> f16 {
        match self {
            DeepChannelData::F16(v) => v[index],
            _ => panic!("channel is not F16"),
        }
    }

    /// Get f32 value at index.
    #[inline]
    pub fn get_f32(&self, index: usize) -> f32 {
        match self {
            DeepChannelData::F32(v) => v[index],
            _ => panic!("channel is not F32"),
        }
    }

    /// Get u32 value at index.
    #[inline]
    pub fn get_u32(&self, index: usize) -> u32 {
        match self {
            DeepChannelData::U32(v) => v[index],
            _ => panic!("channel is not U32"),
        }
    }

    /// Get mutable f16 slice.
    pub fn as_f16_mut(&mut self) -> Option<&mut Vec<f16>> {
        match self {
            DeepChannelData::F16(v) => Some(v),
            _ => None,
        }
    }

    /// Get mutable f32 slice.
    pub fn as_f32_mut(&mut self) -> Option<&mut Vec<f32>> {
        match self {
            DeepChannelData::F32(v) => Some(v),
            _ => None,
        }
    }

    /// Get mutable u32 slice.
    pub fn as_u32_mut(&mut self) -> Option<&mut Vec<u32>> {
        match self {
            DeepChannelData::U32(v) => Some(v),
            _ => None,
        }
    }

    /// Bytes per sample element.
    pub fn bytes_per_sample(&self) -> usize {
        match self {
            DeepChannelData::F16(_) => 2,
            DeepChannelData::F32(_) => 4,
            DeepChannelData::U32(_) => 4,
        }
    }
}

// ============================================================================
// SampleType
// ============================================================================

/// Sample type for deep channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleType {
    /// 16-bit floating point.
    F16,
    /// 32-bit floating point.
    F32,
    /// 32-bit unsigned integer.
    U32,
}

impl SampleType {
    /// Bytes per sample.
    pub fn bytes_per_sample(&self) -> usize {
        match self {
            SampleType::F16 => 2,
            SampleType::F32 => 4,
            SampleType::U32 => 4,
        }
    }
}

// ============================================================================
// DeepSamples - SoA storage for deep image data
// ============================================================================

/// Deep samples storage using Struct-of-Arrays (SoA) layout.
///
/// Uses cumulative offsets (prefix sums) for O(1) sample lookup.
#[derive(Debug, Clone, PartialEq)]
pub struct DeepSamples {
    /// Cumulative sample counts per pixel. Length = width * height.
    pub sample_offsets: Vec<u32>,
    /// Channel data in Struct-of-Arrays layout.
    pub channels: Vec<DeepChannelData>,
    /// Image width in pixels.
    pub width: usize,
    /// Image height in pixels.
    pub height: usize,
}

impl DeepSamples {
    /// Create empty deep samples for given dimensions.
    pub fn new(width: usize, height: usize) -> Self {
        let pixel_count = width * height;
        Self {
            sample_offsets: vec![0; pixel_count],
            channels: Vec::new(),
            width,
            height,
        }
    }

    /// Total number of pixels.
    #[inline]
    pub fn pixel_count(&self) -> usize {
        self.width * self.height
    }

    /// Total number of samples across all pixels.
    #[inline]
    pub fn total_samples(&self) -> usize {
        self.sample_offsets.last().copied().unwrap_or(0) as usize
    }

    /// Get sample count at pixel (x, y).
    #[inline]
    pub fn sample_count(&self, x: usize, y: usize) -> usize {
        let idx = y * self.width + x;
        self.sample_count_at_index(idx)
    }

    /// Get sample count at linear pixel index.
    #[inline]
    pub fn sample_count_at_index(&self, idx: usize) -> usize {
        if idx >= self.sample_offsets.len() {
            return 0;
        }
        let end = self.sample_offsets[idx] as usize;
        let start = if idx == 0 {
            0
        } else {
            self.sample_offsets[idx - 1] as usize
        };
        end - start
    }

    /// Get sample range (start, end) for pixel at linear index.
    #[inline]
    pub fn sample_range(&self, idx: usize) -> (usize, usize) {
        if idx >= self.sample_offsets.len() {
            return (0, 0);
        }
        let end = self.sample_offsets[idx] as usize;
        let start = if idx == 0 {
            0
        } else {
            self.sample_offsets[idx - 1] as usize
        };
        (start, end)
    }

    /// Set sample offsets from cumulative counts.
    pub fn set_cumulative_counts(&mut self, counts: Vec<u32>) -> IoResult<()> {
        // Validate monotonic
        let mut prev = 0u32;
        for (i, &count) in counts.iter().enumerate() {
            if count < prev {
                return Err(IoError::InvalidFile(format!(
                    "sample counts not monotonic at {}: {} < {}",
                    i, count, prev
                )));
            }
            prev = count;
        }

        if counts.len() != self.pixel_count() {
            return Err(IoError::InvalidFile(format!(
                "sample count length {} != pixel count {}",
                counts.len(),
                self.pixel_count()
            )));
        }

        self.sample_offsets = counts;
        Ok(())
    }

    /// Get maximum samples per pixel.
    pub fn max_samples_per_pixel(&self) -> u32 {
        let mut max = 0u32;
        let mut prev = 0u32;
        for &cum in &self.sample_offsets {
            let count = cum - prev;
            if count > max {
                max = count;
            }
            prev = cum;
        }
        max
    }

    /// Validate that channel data lengths match total_samples.
    pub fn validate(&self) -> IoResult<()> {
        let total = self.total_samples();

        for (i, channel) in self.channels.iter().enumerate() {
            let len = channel.len();
            if len != total {
                return Err(IoError::InvalidFile(format!(
                    "channel {} has {} samples, expected {}",
                    i, len, total
                )));
            }
        }

        Ok(())
    }
}

// ============================================================================
// Channel description for deep data
// ============================================================================

/// Channel description for deep EXR.
#[derive(Debug, Clone)]
pub struct DeepChannelDesc {
    /// Channel name (e.g., "R", "G", "B", "A", "Z").
    pub name: String,
    /// Sample data type.
    pub sample_type: SampleType,
}

// ============================================================================
// Read/Write stubs - require low-level EXR implementation
// ============================================================================

/// Reads a deep EXR file.
///
/// **Note**: Full implementation requires deep block decompression which
/// is not available in the current exr crate version.
pub fn read_deep_exr<P: AsRef<Path>>(path: P) -> IoResult<(DeepSamples, Vec<DeepChannelDesc>)> {
    // Check if file is actually deep
    let meta = exr::meta::MetaData::read_from_file(path.as_ref(), false)
        .map_err(|e| IoError::DecodeError(format!("EXR read failed: {}", e)))?;

    let header = meta
        .headers
        .first()
        .ok_or_else(|| IoError::DecodeError("EXR has no layers".into()))?;

    if !header.deep {
        return Err(IoError::InvalidFile("Not a deep EXR file".into()));
    }

    // For now, we can only read the structure, not the actual deep data
    // because the exr crate on crates.io doesn't support deep block decompression
    Err(IoError::UnsupportedFeature(
        "Deep EXR reading requires updated exrs crate with deep data support. \
         Use is_deep_exr() and probe_deep_exr() for metadata inspection."
            .into(),
    ))
}

/// Writes deep samples to an EXR file.
///
/// **Note**: Full implementation requires deep block compression which
/// is not available in the current exr crate version.
pub fn write_deep_exr<P: AsRef<Path>>(
    _path: P,
    _samples: &DeepSamples,
    _channels: &[DeepChannelDesc],
) -> IoResult<()> {
    Err(IoError::UnsupportedFeature(
        "Deep EXR writing requires updated exrs crate with deep data support.".into(),
    ))
}

// ============================================================================
// Working header-only functions
// ============================================================================

/// Checks if an EXR file contains deep data (reads header only).
pub fn is_deep_exr<P: AsRef<Path>>(path: P) -> IoResult<bool> {
    let meta = exr::meta::MetaData::read_from_file(path.as_ref(), false)
        .map_err(|e| IoError::DecodeError(format!("EXR probe failed: {}", e)))?;

    Ok(meta.headers.iter().any(|h| h.deep))
}

/// Statistics about a deep EXR file.
#[derive(Debug, Clone, Default)]
pub struct DeepExrStats {
    /// Image width.
    pub width: u32,
    /// Image height.
    pub height: u32,
    /// Number of channels.
    pub channels: usize,
    /// Channel names.
    pub channel_names: Vec<String>,
    /// Maximum samples in any pixel (from header).
    pub max_samples_per_pixel: u32,
    /// Compression method.
    pub compression: String,
}

/// Gets statistics about a deep EXR file without loading data.
pub fn probe_deep_exr<P: AsRef<Path>>(path: P) -> IoResult<DeepExrStats> {
    let meta = exr::meta::MetaData::read_from_file(path.as_ref(), false)
        .map_err(|e| IoError::DecodeError(format!("EXR probe failed: {}", e)))?;

    let header = meta
        .headers
        .first()
        .ok_or_else(|| IoError::DecodeError("EXR has no layers".into()))?;

    if !header.deep {
        return Err(IoError::InvalidFile("Not a deep EXR file".into()));
    }

    let channel_names: Vec<String> = header
        .channels
        .list
        .iter()
        .map(|ch| ch.name.to_string())
        .collect();

    Ok(DeepExrStats {
        width: header.layer_size.0 as u32,
        height: header.layer_size.1 as u32,
        channels: header.channels.list.len(),
        channel_names,
        max_samples_per_pixel: header.max_samples_per_pixel.unwrap_or(0) as u32,
        compression: format!("{:?}", header.compression),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deep_samples_basic() {
        let mut samples = DeepSamples::new(2, 2);

        // Set cumulative counts: [2, 2, 5, 6]
        // Means: pixel 0 has 2, pixel 1 has 0, pixel 2 has 3, pixel 3 has 1
        samples.set_cumulative_counts(vec![2, 2, 5, 6]).unwrap();

        assert_eq!(samples.total_samples(), 6);
        assert_eq!(samples.sample_count(0, 0), 2);
        assert_eq!(samples.sample_count(1, 0), 0);
        assert_eq!(samples.sample_count(0, 1), 3);
        assert_eq!(samples.sample_count(1, 1), 1);
        assert_eq!(samples.max_samples_per_pixel(), 3);
    }

    #[test]
    fn test_deep_channel_data() {
        let mut channel = DeepChannelData::F32(vec![1.0, 2.0, 3.0]);
        assert_eq!(channel.len(), 3);
        assert_eq!(channel.sample_type(), SampleType::F32);
        assert_eq!(channel.get_f32(1), 2.0);

        if let Some(v) = channel.as_f32_mut() {
            v[0] = 10.0;
        }
        assert_eq!(channel.get_f32(0), 10.0);
    }
}
