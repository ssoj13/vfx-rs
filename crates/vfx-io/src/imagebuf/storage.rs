//! Pixel storage backends for ImageBuf.
//!
//! This module provides the underlying storage mechanisms for ImageBuf pixel data.

use half::f16;
use vfx_core::{DataFormat, ImageSpec};

/// Pixel storage for ImageBuf.
///
/// Supports multiple storage modes:
/// - Owned buffer (allocated by ImageBuf)
/// - Wrapped buffer (external memory)
/// - Empty (for lazy loading)
#[derive(Debug)]
#[allow(missing_docs)]  // Enum variant fields are self-documenting
pub enum PixelStorage {
    /// No pixel data.
    Empty,
    /// Owned f32 buffer (most flexible).
    OwnedF32 {
        data: Vec<f32>,
        width: usize,
        height: usize,
        depth: usize,
        nchannels: usize,
    },
    /// Owned u8 buffer.
    OwnedU8 {
        data: Vec<u8>,
        width: usize,
        height: usize,
        depth: usize,
        nchannels: usize,
    },
    /// Owned u16 buffer.
    OwnedU16 {
        data: Vec<u16>,
        width: usize,
        height: usize,
        depth: usize,
        nchannels: usize,
    },
    /// Wrapped external buffer (raw pointer).
    Wrapped {
        ptr: *mut u8,
        size: usize,
        width: usize,
        height: usize,
        depth: usize,
        nchannels: usize,
        format: DataFormat,
        xstride: usize,
        ystride: usize,
        zstride: usize,
    },
}

// Safety: PixelStorage is Send/Sync because:
// - Owned variants contain only owned data
// - Wrapped variant requires caller to ensure thread safety
unsafe impl Send for PixelStorage {}
unsafe impl Sync for PixelStorage {}

impl Default for PixelStorage {
    fn default() -> Self {
        Self::Empty
    }
}

impl PixelStorage {
    /// Allocates storage for the given spec.
    pub fn allocate(spec: &ImageSpec, zero: bool) -> Self {
        let width = spec.width as usize;
        let height = spec.height as usize;
        let depth = spec.depth.max(1) as usize;
        let nchannels = spec.nchannels as usize;

        if width == 0 || height == 0 || nchannels == 0 {
            return Self::Empty;
        }

        let total = width * height * depth * nchannels;

        match spec.format {
            DataFormat::U8 => {
                let data = if zero {
                    vec![0u8; total]
                } else {
                    Vec::with_capacity(total)
                };
                Self::OwnedU8 {
                    data: if zero { data } else { vec![0u8; total] },
                    width,
                    height,
                    depth,
                    nchannels,
                }
            }
            DataFormat::U16 => {
                Self::OwnedU16 {
                    data: if zero { vec![0u16; total] } else { vec![0u16; total] },
                    width,
                    height,
                    depth,
                    nchannels,
                }
            }
            DataFormat::F16 | DataFormat::F32 | DataFormat::U32 => {
                // Store as f32 internally for flexibility
                Self::OwnedF32 {
                    data: if zero { vec![0.0f32; total] } else { vec![0.0f32; total] },
                    width,
                    height,
                    depth,
                    nchannels,
                }
            }
        }
    }

    /// Wraps external buffer.
    ///
    /// # Safety
    ///
    /// Caller must ensure ptr is valid and data remains valid for the lifetime
    /// of this PixelStorage.
    pub unsafe fn wrap(
        ptr: *mut u8,
        spec: &ImageSpec,
        xstride: Option<usize>,
        ystride: Option<usize>,
        zstride: Option<usize>,
    ) -> Self {
        let width = spec.width as usize;
        let height = spec.height as usize;
        let depth = spec.depth.max(1) as usize;
        let nchannels = spec.nchannels as usize;

        let pixel_size = spec.format.bytes_per_channel() * nchannels;
        let xstride = xstride.unwrap_or(pixel_size);
        let ystride = ystride.unwrap_or(width * xstride);
        let zstride = zstride.unwrap_or(height * ystride);

        let size = depth * zstride;

        Self::Wrapped {
            ptr,
            size,
            width,
            height,
            depth,
            nchannels,
            format: spec.format,
            xstride,
            ystride,
            zstride,
        }
    }

    /// Returns true if storage is empty.
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Returns raw pointer to data, if available.
    pub fn as_ptr(&self) -> Option<*const u8> {
        match self {
            Self::Empty => None,
            Self::OwnedF32 { data, .. } => Some(data.as_ptr() as *const u8),
            Self::OwnedU8 { data, .. } => Some(data.as_ptr()),
            Self::OwnedU16 { data, .. } => Some(data.as_ptr() as *const u8),
            Self::Wrapped { ptr, .. } => Some(*ptr as *const u8),
        }
    }

    /// Returns mutable raw pointer to data, if available.
    pub fn as_mut_ptr(&mut self) -> Option<*mut u8> {
        match self {
            Self::Empty => None,
            Self::OwnedF32 { data, .. } => Some(data.as_mut_ptr() as *mut u8),
            Self::OwnedU8 { data, .. } => Some(data.as_mut_ptr()),
            Self::OwnedU16 { data, .. } => Some(data.as_mut_ptr() as *mut u8),
            Self::Wrapped { ptr, .. } => Some(*ptr),
        }
    }

    /// Gets a single channel value.
    pub fn get_channel(
        &self,
        x: usize,
        y: usize,
        z: usize,
        c: usize,
        _spec: &ImageSpec,
    ) -> f32 {
        match self {
            Self::Empty => 0.0,
            Self::OwnedF32 { data, width, height, nchannels, .. } => {
                if x >= *width || y >= *height || c >= *nchannels {
                    return 0.0;
                }
                let idx = (z * *height + y) * *width * *nchannels + x * *nchannels + c;
                data.get(idx).copied().unwrap_or(0.0)
            }
            Self::OwnedU8 { data, width, height, nchannels, .. } => {
                if x >= *width || y >= *height || c >= *nchannels {
                    return 0.0;
                }
                let idx = (z * *height + y) * *width * *nchannels + x * *nchannels + c;
                data.get(idx).map(|&v| v as f32 / 255.0).unwrap_or(0.0)
            }
            Self::OwnedU16 { data, width, height, nchannels, .. } => {
                if x >= *width || y >= *height || c >= *nchannels {
                    return 0.0;
                }
                let idx = (z * *height + y) * *width * *nchannels + x * *nchannels + c;
                data.get(idx).map(|&v| v as f32 / 65535.0).unwrap_or(0.0)
            }
            Self::Wrapped {
                ptr,
                xstride,
                ystride,
                zstride,
                format,
                width,
                height,
                nchannels,
                ..
            } => {
                if x >= *width || y >= *height || c >= *nchannels {
                    return 0.0;
                }
                let offset = z * *zstride + y * *ystride + x * *xstride;
                unsafe {
                    let base = ptr.add(offset);
                    match format {
                        DataFormat::F32 => {
                            let p = base.add(c * 4) as *const f32;
                            *p
                        }
                        DataFormat::U8 => {
                            let v = *base.add(c);
                            v as f32 / 255.0
                        }
                        DataFormat::U16 | DataFormat::F16 => {
                            let p = base.add(c * 2) as *const u16;
                            if *format == DataFormat::F16 {
                                f16::from_bits(*p).to_f32()
                            } else {
                                *p as f32 / 65535.0
                            }
                        }
                        DataFormat::U32 => {
                            let p = base.add(c * 4) as *const u32;
                            *p as f32
                        }
                    }
                }
            }
        }
    }

    /// Gets all channels for a pixel.
    pub fn get_pixel(
        &self,
        x: usize,
        y: usize,
        z: usize,
        pixel: &mut [f32],
        spec: &ImageSpec,
    ) {
        let nch = spec.nchannels as usize;
        for c in 0..nch.min(pixel.len()) {
            pixel[c] = self.get_channel(x, y, z, c, spec);
        }
    }

    /// Sets all channels for a pixel.
    pub fn set_pixel(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        pixel: &[f32],
        spec: &ImageSpec,
    ) {
        let nch = spec.nchannels as usize;

        match self {
            Self::Empty => {}
            Self::OwnedF32 { data, width, height, nchannels, .. } => {
                if x >= *width || y >= *height {
                    return;
                }
                let base_idx = (z * *height + y) * *width * *nchannels + x * *nchannels;
                for c in 0..nch.min(pixel.len()).min(*nchannels) {
                    if let Some(dest) = data.get_mut(base_idx + c) {
                        *dest = pixel[c];
                    }
                }
            }
            Self::OwnedU8 { data, width, height, nchannels, .. } => {
                if x >= *width || y >= *height {
                    return;
                }
                let base_idx = (z * *height + y) * *width * *nchannels + x * *nchannels;
                for c in 0..nch.min(pixel.len()).min(*nchannels) {
                    if let Some(dest) = data.get_mut(base_idx + c) {
                        *dest = (pixel[c].clamp(0.0, 1.0) * 255.0) as u8;
                    }
                }
            }
            Self::OwnedU16 { data, width, height, nchannels, .. } => {
                if x >= *width || y >= *height {
                    return;
                }
                let base_idx = (z * *height + y) * *width * *nchannels + x * *nchannels;
                for c in 0..nch.min(pixel.len()).min(*nchannels) {
                    if let Some(dest) = data.get_mut(base_idx + c) {
                        *dest = (pixel[c].clamp(0.0, 1.0) * 65535.0) as u16;
                    }
                }
            }
            Self::Wrapped {
                ptr,
                xstride,
                ystride,
                zstride,
                format,
                width,
                height,
                nchannels,
                ..
            } => {
                if x >= *width || y >= *height {
                    return;
                }
                let offset = z * *zstride + y * *ystride + x * *xstride;
                unsafe {
                    let base = ptr.add(offset);
                    for c in 0..nch.min(pixel.len()).min(*nchannels) {
                        match format {
                            DataFormat::F32 => {
                                let p = base.add(c * 4) as *mut f32;
                                *p = pixel[c];
                            }
                            DataFormat::U8 => {
                                *base.add(c) = (pixel[c].clamp(0.0, 1.0) * 255.0) as u8;
                            }
                            DataFormat::U16 => {
                                let p = base.add(c * 2) as *mut u16;
                                *p = (pixel[c].clamp(0.0, 1.0) * 65535.0) as u16;
                            }
                            DataFormat::F16 => {
                                let p = base.add(c * 2) as *mut u16;
                                *p = f16::from_f32(pixel[c]).to_bits();
                            }
                            DataFormat::U32 => {
                                let p = base.add(c * 4) as *mut u32;
                                *p = pixel[c].max(0.0) as u32;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Creates a deep clone (copies wrapped data).
    pub fn deep_clone(&self) -> Self {
        match self {
            Self::Empty => Self::Empty,
            Self::OwnedF32 { data, width, height, depth, nchannels } => {
                Self::OwnedF32 {
                    data: data.clone(),
                    width: *width,
                    height: *height,
                    depth: *depth,
                    nchannels: *nchannels,
                }
            }
            Self::OwnedU8 { data, width, height, depth, nchannels } => {
                Self::OwnedU8 {
                    data: data.clone(),
                    width: *width,
                    height: *height,
                    depth: *depth,
                    nchannels: *nchannels,
                }
            }
            Self::OwnedU16 { data, width, height, depth, nchannels } => {
                Self::OwnedU16 {
                    data: data.clone(),
                    width: *width,
                    height: *height,
                    depth: *depth,
                    nchannels: *nchannels,
                }
            }
            Self::Wrapped {
                ptr,
                size: _,
                width,
                height,
                depth,
                nchannels,
                format,
                xstride,
                ystride,
                zstride,
            } => {
                // Copy wrapped data into owned buffer
                let total = *width * *height * *depth * *nchannels;
                let mut data = vec![0.0f32; total];

                // Copy pixel by pixel
                for z in 0..*depth {
                    for y in 0..*height {
                        for x in 0..*width {
                            let offset = z * *zstride + y * *ystride + x * *xstride;
                            let base_idx = (z * *height + y) * *width * *nchannels + x * *nchannels;

                            unsafe {
                                let base = ptr.add(offset);
                                for c in 0..*nchannels {
                                    let v = match format {
                                        DataFormat::F32 => {
                                            let p = base.add(c * 4) as *const f32;
                                            *p
                                        }
                                        DataFormat::U8 => {
                                            *base.add(c) as f32 / 255.0
                                        }
                                        DataFormat::U16 => {
                                            let p = base.add(c * 2) as *const u16;
                                            *p as f32 / 65535.0
                                        }
                                        DataFormat::F16 => {
                                            let p = base.add(c * 2) as *const u16;
                                            f16::from_bits(*p).to_f32()
                                        }
                                        DataFormat::U32 => {
                                            let p = base.add(c * 4) as *const u32;
                                            *p as f32
                                        }
                                    };
                                    data[base_idx + c] = v;
                                }
                            }
                        }
                    }
                }

                Self::OwnedF32 {
                    data,
                    width: *width,
                    height: *height,
                    depth: *depth,
                    nchannels: *nchannels,
                }
            }
        }
    }

    /// Fills all storage with zeros.
    pub fn fill_zero(&mut self, _spec: &ImageSpec) {
        match self {
            Self::Empty => {}
            Self::OwnedF32 { data, .. } => {
                data.fill(0.0);
            }
            Self::OwnedU8 { data, .. } => {
                data.fill(0);
            }
            Self::OwnedU16 { data, .. } => {
                data.fill(0);
            }
            Self::Wrapped {
                ptr,
                width,
                height,
                depth,
                nchannels,
                format,
                xstride,
                ystride,
                zstride,
                ..
            } => unsafe {
                for z in 0..*depth {
                    for y in 0..*height {
                        for x in 0..*width {
                            let offset = z * *zstride + y * *ystride + x * *xstride;
                            let base = ptr.add(offset);
                            for c in 0..*nchannels {
                                match format {
                                    DataFormat::F32 | DataFormat::U32 => {
                                        let p = base.add(c * 4) as *mut u32;
                                        *p = 0;
                                    }
                                    DataFormat::U8 => {
                                        *base.add(c) = 0;
                                    }
                                    DataFormat::U16 | DataFormat::F16 => {
                                        let p = base.add(c * 2) as *mut u16;
                                        *p = 0;
                                    }
                                }
                            }
                        }
                    }
                }
            },
        }
    }

    /// Fills storage with a constant value per channel.
    pub fn fill(&mut self, values: &[f32], spec: &ImageSpec) {
        let width = spec.width as usize;
        let height = spec.height as usize;
        let depth = spec.depth.max(1) as usize;

        for z in 0..depth {
            for y in 0..height {
                for x in 0..width {
                    self.set_pixel(x, y, z, values, spec);
                }
            }
        }
    }

    /// Converts storage to a different format.
    pub fn convert_to(&self, format: DataFormat, spec: &ImageSpec) -> Self {
        let width = spec.width as usize;
        let height = spec.height as usize;
        let depth = spec.depth.max(1) as usize;
        let nchannels = spec.nchannels as usize;
        let total = width * height * depth * nchannels;

        match format {
            DataFormat::U8 => {
                let mut data = vec![0u8; total];
                for z in 0..depth {
                    for y in 0..height {
                        for x in 0..width {
                            for c in 0..nchannels {
                                let v = self.get_channel(x, y, z, c, spec);
                                let idx = (z * height + y) * width * nchannels + x * nchannels + c;
                                data[idx] = (v.clamp(0.0, 1.0) * 255.0) as u8;
                            }
                        }
                    }
                }
                Self::OwnedU8 { data, width, height, depth, nchannels }
            }
            DataFormat::U16 => {
                let mut data = vec![0u16; total];
                for z in 0..depth {
                    for y in 0..height {
                        for x in 0..width {
                            for c in 0..nchannels {
                                let v = self.get_channel(x, y, z, c, spec);
                                let idx = (z * height + y) * width * nchannels + x * nchannels + c;
                                data[idx] = (v.clamp(0.0, 1.0) * 65535.0) as u16;
                            }
                        }
                    }
                }
                Self::OwnedU16 { data, width, height, depth, nchannels }
            }
            DataFormat::F16 | DataFormat::F32 | DataFormat::U32 => {
                let mut data = vec![0.0f32; total];
                for z in 0..depth {
                    for y in 0..height {
                        for x in 0..width {
                            for c in 0..nchannels {
                                let v = self.get_channel(x, y, z, c, spec);
                                let idx = (z * height + y) * width * nchannels + x * nchannels + c;
                                data[idx] = v;
                            }
                        }
                    }
                }
                Self::OwnedF32 { data, width, height, depth, nchannels }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_allocate() {
        let spec = ImageSpec::rgba(100, 100);
        let storage = PixelStorage::allocate(&spec, true);

        match &storage {
            PixelStorage::OwnedF32 { data, width, height, nchannels, .. } => {
                assert_eq!(*width, 100);
                assert_eq!(*height, 100);
                assert_eq!(*nchannels, 4);
                assert_eq!(data.len(), 100 * 100 * 4);
                assert!(data.iter().all(|&v| v == 0.0));
            }
            _ => panic!("Expected OwnedF32"),
        }
    }

    #[test]
    fn test_storage_pixel_access() {
        let spec = ImageSpec::rgba(10, 10);
        let mut storage = PixelStorage::allocate(&spec, true);

        // Set pixel
        storage.set_pixel(5, 5, 0, &[1.0, 0.5, 0.25, 1.0], &spec);

        // Get pixel
        let mut pixel = [0.0f32; 4];
        storage.get_pixel(5, 5, 0, &mut pixel, &spec);

        assert!((pixel[0] - 1.0).abs() < 0.001);
        assert!((pixel[1] - 0.5).abs() < 0.001);
        assert!((pixel[2] - 0.25).abs() < 0.001);
        assert!((pixel[3] - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_storage_convert() {
        let spec = ImageSpec::rgba(10, 10);
        let mut storage = PixelStorage::allocate(&spec, true);
        storage.set_pixel(0, 0, 0, &[1.0, 0.5, 0.25, 1.0], &spec);

        // Convert to U8
        let u8_storage = storage.convert_to(DataFormat::U8, &spec);
        let mut pixel = [0.0f32; 4];
        u8_storage.get_pixel(0, 0, 0, &mut pixel, &spec);

        // Should be approximately the same (with quantization)
        assert!((pixel[0] - 1.0).abs() < 0.01);
        assert!((pixel[1] - 0.5).abs() < 0.01);
    }
}
