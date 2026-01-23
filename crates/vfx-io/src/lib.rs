//! # vfx-io
//!
//! Image I/O for VFX pipelines.
//!
//! This crate provides reading and writing of common image formats used in
//! visual effects and film production:
//!
//! - **EXR** - OpenEXR for HDR/linear workflow
//! - **HDR** - Radiance RGBE format
//! - **PNG** - Lossless with alpha support
//! - **JPEG** - Lossy compression for previews
//! - **TIFF** - Print/archival with LZW compression
//! - **DPX** - Film scanning/output (10-bit log)
//! - **HEIF/HEIC** - Modern HDR format with PQ/HLG (requires `heif` feature)
//! - **WebP** - Modern lossy/lossless format (requires `webp` feature)
//! - **AVIF** - AV1-based format with HDR support (requires `avif` feature)
//! - **JPEG2000** - JP2/J2K for cinema/archival (requires `jp2` feature)
//!
//! # Architecture
//!
//! The crate uses a trait-based design for extensibility:
//!
//! - [`FormatReader`] - Trait for format readers with options
//! - [`FormatWriter`] - Trait for format writers with options
//! - [`read`] / [`write`] - High-level functions with format auto-detection
//!
//! Each format provides:
//! - Reader struct (e.g., `DpxReader`) implementing `FormatReader`
//! - Writer struct (e.g., `DpxWriter`) implementing `FormatWriter`
//! - Convenience functions (`dpx::read`, `dpx::write`)
//!
//! # Quick Start
//!
//! ```ignore
//! use vfx_io::{read, write};
//!
//! // Read any supported format (auto-detected)
//! let image = read("input.exr")?;
//!
//! // Write to a different format
//! write("output.png", &image)?;
//! ```
//!
//! # Format-Specific Usage
//!
//! ```ignore
//! use vfx_io::dpx::{DpxReader, DpxWriter, DpxWriterOptions, BitDepth};
//!
//! // Read with default options
//! let reader = DpxReader::default();
//! let image = reader.read("scan.0001.dpx")?;
//!
//! // Write with specific bit depth
//! let writer = DpxWriter::with_options(DpxWriterOptions {
//!     bit_depth: BitDepth::Bit10,
//!     ..Default::default()
//! });
//! writer.write("output.0001.dpx", &image)?;
//! ```
//!
//! # Metadata
//!
//! All formats extract metadata into [`Attrs`], a typed attribute container:
//!
//! ```ignore
//! use vfx_io::read;
//!
//! let image = read("photo.jpg")?;
//!
//! // Access metadata
//! if let Some(make) = image.metadata.attrs.get_str("Make") {
//!     println!("Camera: {}", make);
//! }
//! if let Some(iso) = image.metadata.attrs.get_u32("ISO") {
//!     println!("ISO: {}", iso);
//! }
//! ```
//!
//! # Supported Formats
//!
//! | Format | Read | Write | Bit Depths | Features |
//! |--------|------|-------|------------|----------|
//! | EXR | Yes | Yes | 16f, 32f | Layers, compression, metadata |
//! | HDR | Yes | Yes | 32f | RGBE, header metadata |
//! | PNG | Yes | Yes | 8, 16 | Alpha, gamma |
//! | JPEG | Yes | Yes | 8 | Quality setting |
//! | TIFF | Yes | Yes | 8, 16, 32f | LZW, Deflate compression |
//! | DPX | Yes | Yes | 8, 10, 12, 16 | Film metadata, log encoding |
//! | HEIF | Yes | Yes | 8, 10 | HDR PQ/HLG, NCLX profiles |
//!
//! # Feature Flags
//!
//! - `exr` - OpenEXR support (default)
//! - `png` - PNG support (default)
//! - `jpeg` - JPEG support (default)
//! - `tiff` - TIFF support (default)
//! - `dpx` - DPX support (default)
//! - `hdr` - Radiance HDR support (default)
//! - `heif` - HEIF/HEIC support (requires system libheif, see Cargo.toml)
//! - `webp` - WebP support (via image crate)
//! - `avif` - AVIF support (via image crate)
//! - `jp2` - JPEG2000 support (requires OpenJPEG)

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

mod error;
mod traits;
mod detect;

pub mod attrs;
pub mod registry;

#[cfg(feature = "exr")]
pub mod exr;

#[cfg(feature = "png")]
pub mod png;

#[cfg(feature = "jpeg")]
pub mod jpeg;

#[cfg(feature = "tiff")]
pub mod tiff;

#[cfg(feature = "dpx")]
pub mod dpx;

#[cfg(feature = "hdr")]
pub mod hdr;

pub mod heif;

#[cfg(feature = "webp")]
pub mod webp;

#[cfg(feature = "avif")]
pub mod avif;

#[cfg(feature = "jp2")]
pub mod jp2;

/// Adobe Photoshop PSD/PSB format.
#[cfg(feature = "psd")]
pub mod psd;

/// DirectDraw Surface (DDS) GPU texture format.
#[cfg(feature = "dds")]
pub mod dds;

/// KTX2 (Khronos Texture 2.0) format.
#[cfg(feature = "ktx")]
pub mod ktx;

/// ARRI Raw format (.ari) - requires ARRI SDK for decode.
pub mod arriraw;
/// RED REDCODE format (.r3d) - requires RED SDK for decode.
pub mod redcode;
/// Deep EXR types and utilities (stub until exrs crate publishes deep support).
#[cfg(feature = "exr")]
pub mod exr_deep;

pub mod sequence;
pub mod cinema_dng;
pub mod cache;
pub mod texture;
pub mod udim;
pub mod streaming;
pub mod imagebuf;
pub mod deepdata;
pub mod imagebufalgo;
pub mod colorconfig;

// Re-exports
pub use error::{IoError, IoResult};
pub use traits::{FormatCapability, FormatReader, FormatWriter, ReadSeek, WriteSeek};
pub use detect::Format;
pub use attrs::{Attrs, AttrValue};
pub use registry::{FormatRegistry, FormatInfo, FormatReaderDyn, FormatWriterDyn};
pub use colorconfig::ColorConfig;

use std::path::Path;
#[allow(unused_imports)]
use tracing::{debug, trace};

/// Reads an image from a file, auto-detecting the format.
///
/// The format is detected by file extension and magic bytes.
///
/// # Example
///
/// ```ignore
/// use vfx_io::read;
///
/// let image = read("input.exr")?;
/// println!("Size: {}x{}", image.width, image.height);
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be opened
/// - The format is not supported
/// - The file is corrupted
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    let path = path.as_ref();
    trace!(path = %path.display(), "vfx_io::read");
    
    let format = Format::detect(path)?;
    debug!(path = %path.display(), format = ?format, "Reading image");
    
    match format {
        #[cfg(feature = "exr")]
        Format::Exr => exr::read(path),
        
        #[cfg(feature = "png")]
        Format::Png => png::read(path),
        
        #[cfg(feature = "jpeg")]
        Format::Jpeg => jpeg::read(path),
        
        #[cfg(feature = "tiff")]
        Format::Tiff => tiff::read(path),
        
        #[cfg(feature = "dpx")]
        Format::Dpx => dpx::read(path),

        #[cfg(feature = "hdr")]
        Format::Hdr => hdr::read(path),

        #[cfg(feature = "heif")]
        Format::Heif => heif::read_heif(path).map(|(img, _hdr)| img),

        #[cfg(not(feature = "heif"))]
        Format::Heif => Err(IoError::UnsupportedFormat("HEIF support requires 'heif' feature".into())),

        #[cfg(feature = "webp")]
        Format::WebP => webp::read(path),

        #[cfg(not(feature = "webp"))]
        Format::WebP => Err(IoError::UnsupportedFormat("WebP support requires 'webp' feature".into())),

        // AVIF write-only (dav1d decoder needs pkg-config setup)
        Format::Avif => Err(IoError::UnsupportedFormat("AVIF read requires dav1d library".into())),

        #[cfg(feature = "jp2")]
        Format::Jp2 => jp2::read(path),

        #[cfg(not(feature = "jp2"))]
        Format::Jp2 => Err(IoError::UnsupportedFormat("JPEG2000 support requires 'jp2' feature".into())),

        Format::ArriRaw => arriraw::decode(path),
        Format::RedCode => redcode::decode(path, 0),

        Format::Unknown => Err(IoError::UnsupportedFormat(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_string()
        )),
    }
}

/// Reads a specific subimage and miplevel from an image file.
///
/// Most formats only support subimage=0, miplevel=0.
/// EXR supports multiple parts (subimages), TIFF supports pages.
///
/// # Arguments
/// * `path` - Path to the image file
/// * `subimage` - Subimage index (0 for most formats)
/// * `miplevel` - MIP level (0 = full resolution)
pub fn read_subimage<P: AsRef<Path>>(path: P, subimage: usize, miplevel: usize) -> IoResult<ImageData> {
    let path = path.as_ref();
    trace!(path = %path.display(), subimage, miplevel, "vfx_io::read_subimage");
    
    crate::registry::FormatRegistry::global().read_subimage(path, subimage, miplevel)
}

/// Reads deep data from a file.
///
/// Deep data contains multiple samples per pixel at different Z depths,
/// commonly used in deep compositing workflows.
///
/// Currently only EXR format supports deep data.
///
/// # Example
///
/// ```ignore
/// use vfx_io::read_deep;
///
/// let deep = read_deep("deep.exr")?;
/// println!("Pixels: {}, Channels: {}", deep.pixels(), deep.channels());
///
/// // Access samples at pixel 0
/// let num_samples = deep.samples(0);
/// for s in 0..num_samples as usize {
///     let z = deep.deep_value(0, deep.z_channel() as usize, s);
///     let a = deep.deep_value(0, deep.a_channel() as usize, s);
///     println!("Sample {}: Z={}, A={}", s, z, a);
/// }
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read
/// - The format doesn't support deep data
/// - The file is not a deep image
pub fn read_deep<P: AsRef<Path>>(path: P) -> IoResult<deepdata::DeepData> {
    let path = path.as_ref();
    trace!(path = %path.display(), "vfx_io::read_deep");
    
    crate::registry::FormatRegistry::global().read_deep(path)
}

/// Writes an image to a file, detecting format from extension.
///
/// # Example
///
/// ```ignore
/// use vfx_io::{read, write};
///
/// let image = read("input.exr")?;
/// write("output.png", &image)?;
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be created
/// - The format is not supported for writing
/// - The image data is incompatible with the format
pub fn write<P: AsRef<Path>>(path: P, image: &ImageData) -> IoResult<()> {
    let path = path.as_ref();
    trace!(path = %path.display(), "vfx_io::write");
    
    let format = Format::from_extension(path);
    debug!(
        path = %path.display(),
        format = ?format,
        w = image.width,
        h = image.height,
        ch = image.channels,
        "Writing image"
    );
    
    match format {
        #[cfg(feature = "exr")]
        Format::Exr => exr::write(path, image),
        
        #[cfg(feature = "png")]
        Format::Png => png::write(path, image),
        
        #[cfg(feature = "jpeg")]
        Format::Jpeg => jpeg::write(path, image),
        
        #[cfg(feature = "tiff")]
        Format::Tiff => tiff::write(path, image),
        
        #[cfg(feature = "dpx")]
        Format::Dpx => dpx::write(path, image),

        #[cfg(feature = "hdr")]
        Format::Hdr => hdr::write(path, image),

        #[cfg(feature = "heif")]
        Format::Heif => heif::write_heif(path, image, None),

        #[cfg(not(feature = "heif"))]
        Format::Heif => Err(IoError::UnsupportedFormat("HEIF support requires 'heif' feature".into())),

        #[cfg(feature = "webp")]
        Format::WebP => webp::write(path, image),

        #[cfg(not(feature = "webp"))]
        Format::WebP => Err(IoError::UnsupportedFormat("WebP support requires 'webp' feature".into())),

        #[cfg(feature = "avif")]
        Format::Avif => avif::write(path, image),

        #[cfg(not(feature = "avif"))]
        Format::Avif => Err(IoError::UnsupportedFormat("AVIF support requires 'avif' feature".into())),

        // JP2 is read-only - the jpeg2k crate doesn't support creating new images
        Format::Jp2 => Err(IoError::UnsupportedFormat("JPEG2000 write not supported (read-only format)".into())),

        // Camera raw formats are read-only
        Format::ArriRaw => Err(IoError::UnsupportedFormat("ARRIRAW write not supported (camera raw format)".into())),
        Format::RedCode => Err(IoError::UnsupportedFormat("REDCODE write not supported (camera raw format)".into())),

        Format::Unknown => Err(IoError::UnsupportedFormat(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_string()
        )),
    }
}

/// Writes an image to a file with explicit format override.
///
/// If `format_hint` is `Some`, uses that format regardless of file extension.
/// Otherwise, detects format from file extension.
///
/// # Arguments
///
/// * `path` - Output file path
/// * `image` - Image data to write
/// * `format_hint` - Optional format name (e.g., "exr", "png", "tiff")
pub fn write_with_format<P: AsRef<Path>>(
    path: P,
    image: &ImageData,
    format_hint: Option<&str>,
) -> IoResult<()> {
    let path = path.as_ref();
    
    // Use format hint if provided, otherwise detect from extension
    let format = match format_hint {
        Some(name) => {
            let f = Format::from_name(name);
            if f == Format::Unknown {
                // Fall back to extension if hint is invalid
                Format::from_extension(path)
            } else {
                f
            }
        }
        None => Format::from_extension(path),
    };
    
    trace!(path = %path.display(), format = ?format, "vfx_io::write_with_format");
    
    match format {
        #[cfg(feature = "exr")]
        Format::Exr => exr::write(path, image),
        
        #[cfg(feature = "png")]
        Format::Png => png::write(path, image),
        
        #[cfg(feature = "jpeg")]
        Format::Jpeg => jpeg::write(path, image),
        
        #[cfg(feature = "tiff")]
        Format::Tiff => tiff::write(path, image),
        
        #[cfg(feature = "dpx")]
        Format::Dpx => dpx::write(path, image),

        #[cfg(feature = "hdr")]
        Format::Hdr => hdr::write(path, image),

        #[cfg(feature = "heif")]
        Format::Heif => heif::write_heif(path, image, None),

        #[cfg(not(feature = "heif"))]
        Format::Heif => Err(IoError::UnsupportedFormat("HEIF support requires 'heif' feature".into())),

        #[cfg(feature = "webp")]
        Format::WebP => webp::write(path, image),

        #[cfg(not(feature = "webp"))]
        Format::WebP => Err(IoError::UnsupportedFormat("WebP support requires 'webp' feature".into())),

        #[cfg(feature = "avif")]
        Format::Avif => avif::write(path, image),

        #[cfg(not(feature = "avif"))]
        Format::Avif => Err(IoError::UnsupportedFormat("AVIF support requires 'avif' feature".into())),

        Format::Jp2 => Err(IoError::UnsupportedFormat("JPEG2000 write not supported (read-only format)".into())),
        Format::ArriRaw => Err(IoError::UnsupportedFormat("ARRIRAW write not supported (camera raw format)".into())),
        Format::RedCode => Err(IoError::UnsupportedFormat("REDCODE write not supported (camera raw format)".into())),

        Format::Unknown => Err(IoError::UnsupportedFormat(
            format_hint.unwrap_or("unknown").to_string()
        )),
    }
}

/// Probes image file to get dimensions without full decode.
///
/// Reads only the file header to extract width and height.
/// Much faster than `read()` for large files.
///
/// # Example
///
/// ```ignore
/// use vfx_io::probe_dimensions;
///
/// let (width, height) = probe_dimensions("large_render.exr")?;
/// println!("Image is {}x{}", width, height);
/// ```
///
/// # Supported Formats
///
/// - EXR: Reads header attributes
/// - PNG: Reads IHDR chunk (first 33 bytes)
/// - JPEG: Scans for SOF marker
/// - DPX: Reads file/image headers
/// - TIFF: Reads IFD tags
/// - HDR: Parses text header
pub fn probe_dimensions<P: AsRef<Path>>(path: P) -> IoResult<(u32, u32)> {
    use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
    use std::fs::File;
    
    let path = path.as_ref();
    let format = Format::detect(path)?;
    
    match format {
        #[cfg(feature = "png")]
        Format::Png => {
            // PNG: 8 bytes signature + IHDR chunk (4 len + 4 type + 4 width + 4 height)
            let mut file = File::open(path)?;
            let mut buf = [0u8; 24];
            file.read_exact(&mut buf)?;
            
            // Width at offset 16, height at offset 20 (big-endian)
            let width = u32::from_be_bytes([buf[16], buf[17], buf[18], buf[19]]);
            let height = u32::from_be_bytes([buf[20], buf[21], buf[22], buf[23]]);
            Ok((width, height))
        }
        
        #[cfg(feature = "jpeg")]
        Format::Jpeg => {
            // JPEG: scan for SOF0/SOF2 marker
            let mut file = File::open(path)?;
            let mut buf = [0u8; 2];
            
            // Skip SOI marker
            file.read_exact(&mut buf)?;
            if buf != [0xFF, 0xD8] {
                return Err(IoError::DecodeError("Invalid JPEG".into()));
            }
            
            loop {
                // Read marker
                file.read_exact(&mut buf)?;
                if buf[0] != 0xFF {
                    return Err(IoError::DecodeError("Invalid JPEG marker".into()));
                }
                
                let marker = buf[1];
                
                // SOF0, SOF1, SOF2 (baseline, extended, progressive)
                if matches!(marker, 0xC0 | 0xC1 | 0xC2) {
                    let mut header = [0u8; 7];
                    file.read_exact(&mut header)?;
                    // Skip length (2) and precision (1), then height (2) and width (2)
                    let height = u16::from_be_bytes([header[3], header[4]]) as u32;
                    let width = u16::from_be_bytes([header[5], header[6]]) as u32;
                    return Ok((width, height));
                }
                
                // EOI or SOS - no dimensions found
                if marker == 0xD9 || marker == 0xDA {
                    return Err(IoError::DecodeError("JPEG dimensions not found".into()));
                }
                
                // Skip segment (RST markers 0xD0-0xD7 have no length)
                if marker != 0x00 && marker != 0xFF && !matches!(marker, 0xD0..=0xD7) {
                    file.read_exact(&mut buf)?;
                    let len = u16::from_be_bytes(buf) as i64 - 2;
                    if len > 0 {
                        file.seek(SeekFrom::Current(len))?;
                    }
                }
            }
        }
        
        #[cfg(feature = "dpx")]
        Format::Dpx => {
            // DPX: magic (4) + offset (4) + version (8) + file_size (4) + ... image header at 768
            let mut file = File::open(path)?;
            let mut magic = [0u8; 4];
            file.read_exact(&mut magic)?;
            
            let big_endian = &magic == b"SDPX";
            
            // Image header starts at offset 768, width at 772, height at 776
            file.seek(SeekFrom::Start(772))?;
            let mut dims = [0u8; 8];
            file.read_exact(&mut dims)?;
            
            let (width, height) = if big_endian {
                (
                    u32::from_be_bytes([dims[0], dims[1], dims[2], dims[3]]),
                    u32::from_be_bytes([dims[4], dims[5], dims[6], dims[7]]),
                )
            } else {
                (
                    u32::from_le_bytes([dims[0], dims[1], dims[2], dims[3]]),
                    u32::from_le_bytes([dims[4], dims[5], dims[6], dims[7]]),
                )
            };
            Ok((width, height))
        }
        
        #[cfg(feature = "hdr")]
        Format::Hdr => {
            // HDR: text header ending with resolution line "-Y height +X width"
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            
            for line in reader.lines() {
                let line = line?;
                // Resolution format: "-Y height +X width" or "+X width -Y height"
                if line.starts_with("-Y ") || line.starts_with("+Y ") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        let height: u32 = parts[1].parse().map_err(|_| 
                            IoError::DecodeError("Invalid HDR height".into()))?
                        ;
                        let width: u32 = parts[3].parse().map_err(|_| 
                            IoError::DecodeError("Invalid HDR width".into()))?
                        ;
                        return Ok((width, height));
                    }
                }
            }
            Err(IoError::DecodeError("HDR resolution not found".into()))
        }
        
        #[cfg(feature = "exr")]
        Format::Exr => exr::probe_dimensions(path),
        
        #[cfg(feature = "tiff")]
        Format::Tiff => {
            // TIFF: parse IFD for ImageWidth/ImageLength tags
            tiff::probe_dimensions(path)
        }
        
        // Fallback: full read for unsupported formats
        _ => {
            let img = read(path)?;
            Ok((img.width, img.height))
        }
    }
}

/// Probes image dimensions and channel count without full decode.
///
/// Returns (width, height, channels) tuple.
pub fn probe_image_info<P: AsRef<Path>>(path: P) -> IoResult<(u32, u32, u32)> {
    use std::io::{Read, Seek, SeekFrom};
    use std::fs::File;
    
    let path = path.as_ref();
    let format = Format::detect(path)?;
    
    match format {
        #[cfg(feature = "png")]
        Format::Png => {
            // PNG IHDR: width(4) + height(4) + bit_depth(1) + color_type(1)
            let mut file = File::open(path)?;
            let mut buf = [0u8; 26];
            file.read_exact(&mut buf)?;
            
            let width = u32::from_be_bytes([buf[16], buf[17], buf[18], buf[19]]);
            let height = u32::from_be_bytes([buf[20], buf[21], buf[22], buf[23]]);
            let color_type = buf[25];
            
            // PNG color types: 0=gray, 2=RGB, 3=indexed, 4=gray+alpha, 6=RGBA
            let channels = match color_type {
                0 => 1,
                2 => 3,
                3 => 3, // indexed treated as RGB
                4 => 2,
                6 => 4,
                _ => 4,
            };
            Ok((width, height, channels))
        }
        
        #[cfg(feature = "jpeg")]
        Format::Jpeg => {
            // JPEG SOF contains num_components
            let mut file = File::open(path)?;
            let mut buf = [0u8; 2];
            
            file.read_exact(&mut buf)?;
            if buf != [0xFF, 0xD8] {
                return Err(IoError::DecodeError("Invalid JPEG".into()));
            }
            
            loop {
                file.read_exact(&mut buf)?;
                if buf[0] != 0xFF {
                    return Err(IoError::DecodeError("Invalid JPEG marker".into()));
                }
                
                let marker = buf[1];
                
                if matches!(marker, 0xC0 | 0xC1 | 0xC2) {
                    let mut header = [0u8; 8];
                    file.read_exact(&mut header)?;
                    let height = u16::from_be_bytes([header[3], header[4]]) as u32;
                    let width = u16::from_be_bytes([header[5], header[6]]) as u32;
                    let channels = header[7] as u32; // num_components
                    return Ok((width, height, channels));
                }
                
                if marker == 0xD9 || marker == 0xDA {
                    return Err(IoError::DecodeError("JPEG info not found".into()));
                }
                
                if !matches!(marker, 0xD0..=0xD7 | 0x01) {
                    file.read_exact(&mut buf)?;
                    let len = u16::from_be_bytes(buf) as i64 - 2;
                    file.seek(SeekFrom::Current(len))?;
                }
            }
        }
        
        #[cfg(feature = "dpx")]
        Format::Dpx => {
            let img = dpx::read(path)?;
            Ok((img.width, img.height, img.channels))
        }
        
        #[cfg(feature = "hdr")]
        Format::Hdr => {
            // HDR is always RGB (3 channels)
            let (w, h) = probe_dimensions(path)?;
            Ok((w, h, 3))
        }
        
        #[cfg(feature = "exr")]
        Format::Exr => {
            let (w, h) = exr::probe_dimensions(path)?;
            // EXR channel count from metadata
            if let Ok(meta) = vfx_exr::meta::MetaData::read_from_file(path, false) {
                if let Some(header) = meta.headers.first() {
                    let ch = header.channels.list.len() as u32;
                    return Ok((w, h, ch.max(1)));
                }
            }
            Ok((w, h, 4)) // fallback to RGBA
        }
        
        #[cfg(feature = "tiff")]
        Format::Tiff => {
            let img = tiff::read(path)?;
            Ok((img.width, img.height, img.channels))
        }
        
        // Fallback: full read
        _ => {
            let img = read(path)?;
            Ok((img.width, img.height, img.channels))
        }
    }
}

/// Image data container for I/O operations.
///
/// This is a format-agnostic container that holds pixel data
/// along with metadata. It can represent various bit depths
/// and channel configurations.
///
/// # Fields
///
/// - `width`, `height` - Image dimensions in pixels
/// - `channels` - Number of channels (3 for RGB, 4 for RGBA)
/// - `format` - Pixel data format (U8, U16, F16, F32, U32)
/// - `data` - Raw pixel data
/// - `metadata` - Format-specific metadata
///
/// # Example
///
/// ```ignore
/// use vfx_io::{ImageData, PixelFormat};
///
/// // Create a 1920x1080 RGB float image
/// let image = ImageData::new(1920, 1080, 3, PixelFormat::F32);
///
/// // Create from existing data
/// let data = vec![0.5f32; 1920 * 1080 * 3];
/// let image = ImageData::from_f32(1920, 1080, 3, data);
/// ```
#[derive(Debug, Clone)]
pub struct ImageData {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Number of channels (3 for RGB, 4 for RGBA).
    pub channels: u32,
    /// Pixel data format.
    pub format: PixelFormat,
    /// Raw pixel data.
    pub data: PixelData,
    /// Image metadata.
    pub metadata: Metadata,
}

/// Semantic meaning of a channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelKind {
    /// Color data (RGB/YCbCr/etc).
    Color,
    /// Alpha/opacity.
    Alpha,
    /// Depth/Z or similar distance data.
    Depth,
    /// Object or material identifiers.
    Id,
    /// Matte/mask data.
    Mask,
    /// Unknown or non-color data.
    Generic,
}

/// The sample type stored for a channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelSampleType {
    /// 16-bit float (stored as f32 values in memory).
    F16,
    /// 32-bit float.
    F32,
    /// 32-bit unsigned integer.
    U32,
}

/// Channel sample storage.
#[derive(Debug, Clone)]
pub enum ChannelSamples {
    /// Float sample storage (used for F16 and F32).
    F32(Vec<f32>),
    /// Integer sample storage.
    U32(Vec<u32>),
}

impl ChannelSamples {
    /// Number of samples in this channel.
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            Self::F32(data) => data.len(),
            Self::U32(data) => data.len(),
        }
    }

    /// Returns true if no samples.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get reference to F32 data if this variant.
    #[inline]
    pub fn as_f32(&self) -> Option<&Vec<f32>> {
        match self {
            Self::F32(data) => Some(data),
            Self::U32(_) => None,
        }
    }

    /// Get mutable reference to F32 data if this variant.
    #[inline]
    pub fn as_f32_mut(&mut self) -> Option<&mut Vec<f32>> {
        match self {
            Self::F32(data) => Some(data),
            Self::U32(_) => None,
        }
    }

    /// Get reference to U32 data if this variant.
    #[inline]
    pub fn as_u32(&self) -> Option<&Vec<u32>> {
        match self {
            Self::U32(data) => Some(data),
            Self::F32(_) => None,
        }
    }

    /// Convert to F32, casting U32 if needed.
    pub fn to_f32(&self) -> Vec<f32> {
        match self {
            Self::F32(data) => data.clone(),
            Self::U32(data) => data.iter().map(|&v| v as f32).collect(),
        }
    }

    /// Consume and return F32 data, casting U32 if needed.
    pub fn into_f32(self) -> Vec<f32> {
        match self {
            Self::F32(data) => data,
            Self::U32(data) => data.into_iter().map(|v| v as f32).collect(),
        }
    }
}

/// A single image channel.
#[derive(Debug, Clone)]
pub struct ImageChannel {
    /// Channel name (e.g., "R", "G", "B", "A", "Z", "ID").
    pub name: String,
    /// Semantic meaning of this channel.
    pub kind: ChannelKind,
    /// The intended sample type for serialization.
    pub sample_type: ChannelSampleType,
    /// Channel samples.
    pub samples: ChannelSamples,
    /// Channel subsampling (x, y).
    pub sampling: (usize, usize),
    /// Whether to quantize linearly (for lossy compression hints).
    pub quantize_linearly: bool,
}

/// A single named image layer with arbitrary channels.
#[derive(Debug, Clone)]
pub struct ImageLayer {
    /// Layer name (e.g., "beauty", "spec", "depth").
    pub name: String,
    /// Layer width in pixels.
    pub width: u32,
    /// Layer height in pixels.
    pub height: u32,
    /// Ordered list of channels in this layer.
    pub channels: Vec<ImageChannel>,
}

/// A multi-layer image container.
#[derive(Debug, Clone, Default)]
pub struct LayeredImage {
    /// Ordered list of layers.
    pub layers: Vec<ImageLayer>,
    /// Image-level metadata.
    pub metadata: Metadata,
}

/// Re-export PixelFormat from vfx-core for backward compatibility.
///
/// This is now an alias for [`vfx_core::DataFormat`].
pub use vfx_core::DataFormat as PixelFormat;

/// Raw pixel data storage.
///
/// The variant matches the [`PixelFormat`].
#[derive(Debug, Clone)]
pub enum PixelData {
    /// 8-bit unsigned data.
    U8(Vec<u8>),
    /// 16-bit unsigned data.
    U16(Vec<u16>),
    /// 32-bit float data (also used for F16 after conversion).
    F32(Vec<f32>),
    /// 32-bit unsigned data.
    U32(Vec<u32>),
}

/// Image metadata container.
///
/// Stores both common metadata fields and format-specific attributes.
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    /// Color space name (e.g., "sRGB", "ACEScg", "linear", "log").
    pub colorspace: Option<String>,
    /// Gamma value if applicable.
    pub gamma: Option<f32>,
    /// DPI/PPI for print.
    pub dpi: Option<f32>,
    /// Typed attributes (format-specific).
    pub attrs: Attrs,
}

impl ImageData {
    /// Creates a new ImageData with the given dimensions and format.
    ///
    /// Pixel data is initialized to zero.
    pub fn new(width: u32, height: u32, channels: u32, format: PixelFormat) -> Self {
        let size = (width * height * channels) as usize;
        let data = match format {
            PixelFormat::U8 => PixelData::U8(vec![0u8; size]),
            PixelFormat::U16 => PixelData::U16(vec![0u16; size]),
            PixelFormat::F16 | PixelFormat::F32 => PixelData::F32(vec![0.0f32; size]),
            PixelFormat::U32 => PixelData::U32(vec![0u32; size]),
        };
        
        Self {
            width,
            height,
            channels,
            format,
            data,
            metadata: Metadata::default(),
        }
    }
    
    /// Creates ImageData from f32 pixel data.
    pub fn from_f32(width: u32, height: u32, channels: u32, data: Vec<f32>) -> Self {
        Self {
            width,
            height,
            channels,
            format: PixelFormat::F32,
            data: PixelData::F32(data),
            metadata: Metadata::default(),
        }
    }
    
    /// Creates ImageData from u8 pixel data.
    pub fn from_u8(width: u32, height: u32, channels: u32, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            channels,
            format: PixelFormat::U8,
            data: PixelData::U8(data),
            metadata: Metadata::default(),
        }
    }

    /// Creates ImageData from u32 pixel data.
    pub fn from_u32(width: u32, height: u32, channels: u32, data: Vec<u32>) -> Self {
        Self {
            width,
            height,
            channels,
            format: PixelFormat::U32,
            data: PixelData::U32(data),
            metadata: Metadata::default(),
        }
    }
    
    /// Returns the total number of pixels.
    #[inline]
    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }
    
    /// Returns the total number of samples (pixels * channels).
    #[inline]
    pub fn sample_count(&self) -> usize {
        (self.width * self.height * self.channels) as usize
    }
    
    /// Converts pixel data to f32 (for processing).
    ///
    /// Values are normalized to 0.0-1.0 range for U8/U16; U32 is cast without normalization.
    pub fn to_f32(&self) -> Vec<f32> {
        match &self.data {
            PixelData::U8(data) => data.iter().map(|&v| v as f32 / 255.0).collect(),
            PixelData::U16(data) => data.iter().map(|&v| v as f32 / 65535.0).collect(),
            PixelData::F32(data) => data.clone(),
            PixelData::U32(data) => data.iter().map(|&v| v as f32).collect(),
        }
    }
    
    /// Converts pixel data to u8 (for display/saving).
    ///
    /// Float values are clamped to 0.0-1.0 and scaled to 0-255.
    pub fn to_u8(&self) -> Vec<u8> {
        match &self.data {
            PixelData::U8(data) => data.clone(),
            PixelData::U16(data) => data.iter().map(|&v| (v >> 8) as u8).collect(),
            PixelData::F32(data) => data.iter().map(|&v| (v.clamp(0.0, 1.0) * 255.0) as u8).collect(),
            PixelData::U32(data) => data.iter().map(|&v| v.min(u8::MAX as u32) as u8).collect(),
        }
    }

    /// Converts pixel data to u16 (for 16-bit output).
    ///
    /// Float values are clamped to 0.0-1.0 and scaled to 0-65535.
    pub fn to_u16(&self) -> Vec<u16> {
        match &self.data {
            PixelData::U8(data) => data.iter().map(|&v| (v as u16) << 8 | v as u16).collect(),
            PixelData::U16(data) => data.clone(),
            PixelData::F32(data) => data.iter().map(|&v| (v.clamp(0.0, 1.0) * 65535.0) as u16).collect(),
            PixelData::U32(data) => data.iter().map(|&v| v.min(u16::MAX as u32) as u16).collect(),
        }
    }

    /// Converts image to specified pixel format (bit depth).
    ///
    /// # Example
    /// ```ignore
    /// let f32_image = image.convert_to(PixelFormat::F32);
    /// let u8_image = image.convert_to(PixelFormat::U8);
    /// ```
    pub fn convert_to(&self, format: PixelFormat) -> Self {
        if self.format == format {
            return self.clone();
        }
        
        let data = match format {
            PixelFormat::U8 => PixelData::U8(self.to_u8()),
            PixelFormat::U16 => PixelData::U16(self.to_u16()),
            PixelFormat::F16 | PixelFormat::F32 => PixelData::F32(self.to_f32()),
            PixelFormat::U32 => {
                // Convert via f32, then to u32 (assumes normalized 0-1 data)
                let f32_data = self.to_f32();
                PixelData::U32(f32_data.iter().map(|&v| (v.max(0.0) * u32::MAX as f32) as u32).collect())
            }
        };
        
        Self {
            width: self.width,
            height: self.height,
            channels: self.channels,
            format,
            data,
            metadata: self.metadata.clone(),
        }
    }

    /// Converts this image into a single named layer with planar channels.
    pub fn to_layer(&self, name: impl Into<String>) -> ImageLayer {
        let name = name.into();
        let channel_names = default_channel_names(self.channels as usize);
        let pixel_count = self.pixel_count();
        let channels = self.channels as usize;

        let mut out_channels = Vec::with_capacity(channels);
        for (ch_index, ch_name) in channel_names.into_iter().enumerate() {
            let quantize_linearly = matches!(ch_name.as_str(), "A") || ch_name.starts_with("C");
            match &self.data {
                PixelData::U32(data) => {
                    let mut samples = Vec::with_capacity(pixel_count);
                    for i in 0..pixel_count {
                        let idx = i * channels + ch_index;
                        samples.push(*data.get(idx).unwrap_or(&0u32));
                    }
                    let kind = channel_kind_from_name(&ch_name, ChannelSampleType::U32);
                    out_channels.push(ImageChannel {
                        name: ch_name,
                        kind,
                        sample_type: ChannelSampleType::U32,
                        samples: ChannelSamples::U32(samples),
                        sampling: (1, 1),
                        quantize_linearly,
                    });
                }
                _ => {
                    let interleaved = self.to_f32();
                    let mut samples = Vec::with_capacity(pixel_count);
                    for i in 0..pixel_count {
                        let idx = i * channels + ch_index;
                        samples.push(interleaved.get(idx).copied().unwrap_or(0.0));
                    }
                    let kind = channel_kind_from_name(&ch_name, ChannelSampleType::F32);
                    out_channels.push(ImageChannel {
                        name: ch_name,
                        kind,
                        sample_type: ChannelSampleType::F32,
                        samples: ChannelSamples::F32(samples),
                        sampling: (1, 1),
                        quantize_linearly,
                    });
                }
            }
        }

        ImageLayer {
            name,
            width: self.width,
            height: self.height,
            channels: out_channels,
        }
    }

    /// Converts this image into a layered container with a single layer.
    pub fn to_layered(&self, name: impl Into<String>) -> LayeredImage {
        LayeredImage {
            layers: vec![self.to_layer(name)],
            metadata: self.metadata.clone(),
        }
    }

    /// Validates that this image is safe for color processing operations.
    pub fn ensure_color_processing(&self, allow_non_color: bool, op: &str) -> IoResult<()> {
        if allow_non_color {
            return Ok(());
        }

        let layer = self.to_layer("input");
        for channel in &layer.channels {
            match channel.kind {
                ChannelKind::Color | ChannelKind::Alpha | ChannelKind::Depth => {}
                ChannelKind::Id | ChannelKind::Mask | ChannelKind::Generic => {
                    return Err(IoError::UnsupportedOperation(format!(
                        "{} is not supported for channel '{}' (kind: {:?})",
                        op, channel.name, channel.kind
                    )));
                }
            }
        }

        Ok(())
    }
}

impl ImageLayer {
    /// Attempts to convert this layer into a packed ImageData.
    ///
    /// If all channels are U32, the output is U32; otherwise U32 channels are cast to f32.
    pub fn to_image_data(&self) -> IoResult<ImageData> {
        self.to_image_data_with_order(&[])
    }

    /// Attempts to convert this layer into ImageData using an explicit channel order.
    ///
    /// If `order` is empty, a preferred default order is used.
    /// If all channels are U32, the output is U32; otherwise U32 channels are cast to f32.
    pub fn to_image_data_with_order(&self, order: &[&str]) -> IoResult<ImageData> {
        if self.channels.is_empty() {
            return Err(IoError::DecodeError("Layer has no channels".into()));
        }

        let pixel_count = (self.width as usize) * (self.height as usize);
        for channel in &self.channels {
            if channel.sampling != (1, 1) {
                return Err(IoError::DecodeError(
                    "Cannot convert subsampled channels to ImageData".into(),
                ));
            }
            match (&channel.sample_type, &channel.samples) {
                (ChannelSampleType::F16 | ChannelSampleType::F32, ChannelSamples::F32(values)) => {
                    if values.len() != pixel_count {
                        return Err(IoError::DecodeError(format!(
                            "Channel {} has {} samples, expected {}",
                            channel.name,
                            values.len(),
                            pixel_count
                        )));
                    }
                }
                (ChannelSampleType::U32, ChannelSamples::U32(values)) => {
                    if values.len() != pixel_count {
                        return Err(IoError::DecodeError(format!(
                            "Channel {} has {} samples, expected {}",
                            channel.name,
                            values.len(),
                            pixel_count
                        )));
                    }
                }
                _ => {
                    return Err(IoError::DecodeError(
                        "Unsupported channel sample storage".into(),
                    ));
                }
            }
        }

        let order = if order.is_empty() {
            preferred_channel_order(&self.channels)
        } else {
            let mut indices = Vec::with_capacity(order.len());
            for &name in order {
                let idx = self
                    .channels
                    .iter()
                    .position(|ch| ch.name == name)
                    .ok_or_else(|| {
                        IoError::DecodeError(format!(
                            "Channel {} not found in layer",
                            name
                        ))
                    })?;
                indices.push(idx);
            }
            indices
        };
        let all_u32 = order.iter().all(|&idx| matches!(self.channels[idx].sample_type, ChannelSampleType::U32));
        if all_u32 {
            let mut interleaved = Vec::with_capacity(pixel_count * order.len());
            for i in 0..pixel_count {
                for &idx in &order {
                    let channel = &self.channels[idx];
                    let ChannelSamples::U32(values) = &channel.samples else {
                        return Err(IoError::DecodeError(
                            "Unsupported channel sample storage".into(),
                        ));
                    };
                    interleaved.push(values[i]);
                }
            }

            return Ok(ImageData {
                width: self.width,
                height: self.height,
                channels: order.len() as u32,
                format: PixelFormat::U32,
                data: PixelData::U32(interleaved),
                metadata: Metadata::default(),
            });
        }

        let mut interleaved = Vec::with_capacity(pixel_count * order.len());

        for i in 0..pixel_count {
            for &idx in &order {
                let channel = &self.channels[idx];
                match &channel.samples {
                    ChannelSamples::F32(values) => {
                        interleaved.push(values[i]);
                    }
                    ChannelSamples::U32(values) => {
                        interleaved.push(values[i] as f32);
                    }
                }
            }
        }

        Ok(ImageData {
            width: self.width,
            height: self.height,
            channels: order.len() as u32,
            format: PixelFormat::F32,
            data: PixelData::F32(interleaved),
            metadata: Metadata::default(),
        })
    }
}

impl LayeredImage {
    /// Attempts to convert a single-layer image into ImageData.
    pub fn to_image_data(&self) -> IoResult<ImageData> {
        match self.layers.as_slice() {
            [layer] => layer.to_image_data(),
            [] => Err(IoError::DecodeError("No layers available".into())),
            _ => Err(IoError::DecodeError(
                "Multiple layers cannot be converted to ImageData".into(),
            )),
        }
    }

    /// Attempts to convert a single-layer image into ImageData using an explicit channel order.
    pub fn to_image_data_with_order(&self, order: &[&str]) -> IoResult<ImageData> {
        match self.layers.as_slice() {
            [layer] => layer.to_image_data_with_order(order),
            [] => Err(IoError::DecodeError("No layers available".into())),
            _ => Err(IoError::DecodeError(
                "Multiple layers cannot be converted to ImageData".into(),
            )),
        }
    }
}

fn default_channel_names(count: usize) -> Vec<String> {
    match count {
        1 => vec!["Y".to_string()],
        2 => vec!["Y".to_string(), "A".to_string()],
        3 => vec!["R".to_string(), "G".to_string(), "B".to_string()],
        4 => vec![
            "R".to_string(),
            "G".to_string(),
            "B".to_string(),
            "A".to_string(),
        ],
        _ => (0..count).map(|i| format!("C{}", i)).collect(),
    }
}

pub(crate) fn channel_kind_from_name(name: &str, sample_type: ChannelSampleType) -> ChannelKind {
    let upper = name.to_ascii_uppercase();
    match upper.as_str() {
        "A" | "ALPHA" => ChannelKind::Alpha,
        "Z" | "DEPTH" => ChannelKind::Depth,
        "ID" | "OBJECTID" | "OBJECT_ID" => ChannelKind::Id,
        "MASK" | "MATTE" => ChannelKind::Mask,
        "R" | "G" | "B" | "Y" => {
            if sample_type == ChannelSampleType::U32 {
                ChannelKind::Id
            } else {
                ChannelKind::Color
            }
        }
        _ => {
            if sample_type == ChannelSampleType::U32 {
                ChannelKind::Id
            } else {
                ChannelKind::Generic
            }
        }
    }
}

fn preferred_channel_order(channels: &[ImageChannel]) -> Vec<usize> {
    let mut indices = Vec::with_capacity(channels.len());
    let find = |name: &str| channels.iter().position(|ch| ch.name == name);

    if channels.len() == 1 {
        if let Some(y) = find("Y") {
            return vec![y];
        }
    }
    if channels.len() == 2 {
        if let (Some(y), Some(a)) = (find("Y"), find("A")) {
            return vec![y, a];
        }
    }
    if channels.len() >= 3 {
        if let (Some(r), Some(g), Some(b)) = (find("R"), find("G"), find("B")) {
            indices.push(r);
            indices.push(g);
            indices.push(b);
            if channels.len() == 4 {
                if let Some(a) = find("A") {
                    indices.push(a);
                    return indices;
                }
            }
            if channels.len() == 3 {
                return indices;
            }
            indices.clear();
        }
    }

    (0..channels.len()).collect()
}

// === Backwards compatibility ===
// Keep old metadata module re-exports working

/// Old AttrValue re-export for backwards compatibility.
#[doc(hidden)]
#[deprecated(since = "0.2.0", note = "Use vfx_io::attrs::AttrValue instead")]
pub mod metadata {
    //! Legacy metadata module (deprecated).
    //!
    //! Use `vfx_io::attrs` instead.
    pub use crate::attrs::{Attrs, AttrValue};
}
