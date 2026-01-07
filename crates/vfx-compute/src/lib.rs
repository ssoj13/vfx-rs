//! Unified GPU/CPU compute backend for VFX image processing.
//!
//! `vfx-compute` provides hardware-accelerated color transforms and image operations
//! with automatic backend selection between CPU (rayon), wgpu (Vulkan/Metal/DX12),
//! and CUDA.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │            User-facing API (ColorProcessor, ImageProcessor)     │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                     AnyExecutor (enum dispatch)                 │
//! │         Cpu(TiledExecutor) | Wgpu(TiledExecutor) | Cuda(...)   │
//! ├─────────────────────────────────────────────────────────────────┤
//! │              TiledExecutor<G: GpuPrimitives>                    │
//! │           Automatic tiling for large images                     │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                   GpuPrimitives trait                           │
//! │   upload/download + exec_* operations with associated types     │
//! ├──────────────────┬─────────────────────┬────────────────────────┤
//! │   CpuPrimitives  │   WgpuPrimitives    │    CudaPrimitives      │
//! │     (rayon)      │  (compute shaders)  │     (cudarc)           │
//! └──────────────────┴─────────────────────┴────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ```ignore
//! use vfx_compute::{ColorProcessor, ImageProcessor, ComputeImage, Backend};
//!
//! // Auto-select best backend (CUDA > wgpu > CPU)
//! let color = ColorProcessor::new(Backend::Auto)?;
//! let image = ImageProcessor::new(Backend::Auto)?;
//!
//! // Create image from f32 data
//! let mut img = ComputeImage::from_f32(pixels, 1920, 1080, 3)?;
//!
//! // Color operations
//! color.apply_exposure(&mut img, 1.0)?;  // +1 stop
//! color.apply_cdl(&mut img, &cdl)?;       // CDL grade
//!
//! // Image operations
//! image.blur(&mut img, 2.0)?;             // Gaussian blur
//! let half = image.resize_half(&img)?;    // Downsample
//! ```
//!
//! # Backend Selection
//!
//! ```ignore
//! use vfx_compute::{Backend, describe_backends};
//!
//! // Show available backends
//! println!("{}", describe_backends());
//!
//! // Force specific backend
//! let proc = ColorProcessor::new(Backend::Cpu)?;   // CPU only
//! let proc = ColorProcessor::new(Backend::Wgpu)?;  // GPU via wgpu
//! let proc = ColorProcessor::new(Backend::Cuda)?;  // NVIDIA CUDA
//!
//! // Or use VFX_BACKEND environment variable:
//! // VFX_BACKEND=cpu ./my_app
//! ```
//!
//! # Feature Flags
//!
//! - `wgpu` - Enable GPU acceleration via wgpu (Vulkan/Metal/DX12)
//! - `cuda` - Enable NVIDIA CUDA backend
//! - `io` - Integration with vfx-io image loading

pub mod backend;
pub mod image;
pub mod color;
pub mod ops;
pub mod processor;
pub mod pipeline;
pub mod convert;
pub mod layer;
mod shaders;

pub use backend::{
    Backend, GpuLimits, ProcessingStrategy, TileWorkflow,
    detect_backends, select_best_backend, describe_backends,
};
pub use image::ComputeImage;
pub use color::{ColorProcessor, Cdl};
pub use ops::{ImageProcessor, ResizeFilter, BlendMode};
pub use processor::{
    Processor, ProcessorBuilder, ProcessorConfig,
    ColorOpBatch, BatchOp,
    DEFAULT_TILE_SIZE, MIN_TILE_SIZE, MAX_TILE_SIZE, DEFAULT_RAM_PERCENT,
};
pub use pipeline::{
    ComputePipeline, ComputePipelineBuilder,
    ImageInput, ImageOutput, ProcessResult, ComputeOp,
};
pub use convert::Processable;
#[cfg(feature = "io")]
pub use convert::{LayerMeta, from_image_data, from_image_data_direct, to_image_data, from_layer, to_layer};
#[cfg(feature = "io")]
pub use layer::{LayerProcessor, ChannelGroup, ChannelClassification};

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
