//! Deep EXR data structures and I/O.
//!
//! Full deep EXR support using vfx-exr (custom exrs fork).

use crate::{IoError, IoResult};
use crate::deepdata::DeepData;
use half::f16;
use std::path::Path;
use vfx_core::TypeDesc;

// Re-export vfx-exr deep types for direct use
pub use vfx_exr::image::deep::{
    DeepSamples as ExrDeepSamples, 
    DeepChannelData as ExrDeepChannelData,
};
pub use vfx_exr::image::read::deep::{
    read_first_deep_layer_from_file as read_exr_deep,
    read_all_deep_layers_from_file as read_exr_deep_all,
    read_deep as read_exr_deep_builder,
    DeepImage, DeepLayersImage,
};
pub use vfx_exr::image::write::deep::{
    write_deep_image_to_file as write_exr_deep,
    write_deep_scanlines_to_file as write_exr_deep_scanlines,
};

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
// Read/Write and conversions
// ============================================================================

/// Reads a deep EXR file.
///
/// Returns deep samples and channel descriptions.
pub fn read_deep_exr<P: AsRef<Path>>(path: P) -> IoResult<(DeepSamples, Vec<DeepChannelDesc>)> {
    use vfx_exr::image::read::deep::read_first_deep_layer_from_file;
    
    let image = read_first_deep_layer_from_file(path.as_ref())
        .map_err(|e| IoError::DecodeError(format!("Deep EXR read failed: {}", e)))?;
    
    let layer = &image.layer_data;
    let exr_samples = &layer.channel_data.list[0].sample_data;
    
    // Convert vfx-exr DeepSamples to our format
    let mut samples = DeepSamples::new(exr_samples.width, exr_samples.height);
    samples.sample_offsets = exr_samples.sample_offsets.clone();
    
    // Convert channels
    for exr_channel in &exr_samples.channels {
        let channel = match exr_channel {
            ExrDeepChannelData::F16(v) => DeepChannelData::F16(v.clone()),
            ExrDeepChannelData::F32(v) => DeepChannelData::F32(v.clone()),
            ExrDeepChannelData::U32(v) => DeepChannelData::U32(v.clone()),
        };
        samples.channels.push(channel);
    }
    
    // Build channel descriptions
    let channels: Vec<DeepChannelDesc> = layer.channel_data.list.iter()
        .map(|ch| DeepChannelDesc {
            name: ch.name.to_string(),
            sample_type: match ch.sample_data.channels.first() {
                Some(ExrDeepChannelData::F16(_)) => SampleType::F16,
                Some(ExrDeepChannelData::F32(_)) => SampleType::F32,
                Some(ExrDeepChannelData::U32(_)) => SampleType::U32,
                None => SampleType::F32, // default
            },
        })
        .collect();
    
    Ok((samples, channels))
}

/// Converts DeepSamples (SoA) into DeepData (AoS).
///
/// This bridges vfx-exr's deep layout (SoA with cumulative offsets) to
/// vfx-io's OIIO-like DeepData (AoS).
pub fn deep_samples_to_deepdata(
    samples: &DeepSamples,
    channels: &[DeepChannelDesc],
) -> IoResult<DeepData> {
    if samples.sample_offsets.len() != samples.pixel_count() {
        return Err(IoError::InvalidFile(format!(
            "sample offsets length {} != pixel count {}",
            samples.sample_offsets.len(),
            samples.pixel_count()
        )));
    }
    if channels.len() != samples.channels.len() {
        return Err(IoError::InvalidFile(format!(
            "channel count {} != data channels {}",
            channels.len(),
            samples.channels.len()
        )));
    }

    // Convert cumulative offsets into per-pixel sample counts for DeepData.
    let mut counts = Vec::with_capacity(samples.pixel_count());
    let mut prev = 0u32;
    for &cum in &samples.sample_offsets {
        if cum < prev {
            return Err(IoError::InvalidFile("sample offsets not monotonic".into()));
        }
        counts.push(cum - prev);
        prev = cum;
    }

    let channel_types: Vec<TypeDesc> = channels
        .iter()
        .map(|ch| match ch.sample_type {
            SampleType::F16 => TypeDesc::HALF,
            SampleType::F32 => TypeDesc::FLOAT,
            SampleType::U32 => TypeDesc::UINT32,
        })
        .collect();
    let channel_names: Vec<&str> = channels.iter().map(|ch| ch.name.as_str()).collect();

    let deep = DeepData::new(samples.pixel_count() as i64, &channel_types, &channel_names);
    deep.set_all_samples(&counts);

    for (ch_idx, ch_data) in samples.channels.iter().enumerate() {
        for pixel_idx in 0..samples.pixel_count() {
            let (start, end) = samples.sample_range(pixel_idx);
            for (local_sample, global_sample) in (start..end).enumerate() {
                match ch_data {
                    DeepChannelData::F16(values) => {
                        let value = values[global_sample].to_f32();
                        // DeepData stores f16/f32 as f32; keep conversion in one place.
                        deep.set_deep_value_f32(pixel_idx as i64, ch_idx, local_sample, value);
                    }
                    DeepChannelData::F32(values) => {
                        deep.set_deep_value_f32(
                            pixel_idx as i64,
                            ch_idx,
                            local_sample,
                            values[global_sample],
                        );
                    }
                    DeepChannelData::U32(values) => {
                        deep.set_deep_value_u32(
                            pixel_idx as i64,
                            ch_idx,
                            local_sample,
                            values[global_sample],
                        );
                    }
                }
            }
        }
    }

    Ok(deep)
}

/// Converts DeepData (AoS) into DeepSamples (SoA) and channel descriptors.
///
/// The resulting channels are sorted alphabetically to satisfy EXR header rules.
pub fn deepdata_to_deep_samples(
    deep: &DeepData,
    width: usize,
    height: usize,
) -> IoResult<(DeepSamples, Vec<DeepChannelDesc>)> {
    let pixel_count = width * height;
    if deep.pixels() != pixel_count as i64 {
        return Err(IoError::InvalidFile(format!(
            "deep pixels {} != width*height {}",
            deep.pixels(),
            pixel_count
        )));
    }

    let counts = deep.all_samples();
    if counts.len() != pixel_count {
        return Err(IoError::InvalidFile(format!(
            "sample count length {} != pixel count {}",
            counts.len(),
            pixel_count
        )));
    }

    // Convert per-pixel counts into cumulative offsets for SoA layout.
    let mut sample_offsets = Vec::with_capacity(pixel_count);
    let mut total = 0u32;
    for count in &counts {
        total = total
            .checked_add(*count)
            .ok_or_else(|| IoError::InvalidFile("sample count overflow".into()))?;
        sample_offsets.push(total);
    }

    // Build and sort channel descriptors to meet EXR channel ordering requirements.
    let mut channel_meta = Vec::with_capacity(deep.channels());
    for idx in 0..deep.channels() {
        let name = deep.channelname(idx);
        let sample_type = match deep.channeltype(idx).basetype {
            vfx_core::BaseType::Half => SampleType::F16,
            vfx_core::BaseType::Float => SampleType::F32,
            vfx_core::BaseType::UInt32 => SampleType::U32,
            other => {
                return Err(IoError::UnsupportedFeature(format!(
                    "unsupported deep channel type: {:?}",
                    other
                )));
            }
        };
        channel_meta.push((name, sample_type, idx));
    }
    channel_meta.sort_by(|a, b| a.0.cmp(&b.0));

    let channel_descs: Vec<DeepChannelDesc> = channel_meta
        .iter()
        .map(|(name, sample_type, _)| DeepChannelDesc {
            name: name.clone(),
            sample_type: *sample_type,
        })
        .collect();

    let total_samples = total as usize;
    let mut channels = Vec::with_capacity(deep.channels());
    for (sorted_idx, ch_desc) in channel_descs.iter().enumerate() {
        let original_idx = channel_meta[sorted_idx].2;
        match ch_desc.sample_type {
            SampleType::F16 => {
                let mut data = vec![f16::ZERO; total_samples];
                for pixel_idx in 0..pixel_count {
                    let count = counts[pixel_idx] as usize;
                    let start = if pixel_idx == 0 {
                        0
                    } else {
                        sample_offsets[pixel_idx - 1] as usize
                    };
                    for sample_idx in 0..count {
                        let value = deep.deep_value(pixel_idx as i64, original_idx, sample_idx);
                        data[start + sample_idx] = f16::from_f32(value);
                    }
                }
                channels.push(DeepChannelData::F16(data));
            }
            SampleType::F32 => {
                let mut data = vec![0.0f32; total_samples];
                for pixel_idx in 0..pixel_count {
                    let count = counts[pixel_idx] as usize;
                    let start = if pixel_idx == 0 {
                        0
                    } else {
                        sample_offsets[pixel_idx - 1] as usize
                    };
                    for sample_idx in 0..count {
                        data[start + sample_idx] =
                            deep.deep_value(pixel_idx as i64, original_idx, sample_idx);
                    }
                }
                channels.push(DeepChannelData::F32(data));
            }
            SampleType::U32 => {
                let mut data = vec![0u32; total_samples];
                for pixel_idx in 0..pixel_count {
                    let count = counts[pixel_idx] as usize;
                    let start = if pixel_idx == 0 {
                        0
                    } else {
                        sample_offsets[pixel_idx - 1] as usize
                    };
                    for sample_idx in 0..count {
                        data[start + sample_idx] =
                            deep.deep_value_uint(pixel_idx as i64, original_idx, sample_idx);
                    }
                }
                channels.push(DeepChannelData::U32(data));
            }
        }
    }

    Ok((
        DeepSamples {
            sample_offsets,
            channels,
            width,
            height,
        },
        channel_descs,
    ))
}

/// Writes deep samples to an EXR file.
///
/// This expects channels to be sorted (caller should pass sorted descriptors).
pub fn write_deep_exr<P: AsRef<Path>>(
    path: P,
    samples: &DeepSamples,
    channels: &[DeepChannelDesc],
    compression: vfx_exr::meta::attribute::Compression,
) -> IoResult<()> {
    use vfx_exr::meta::attribute::{ChannelDescription, ChannelList, SampleType as ExrSampleType, Text};
    use vfx_exr::image::write::deep::write_deep_scanlines_to_file;
    use vfx_exr::prelude::SmallVec;

    if channels.len() != samples.channels.len() {
        return Err(IoError::InvalidFile(format!(
            "channel count {} != data channels {}",
            channels.len(),
            samples.channels.len()
        )));
    }

    let mut list: SmallVec<[ChannelDescription; 5]> = SmallVec::new();
    for ch in channels {
        let name = Text::new_or_none(&ch.name).ok_or_else(|| {
            IoError::EncodeError(format!(
                "EXR encode error: channel name contains unsupported characters: {}",
                ch.name
            ))
        })?;
        list.push(ChannelDescription {
            name,
            sample_type: match ch.sample_type {
                SampleType::F16 => ExrSampleType::F16,
                SampleType::F32 => ExrSampleType::F32,
                SampleType::U32 => ExrSampleType::U32,
            },
            quantize_linearly: false,
            sampling: vfx_exr::math::Vec2(1, 1),
        });
    }
    let channel_list = ChannelList::new(list);

    // Convert vfx-io DeepSamples to vfx-exr DeepSamples for writing.
    let mut exr_samples = vfx_exr::image::deep::DeepSamples {
        sample_offsets: samples.sample_offsets.clone(),
        channels: Vec::with_capacity(samples.channels.len()),
        width: samples.width,
        height: samples.height,
    };
    for ch in &samples.channels {
        let converted = match ch {
            DeepChannelData::F16(v) => ExrDeepChannelData::F16(v.clone()),
            DeepChannelData::F32(v) => ExrDeepChannelData::F32(v.clone()),
            DeepChannelData::U32(v) => ExrDeepChannelData::U32(v.clone()),
        };
        exr_samples.channels.push(converted);
    }

    write_deep_scanlines_to_file(path, &exr_samples, &channel_list, compression)
        .map_err(|e| IoError::EncodeError(format!("Deep EXR write failed: {}", e)))?;

    Ok(())
}

// ============================================================================
// Working header-only functions
// ============================================================================

/// Checks if an EXR file contains deep data (reads header only).
pub fn is_deep_exr<P: AsRef<Path>>(path: P) -> IoResult<bool> {
    let meta = vfx_exr::meta::MetaData::read_from_file(path.as_ref(), false)
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
    let meta = vfx_exr::meta::MetaData::read_from_file(path.as_ref(), false)
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
