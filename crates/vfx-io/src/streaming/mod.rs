//! Streaming I/O for large images.
//!
//! This module provides infrastructure for processing images larger than
//! available RAM by reading/writing them in tiles or regions.
//!
//! # Overview
//!
//! Traditional image I/O loads the entire image into memory, which fails
//! for very large images (8K+, 16K+, or film scans). Streaming I/O solves
//! this by:
//!
//! 1. Reading only the regions needed for processing
//! 2. Writing output incrementally as tiles complete
//! 3. Caching recently-used regions for efficiency
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Streaming Pipeline                       │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                             │
//! │  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
//! │  │StreamingSource│───▶│  Processor   │───▶│StreamingOutput│ │
//! │  └──────────────┘    └──────────────┘    └──────────────┘  │
//! │         │                   │                    │          │
//! │         ▼                   ▼                    ▼          │
//! │  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
//! │  │ TiffSource   │    │  Color Xform │    │  TiffOutput  │  │
//! │  │ ExrSource    │    │    Warp      │    │  ExrOutput   │  │
//! │  │ MemorySource │    │   Composite  │    │ MemoryOutput │  │
//! │  └──────────────┘    └──────────────┘    └──────────────┘  │
//! │                                                             │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ## Auto-streaming (recommended)
//!
//! ```ignore
//! use vfx_io::streaming::open_streaming;
//!
//! // Automatically selects streaming vs memory based on file size
//! let mut source = open_streaming("huge_image.tif")?;
//!
//! // Process in tiles
//! for tile in source.tiles(512, 512) {
//!     let region = source.read_region(tile.x, tile.y, tile.w, tile.h)?;
//!     // Process region...
//! }
//! ```
//!
//! ## Manual control
//!
//! ```ignore
//! use vfx_io::streaming::{MemorySource, StreamingSource};
//! use vfx_io::read;
//!
//! // For small images, MemorySource is fine
//! let image = read("small.png")?;
//! let mut source = MemorySource::new(image);
//!
//! let region = source.read_region(100, 100, 256, 256)?;
//! ```
//!
//! # Format Support
//!
//! | Format | True Streaming | Notes |
//! |--------|----------------|-------|
//! | TIFF   | Yes            | Tiled TIFF optimal, strips OK |
//! | EXR    | Yes            | Scanline or tiled |
//! | PNG    | No             | Falls back to MemorySource |
//! | JPEG   | No             | Falls back to MemorySource |
//! | DPX    | Partial        | Scanline access |
//!
//! # Memory Estimation
//!
//! Use [`should_use_streaming`] to decide between streaming and memory:
//!
//! ```ignore
//! use vfx_io::streaming::should_use_streaming;
//!
//! if should_use_streaming("image.tif", available_ram)? {
//!     // Use streaming pipeline
//! } else {
//!     // Load into memory
//! }
//! ```
//!
//! # Ported from stool-rs
//!
//! This module is based on `stool-rs/warper/src/backend/streaming_io.rs`
//! with adaptations for vfx-rs multi-format support and color pipeline
//! integration.

mod traits;
mod source;
pub mod format;
pub mod pipeline;

#[cfg(feature = "tiff")]
mod tiff;

#[cfg(feature = "exr")]
mod exr;

// Re-export core types
pub use traits::{
    Region,
    StreamingSource,
    StreamingOutput,
    BoxedSource,
    BoxedOutput,
    RGBA_CHANNELS,
    DEFAULT_CACHE_SIZE,
};

pub use source::{
    MemorySource,
    MemoryOutput,
    FileOutput,
};

pub use format::{
    MemoryEstimate,
    ProcessingStrategy,
    estimate_memory,
    estimate_from_dims,
    should_stream,
    bytes_per_pixel,
    RGBA_F32_BPP,
    MEMORY_SAFETY_MARGIN,
    STREAMING_THRESHOLD_RATIO,
};

pub use pipeline::{
    TileSpec,
    tile_iterator,
    StreamingPipeline,
    ProgressCallback,
    run_with_progress,
};

// Factory functions are defined at module level and exported here
// open_streaming() and open_streaming_auto() are available directly

#[cfg(feature = "tiff")]
pub use tiff::{TiffStreamingSource, TiffStreamingOutput};

#[cfg(feature = "exr")]
pub use exr::{ExrStreamingSource, ExrStreamingOutput};

use std::path::Path;
use crate::{IoResult, Format};

/// Opens a streaming source with automatic format detection.
///
/// Selects the optimal implementation based on file extension:
/// - `.tif`, `.tiff` -> `TiffStreamingSource` (true random access)
/// - `.exr` -> `ExrStreamingSource` (lazy loading)
/// - Others -> `MemorySource` (full decode)
///
/// # Example
///
/// ```ignore
/// use vfx_io::streaming::open_streaming;
///
/// let mut source = open_streaming("input.tif")?;
/// let (w, h) = source.dimensions();
/// println!("Image: {}x{}", w, h);
///
/// // Read only the region we need
/// let region = source.read_region(0, 0, 512, 512)?;
/// ```
///
/// # Format Support
///
/// | Format | Implementation | Random Access |
/// |--------|----------------|---------------|
/// | TIFF   | TiffStreamingSource | Yes (tile/strip) |
/// | EXR    | ExrStreamingSource | No (lazy load) |
/// | Others | MemorySource | No (full load) |
pub fn open_streaming<P: AsRef<Path>>(path: P) -> IoResult<BoxedSource> {
    let path = path.as_ref();
    let format = Format::detect(path)?;

    match format {
        #[cfg(feature = "tiff")]
        Format::Tiff => {
            Ok(Box::new(TiffStreamingSource::open(path)?))
        }
        
        #[cfg(feature = "exr")]
        Format::Exr => {
            Ok(Box::new(ExrStreamingSource::open(path)?))
        }
        
        // Fallback: load into memory
        _ => {
            let image = crate::read(path)?;
            Ok(Box::new(MemorySource::new(image)))
        }
    }
}

/// Creates a streaming source, using streaming only if beneficial.
///
/// Checks the file size and available memory to decide whether to use
/// streaming or just load the image into memory.
///
/// # Arguments
///
/// * `path` - Path to the image file
/// * `available_bytes` - Available RAM in bytes (use `None` for auto-detect)
///
/// # Example
///
/// ```ignore
/// use vfx_io::streaming::open_streaming_auto;
///
/// // Let the library decide based on file and system
/// let mut source = open_streaming_auto("image.tif", None)?;
/// ```
pub fn open_streaming_auto<P: AsRef<Path>>(
    path: P,
    available_bytes: Option<u64>,
) -> IoResult<BoxedSource> {
    let path = path.as_ref();
    
    // Try to estimate memory requirements
    let estimate = match format::estimate_memory(path) {
        Ok(est) => est,
        Err(_) => {
            // Can't estimate - fall back to regular open
            return open_streaming(path);
        }
    };

    // Use system memory if not specified (default 8 GB assumption)
    let available = available_bytes.unwrap_or(8 * 1024 * 1024 * 1024);

    let strategy = format::ProcessingStrategy::recommend(&estimate, available);

    match strategy {
        format::ProcessingStrategy::InMemory => {
            // Small enough to load directly
            let image = crate::read(path)?;
            Ok(Box::new(MemorySource::new(image)))
        }
        _ => {
            // Use streaming
            open_streaming(path)
        }
    }
}

use crate::PixelFormat;

/// Creates a streaming output with automatic format detection.
///
/// Selects the optimal implementation based on file extension:
/// - `.tif`, `.tiff` -> `TiffStreamingOutput` (buffered write)
/// - `.exr` -> `ExrStreamingOutput` (buffered write)
/// - Others -> `MemoryOutput` (accumulate and write on finalize)
///
/// # Arguments
///
/// * `path` - Output path (determines format from extension)
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
/// * `format` - Target pixel format (U8, U16, F16, F32)
///
/// # Example
///
/// ```ignore
/// use vfx_io::streaming::create_streaming_output;
/// use vfx_io::PixelFormat;
///
/// let mut output = create_streaming_output("output.tif", 4096, 4096, PixelFormat::F32)?;
///
/// // Write regions as they're processed
/// output.write_region(&region)?;
///
/// // Finalize writes the file
/// output.finalize()?;
/// ```
pub fn create_streaming_output<P: AsRef<Path>>(
    path: P,
    width: u32,
    height: u32,
    format: PixelFormat,
) -> IoResult<BoxedOutput> {
    let path = path.as_ref();
    let detected = Format::from_extension(path);

    match detected {
        #[cfg(feature = "tiff")]
        Format::Tiff => {
            Ok(Box::new(TiffStreamingOutput::new(path, width, height, format)?))
        }
        
        #[cfg(feature = "exr")]
        Format::Exr => {
            Ok(Box::new(ExrStreamingOutput::new(path, width, height, format)?))
        }
        
        // Fallback: file-backed memory output
        _ => {
            Ok(Box::new(FileOutput::new(path, width, height, format)))
        }
    }
}

/// Result of auto-read: either full image or streaming source.
///
/// Use pattern matching to handle both cases:
///
/// ```ignore
/// match read_auto("image.tif", None)? {
///     ImageOrStream::Image(img) => {
///         // Small image - process directly
///         process(&img);
///     }
///     ImageOrStream::Stream(mut src) => {
///         // Large image - process in tiles
///         let (w, h) = src.dimensions();
///         for y in (0..h).step_by(512) {
///             for x in (0..w).step_by(512) {
///                 let region = src.read_region(x, y, 512, 512)?;
///                 process_region(&region);
///             }
///         }
///     }
/// }
/// ```
pub enum ImageOrStream {
    /// Small image loaded entirely into memory.
    Image(crate::ImageData),
    /// Large image opened for streaming access.
    Stream(BoxedSource),
}

impl ImageOrStream {
    /// Returns true if this is a full in-memory image.
    #[inline]
    pub fn is_image(&self) -> bool {
        matches!(self, Self::Image(_))
    }

    /// Returns true if this is a streaming source.
    #[inline]
    pub fn is_stream(&self) -> bool {
        matches!(self, Self::Stream(_))
    }

    /// Unwraps to ImageData, panics if Stream.
    #[inline]
    pub fn unwrap_image(self) -> crate::ImageData {
        match self {
            Self::Image(img) => img,
            Self::Stream(_) => panic!("called unwrap_image on Stream"),
        }
    }

    /// Unwraps to BoxedSource, panics if Image.
    #[inline]
    pub fn unwrap_stream(self) -> BoxedSource {
        match self {
            Self::Image(_) => panic!("called unwrap_stream on Image"),
            Self::Stream(src) => src,
        }
    }
}

/// Default available memory assumption (8 GB).
const DEFAULT_AVAILABLE_MEMORY: u64 = 8 * 1024 * 1024 * 1024;

/// Reads an image with automatic streaming decision.
///
/// Analyzes the file to estimate memory requirements and decides:
/// - Small images: Load fully into memory (fast for small files)
/// - Large images: Return streaming source (prevents OOM)
///
/// # Arguments
///
/// * `path` - Path to the image file
/// * `available_memory` - Available RAM in bytes, or `None` for 8 GB default
///
/// # Example
///
/// ```ignore
/// use vfx_io::streaming::{read_auto, ImageOrStream};
///
/// // Auto-detect based on 16 GB available
/// let result = read_auto("huge_scan.tif", Some(16 * 1024 * 1024 * 1024))?;
///
/// match result {
///     ImageOrStream::Image(img) => println!("Loaded {}x{}", img.width, img.height),
///     ImageOrStream::Stream(src) => println!("Streaming {}x{}", src.dimensions().0, src.dimensions().1),
/// }
/// ```
pub fn read_auto<P: AsRef<Path>>(
    path: P,
    available_memory: Option<u64>,
) -> IoResult<ImageOrStream> {
    let path = path.as_ref();
    let available = available_memory.unwrap_or(DEFAULT_AVAILABLE_MEMORY);

    // Try to estimate memory
    let estimate = match format::estimate_memory(path) {
        Ok(est) => est,
        Err(_) => {
            // Can't estimate - load into memory
            return Ok(ImageOrStream::Image(crate::read(path)?));
        }
    };

    let strategy = format::ProcessingStrategy::recommend(&estimate, available);

    match strategy {
        format::ProcessingStrategy::InMemory => {
            Ok(ImageOrStream::Image(crate::read(path)?))
        }
        _ => {
            Ok(ImageOrStream::Stream(open_streaming(path)?))
        }
    }
}
