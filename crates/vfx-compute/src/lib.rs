//! Unified compute backend for VFX workflows.
//!
//! Provides CPU (rayon) and GPU (wgpu) backends for color transforms
//! and image operations with automatic backend selection.
//!
//! # Architecture
//!
//! ```text
//! Processor (unified API)
//!     └── Backend (CPU or wgpu)
//!             └── GpuPrimitives trait
//!                     ├── CpuPrimitives (rayon)
//!                     └── WgpuPrimitives (compute shaders)
//! ```
//!
//! # Example
//!
//! ```ignore
//! use vfx_compute::{Processor, ComputeImage};
//!
//! let proc = Processor::auto()?;
//! let mut img = ComputeImage::from_f32(data, 1920, 1080, 3)?;
//!
//! proc.apply_exposure(&mut img, 1.5)?;
//! proc.apply_saturation(&mut img, 1.2)?;
//! ```

pub mod backend;
pub mod image;
pub mod color;
pub mod ops;
pub mod processor;
mod shaders;

pub use backend::{Backend, GpuLimits, detect_backends, select_best_backend, describe_backends};
pub use image::ComputeImage;
pub use color::{ColorProcessor, Cdl};
pub use ops::{ImageProcessor, ResizeFilter};
pub use processor::Processor;

use thiserror::Error;

/// GPU operation errors
#[derive(Error, Debug)]
pub enum ComputeError {
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

pub type ComputeResult<T> = Result<T, ComputeError>;
