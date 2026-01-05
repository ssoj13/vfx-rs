//! Format detection utilities.
//!
//! Detects image formats from file extensions and magic bytes.

use crate::IoResult;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Supported image formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// OpenEXR format.
    Exr,
    /// PNG format.
    Png,
    /// JPEG format.
    Jpeg,
    /// TIFF format.
    Tiff,
    /// DPX format.
    Dpx,
    /// Radiance HDR format.
    Hdr,
    /// HEIF/HEIC format.
    Heif,
    /// WebP format.
    WebP,
    /// AVIF format.
    Avif,
    /// JPEG2000 format.
    Jp2,
    /// Unknown/unsupported format.
    Unknown,
}

impl Format {
    /// Detects format from file path (extension + magic bytes).
    ///
    /// First checks magic bytes, falls back to extension.
    pub fn detect<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        let path = path.as_ref();
        
        // Try magic bytes first
        if let Ok(format) = Self::from_magic_bytes(path) {
            if format != Format::Unknown {
                return Ok(format);
            }
        }
        
        // Fall back to extension
        Ok(Self::from_extension(path))
    }
    
    /// Detects format from file extension only.
    pub fn from_extension<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());
        
        match ext.as_deref() {
            Some("exr") => Format::Exr,
            Some("png") => Format::Png,
            Some("jpg") | Some("jpeg") => Format::Jpeg,
            Some("tif") | Some("tiff") => Format::Tiff,
            Some("dpx") => Format::Dpx,
            Some("hdr") | Some("pic") | Some("rgbe") => Format::Hdr,
            Some("heif") | Some("heic") | Some("hif") => Format::Heif,
            Some("webp") => Format::WebP,
            Some("avif") => Format::Avif,
            Some("jp2") | Some("j2k") | Some("j2c") | Some("jpx") => Format::Jp2,
            _ => Format::Unknown,
        }
    }
    
    /// Detects format from file magic bytes.
    pub fn from_magic_bytes<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        let mut file = File::open(path)?;
        let mut header = [0u8; 8];
        
        let bytes_read = file.read(&mut header)?;
        if bytes_read < 4 {
            return Ok(Format::Unknown);
        }
        
        Ok(Self::from_bytes(&header[..bytes_read]))
    }
    
    /// Detects format from raw bytes (magic number check).
    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.len() < 4 {
            return Format::Unknown;
        }
        
        // EXR: 0x76 0x2f 0x31 0x01
        if bytes.len() >= 4 && bytes[0..4] == [0x76, 0x2f, 0x31, 0x01] {
            return Format::Exr;
        }
        
        // PNG: 0x89 0x50 0x4E 0x47 0x0D 0x0A 0x1A 0x0A
        if bytes.len() >= 8 && bytes[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
            return Format::Png;
        }
        
        // JPEG: 0xFF 0xD8 0xFF
        if bytes.len() >= 3 && bytes[0..3] == [0xFF, 0xD8, 0xFF] {
            return Format::Jpeg;
        }
        
        // TIFF: II (little-endian) or MM (big-endian)
        if bytes.len() >= 4 {
            // Little-endian TIFF
            if bytes[0..4] == [0x49, 0x49, 0x2A, 0x00] {
                return Format::Tiff;
            }
            // Big-endian TIFF
            if bytes[0..4] == [0x4D, 0x4D, 0x00, 0x2A] {
                return Format::Tiff;
            }
            // DPX big-endian: SDPX
            if bytes[0..4] == [0x53, 0x44, 0x50, 0x58] {
                return Format::Dpx;
            }
            // DPX little-endian: XPDS
            if bytes[0..4] == [0x58, 0x50, 0x44, 0x53] {
                return Format::Dpx;
            }
        }

        // HDR: "#?"
        if bytes.len() >= 2 && bytes[0..2] == [b'#', b'?'] {
            return Format::Hdr;
        }

        // HEIF/HEIC: ftyp at offset 4, with heic/heix/mif1/msf1 brand
        if bytes.len() >= 12 && bytes[4..8] == [b'f', b't', b'y', b'p'] {
            let brand = &bytes[8..12];
            if brand == b"heic" || brand == b"heix" || brand == b"mif1" || brand == b"msf1" || brand == b"hevc" {
                return Format::Heif;
            }
            // AVIF: ftyp with avif brand
            if brand == b"avif" || brand == b"avis" || brand == b"av01" {
                return Format::Avif;
            }
        }
        
        // WebP: RIFF....WEBP
        if bytes.len() >= 12 && bytes[0..4] == [b'R', b'I', b'F', b'F'] && bytes[8..12] == [b'W', b'E', b'B', b'P'] {
            return Format::WebP;
        }
        
        // JPEG2000: JP2 container or raw codestream
        if bytes.len() >= 12 {
            // JP2 container: 0x0000000C 6A502020
            if bytes[0..12] == [0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20, 0x0D, 0x0A, 0x87, 0x0A] {
                return Format::Jp2;
            }
            // Raw J2K codestream: FF 4F FF 51
            if bytes[0..4] == [0xFF, 0x4F, 0xFF, 0x51] {
                return Format::Jp2;
            }
        }

        Format::Unknown
    }
    
    /// Returns the typical file extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            Format::Exr => "exr",
            Format::Png => "png",
            Format::Jpeg => "jpg",
            Format::Tiff => "tif",
            Format::Dpx => "dpx",
            Format::Hdr => "hdr",
            Format::Heif => "heif",
            Format::WebP => "webp",
            Format::Avif => "avif",
            Format::Jp2 => "jp2",
            Format::Unknown => "",
        }
    }
    
    /// Returns the MIME type for this format.
    pub fn mime_type(&self) -> &'static str {
        match self {
            Format::Exr => "image/x-exr",
            Format::Png => "image/png",
            Format::Jpeg => "image/jpeg",
            Format::Tiff => "image/tiff",
            Format::Dpx => "image/x-dpx",
            Format::Hdr => "image/vnd.radiance",
            Format::Heif => "image/heif",
            Format::WebP => "image/webp",
            Format::Avif => "image/avif",
            Format::Jp2 => "image/jp2",
            Format::Unknown => "application/octet-stream",
        }
    }
    
    /// Returns true if this format supports HDR/float data.
    pub fn supports_hdr(&self) -> bool {
        matches!(self, Format::Exr | Format::Tiff | Format::Hdr | Format::Heif | Format::Avif)
    }
    
    /// Returns true if this format supports alpha channel.
    pub fn supports_alpha(&self) -> bool {
        matches!(self, Format::Exr | Format::Png | Format::Tiff | Format::Heif | Format::WebP | Format::Avif | Format::Jp2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_detection() {
        assert_eq!(Format::from_extension("test.exr"), Format::Exr);
        assert_eq!(Format::from_extension("test.EXR"), Format::Exr);
        assert_eq!(Format::from_extension("test.png"), Format::Png);
        assert_eq!(Format::from_extension("test.jpg"), Format::Jpeg);
        assert_eq!(Format::from_extension("test.jpeg"), Format::Jpeg);
        assert_eq!(Format::from_extension("test.tif"), Format::Tiff);
        assert_eq!(Format::from_extension("test.tiff"), Format::Tiff);
        assert_eq!(Format::from_extension("test.dpx"), Format::Dpx);
        assert_eq!(Format::from_extension("test.hdr"), Format::Hdr);
        assert_eq!(Format::from_extension("test.pic"), Format::Hdr);
        assert_eq!(Format::from_extension("test.unknown"), Format::Unknown);
    }

    #[test]
    fn test_magic_bytes() {
        // EXR magic
        let exr = [0x76, 0x2f, 0x31, 0x01, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(Format::from_bytes(&exr), Format::Exr);
        
        // PNG magic
        let png = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(Format::from_bytes(&png), Format::Png);
        
        // JPEG magic
        let jpeg = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        assert_eq!(Format::from_bytes(&jpeg), Format::Jpeg);
        
        // TIFF little-endian
        let tiff_le = [0x49, 0x49, 0x2A, 0x00, 0x08, 0x00, 0x00, 0x00];
        assert_eq!(Format::from_bytes(&tiff_le), Format::Tiff);
        
        // TIFF big-endian
        let tiff_be = [0x4D, 0x4D, 0x00, 0x2A, 0x00, 0x00, 0x00, 0x08];
        assert_eq!(Format::from_bytes(&tiff_be), Format::Tiff);
        
        // DPX big-endian
        let dpx_be = [0x53, 0x44, 0x50, 0x58, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(Format::from_bytes(&dpx_be), Format::Dpx);

        // HDR magic
        let hdr = [b'#', b'?', b'R', b'A', b'D', b'I', b'A', b'N'];
        assert_eq!(Format::from_bytes(&hdr), Format::Hdr);
        
        // Unknown
        let unknown = [0x00, 0x00, 0x00, 0x00];
        assert_eq!(Format::from_bytes(&unknown), Format::Unknown);
    }

    #[test]
    fn test_format_properties() {
        assert_eq!(Format::Exr.extension(), "exr");
        assert_eq!(Format::Png.mime_type(), "image/png");
    }
}
