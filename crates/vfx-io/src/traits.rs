//! I/O traits for image readers and writers.
//!
//! These traits define the interface for format-specific implementations.

use crate::{ImageData, IoResult};
use std::path::Path;

/// Trait for image format readers.
///
/// Implement this trait to add support for reading a new image format.
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::{ImageReader, ImageData, IoResult};
///
/// struct MyFormatReader;
///
/// impl ImageReader for MyFormatReader {
///     fn read<P: AsRef<Path>>(&self, path: P) -> IoResult<ImageData> {
///         // Read implementation
///     }
///     
///     fn read_from_memory(&self, data: &[u8]) -> IoResult<ImageData> {
///         // Memory read implementation
///     }
/// }
/// ```
pub trait ImageReader {
    /// Reads an image from a file path.
    fn read<P: AsRef<Path>>(&self, path: P) -> IoResult<ImageData>;
    
    /// Reads an image from memory.
    fn read_from_memory(&self, data: &[u8]) -> IoResult<ImageData>;
}

/// Trait for image format writers.
///
/// Implement this trait to add support for writing a new image format.
pub trait ImageWriter {
    /// Writes an image to a file path.
    fn write<P: AsRef<Path>>(&self, path: P, image: &ImageData) -> IoResult<()>;
    
    /// Writes an image to memory.
    fn write_to_memory(&self, image: &ImageData) -> IoResult<Vec<u8>>;
}
