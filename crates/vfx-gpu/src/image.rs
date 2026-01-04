//! GPU image representation.

use crate::backend::{Backend, ProcessingBackend, create_backend};
use crate::{GpuError, GpuResult};

/// Image stored in GPU/CPU memory for processing.
pub struct GpuImage {
    /// Raw pixel data (f32).
    pub(crate) data: Vec<f32>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Number of channels (3 or 4).
    pub channels: u32,
}

impl GpuImage {
    /// Create from f32 data.
    pub fn from_f32(data: Vec<f32>, width: u32, height: u32, channels: u32) -> GpuResult<Self> {
        let expected = (width as usize) * (height as usize) * (channels as usize);
        if data.len() != expected {
            return Err(GpuError::BufferSizeMismatch { 
                expected, 
                actual: data.len() 
            });
        }
        Ok(Self { data, width, height, channels })
    }
    
    /// Create from vfx-io ImageData.
    pub fn from_image_data(img: &vfx_core::ImageData) -> Self {
        Self {
            data: img.to_f32(),
            width: img.width,
            height: img.height,
            channels: img.channels,
        }
    }
    
    /// Convert to vfx-io ImageData.
    pub fn to_image_data(&self) -> vfx_core::ImageData {
        vfx_core::ImageData::from_f32(
            self.width,
            self.height,
            self.channels,
            self.data.clone(),
        )
    }
    
    /// Get pixel data.
    pub fn data(&self) -> &[f32] {
        &self.data
    }
    
    /// Get mutable pixel data.
    pub fn data_mut(&mut self) -> &mut [f32] {
        &mut self.data
    }
    
    /// Image dimensions.
    pub fn dimensions(&self) -> (u32, u32, u32) {
        (self.width, self.height, self.channels)
    }
    
    /// Size in bytes.
    pub fn size_bytes(&self) -> usize {
        self.data.len() * 4
    }
    
    /// Clone the image.
    pub fn duplicate(&self) -> Self {
        Self {
            data: self.data.clone(),
            width: self.width,
            height: self.height,
            channels: self.channels,
        }
    }
}

impl Clone for GpuImage {
    fn clone(&self) -> Self {
        self.duplicate()
    }
}
