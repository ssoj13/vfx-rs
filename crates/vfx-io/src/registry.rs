//! Format registry for dynamic format detection and handling.
//!
//! The registry provides a centralized way to:
//! - Register format readers and writers
//! - Auto-detect formats by magic bytes or extension
//! - Get format handlers by name
//!
//! # Architecture
//!
//! The registry uses a singleton pattern via [`FormatRegistry::global()`].
//! Built-in formats are registered automatically at startup.
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::registry::FormatRegistry;
//!
//! let registry = FormatRegistry::global();
//!
//! // List all registered formats
//! for name in registry.format_names() {
//!     println!("Format: {}", name);
//! }
//!
//! // Check if format is supported
//! if registry.supports_extension("exr") {
//!     println!("EXR is supported!");
//! }
//!
//! // Detect format from file header
//! let header = &[0x76, 0x2F, 0x31, 0x01]; // EXR magic
//! if let Some(name) = registry.detect_format(header) {
//!     println!("Detected: {}", name);
//! }
//! ```

use crate::{ImageData, IoResult, FormatReader, FormatWriter, FormatCapability};
use crate::deepdata::DeepData;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};

/// Format information entry in the registry.
#[derive(Clone)]
pub struct FormatInfo {
    /// Human-readable format name (e.g., "OpenEXR", "PNG").
    pub name: &'static str,
    /// File extensions without dots (e.g., ["exr"], ["jpg", "jpeg"]).
    pub extensions: &'static [&'static str],
    /// Function to check if header bytes match this format.
    pub can_read: fn(&[u8]) -> bool,
    /// Function to read from a path.
    pub read_path: fn(&Path) -> IoResult<ImageData>,
    /// Function to read from memory.
    pub read_memory: fn(&[u8]) -> IoResult<ImageData>,
    /// Function to read specific subimage/miplevel from path.
    pub read_subimage_path: Option<fn(&Path, usize, usize) -> IoResult<ImageData>>,
    /// Function to get number of subimages.
    pub num_subimages: Option<fn(&Path) -> IoResult<usize>>,
    /// Function to get number of miplevels for subimage.
    pub num_miplevels: Option<fn(&Path, usize) -> IoResult<usize>>,
    /// Function to write to a path (None if write not supported).
    pub write_path: Option<fn(&Path, &ImageData) -> IoResult<()>>,
    /// Function to write to memory (None if write not supported).
    pub write_memory: Option<fn(&ImageData) -> IoResult<Vec<u8>>>,
    /// Capabilities supported by this format.
    pub capabilities: &'static [FormatCapability],
    /// Function to read deep data from path (None if deep not supported).
    pub read_deep_path: Option<fn(&Path) -> IoResult<DeepData>>,
}

/// Dynamic format reader trait (object-safe).
pub trait FormatReaderDyn: Send + Sync {
    /// Format name.
    fn format_name(&self) -> &'static str;
    /// Supported extensions.
    fn extensions(&self) -> &'static [&'static str];
    /// Check if header matches.
    fn can_read(&self, header: &[u8]) -> bool;
    /// Read from path.
    fn read_path(&self, path: &Path) -> IoResult<ImageData>;
    /// Read from memory.
    fn read_memory(&self, data: &[u8]) -> IoResult<ImageData>;
}

/// Dynamic format writer trait (object-safe).
pub trait FormatWriterDyn: Send + Sync {
    /// Format name.
    fn format_name(&self) -> &'static str;
    /// Supported extensions.
    fn extensions(&self) -> &'static [&'static str];
    /// Write to path.
    fn write_path(&self, path: &Path, image: &ImageData) -> IoResult<()>;
    /// Write to memory.
    fn write_memory(&self, image: &ImageData) -> IoResult<Vec<u8>>;
}

/// Central registry for image format handlers.
///
/// Provides format detection, reader/writer creation, and format enumeration.
///
/// # Thread Safety
///
/// The registry is thread-safe and uses internal synchronization.
/// The global instance can be accessed from any thread.
///
/// # Example
///
/// ```ignore
/// use vfx_io::registry::FormatRegistry;
///
/// let registry = FormatRegistry::global();
///
/// // Get all format names
/// let formats: Vec<_> = registry.format_names().collect();
/// println!("Supported: {:?}", formats);
///
/// // Check extension support
/// assert!(registry.supports_extension("png"));
/// ```
pub struct FormatRegistry {
    formats: HashMap<&'static str, Arc<FormatInfo>>,
    by_extension: HashMap<&'static str, &'static str>,
}

impl FormatRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            formats: HashMap::new(),
            by_extension: HashMap::new(),
        }
    }

    /// Returns the global registry instance with built-in formats.
    ///
    /// Built-in formats are registered automatically based on enabled features.
    pub fn global() -> &'static FormatRegistry {
        static INSTANCE: OnceLock<FormatRegistry> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            let mut registry = FormatRegistry::new();
            registry.register_builtin_formats();
            registry
        })
    }

    /// Registers built-in formats based on enabled features.
    fn register_builtin_formats(&mut self) {
        #[cfg(feature = "dpx")]
        self.register(FormatInfo {
            name: "DPX",
            extensions: &["dpx"],
            can_read: |h| {
                h.len() >= 4
                    && ((h[0] == 0x53 && h[1] == 0x44 && h[2] == 0x50 && h[3] == 0x58)
                        || (h[0] == 0x58 && h[1] == 0x50 && h[2] == 0x44 && h[3] == 0x53))
            },
            read_path: |p| crate::dpx::read(p),
            read_memory: |d| crate::dpx::DpxReader::new().read_from_memory(d),
            read_subimage_path: None,
            num_subimages: None,
            num_miplevels: None,
            write_path: Some(|p, i| crate::dpx::write(p, i)),
            write_memory: Some(|i| crate::dpx::DpxWriter::new().write_to_memory(i)),
            capabilities: &[FormatCapability::IoProxy],
            read_deep_path: None, // DPX doesn't support deep data
        });

        #[cfg(feature = "exr")]
        self.register(FormatInfo {
            name: "OpenEXR",
            extensions: &["exr"],
            can_read: |h| h.len() >= 4 && h[0] == 0x76 && h[1] == 0x2F && h[2] == 0x31 && h[3] == 0x01,
            read_path: |p| crate::exr::read(p),
            read_memory: |d| crate::exr::ExrReader::new().read_from_memory(d),
            // EXR multipart support - read specific layer/subimage
            read_subimage_path: Some(|p, subimage, miplevel| crate::exr::read_layer(p, subimage, miplevel)),
            num_subimages: Some(|p| crate::exr::num_layers(p)),
            num_miplevels: None, // TODO: implement miplevel counting for EXR
            write_path: Some(|p, i| crate::exr::write(p, i)),
            write_memory: Some(|i| crate::exr::ExrWriter::new().write_to_memory(i)),
            capabilities: &[
                FormatCapability::MultiImage,
                FormatCapability::MipMap,
                FormatCapability::Tiles,
                FormatCapability::DeepData,
                FormatCapability::IoProxy,
                FormatCapability::ArbitraryMetadata,
            ],
            read_deep_path: Some(|p| {
                let (samples, channels) = crate::exr_deep::read_deep_exr(p)?;
                crate::exr_deep::deep_samples_to_deepdata(&samples, &channels)
            }),
        });

        #[cfg(feature = "png")]
        self.register(FormatInfo {
            name: "PNG",
            extensions: &["png"],
            can_read: |h| {
                h.len() >= 8
                    && h[0] == 0x89
                    && h[1] == 0x50
                    && h[2] == 0x4E
                    && h[3] == 0x47
                    && h[4] == 0x0D
                    && h[5] == 0x0A
                    && h[6] == 0x1A
                    && h[7] == 0x0A
            },
            read_path: |p| crate::png::read(p),
            read_memory: |d| crate::png::PngReader::new().read_from_memory(d),
            read_subimage_path: None,
            num_subimages: None,
            num_miplevels: None,
            write_path: Some(|p, i| crate::png::write(p, i)),
            write_memory: Some(|i| crate::png::PngWriter::new().write_to_memory(i)),
            capabilities: &[FormatCapability::IoProxy],
            read_deep_path: None, // PNG doesn't support deep data
        });

        #[cfg(feature = "jpeg")]
        self.register(FormatInfo {
            name: "JPEG",
            extensions: &["jpg", "jpeg"],
            can_read: |h| h.len() >= 3 && h[0] == 0xFF && h[1] == 0xD8 && h[2] == 0xFF,
            read_path: |p| crate::jpeg::read(p),
            read_memory: |d| crate::jpeg::JpegReader::new().read_from_memory(d),
            read_subimage_path: None,
            num_subimages: None,
            num_miplevels: None,
            write_path: Some(|p, i| crate::jpeg::write(p, i)),
            write_memory: Some(|i| crate::jpeg::JpegWriter::new().write_to_memory(i)),
            capabilities: &[FormatCapability::IoProxy, FormatCapability::Exif],
            read_deep_path: None, // JPEG doesn't support deep data
        });

        #[cfg(feature = "tiff")]
        self.register(FormatInfo {
            name: "TIFF",
            extensions: &["tiff", "tif"],
            can_read: |h| {
                if h.len() < 4 {
                    return false;
                }
                let le = h[0] == b'I' && h[1] == b'I' && h[2] == 0x2A && h[3] == 0x00;
                let be = h[0] == b'M' && h[1] == b'M' && h[2] == 0x00 && h[3] == 0x2A;
                le || be
            },
            read_path: |p| crate::tiff::read(p),
            read_memory: |d| crate::tiff::TiffReader::new().read_from_memory(d),
            // TIFF supports pages/directories - TODO: implement real subimage support
            read_subimage_path: None,
            num_subimages: None,
            num_miplevels: None,
            write_path: Some(|p, i| crate::tiff::write(p, i)),
            write_memory: Some(|i| crate::tiff::TiffWriter::new().write_to_memory(i)),
            capabilities: &[
                FormatCapability::MultiImage,
                FormatCapability::Tiles,
                FormatCapability::IoProxy,
                FormatCapability::Exif,
            ],
            read_deep_path: None, // TIFF doesn't support deep data
        });

        #[cfg(feature = "hdr")]
        self.register(FormatInfo {
            name: "Radiance HDR",
            extensions: &["hdr", "pic"],
            can_read: |h| h.len() >= 2 && h[0] == b'#' && h[1] == b'?',
            read_path: |p| crate::hdr::read(p),
            read_memory: |d| crate::hdr::HdrReader::new().read_from_memory(d),
            read_subimage_path: None,
            num_subimages: None,
            num_miplevels: None,
            write_path: Some(|p, i| crate::hdr::write(p, i)),
            write_memory: Some(|i| crate::hdr::HdrWriter::new().write_to_memory(i)),
            capabilities: &[FormatCapability::IoProxy],
            read_deep_path: None, // HDR doesn't support deep data
        });
    }

    /// Registers a format in the registry.
    pub fn register(&mut self, info: FormatInfo) {
        let name = info.name;
        for ext in info.extensions {
            self.by_extension.insert(ext, name);
        }
        self.formats.insert(name, Arc::new(info));
    }

    /// Returns an iterator over registered format names.
    pub fn format_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.formats.keys().copied()
    }

    /// Returns format info by name.
    pub fn get(&self, name: &str) -> Option<&FormatInfo> {
        self.formats.get(name).map(|arc| arc.as_ref())
    }

    /// Returns format info by file extension.
    pub fn get_by_extension(&self, ext: &str) -> Option<&FormatInfo> {
        let ext_lower = ext.to_lowercase();
        self.by_extension
            .get(ext_lower.as_str())
            .and_then(|name| self.formats.get(name))
            .map(|arc| arc.as_ref())
    }

    /// Checks if an extension is supported.
    pub fn supports_extension(&self, ext: &str) -> bool {
        self.by_extension.contains_key(ext.to_lowercase().as_str())
    }

    /// Detects format from file header bytes.
    ///
    /// Returns the format name if detected, None otherwise.
    pub fn detect_format(&self, header: &[u8]) -> Option<&'static str> {
        for (name, info) in &self.formats {
            if (info.can_read)(header) {
                return Some(name);
            }
        }
        None
    }

    /// Reads an image from a file using auto-detection.
    ///
    /// First tries to detect format by magic bytes, falls back to extension.
    pub fn read(&self, path: &Path) -> IoResult<ImageData> {
        // Try magic bytes detection first
        let header = std::fs::read(path)?;
        if let Some(name) = self.detect_format(&header[..header.len().min(16)]) {
            if let Some(info) = self.formats.get(name) {
                return (info.read_memory)(&header);
            }
        }

        // Fall back to extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if let Some(info) = self.get_by_extension(ext) {
                return (info.read_memory)(&header);
            }
        }

        Err(crate::IoError::UnsupportedFormat(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_string(),
        ))
    }

    /// Reads a specific subimage/miplevel from a file.
    ///
    /// Falls back to regular read for subimage=0, miplevel=0 if format
    /// doesn't have subimage support.
    pub fn read_subimage(&self, path: &Path, subimage: usize, miplevel: usize) -> IoResult<ImageData> {
        // Detect format
        let header = std::fs::read(path)?;
        let format_name = self.detect_format(&header[..header.len().min(16)])
            .or_else(|| path.extension().and_then(|e| e.to_str()).and_then(|ext| self.by_extension.get(ext.to_lowercase().as_str()).copied()));
        
        if let Some(name) = format_name {
            if let Some(info) = self.formats.get(name) {
                // Try format-specific subimage read
                if let Some(read_sub) = info.read_subimage_path {
                    return read_sub(path, subimage, miplevel);
                }
                // Fall back to regular read for subimage=0, miplevel=0
                if subimage == 0 && miplevel == 0 {
                    return (info.read_memory)(&header);
                }
                return Err(crate::IoError::UnsupportedFeature(
                    format!("format '{}' doesn't support subimage {} miplevel {}", name, subimage, miplevel)
                ));
            }
        }
        
        Err(crate::IoError::UnsupportedFormat(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_string(),
        ))
    }

    /// Gets number of subimages in a file.
    pub fn num_subimages(&self, path: &Path) -> IoResult<usize> {
        let header = std::fs::read(path)?;
        let format_name = self.detect_format(&header[..header.len().min(16)])
            .or_else(|| path.extension().and_then(|e| e.to_str()).and_then(|ext| self.by_extension.get(ext.to_lowercase().as_str()).copied()));
        
        if let Some(name) = format_name {
            if let Some(info) = self.formats.get(name) {
                if let Some(num_sub) = info.num_subimages {
                    return num_sub(path);
                }
                return Ok(1); // Default: single subimage
            }
        }
        Ok(1)
    }

    /// Gets number of miplevels for a subimage.
    pub fn num_miplevels(&self, path: &Path, subimage: usize) -> IoResult<usize> {
        let header = std::fs::read(path)?;
        let format_name = self.detect_format(&header[..header.len().min(16)])
            .or_else(|| path.extension().and_then(|e| e.to_str()).and_then(|ext| self.by_extension.get(ext.to_lowercase().as_str()).copied()));
        
        if let Some(name) = format_name {
            if let Some(info) = self.formats.get(name) {
                if let Some(num_mip) = info.num_miplevels {
                    return num_mip(path, subimage);
                }
                return Ok(1); // Default: single miplevel
            }
        }
        Ok(1)
    }

    /// Writes an image to a file.
    pub fn write(&self, path: &Path, image: &ImageData) -> IoResult<()> {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if let Some(info) = self.get_by_extension(ext) {
                if let Some(write_fn) = info.write_path {
                    return write_fn(path, image);
                }
            }
        }

        Err(crate::IoError::UnsupportedFormat(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_string(),
        ))
    }

    /// Checks if a format supports a specific capability.
    pub fn supports(&self, format_name: &str, capability: FormatCapability) -> bool {
        self.formats.get(format_name)
            .map(|info| info.capabilities.contains(&capability))
            .unwrap_or(false)
    }

    /// Returns all capabilities for a format.
    pub fn capabilities(&self, format_name: &str) -> &[FormatCapability] {
        self.formats.get(format_name)
            .map(|info| info.capabilities)
            .unwrap_or(&[])
    }

    /// Checks if a format (by extension) supports a capability.
    pub fn supports_by_extension(&self, ext: &str, capability: FormatCapability) -> bool {
        self.get_by_extension(ext)
            .map(|info| info.capabilities.contains(&capability))
            .unwrap_or(false)
    }

    /// Reads deep data from a file.
    ///
    /// Returns error if format doesn't support deep data.
    pub fn read_deep(&self, path: &Path) -> IoResult<DeepData> {
        // Detect format
        let header = std::fs::read(path)?;
        let format_name = self.detect_format(&header[..header.len().min(16)])
            .or_else(|| path.extension().and_then(|e| e.to_str()).and_then(|ext| self.by_extension.get(ext.to_lowercase().as_str()).copied()));
        
        if let Some(name) = format_name {
            if let Some(info) = self.formats.get(name) {
                if let Some(read_deep) = info.read_deep_path {
                    return read_deep(path);
                }
                return Err(crate::IoError::UnsupportedFeature(
                    format!("format '{}' doesn't support deep data", name)
                ));
            }
        }
        
        Err(crate::IoError::UnsupportedFormat(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_string(),
        ))
    }
}

impl Default for FormatRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_registry() {
        let registry = FormatRegistry::global();

        // Check that built-in formats are registered
        let names: Vec<_> = registry.format_names().collect();
        assert!(!names.is_empty());

        #[cfg(feature = "png")]
        assert!(registry.supports_extension("png"));

        #[cfg(feature = "exr")]
        assert!(registry.supports_extension("exr"));
    }

    #[test]
    fn test_format_detection() {
        let registry = FormatRegistry::global();

        #[cfg(feature = "png")]
        {
            let png_header = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
            assert_eq!(registry.detect_format(png_header), Some("PNG"));
        }

        #[cfg(feature = "jpeg")]
        {
            let jpeg_header = &[0xFF, 0xD8, 0xFF, 0xE0];
            assert_eq!(registry.detect_format(jpeg_header), Some("JPEG"));
        }

        #[cfg(feature = "exr")]
        {
            let exr_header = &[0x76, 0x2F, 0x31, 0x01];
            assert_eq!(registry.detect_format(exr_header), Some("OpenEXR"));
        }
    }

    #[test]
    fn test_extension_lookup() {
        let registry = FormatRegistry::global();

        #[cfg(feature = "jpeg")]
        {
            // Both jpg and jpeg should work
            assert!(registry.get_by_extension("jpg").is_some());
            assert!(registry.get_by_extension("jpeg").is_some());
            assert_eq!(
                registry.get_by_extension("jpg").unwrap().name,
                registry.get_by_extension("jpeg").unwrap().name
            );
        }

        #[cfg(feature = "tiff")]
        {
            // Both tiff and tif should work
            assert!(registry.get_by_extension("tiff").is_some());
            assert!(registry.get_by_extension("tif").is_some());
        }
    }
}
