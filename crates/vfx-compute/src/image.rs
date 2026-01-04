//! GPU image representation.

use crate::{ComputeError, ComputeResult};

/// Image stored in GPU/CPU memory for processing.
#[derive(Clone)]
pub struct ComputeImage {
    /// Raw pixel data (f32).
    pub(crate) data: Vec<f32>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Number of channels (3 or 4).
    pub channels: u32,
}

impl ComputeImage {
    /// Create from f32 data.
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
    pub fn new(width: u32, height: u32, channels: u32) -> Self {
        let size = (width as usize) * (height as usize) * (channels as usize);
        Self {
            data: vec![0.0; size],
            width,
            height,
            channels,
        }
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
