//! GPU-compatible image representation.
//!
//! [`ComputeImage`] is the core image type for all GPU/CPU processing.
//! It stores pixel data as f32 values in linear color space.
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

/// Image stored in memory for GPU/CPU processing.
///
/// Pixel data is stored as contiguous f32 values in row-major order:
/// `[R0, G0, B0, R1, G1, B1, ...]` for RGB or
/// `[R0, G0, B0, A0, R1, G1, B1, A1, ...]` for RGBA.
///
/// Values are expected to be in linear color space (not gamma-encoded).
/// The valid range is typically [0.0, 1.0] but HDR values > 1.0 are supported.
#[derive(Clone)]
pub struct ComputeImage {
    /// Raw pixel data in f32 format.
    pub(crate) data: Vec<f32>,
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
        Ok(Self { data, width, height, channels })
    }

    /// Create empty image filled with zeros.
    ///
    /// Useful for creating destination buffers for operations like resize or composite.
    pub fn new(width: u32, height: u32, channels: u32) -> Self {
        let size = (width as usize) * (height as usize) * (channels as usize);
        Self {
            data: vec![0.0; size],
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
    #[inline]
    pub fn data_mut(&mut self) -> &mut [f32] {
        &mut self.data
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
    pub fn duplicate(&self) -> Self {
        Self {
            data: self.data.clone(),
            width: self.width,
            height: self.height,
            channels: self.channels,
        }
    }
}

impl std::fmt::Debug for ComputeImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComputeImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("channels", &self.channels)
            .field("size_bytes", &self.size_bytes())
            .finish()
    }
}
