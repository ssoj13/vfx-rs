//! GPU-compatible image representation.
//!
//! [`ComputeImage`] is the core image type for all GPU/CPU processing.
//! It stores pixel data as f32 values in linear color space.
//!
//! # Memory Model
//!
//! Uses `Arc<Vec<f32>>` for copy-on-write semantics, enabling:
//! - Zero-copy cloning (shares underlying data)
//! - Efficient conversions to/from `vfx_core::Image`
//! - Thread-safe sharing for parallel processing
//!
//! # Example
//!
//! ```ignore
//! use vfx_compute::ComputeImage;
//!
//! // Create from f32 data (RGB)
//! let data = vec![0.5; 1920 * 1080 * 3];  // Mid-gray
//! let img = ComputeImage::from_f32(data, 1920, 1080, 3)?;
//!
//! // Create empty image
//! let empty = ComputeImage::new(1920, 1080, 4);  // RGBA
//!
//! // Access pixel data
//! let pixels: &[f32] = img.data();
//! ```

use crate::{ComputeError, ComputeResult};
use std::sync::Arc;

/// Image stored in memory for GPU/CPU processing.
///
/// Pixel data is stored as contiguous f32 values in row-major order:
/// `[R0, G0, B0, R1, G1, B1, ...]` for RGB or
/// `[R0, G0, B0, A0, R1, G1, B1, A1, ...]` for RGBA.
///
/// Values are expected to be in linear color space (not gamma-encoded).
/// The valid range is typically [0.0, 1.0] but HDR values > 1.0 are supported.
///
/// # Memory Sharing
///
/// The pixel buffer uses `Arc<Vec<f32>>` for copy-on-write semantics:
/// - `clone()` is O(1), shares the underlying buffer
/// - Mutations trigger a copy only if the buffer is shared
/// - Enables zero-copy conversions with `vfx_core::Image`
#[derive(Clone)]
pub struct ComputeImage {
    /// Raw pixel data in f32 format (Arc for COW semantics).
    data: Arc<Vec<f32>>,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Number of color channels (typically 3 for RGB or 4 for RGBA).
    pub channels: u32,
}

impl ComputeImage {
    /// Create image from f32 pixel data.
    ///
    /// # Arguments
    /// * `data` - Pixel values in row-major order
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels  
    /// * `channels` - Number of channels (3 or 4)
    ///
    /// # Errors
    /// Returns [`ComputeError::BufferSizeMismatch`] if `data.len() != width * height * channels`
    pub fn from_f32(data: Vec<f32>, width: u32, height: u32, channels: u32) -> ComputeResult<Self> {
        let expected = (width as usize) * (height as usize) * (channels as usize);
        if data.len() != expected {
            return Err(ComputeError::BufferSizeMismatch { 
                expected, 
                actual: data.len() 
            });
        }
        Ok(Self { 
            data: Arc::new(data), 
            width, 
            height, 
            channels 
        })
    }

    /// Create image from Arc-wrapped data (zero-copy).
    ///
    /// Useful when converting from `vfx_core::Image` or sharing buffers.
    pub fn from_arc(data: Arc<Vec<f32>>, width: u32, height: u32, channels: u32) -> ComputeResult<Self> {
        let expected = (width as usize) * (height as usize) * (channels as usize);
        if data.len() != expected {
            return Err(ComputeError::BufferSizeMismatch { 
                expected, 
                actual: data.len() 
            });
        }
        Ok(Self { data, width, height, channels })
    }

    /// Create empty image filled with zeros.
    ///
    /// Useful for creating destination buffers for operations like resize or composite.
    pub fn new(width: u32, height: u32, channels: u32) -> Self {
        let size = (width as usize) * (height as usize) * (channels as usize);
        Self {
            data: Arc::new(vec![0.0; size]),
            width,
            height,
            channels,
        }
    }

    /// Get immutable reference to pixel data.
    #[inline]
    pub fn data(&self) -> &[f32] {
        &self.data
    }

    /// Get mutable reference to pixel data.
    ///
    /// If the buffer is shared (Arc refcount > 1), this triggers a copy
    /// to ensure exclusive access (copy-on-write).
    #[inline]
    pub fn data_mut(&mut self) -> &mut [f32] {
        Arc::make_mut(&mut self.data).as_mut_slice()
    }

    /// Get the underlying Arc (for zero-copy sharing).
    #[inline]
    pub fn data_arc(&self) -> &Arc<Vec<f32>> {
        &self.data
    }

    /// Consume and return the inner data.
    ///
    /// If uniquely owned, returns the Vec directly (no copy).
    /// If shared, clones the data.
    #[inline]
    pub fn into_vec(self) -> Vec<f32> {
        Arc::try_unwrap(self.data).unwrap_or_else(|arc| (*arc).clone())
    }

    /// Ensures this image has exclusive ownership of its data.
    ///
    /// Call this before extensive mutations to avoid repeated CoW clones.
    #[inline]
    pub fn make_mut(&mut self) {
        let _ = Arc::make_mut(&mut self.data);
    }

    /// Get image dimensions as (width, height, channels).
    #[inline]
    pub fn dimensions(&self) -> (u32, u32, u32) {
        (self.width, self.height, self.channels)
    }

    /// Calculate size in bytes (data.len() * sizeof(f32)).
    #[inline]
    pub fn size_bytes(&self) -> usize {
        self.data.len() * 4
    }

    /// Create a deep copy of the image.
    ///
    /// Unlike `clone()` which shares data, this always allocates new memory.
    pub fn duplicate(&self) -> Self {
        Self {
            data: Arc::new((*self.data).clone()),
            width: self.width,
            height: self.height,
            channels: self.channels,
        }
    }

    /// Check if this image is the sole owner of its data.
    #[inline]
    pub fn is_unique(&self) -> bool {
        Arc::strong_count(&self.data) == 1
    }

    /// Replace the internal data buffer.
    ///
    /// # Panics
    /// Panics if data.len() != width * height * channels
    #[inline]
    pub fn set_data(&mut self, data: Vec<f32>) {
        let expected = (self.width as usize) * (self.height as usize) * (self.channels as usize);
        assert_eq!(data.len(), expected, "data size mismatch");
        self.data = Arc::new(data);
    }

    /// Take the data out, leaving empty buffer.
    ///
    /// If uniquely owned, returns Vec directly. If shared, clones.
    /// Leaves image in invalid state - must call set_data before using.
    pub fn take_data(&mut self) -> Vec<f32> {
        let empty = Arc::new(Vec::new());
        let old = std::mem::replace(&mut self.data, empty);
        Arc::try_unwrap(old).unwrap_or_else(|arc| (*arc).clone())
    }
}

impl std::fmt::Debug for ComputeImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComputeImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("channels", &self.channels)
            .field("size_bytes", &self.size_bytes())
            .field("shared", &!self.is_unique())
            .finish()
    }
}

// Conversions with vfx_core::Image

impl<C: vfx_core::ColorSpace, const N: usize> From<vfx_core::Image<C, f32, N>> for ComputeImage {
    /// Convert from vfx_core::Image to ComputeImage.
    ///
    /// This is a zero-copy operation if both types use the same Arc.
    fn from(img: vfx_core::Image<C, f32, N>) -> Self {
        // vfx_core::Image stores data in Arc<Vec<T>>, we need to extract it
        // Since Image.data() returns &[T], we need to clone the data
        // TODO: Add Image::into_arc() to vfx_core for true zero-copy
        let (w, h) = img.dimensions();
        Self {
            data: Arc::new(img.data().to_vec()),
            width: w,
            height: h,
            channels: N as u32,
        }
    }
}

impl ComputeImage {
    /// Convert to vfx_core::Image with specified color space.
    ///
    /// # Type Parameters
    /// * `C` - Target color space marker
    /// * `N` - Number of channels (must match self.channels)
    ///
    /// # Panics
    /// Panics if `N != self.channels`
    pub fn into_image<C: vfx_core::ColorSpace, const N: usize>(self) -> vfx_core::Image<C, f32, N> {
        assert_eq!(N as u32, self.channels, "channel count mismatch");
        let (w, h) = (self.width, self.height);
        let data = self.into_vec();
        vfx_core::Image::from_data(w, h, data)
            .expect("dimension validation already done")
    }

    /// Create a vfx_core::Image view (copies data).
    pub fn to_image<C: vfx_core::ColorSpace, const N: usize>(&self) -> vfx_core::Image<C, f32, N> {
        assert_eq!(N as u32, self.channels, "channel count mismatch");
        let data = self.data.as_ref().clone();
        vfx_core::Image::from_data(self.width, self.height, data)
            .expect("dimension validation already done")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let img = ComputeImage::new(100, 100, 4);
        assert_eq!(img.width, 100);
        assert_eq!(img.height, 100);
        assert_eq!(img.channels, 4);
        assert_eq!(img.data().len(), 100 * 100 * 4);
    }

    #[test]
    fn test_from_f32() {
        let data = vec![0.5; 10 * 10 * 3];
        let img = ComputeImage::from_f32(data, 10, 10, 3).unwrap();
        assert_eq!(img.data()[0], 0.5);
    }

    #[test]
    fn test_buffer_mismatch() {
        let data = vec![0.0; 100]; // Wrong size
        let result = ComputeImage::from_f32(data, 10, 10, 4);
        assert!(result.is_err());
    }

    #[test]
    fn test_clone_shares_data() {
        let img1 = ComputeImage::new(100, 100, 4);
        let img2 = img1.clone();
        
        // Both should share the same Arc
        assert!(!img1.is_unique());
        assert!(!img2.is_unique());
    }

    #[test]
    fn test_cow_behavior() {
        let img1 = ComputeImage::new(10, 10, 3);
        let mut img2 = img1.clone();
        
        // Before mutation, both share data
        assert!(!img1.is_unique());
        
        // Mutation triggers copy
        img2.data_mut()[0] = 1.0;
        
        // Now img1 is unique (img2 has its own copy)
        assert!(img1.is_unique());
        assert!(img2.is_unique());
        
        // Original unchanged
        assert_eq!(img1.data()[0], 0.0);
        assert_eq!(img2.data()[0], 1.0);
    }

    #[test]
    fn test_duplicate_creates_copy() {
        let img1 = ComputeImage::new(10, 10, 3);
        let img2 = img1.duplicate();
        
        // Both should be unique (duplicate always copies)
        assert!(img1.is_unique());
        assert!(img2.is_unique());
    }

    #[test]
    fn test_into_vec_unique() {
        let img = ComputeImage::new(10, 10, 3);
        assert!(img.is_unique());
        let vec = img.into_vec();
        assert_eq!(vec.len(), 10 * 10 * 3);
    }

    #[test]
    fn test_into_vec_shared() {
        let img1 = ComputeImage::new(10, 10, 3);
        let img2 = img1.clone();
        
        // img1 is shared, into_vec should clone
        let vec = img1.into_vec();
        assert_eq!(vec.len(), 10 * 10 * 3);
        
        // img2 still valid
        assert_eq!(img2.data().len(), 10 * 10 * 3);
    }

    #[test]
    fn test_conversion_roundtrip() {
        use vfx_core::LinearSrgb;
        
        let compute = ComputeImage::from_f32(vec![0.5; 10 * 10 * 4], 10, 10, 4).unwrap();
        let core: vfx_core::Image<LinearSrgb, f32, 4> = compute.to_image();
        let back: ComputeImage = core.into();
        
        assert_eq!(back.width, 10);
        assert_eq!(back.height, 10);
        assert_eq!(back.channels, 4);
        assert_eq!(back.data()[0], 0.5);
    }
}
