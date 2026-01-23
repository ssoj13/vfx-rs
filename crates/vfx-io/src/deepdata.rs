//! OIIO-compatible DeepData implementation.
//!
//! DeepData holds the contents of an image of "deep" pixels - pixels with
//! multiple depth samples. This is commonly used in deep compositing workflows
//! where each pixel can have multiple layers at different Z depths.
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::deepdata::DeepData;
//! use vfx_core::{TypeDesc, BaseType};
//!
//! // Create a DeepData with 100 pixels, 5 channels (RGBA + Z)
//! let channel_types = vec![
//!     TypeDesc::FLOAT, // R
//!     TypeDesc::FLOAT, // G
//!     TypeDesc::FLOAT, // B
//!     TypeDesc::FLOAT, // A
//!     TypeDesc::FLOAT, // Z
//! ];
//! let channel_names = vec!["R", "G", "B", "A", "Z"];
//! let mut deep = DeepData::new(100, &channel_types, &channel_names);
//!
//! // Set samples for first pixel
//! deep.set_samples(0, 2); // 2 depth samples
//!
//! // Set values for first sample
//! deep.set_deep_value(0, 0, 0, 1.0); // R = 1.0
//! deep.set_deep_value(0, 4, 0, 0.5); // Z = 0.5
//! ```

use std::sync::RwLock;

use vfx_core::{BaseType, ImageSpec, TypeDesc};

/// DeepData holds pixel data with multiple samples per pixel at different depths.
#[derive(Debug)]
pub struct DeepData {
    inner: RwLock<DeepDataInner>,
}

#[derive(Debug, Clone)]
struct DeepDataInner {
    /// Number of pixels
    npixels: i64,
    /// Number of channels
    nchannels: usize,
    /// Type for each channel
    channeltypes: Vec<TypeDesc>,
    /// Size in bytes for each channel
    channelsizes: Vec<usize>,
    /// Byte offset within a sample for each channel
    channeloffsets: Vec<usize>,
    /// Name for each channel
    channelnames: Vec<String>,
    /// Number of samples for each pixel
    nsamples: Vec<u32>,
    /// Capacity (allocated samples) for each pixel
    capacity: Vec<u32>,
    /// Cumulative capacity before each pixel
    cumcapacity: Vec<u32>,
    /// Raw sample data
    data: Vec<u8>,
    /// Total size of one sample in bytes
    samplesize: usize,
    /// Z channel index (-1 if not present)
    z_channel: i32,
    /// Zback channel index (-1 if not present, equals z_channel if no Zback)
    zback_channel: i32,
    /// Alpha channel index (-1 if not present)
    alpha_channel: i32,
    /// AR (red alpha) channel index
    ar_channel: i32,
    /// AG (green alpha) channel index
    ag_channel: i32,
    /// AB (blue alpha) channel index
    ab_channel: i32,
    /// Whether data has been allocated
    allocated: bool,
}

impl Default for DeepDataInner {
    fn default() -> Self {
        Self {
            npixels: 0,
            nchannels: 0,
            channeltypes: Vec::new(),
            channelsizes: Vec::new(),
            channeloffsets: Vec::new(),
            channelnames: Vec::new(),
            nsamples: Vec::new(),
            capacity: Vec::new(),
            cumcapacity: Vec::new(),
            data: Vec::new(),
            samplesize: 0,
            z_channel: -1,
            zback_channel: -1,
            alpha_channel: -1,
            ar_channel: -1,
            ag_channel: -1,
            ab_channel: -1,
            allocated: false,
        }
    }
}

impl DeepData {
    /// Creates an empty DeepData.
    pub fn new_empty() -> Self {
        Self {
            inner: RwLock::new(DeepDataInner::default()),
        }
    }

    /// Creates DeepData from an ImageSpec.
    pub fn from_spec(spec: &ImageSpec) -> Self {
        let dd = Self::new_empty();
        dd.init_from_spec(spec);
        dd
    }

    /// Creates DeepData with the specified configuration.
    pub fn new(npix: i64, channeltypes: &[TypeDesc], channelnames: &[&str]) -> Self {
        let dd = Self::new_empty();
        let names: Vec<String> = channelnames.iter().map(|s| s.to_string()).collect();
        dd.init(npix, channeltypes, &names);
        dd
    }

    /// Resets DeepData to empty state.
    pub fn clear(&self) {
        let mut inner = self.inner.write().unwrap();
        *inner = DeepDataInner::default();
    }

    /// Frees all allocated memory.
    pub fn free(&self) {
        let mut inner = self.inner.write().unwrap();
        inner.data = Vec::new();
        inner.data.shrink_to_fit();
        inner.nsamples = Vec::new();
        inner.nsamples.shrink_to_fit();
        inner.capacity = Vec::new();
        inner.capacity.shrink_to_fit();
        inner.cumcapacity = Vec::new();
        inner.cumcapacity.shrink_to_fit();
        *inner = DeepDataInner::default();
    }

    /// Initializes DeepData with the specified configuration.
    pub fn init(&self, npix: i64, channeltypes: &[TypeDesc], channelnames: &[String]) {
        let mut inner = self.inner.write().unwrap();
        inner.npixels = npix;
        inner.nchannels = channeltypes.len();
        inner.channeltypes = channeltypes.to_vec();
        inner.channelnames = channelnames.to_vec();

        // Calculate channel sizes and offsets
        inner.channelsizes.clear();
        inner.channeloffsets.clear();
        let mut offset = 0usize;
        for ct in channeltypes {
            let size = ct.size();
            inner.channelsizes.push(size);
            inner.channeloffsets.push(offset);
            offset += size;
        }
        inner.samplesize = offset;

        // Initialize per-pixel arrays
        inner.nsamples = vec![0u32; npix as usize];
        inner.capacity = vec![0u32; npix as usize];
        inner.cumcapacity = vec![0u32; npix as usize];

        // Find special channels
        inner.z_channel = -1;
        inner.zback_channel = -1;
        inner.alpha_channel = -1;
        inner.ar_channel = -1;
        inner.ag_channel = -1;
        inner.ab_channel = -1;

        for (i, name) in channelnames.iter().enumerate() {
            let name_lower = name.to_lowercase();
            match name_lower.as_str() {
                "z" => inner.z_channel = i as i32,
                "zback" => inner.zback_channel = i as i32,
                "a" | "alpha" => inner.alpha_channel = i as i32,
                "ar" => inner.ar_channel = i as i32,
                "ag" => inner.ag_channel = i as i32,
                "ab" => inner.ab_channel = i as i32,
                _ => {}
            }
        }

        // If no Zback, use Z
        if inner.zback_channel < 0 && inner.z_channel >= 0 {
            inner.zback_channel = inner.z_channel;
        }

        inner.allocated = false;
    }

    /// Initializes DeepData from an ImageSpec.
    pub fn init_from_spec(&self, spec: &ImageSpec) {
        let npix = spec.width as i64 * spec.height as i64 * spec.depth.max(1) as i64;

        // Build channel types
        let channeltypes: Vec<TypeDesc> = if spec.channelformats.is_empty() {
            vec![TypeDesc::from_basetype(BaseType::Float); spec.nchannels as usize]
        } else {
            spec.channelformats.iter().map(|&f| TypeDesc::from_format(f)).collect()
        };

        // Build channel names
        let channelnames: Vec<String> = if spec.channel_names.is_empty() {
            (0..spec.nchannels as usize)
                .map(|i| format!("channel{}", i))
                .collect()
        } else {
            spec.channel_names.clone()
        };

        self.init(npix, &channeltypes, &channelnames);
    }

    /// Returns whether DeepData is initialized.
    pub fn initialized(&self) -> bool {
        let inner = self.inner.read().unwrap();
        inner.npixels > 0
    }

    /// Returns whether data has been allocated.
    pub fn allocated(&self) -> bool {
        let inner = self.inner.read().unwrap();
        inner.allocated
    }

    /// Returns the total number of pixels.
    pub fn pixels(&self) -> i64 {
        let inner = self.inner.read().unwrap();
        inner.npixels
    }

    /// Returns the number of channels.
    pub fn channels(&self) -> usize {
        let inner = self.inner.read().unwrap();
        inner.nchannels
    }

    /// Returns the Z channel index, or -1 if not present.
    pub fn z_channel(&self) -> i32 {
        let inner = self.inner.read().unwrap();
        inner.z_channel
    }

    /// Returns the Zback channel index (or Z if no Zback).
    pub fn zback_channel(&self) -> i32 {
        let inner = self.inner.read().unwrap();
        inner.zback_channel
    }

    /// Returns the alpha channel index, or -1 if not present.
    pub fn a_channel(&self) -> i32 {
        let inner = self.inner.read().unwrap();
        inner.alpha_channel
    }

    /// Returns the AR channel index (or A if no AR).
    pub fn ar_channel(&self) -> i32 {
        let inner = self.inner.read().unwrap();
        if inner.ar_channel >= 0 {
            inner.ar_channel
        } else {
            inner.alpha_channel
        }
    }

    /// Returns the AG channel index (or A if no AG).
    pub fn ag_channel(&self) -> i32 {
        let inner = self.inner.read().unwrap();
        if inner.ag_channel >= 0 {
            inner.ag_channel
        } else {
            inner.alpha_channel
        }
    }

    /// Returns the AB channel index (or A if no AB).
    pub fn ab_channel(&self) -> i32 {
        let inner = self.inner.read().unwrap();
        if inner.ab_channel >= 0 {
            inner.ab_channel
        } else {
            inner.alpha_channel
        }
    }

    /// Returns the name of channel `c`.
    pub fn channelname(&self, c: usize) -> String {
        let inner = self.inner.read().unwrap();
        inner.channelnames.get(c).cloned().unwrap_or_default()
    }

    /// Returns the type of channel `c`.
    pub fn channeltype(&self, c: usize) -> TypeDesc {
        let inner = self.inner.read().unwrap();
        inner.channeltypes.get(c).cloned().unwrap_or_default()
    }

    /// Returns the size in bytes of one sample of channel `c`.
    pub fn channelsize(&self, c: usize) -> usize {
        let inner = self.inner.read().unwrap();
        inner.channelsizes.get(c).copied().unwrap_or(0)
    }

    /// Returns the size in bytes of all channels for one sample.
    pub fn samplesize(&self) -> usize {
        let inner = self.inner.read().unwrap();
        inner.samplesize
    }

    /// Returns whether this DeepData has the same channel types as `other`.
    pub fn same_channeltypes(&self, other: &DeepData) -> bool {
        let inner = self.inner.read().unwrap();
        let other_inner = other.inner.read().unwrap();
        inner.channeltypes == other_inner.channeltypes
    }

    /// Returns the number of samples for the given pixel.
    pub fn samples(&self, pixel: i64) -> u32 {
        let inner = self.inner.read().unwrap();
        inner.nsamples.get(pixel as usize).copied().unwrap_or(0)
    }

    /// Sets the number of samples for the given pixel.
    pub fn set_samples(&self, pixel: i64, samps: u32) {
        let mut inner = self.inner.write().unwrap();
        if (pixel as usize) < inner.nsamples.len() {
            inner.nsamples[pixel as usize] = samps;
            // Ensure capacity is at least as large as samples
            if inner.capacity[pixel as usize] < samps {
                inner.capacity[pixel as usize] = samps;
            }
        }
    }

    /// Sets the number of samples for all pixels.
    pub fn set_all_samples(&self, samples: &[u32]) {
        let mut inner = self.inner.write().unwrap();
        if samples.len() == inner.nsamples.len() {
            inner.nsamples = samples.to_vec();
            // Ensure capacity is at least as large as samples
            for i in 0..samples.len() {
                if inner.capacity[i] < samples[i] {
                    inner.capacity[i] = samples[i];
                }
            }
        }
    }

    /// Sets the capacity for the given pixel.
    ///
    /// If data is already allocated and the new capacity is larger,
    /// this will reallocate and move data to accommodate the change.
    pub fn set_capacity(&self, pixel: i64, cap: u32) {
        let mut inner = self.inner.write().unwrap();
        let pidx = pixel as usize;
        if pidx >= inner.capacity.len() {
            return;
        }

        let old_cap = inner.capacity[pidx];
        if cap == old_cap {
            return;
        }

        inner.capacity[pidx] = cap;

        // If not yet allocated, just update capacity array
        if !inner.allocated {
            return;
        }

        // Need to reallocate and move data
        Self::reallocate_for_pixel(&mut inner, pidx, old_cap, cap);
    }

    /// Returns the capacity for the given pixel.
    pub fn capacity(&self, pixel: i64) -> u32 {
        let inner = self.inner.read().unwrap();
        inner.capacity.get(pixel as usize).copied().unwrap_or(0)
    }

    /// Allocates data storage if not already done.
    fn ensure_allocated(&self) {
        let mut inner = self.inner.write().unwrap();
        Self::ensure_allocated_inner(&mut inner);
    }

    /// Internal allocation helper that works with mutable reference.
    fn ensure_allocated_inner(inner: &mut DeepDataInner) {
        if inner.allocated {
            return;
        }

        // Calculate cumulative capacities
        let mut total: u32 = 0;
        for i in 0..inner.npixels as usize {
            inner.cumcapacity[i] = total;
            total += inner.capacity[i];
        }

        // Allocate data
        inner.data = vec![0u8; (total as usize) * inner.samplesize];
        inner.allocated = true;
    }

    /// Reallocates data when a pixel's capacity changes.
    ///
    /// Moves all subsequent pixel data to accommodate the change.
    fn reallocate_for_pixel(inner: &mut DeepDataInner, pidx: usize, old_cap: u32, new_cap: u32) {
        if !inner.allocated {
            return;
        }

        let samplesize = inner.samplesize;
        let cap_delta = new_cap as i64 - old_cap as i64;
        let byte_delta = cap_delta * samplesize as i64;

        if byte_delta == 0 {
            return;
        }

        // Get old offset for this pixel's data
        let old_pixel_offset = inner.cumcapacity[pidx] as usize * samplesize;
        let old_pixel_end = old_pixel_offset + old_cap as usize * samplesize;

        if byte_delta > 0 {
            // Growing: need to expand data and move subsequent pixels forward
            let growth = byte_delta as usize;
            inner.data.resize(inner.data.len() + growth, 0);

            // Move all data after this pixel's old end to make room
            if old_pixel_end < inner.data.len() - growth {
                let src_end = inner.data.len() - growth;
                inner.data.copy_within(old_pixel_end..src_end, old_pixel_end + growth);
            }

            // Zero the new capacity space
            let new_space_start = old_pixel_offset + old_cap as usize * samplesize;
            let new_space_end = new_space_start + growth;
            if new_space_end <= inner.data.len() {
                inner.data[new_space_start..new_space_end].fill(0);
            }
        } else {
            // Shrinking: move subsequent pixels backward
            let shrink = (-byte_delta) as usize;

            // Move all data after this pixel's old end backward
            if old_pixel_end < inner.data.len() {
                inner.data.copy_within(old_pixel_end.., old_pixel_end - shrink);
            }

            inner.data.truncate(inner.data.len() - shrink);
        }

        // Update cumulative capacities for all subsequent pixels
        for i in (pidx + 1)..inner.npixels as usize {
            inner.cumcapacity[i] = (inner.cumcapacity[i] as i64 + cap_delta) as u32;
        }
    }

    /// Inserts `n` samples at the given position in the pixel.
    ///
    /// Automatically grows capacity if needed.
    pub fn insert_samples(&self, pixel: i64, samplepos: usize, n: usize) {
        let mut inner = self.inner.write().unwrap();
        Self::ensure_allocated_inner(&mut inner);

        let pidx = pixel as usize;
        if pidx >= inner.nsamples.len() {
            return;
        }

        let old_samples = inner.nsamples[pidx] as usize;
        let new_samples = old_samples + n;

        // Check if we need more capacity and grow if necessary
        let current_cap = inner.capacity[pidx] as usize;
        if new_samples > current_cap {
            // Grow capacity to at least new_samples (with some headroom)
            let new_cap = (new_samples as u32).max(current_cap as u32 * 2).max(8);
            let old_cap = inner.capacity[pidx];
            inner.capacity[pidx] = new_cap;
            Self::reallocate_for_pixel(&mut inner, pidx, old_cap, new_cap);
        }

        // Get data offset for this pixel
        let pixel_offset = inner.cumcapacity[pidx] as usize * inner.samplesize;
        let insert_offset = pixel_offset + samplepos * inner.samplesize;
        let move_size = (old_samples - samplepos) * inner.samplesize;

        // Move existing samples after insertion point
        if move_size > 0 {
            let src = insert_offset;
            let dst = insert_offset + n * inner.samplesize;
            // Use copy_within for safe overlap handling
            inner.data.copy_within(src..src + move_size, dst);
        }

        // Zero the new samples
        let zero_start = insert_offset;
        let zero_end = insert_offset + n * inner.samplesize;
        inner.data[zero_start..zero_end].fill(0);

        inner.nsamples[pidx] = new_samples as u32;
    }

    /// Erases `n` samples from the given position in the pixel.
    pub fn erase_samples(&self, pixel: i64, samplepos: usize, n: usize) {
        let mut inner = self.inner.write().unwrap();
        let pidx = pixel as usize;
        if pidx >= inner.nsamples.len() {
            return;
        }

        let old_samples = inner.nsamples[pidx] as usize;
        if samplepos + n > old_samples {
            return;
        }

        if !inner.allocated {
            // If not allocated, just reduce the count
            inner.nsamples[pidx] = (old_samples - n) as u32;
            return;
        }

        // Get data offset for this pixel
        let pixel_offset = inner.cumcapacity[pidx] as usize * inner.samplesize;
        let erase_offset = pixel_offset + samplepos * inner.samplesize;
        let src = erase_offset + n * inner.samplesize;
        let move_size = (old_samples - samplepos - n) * inner.samplesize;

        // Move samples after erased region
        if move_size > 0 {
            inner.data.copy_within(src..src + move_size, erase_offset);
        }

        inner.nsamples[pidx] = (old_samples - n) as u32;
    }

    /// Gets the value of a sample as f32.
    pub fn deep_value(&self, pixel: i64, channel: usize, sample: usize) -> f32 {
        let inner = self.inner.read().unwrap();
        if !inner.allocated {
            return 0.0;
        }

        let pidx = pixel as usize;
        if pidx >= inner.nsamples.len() || sample >= inner.nsamples[pidx] as usize {
            return 0.0;
        }
        if channel >= inner.nchannels {
            return 0.0;
        }

        let offset = (inner.cumcapacity[pidx] as usize + sample) * inner.samplesize
            + inner.channeloffsets[channel];

        let ct = &inner.channeltypes[channel];
        match ct.basetype {
            BaseType::Float => {
                let bytes: [u8; 4] = inner.data[offset..offset + 4].try_into().unwrap_or([0; 4]);
                f32::from_ne_bytes(bytes)
            }
            BaseType::Half => {
                let bytes: [u8; 2] = inner.data[offset..offset + 2].try_into().unwrap_or([0; 2]);
                half::f16::from_ne_bytes(bytes).to_f32()
            }
            BaseType::UInt8 => inner.data[offset] as f32 / 255.0,
            BaseType::UInt16 => {
                let bytes: [u8; 2] = inner.data[offset..offset + 2].try_into().unwrap_or([0; 2]);
                u16::from_ne_bytes(bytes) as f32 / 65535.0
            }
            BaseType::UInt32 => {
                let bytes: [u8; 4] = inner.data[offset..offset + 4].try_into().unwrap_or([0; 4]);
                u32::from_ne_bytes(bytes) as f32
            }
            BaseType::Int8 => (inner.data[offset] as i8) as f32 / 127.0,
            BaseType::Int16 => {
                let bytes: [u8; 2] = inner.data[offset..offset + 2].try_into().unwrap_or([0; 2]);
                i16::from_ne_bytes(bytes) as f32 / 32767.0
            }
            BaseType::Int32 => {
                let bytes: [u8; 4] = inner.data[offset..offset + 4].try_into().unwrap_or([0; 4]);
                i32::from_ne_bytes(bytes) as f32
            }
            BaseType::Double => {
                let bytes: [u8; 8] = inner.data[offset..offset + 8].try_into().unwrap_or([0; 8]);
                f64::from_ne_bytes(bytes) as f32
            }
            _ => 0.0,
        }
    }

    /// Gets the value of a sample as u32.
    pub fn deep_value_uint(&self, pixel: i64, channel: usize, sample: usize) -> u32 {
        let inner = self.inner.read().unwrap();
        if !inner.allocated {
            return 0;
        }

        let pidx = pixel as usize;
        if pidx >= inner.nsamples.len() || sample >= inner.nsamples[pidx] as usize {
            return 0;
        }
        if channel >= inner.nchannels {
            return 0;
        }

        let offset = (inner.cumcapacity[pidx] as usize + sample) * inner.samplesize
            + inner.channeloffsets[channel];

        let ct = &inner.channeltypes[channel];
        match ct.basetype {
            BaseType::UInt32 => {
                let bytes: [u8; 4] = inner.data[offset..offset + 4].try_into().unwrap_or([0; 4]);
                u32::from_ne_bytes(bytes)
            }
            BaseType::UInt16 => {
                let bytes: [u8; 2] = inner.data[offset..offset + 2].try_into().unwrap_or([0; 2]);
                u16::from_ne_bytes(bytes) as u32
            }
            BaseType::UInt8 => inner.data[offset] as u32,
            BaseType::Float => {
                let bytes: [u8; 4] = inner.data[offset..offset + 4].try_into().unwrap_or([0; 4]);
                f32::from_ne_bytes(bytes) as u32
            }
            _ => 0,
        }
    }

    /// Sets the value of a sample (float).
    pub fn set_deep_value_f32(&self, pixel: i64, channel: usize, sample: usize, value: f32) {
        self.ensure_allocated();

        let mut inner = self.inner.write().unwrap();
        let pidx = pixel as usize;
        if pidx >= inner.nsamples.len() || sample >= inner.nsamples[pidx] as usize {
            return;
        }
        if channel >= inner.nchannels {
            return;
        }

        let offset = (inner.cumcapacity[pidx] as usize + sample) * inner.samplesize
            + inner.channeloffsets[channel];

        let ct = inner.channeltypes[channel].clone();
        match ct.basetype {
            BaseType::Float => {
                let bytes = value.to_ne_bytes();
                inner.data[offset..offset + 4].copy_from_slice(&bytes);
            }
            BaseType::Half => {
                let h = half::f16::from_f32(value);
                let bytes = h.to_ne_bytes();
                inner.data[offset..offset + 2].copy_from_slice(&bytes);
            }
            BaseType::UInt8 => {
                inner.data[offset] = (value.clamp(0.0, 1.0) * 255.0) as u8;
            }
            BaseType::UInt16 => {
                let v = (value.clamp(0.0, 1.0) * 65535.0) as u16;
                let bytes = v.to_ne_bytes();
                inner.data[offset..offset + 2].copy_from_slice(&bytes);
            }
            BaseType::UInt32 => {
                let bytes = (value.max(0.0) as u32).to_ne_bytes();
                inner.data[offset..offset + 4].copy_from_slice(&bytes);
            }
            BaseType::Double => {
                let bytes = (value as f64).to_ne_bytes();
                inner.data[offset..offset + 8].copy_from_slice(&bytes);
            }
            _ => {}
        }
    }

    /// Sets the value of a sample (u32).
    pub fn set_deep_value_u32(&self, pixel: i64, channel: usize, sample: usize, value: u32) {
        self.ensure_allocated();

        let mut inner = self.inner.write().unwrap();
        let pidx = pixel as usize;
        if pidx >= inner.nsamples.len() || sample >= inner.nsamples[pidx] as usize {
            return;
        }
        if channel >= inner.nchannels {
            return;
        }

        let offset = (inner.cumcapacity[pidx] as usize + sample) * inner.samplesize
            + inner.channeloffsets[channel];

        let ct = inner.channeltypes[channel].clone();
        match ct.basetype {
            BaseType::UInt32 => {
                let bytes = value.to_ne_bytes();
                inner.data[offset..offset + 4].copy_from_slice(&bytes);
            }
            BaseType::UInt16 => {
                let bytes = (value as u16).to_ne_bytes();
                inner.data[offset..offset + 2].copy_from_slice(&bytes);
            }
            BaseType::UInt8 => {
                inner.data[offset] = value as u8;
            }
            BaseType::Float => {
                let bytes = (value as f32).to_ne_bytes();
                inner.data[offset..offset + 4].copy_from_slice(&bytes);
            }
            _ => {}
        }
    }

    /// Returns all channel types.
    pub fn all_channeltypes(&self) -> Vec<TypeDesc> {
        let inner = self.inner.read().unwrap();
        inner.channeltypes.clone()
    }

    /// Returns all sample counts.
    pub fn all_samples(&self) -> Vec<u32> {
        let inner = self.inner.read().unwrap();
        inner.nsamples.clone()
    }

    /// Returns all raw data.
    pub fn all_data(&self) -> Vec<u8> {
        let inner = self.inner.read().unwrap();
        inner.data.clone()
    }

    /// Copies a single sample from src to this DeepData.
    pub fn copy_deep_sample(
        &self,
        pixel: i64,
        sample: usize,
        src: &DeepData,
        srcpixel: i64,
        srcsample: usize,
    ) -> bool {
        if !self.same_channeltypes(src) {
            return false;
        }

        self.ensure_allocated();

        let src_inner = src.inner.read().unwrap();
        let mut inner = self.inner.write().unwrap();

        let pidx = pixel as usize;
        let srcpidx = srcpixel as usize;

        if pidx >= inner.nsamples.len() || sample >= inner.nsamples[pidx] as usize {
            return false;
        }
        if srcpidx >= src_inner.nsamples.len() || srcsample >= src_inner.nsamples[srcpidx] as usize {
            return false;
        }

        let samplesize = inner.samplesize;
        let dst_offset =
            (inner.cumcapacity[pidx] as usize + sample) * samplesize;
        let src_offset =
            (src_inner.cumcapacity[srcpidx] as usize + srcsample) * src_inner.samplesize;

        inner.data[dst_offset..dst_offset + samplesize]
            .copy_from_slice(&src_inner.data[src_offset..src_offset + src_inner.samplesize]);

        true
    }

    /// Copies an entire pixel from src to this DeepData.
    pub fn copy_deep_pixel(&self, pixel: i64, src: &DeepData, srcpixel: i64) -> bool {
        if !self.same_channeltypes(src) {
            return false;
        }

        let src_samples = src.samples(srcpixel);
        self.set_samples(pixel, src_samples);
        self.ensure_allocated();

        for s in 0..src_samples as usize {
            if !self.copy_deep_sample(pixel, s, src, srcpixel, s) {
                return false;
            }
        }

        true
    }

    /// Sorts samples in a pixel by Z depth.
    pub fn sort(&self, pixel: i64) {
        self.ensure_allocated();

        let z_ch = self.z_channel();
        if z_ch < 0 {
            return;
        }

        let nsamps = self.samples(pixel) as usize;
        if nsamps <= 1 {
            return;
        }

        // Collect (z, sample_index) pairs
        let mut indices: Vec<(f32, usize)> = (0..nsamps)
            .map(|s| (self.deep_value(pixel, z_ch as usize, s), s))
            .collect();

        // Sort by Z
        indices.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Check if already sorted
        let already_sorted = indices.iter().enumerate().all(|(i, &(_, s))| i == s);
        if already_sorted {
            return;
        }

        // Reorder samples
        let mut inner = self.inner.write().unwrap();
        let pidx = pixel as usize;
        let sample_size = inner.samplesize;
        let pixel_offset = inner.cumcapacity[pidx] as usize * sample_size;

        // Make a copy of the pixel's data
        let original: Vec<u8> = inner.data[pixel_offset..pixel_offset + nsamps * sample_size].to_vec();

        // Reorder
        for (new_idx, &(_, old_idx)) in indices.iter().enumerate() {
            let src_start = old_idx * sample_size;
            let dst_start = pixel_offset + new_idx * sample_size;
            inner.data[dst_start..dst_start + sample_size]
                .copy_from_slice(&original[src_start..src_start + sample_size]);
        }
    }

    /// Returns the Z depth at which the pixel reaches full opacity.
    pub fn opaque_z(&self, pixel: i64) -> f32 {
        let z_ch = self.z_channel();
        let a_ch = self.a_channel();
        if z_ch < 0 || a_ch < 0 {
            return f32::INFINITY;
        }

        let nsamps = self.samples(pixel) as usize;
        let mut accumulated_alpha = 0.0f32;

        for s in 0..nsamps {
            let alpha = self.deep_value(pixel, a_ch as usize, s);
            accumulated_alpha += alpha * (1.0 - accumulated_alpha);

            if accumulated_alpha >= 0.9999 {
                return self.deep_value(pixel, z_ch as usize, s);
            }
        }

        f32::INFINITY
    }

    /// Removes samples hidden behind opaque samples.
    pub fn occlusion_cull(&self, pixel: i64) {
        let opaque_depth = self.opaque_z(pixel);
        if opaque_depth == f32::INFINITY {
            return;
        }

        let z_ch = self.z_channel();
        if z_ch < 0 {
            return;
        }

        let nsamps = self.samples(pixel) as usize;
        let mut to_remove = Vec::new();

        for s in (0..nsamps).rev() {
            let z = self.deep_value(pixel, z_ch as usize, s);
            if z > opaque_depth {
                to_remove.push(s);
            }
        }

        // Erase from back to front
        for s in to_remove {
            self.erase_samples(pixel, s, 1);
        }
    }

    /// Splits samples at the given depth.
    pub fn split(&self, pixel: i64, depth: f32) -> bool {
        let z_ch = self.z_channel();
        let zback_ch = self.zback_channel();
        if z_ch < 0 {
            return false;
        }

        let nsamps = self.samples(pixel) as usize;
        let mut split_occurred = false;

        // Process samples from back to front to handle index changes
        for s in (0..nsamps).rev() {
            let z = self.deep_value(pixel, z_ch as usize, s);
            let zback = if zback_ch != z_ch {
                self.deep_value(pixel, zback_ch as usize, s)
            } else {
                z
            };

            // Check if sample spans the split depth
            if z < depth && depth < zback {
                // Need to split this sample
                self.insert_samples(pixel, s + 1, 1);

                // Copy original sample to new position
                let inner = self.inner.read().unwrap();
                let nch = inner.nchannels;
                drop(inner);

                for c in 0..nch {
                    let v = self.deep_value(pixel, c, s);
                    self.set_deep_value_f32(pixel, c, s + 1, v);
                }

                // Adjust Z values
                self.set_deep_value_f32(pixel, zback_ch as usize, s, depth);
                self.set_deep_value_f32(pixel, z_ch as usize, s + 1, depth);

                // Adjust alpha if present
                let a_ch = self.a_channel();
                if a_ch >= 0 {
                    let alpha = self.deep_value(pixel, a_ch as usize, s);
                    let t1 = (depth - z) / (zback - z);
                    let t2 = 1.0 - t1;

                    // Distribute alpha proportionally
                    let a1 = 1.0 - (1.0 - alpha).powf(t1);
                    let a2 = 1.0 - (1.0 - alpha).powf(t2);

                    self.set_deep_value_f32(pixel, a_ch as usize, s, a1);
                    self.set_deep_value_f32(pixel, a_ch as usize, s + 1, a2);
                }

                split_occurred = true;
            }
        }

        split_occurred
    }

    /// Merges overlapping samples in a pixel.
    pub fn merge_overlaps(&self, pixel: i64) {
        let z_ch = self.z_channel();
        let zback_ch = self.zback_channel();
        if z_ch < 0 {
            return;
        }

        loop {
            let nsamps = self.samples(pixel) as usize;
            if nsamps <= 1 {
                break;
            }

            let mut merged = false;

            for s in 0..nsamps - 1 {
                let z1 = self.deep_value(pixel, z_ch as usize, s);
                let zback1 = self.deep_value(pixel, zback_ch as usize, s);
                let z2 = self.deep_value(pixel, z_ch as usize, s + 1);
                let zback2 = self.deep_value(pixel, zback_ch as usize, s + 1);

                // Check for exact overlap
                if (z1 - z2).abs() < 1e-6 && (zback1 - zback2).abs() < 1e-6 {
                    // Merge: combine alphas and colors
                    let a_ch = self.a_channel();
                    if a_ch >= 0 {
                        let a1 = self.deep_value(pixel, a_ch as usize, s);
                        let a2 = self.deep_value(pixel, a_ch as usize, s + 1);
                        let combined = a1 + a2 * (1.0 - a1);
                        self.set_deep_value_f32(pixel, a_ch as usize, s, combined);

                        // Blend colors
                        let inner = self.inner.read().unwrap();
                        let nch = inner.nchannels;
                        drop(inner);

                        for c in 0..nch {
                            if c != a_ch as usize && c != z_ch as usize && c != zback_ch as usize {
                                let c1 = self.deep_value(pixel, c, s);
                                let c2 = self.deep_value(pixel, c, s + 1);
                                let blended = if combined > 0.0 {
                                    (c1 * a1 + c2 * a2 * (1.0 - a1)) / combined
                                } else {
                                    (c1 + c2) * 0.5
                                };
                                self.set_deep_value_f32(pixel, c, s, blended);
                            }
                        }
                    }

                    // Erase the merged sample
                    self.erase_samples(pixel, s + 1, 1);
                    merged = true;
                    break;
                }
            }

            if !merged {
                break;
            }
        }
    }

    /// Merges a source pixel's samples into this pixel.
    pub fn merge_deep_pixels(&self, pixel: i64, src: &DeepData, srcpixel: i64) {
        let src_samples = src.samples(srcpixel);
        let cur_samples = self.samples(pixel);

        // Extend capacity
        let new_total = cur_samples + src_samples;
        self.set_capacity(pixel, new_total);
        self.set_samples(pixel, new_total);

        // Copy source samples
        for s in 0..src_samples as usize {
            self.copy_deep_sample(pixel, (cur_samples as usize) + s, src, srcpixel, s);
        }

        // Sort and merge
        self.sort(pixel);
        self.merge_overlaps(pixel);
    }
}

impl Clone for DeepData {
    fn clone(&self) -> Self {
        let inner = self.inner.read().unwrap();
        Self {
            inner: RwLock::new(inner.clone()),
        }
    }
}

impl Default for DeepData {
    fn default() -> Self {
        Self::new_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deepdata_creation() {
        let types = vec![TypeDesc::FLOAT, TypeDesc::FLOAT, TypeDesc::FLOAT, TypeDesc::FLOAT, TypeDesc::FLOAT];
        let names = vec!["R", "G", "B", "A", "Z"];
        let dd = DeepData::new(100, &types, &names);

        assert_eq!(dd.pixels(), 100);
        assert_eq!(dd.channels(), 5);
        assert_eq!(dd.z_channel(), 4);
        assert_eq!(dd.a_channel(), 3);
    }

    #[test]
    fn test_deepdata_samples() {
        let types = vec![TypeDesc::FLOAT, TypeDesc::FLOAT];
        let names = vec!["A", "Z"];
        let dd = DeepData::new(10, &types, &names);

        dd.set_samples(0, 3);
        assert_eq!(dd.samples(0), 3);

        dd.set_deep_value_f32(0, 0, 0, 0.5);
        dd.set_deep_value_f32(0, 1, 0, 1.0);

        assert!((dd.deep_value(0, 0, 0) - 0.5).abs() < 0.001);
        assert!((dd.deep_value(0, 1, 0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_deepdata_sort() {
        let types = vec![TypeDesc::FLOAT, TypeDesc::FLOAT];
        let names = vec!["A", "Z"];
        let dd = DeepData::new(1, &types, &names);

        dd.set_samples(0, 3);
        dd.set_deep_value_f32(0, 1, 0, 3.0); // Z = 3
        dd.set_deep_value_f32(0, 1, 1, 1.0); // Z = 1
        dd.set_deep_value_f32(0, 1, 2, 2.0); // Z = 2

        dd.set_deep_value_f32(0, 0, 0, 0.3); // A for Z=3
        dd.set_deep_value_f32(0, 0, 1, 0.1); // A for Z=1
        dd.set_deep_value_f32(0, 0, 2, 0.2); // A for Z=2

        dd.sort(0);

        // After sort, should be ordered by Z
        assert!((dd.deep_value(0, 1, 0) - 1.0).abs() < 0.001); // Z = 1
        assert!((dd.deep_value(0, 1, 1) - 2.0).abs() < 0.001); // Z = 2
        assert!((dd.deep_value(0, 1, 2) - 3.0).abs() < 0.001); // Z = 3

        assert!((dd.deep_value(0, 0, 0) - 0.1).abs() < 0.001); // A for Z=1
        assert!((dd.deep_value(0, 0, 1) - 0.2).abs() < 0.001); // A for Z=2
        assert!((dd.deep_value(0, 0, 2) - 0.3).abs() < 0.001); // A for Z=3
    }

    #[test]
    fn test_deepdata_insert_erase() {
        let types = vec![TypeDesc::FLOAT];
        let names = vec!["Z"];
        let dd = DeepData::new(1, &types, &names);

        dd.set_capacity(0, 5);
        dd.set_samples(0, 2);
        dd.set_deep_value_f32(0, 0, 0, 1.0);
        dd.set_deep_value_f32(0, 0, 1, 3.0);

        // Insert at position 1
        dd.insert_samples(0, 1, 1);
        dd.set_deep_value_f32(0, 0, 1, 2.0);

        assert_eq!(dd.samples(0), 3);
        assert!((dd.deep_value(0, 0, 0) - 1.0).abs() < 0.001);
        assert!((dd.deep_value(0, 0, 1) - 2.0).abs() < 0.001);
        assert!((dd.deep_value(0, 0, 2) - 3.0).abs() < 0.001);

        // Erase middle sample
        dd.erase_samples(0, 1, 1);
        assert_eq!(dd.samples(0), 2);
        assert!((dd.deep_value(0, 0, 0) - 1.0).abs() < 0.001);
        assert!((dd.deep_value(0, 0, 1) - 3.0).abs() < 0.001);
    }
}
