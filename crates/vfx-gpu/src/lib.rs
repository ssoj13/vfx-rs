//! GPU-accelerated image processing for VFX workflows.
//!
//! Provides compute shader implementations for color transforms
//! and image operations using wgpu for cross-platform GPU access.
//!
//! # Architecture
//!
//! ```text
//! ColorProcessor / ImageProcessor
//!     └── Backend (CPU or wgpu)
//!             └── GpuPrimitives trait
//!                     ├── CpuPrimitives (rayon)
//!                     └── WgpuPrimitives (compute shaders)
//! ```
//!
//! # Example
//!
//! ```ignore
//! use vfx_gpu::{ColorProcessor, Backend};
//!
//! let processor = ColorProcessor::new(Backend::Auto)?;
//! let mut img = processor.upload(&image_data)?;
//! processor.apply_matrix(&mut img, &matrix)?;
//! let result = processor.download(&img)?;
//! ```

pub mod backend;
pub mod image;
pub mod color;
pub mod ops;
mod shaders;

pub use backend::{Backend, GpuLimits, detect_backends, select_best_backend};
pub use image::GpuImage;
pub use color::ColorProcessor;
pub use ops::ImageProcessor;

use thiserror::Error;

/// GPU operation errors
#[derive(Error, Debug)]
pub enum GpuError {
    #[error("No suitable GPU adapter found")]
    NoAdapter,
    
    #[error("Backend not available: {0}")]
    BackendNotAvailable(String),
    
    #[error("Failed to create device: {0}")]
    DeviceCreation(String),
    
    #[error("Failed to create buffer: {0}")]
    BufferCreation(String),
    
    #[error("Failed to compile shader: {0}")]
    ShaderCompilation(String),
    
    #[error("Buffer size mismatch: expected {expected}, got {actual}")]
    BufferSizeMismatch { expected: usize, actual: usize },
    
    #[error("Image too large: {width}x{height} exceeds GPU limit {limit}")]
    ImageTooLarge { width: u32, height: u32, limit: u32 },
    
    #[error("Invalid dimensions: {0}x{1}")]
    InvalidDimensions(u32, u32),
    
    #[error("GPU operation failed: {0}")]
    OperationFailed(String),
}

pub type GpuResult<T> = Result<T, GpuError>;
