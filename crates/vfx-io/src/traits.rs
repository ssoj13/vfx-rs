//! I/O traits for image format readers and writers.
//!
//! This module defines the unified interface for all format implementations.
//! Each format (EXR, PNG, DPX, etc.) implements these traits to provide
//! consistent read/write operations.
//!
//! # Architecture
//!
//! The trait system follows a struct + trait pattern:
//!
//! ```text
//! +-----------------+     +------------------+
//! | FormatReader<O> |     | FormatWriter<O>  |
//! +-----------------+     +------------------+
//!         ^                       ^
//!         |                       |
//! +-------+-------+       +-------+-------+
//! | ExrReader     |       | ExrWriter     |
//! | PngReader     |       | PngWriter     |
//! | DpxReader     |       | DpxWriter     |
//! | ...           |       | ...           |
//! +---------------+       +---------------+
//! ```
//!
//! # Usage
//!
//! ## Simple (using free functions)
//!
//! ```rust,ignore
//! use vfx_io::{read, write};
//!
//! let image = read("input.exr")?;
//! write("output.png", &image)?;
//! ```
//!
//! ## With options (using structs)
//!
//! ```rust,ignore
//! use vfx_io::dpx::{DpxWriter, DpxWriterOptions, BitDepth};
//!
//! let writer = DpxWriter::with_options(DpxWriterOptions {
//!     bit_depth: BitDepth::Bit10,
//!     ..Default::default()
//! });
//! writer.write("output.dpx", &image)?;
//! ```
//!
//! ## From memory
//!
//! ```rust,ignore
//! use vfx_io::png::PngReader;
//!
//! let data = std::fs::read("image.png")?;
//! let reader = PngReader::new();
//! let image = reader.read_from_memory(&data)?;
//! ```

use crate::{ImageData, IoResult};
use std::io::{Read, Seek, Write};
use std::path::Path;

/// Combined trait bound for readers (Read + Seek).
///
/// Required for random-access parsing of file headers.
/// Automatically implemented for any type that implements both traits.
pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

/// Combined trait bound for writers.
pub trait WriteSeek: Write + Seek {}
impl<T: Write + Seek> WriteSeek for T {}

/// Format reader trait.
///
/// Implement this trait to add read support for a new image format.
/// Each format provides a reader struct (e.g., `DpxReader`) that
/// implements this trait.
///
/// # Type Parameter
///
/// * `O` - Reader options type. Use `()` if no options needed.
///
/// # Required Methods
///
/// * [`format_name`](Self::format_name) - Format identifier (e.g., "DPX")
/// * [`extensions`](Self::extensions) - File extensions (e.g., `["dpx"]`)
/// * [`can_read`](Self::can_read) - Magic byte detection
/// * [`read`](Self::read) - Read from file path
/// * [`read_from_memory`](Self::read_from_memory) - Read from byte buffer
///
/// # Example Implementation
///
/// ```rust,ignore
/// use vfx_io::{FormatReader, ImageData, IoResult};
///
/// #[derive(Debug, Clone, Default)]
/// pub struct MyReaderOptions {
///     pub strict: bool,
/// }
///
/// pub struct MyReader {
///     options: MyReaderOptions,
/// }
///
/// impl FormatReader<MyReaderOptions> for MyReader {
///     fn format_name(&self) -> &'static str { "MYFORMAT" }
///     fn extensions(&self) -> &'static [&'static str] { &["myf"] }
///     
///     fn can_read(&self, header: &[u8]) -> bool {
///         header.starts_with(b"MYF\x00")
///     }
///     
///     fn read<P: AsRef<Path>>(&self, path: P) -> IoResult<ImageData> {
///         let data = std::fs::read(path.as_ref())?;
///         self.read_from_memory(&data)
///     }
///     
///     fn read_from_memory(&self, data: &[u8]) -> IoResult<ImageData> {
///         // Parse format...
///         todo!()
///     }
///     
///     fn with_options(options: MyReaderOptions) -> Self {
///         Self { options }
///     }
/// }
/// ```
pub trait FormatReader<O: Default = ()>: Send + Sync {
    /// Format name for identification and error messages.
    ///
    /// Should be uppercase (e.g., "EXR", "DPX", "PNG").
    fn format_name(&self) -> &'static str;

    /// File extensions this format uses (lowercase, without dot).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// fn extensions(&self) -> &'static [&'static str] {
    ///     &["dpx", "cin"] // DPX and Cineon
    /// }
    /// ```
    fn extensions(&self) -> &'static [&'static str];

    /// Checks if this reader can parse the file based on magic bytes.
    ///
    /// Called during format auto-detection. Should be fast and
    /// only examine the first few bytes.
    ///
    /// # Arguments
    ///
    /// * `header` - First 16+ bytes of the file
    ///
    /// # Returns
    ///
    /// `true` if this format can likely parse the file.
    fn can_read(&self, header: &[u8]) -> bool;

    /// Reads an image from a file path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the image file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - File cannot be opened
    /// - File is not valid for this format
    /// - Memory allocation fails
    fn read<P: AsRef<Path>>(&self, path: P) -> IoResult<ImageData>;

    /// Reads an image from a memory buffer.
    ///
    /// Useful for embedded images, network data, or testing.
    ///
    /// # Arguments
    ///
    /// * `data` - Complete file contents as bytes
    fn read_from_memory(&self, data: &[u8]) -> IoResult<ImageData>;

    /// Creates a reader with specific options.
    ///
    /// # Arguments
    ///
    /// * `options` - Format-specific reader options
    fn with_options(options: O) -> Self
    where
        Self: Sized;
}

/// Format writer trait.
///
/// Implement this trait to add write support for an image format.
/// Each format provides a writer struct (e.g., `DpxWriter`) that
/// implements this trait.
///
/// # Type Parameter
///
/// * `O` - Writer options type. Use `()` if no options needed.
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::dpx::{DpxWriter, DpxWriterOptions, BitDepth};
///
/// // Writer with custom options
/// let writer = DpxWriter::with_options(DpxWriterOptions {
///     bit_depth: BitDepth::Bit10,
///     ..Default::default()
/// });
///
/// writer.write("output.dpx", &image)?;
///
/// // Or write to memory
/// let bytes = writer.write_to_memory(&image)?;
/// ```
pub trait FormatWriter<O: Default = ()>: Send + Sync {
    /// Format name for identification.
    fn format_name(&self) -> &'static str;

    /// File extensions this format uses.
    fn extensions(&self) -> &'static [&'static str];

    /// Writes an image to a file path.
    ///
    /// # Arguments
    ///
    /// * `path` - Output file path
    /// * `image` - Image data to write
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - File cannot be created
    /// - Image data is incompatible with format
    /// - I/O error occurs
    fn write<P: AsRef<Path>>(&self, path: P, image: &ImageData) -> IoResult<()>;

    /// Writes an image to a memory buffer.
    ///
    /// # Arguments
    ///
    /// * `image` - Image data to write
    ///
    /// # Returns
    ///
    /// Complete file contents as a byte vector.
    fn write_to_memory(&self, image: &ImageData) -> IoResult<Vec<u8>>;

    /// Creates a writer with specific options.
    fn with_options(options: O) -> Self
    where
        Self: Sized;
}

// === Legacy traits for backwards compatibility ===

/// Legacy reader trait (deprecated, use FormatReader instead).
///
/// Kept for backwards compatibility during migration.
#[deprecated(since = "0.2.0", note = "Use FormatReader instead")]
#[allow(dead_code)]
pub trait ImageReader {
    /// Reads an image from a file path.
    fn read<P: AsRef<Path>>(&self, path: P) -> IoResult<ImageData>;

    /// Reads an image from memory.
    fn read_from_memory(&self, data: &[u8]) -> IoResult<ImageData>;
}

/// Legacy writer trait (deprecated, use FormatWriter instead).
#[deprecated(since = "0.2.0", note = "Use FormatWriter instead")]
#[allow(dead_code)]
pub trait ImageWriter {
    /// Writes an image to a file path.
    fn write<P: AsRef<Path>>(&self, path: P, image: &ImageData) -> IoResult<()>;

    /// Writes an image to memory.
    fn write_to_memory(&self, image: &ImageData) -> IoResult<Vec<u8>>;
}
