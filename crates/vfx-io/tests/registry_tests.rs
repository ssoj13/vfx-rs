//! Integration tests for FormatRegistry.

use std::path::PathBuf;
use vfx_io::registry::FormatRegistry;
use vfx_io::{ImageData, PixelData, PixelFormat, Metadata};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Create a simple test image.
fn test_image(width: u32, height: u32) -> ImageData {
    let channels = 3u32;
    let size = (width * height * channels) as usize;
    let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
    
    ImageData {
        width,
        height,
        channels,
        format: PixelFormat::U8,
        data: PixelData::U8(data),
        metadata: Metadata::default(),
    }
}

#[test]
fn registry_global_has_all_formats() {
    let registry = FormatRegistry::global();
    let names: Vec<_> = registry.format_names().collect();
    
    // Should have all built-in formats
    #[cfg(feature = "png")]
    assert!(names.contains(&"PNG"), "PNG not found in registry");
    
    #[cfg(feature = "jpeg")]
    assert!(names.contains(&"JPEG"), "JPEG not found in registry");
    
    #[cfg(feature = "tiff")]
    assert!(names.contains(&"TIFF"), "TIFF not found in registry");
    
    #[cfg(feature = "exr")]
    assert!(names.contains(&"OpenEXR"), "OpenEXR not found in registry");
    
    #[cfg(feature = "hdr")]
    assert!(names.contains(&"Radiance HDR"), "Radiance HDR not found in registry");
    
    #[cfg(feature = "dpx")]
    assert!(names.contains(&"DPX"), "DPX not found in registry");
}

#[test]
fn registry_extension_case_insensitive() {
    let registry = FormatRegistry::global();
    
    #[cfg(feature = "png")]
    {
        assert!(registry.supports_extension("png"));
        assert!(registry.supports_extension("PNG"));
        assert!(registry.supports_extension("Png"));
    }
    
    #[cfg(feature = "jpeg")]
    {
        assert!(registry.supports_extension("jpg"));
        assert!(registry.supports_extension("JPG"));
        assert!(registry.supports_extension("jpeg"));
        assert!(registry.supports_extension("JPEG"));
    }
}

#[test]
fn registry_detect_png_magic() {
    let registry = FormatRegistry::global();
    
    #[cfg(feature = "png")]
    {
        // PNG magic: 89 50 4E 47 0D 0A 1A 0A
        let png_magic = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(registry.detect_format(&png_magic), Some("PNG"));
    }
}

#[test]
fn registry_detect_jpeg_magic() {
    let registry = FormatRegistry::global();
    
    #[cfg(feature = "jpeg")]
    {
        // JPEG magic: FF D8 FF
        let jpeg_magic = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        assert_eq!(registry.detect_format(&jpeg_magic), Some("JPEG"));
    }
}

#[test]
fn registry_detect_exr_magic() {
    let registry = FormatRegistry::global();
    
    #[cfg(feature = "exr")]
    {
        // EXR magic: 76 2F 31 01
        let exr_magic = [0x76, 0x2F, 0x31, 0x01];
        assert_eq!(registry.detect_format(&exr_magic), Some("OpenEXR"));
    }
}

#[test]
fn registry_detect_tiff_le_magic() {
    let registry = FormatRegistry::global();
    
    #[cfg(feature = "tiff")]
    {
        // TIFF LE magic: II 2A 00
        let tiff_le = [b'I', b'I', 0x2A, 0x00];
        assert_eq!(registry.detect_format(&tiff_le), Some("TIFF"));
    }
}

#[test]
fn registry_detect_tiff_be_magic() {
    let registry = FormatRegistry::global();
    
    #[cfg(feature = "tiff")]
    {
        // TIFF BE magic: MM 00 2A
        let tiff_be = [b'M', b'M', 0x00, 0x2A];
        assert_eq!(registry.detect_format(&tiff_be), Some("TIFF"));
    }
}

#[test]
fn registry_detect_hdr_magic() {
    let registry = FormatRegistry::global();
    
    #[cfg(feature = "hdr")]
    {
        // HDR magic: #?
        let hdr_magic = b"#?RADIANCE\n";
        assert_eq!(registry.detect_format(hdr_magic), Some("Radiance HDR"));
    }
}

#[test]
fn registry_detect_dpx_be_magic() {
    let registry = FormatRegistry::global();
    
    #[cfg(feature = "dpx")]
    {
        // DPX BE magic: SDPX
        let dpx_be = [0x53, 0x44, 0x50, 0x58];
        assert_eq!(registry.detect_format(&dpx_be), Some("DPX"));
    }
}

#[test]
fn registry_detect_dpx_le_magic() {
    let registry = FormatRegistry::global();
    
    #[cfg(feature = "dpx")]
    {
        // DPX LE magic: XPDS
        let dpx_le = [0x58, 0x50, 0x44, 0x53];
        assert_eq!(registry.detect_format(&dpx_le), Some("DPX"));
    }
}

#[test]
fn registry_unknown_magic() {
    let registry = FormatRegistry::global();
    
    // Random garbage should not match any format
    let unknown = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];
    assert_eq!(registry.detect_format(&unknown), None);
}

#[test]
fn registry_read_png_from_path() {
    let registry = FormatRegistry::global();
    
    #[cfg(feature = "png")]
    {
        let path = fixture_path("sample.png");
        let image = registry.read(&path).expect("read PNG via registry");
        assert_eq!(image.width, 640);
        assert_eq!(image.height, 426);
    }
}

#[test]
fn registry_read_jpeg_from_path() {
    let registry = FormatRegistry::global();
    
    #[cfg(feature = "jpeg")]
    {
        let path = fixture_path("owl.jpg");
        let image = registry.read(&path).expect("read JPEG via registry");
        assert_eq!(image.width, 1446);
        assert_eq!(image.height, 1920);
    }
}

#[test]
fn registry_read_tiff_from_path() {
    let registry = FormatRegistry::global();
    
    #[cfg(feature = "tiff")]
    {
        let path = fixture_path("sample.tiff");
        let image = registry.read(&path).expect("read TIFF via registry");
        assert_eq!(image.width, 640);
        assert_eq!(image.height, 426);
    }
}

#[test]
fn registry_read_exr_from_path() {
    let registry = FormatRegistry::global();
    
    #[cfg(feature = "exr")]
    {
        let path = fixture_path("test.exr");
        let image = registry.read(&path).expect("read EXR via registry");
        assert_eq!(image.width, 911);
        assert_eq!(image.height, 876);
    }
}

#[test]
fn registry_read_hdr_from_path() {
    let registry = FormatRegistry::global();
    
    #[cfg(feature = "hdr")]
    {
        let path = fixture_path("test.hdr");
        let image = registry.read(&path).expect("read HDR via registry");
        assert_eq!(image.width, 1024);
        assert_eq!(image.height, 512);
    }
}

#[test]
fn registry_format_info_has_extensions() {
    let registry = FormatRegistry::global();
    
    #[cfg(feature = "png")]
    {
        let info = registry.get("PNG").expect("PNG format info");
        assert!(info.extensions.contains(&"png"));
    }
    
    #[cfg(feature = "jpeg")]
    {
        let info = registry.get("JPEG").expect("JPEG format info");
        assert!(info.extensions.contains(&"jpg"));
        assert!(info.extensions.contains(&"jpeg"));
    }
    
    #[cfg(feature = "tiff")]
    {
        let info = registry.get("TIFF").expect("TIFF format info");
        assert!(info.extensions.contains(&"tiff"));
        assert!(info.extensions.contains(&"tif"));
    }
}

#[test]
fn registry_write_memory_roundtrip_png() {
    let registry = FormatRegistry::global();
    
    #[cfg(feature = "png")]
    {
        let original = test_image(64, 48);
        
        // Get PNG format info
        let info = registry.get("PNG").expect("PNG format");
        
        // Write to memory
        let write_fn = info.write_memory.expect("PNG write_memory");
        let bytes = write_fn(&original).expect("write PNG to memory");
        
        // Verify PNG magic
        assert!(bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]));
        
        // Read back
        let decoded = (info.read_memory)(&bytes).expect("read PNG from memory");
        assert_eq!(decoded.width, original.width);
        assert_eq!(decoded.height, original.height);
    }
}

#[test]
fn registry_write_memory_roundtrip_jpeg() {
    let registry = FormatRegistry::global();
    
    #[cfg(feature = "jpeg")]
    {
        let original = test_image(64, 48);
        
        let info = registry.get("JPEG").expect("JPEG format");
        let write_fn = info.write_memory.expect("JPEG write_memory");
        let bytes = write_fn(&original).expect("write JPEG to memory");
        
        // Verify JPEG magic
        assert!(bytes.starts_with(&[0xFF, 0xD8, 0xFF]));
        
        let decoded = (info.read_memory)(&bytes).expect("read JPEG from memory");
        assert_eq!(decoded.width, original.width);
        assert_eq!(decoded.height, original.height);
    }
}

#[test]
fn registry_write_memory_roundtrip_tiff() {
    let registry = FormatRegistry::global();
    
    #[cfg(feature = "tiff")]
    {
        let original = test_image(64, 48);
        
        let info = registry.get("TIFF").expect("TIFF format");
        let write_fn = info.write_memory.expect("TIFF write_memory");
        let bytes = write_fn(&original).expect("write TIFF to memory");
        
        // Verify TIFF magic (LE or BE)
        let is_tiff = (bytes[0] == b'I' && bytes[1] == b'I') 
                   || (bytes[0] == b'M' && bytes[1] == b'M');
        assert!(is_tiff, "Invalid TIFF magic");
        
        let decoded = (info.read_memory)(&bytes).expect("read TIFF from memory");
        assert_eq!(decoded.width, original.width);
        assert_eq!(decoded.height, original.height);
    }
}

#[test]
fn registry_empty_header_no_panic() {
    let registry = FormatRegistry::global();
    
    // Empty header should not panic
    let empty: &[u8] = &[];
    assert_eq!(registry.detect_format(empty), None);
    
    // Single byte should not panic
    let one = &[0x89];
    assert_eq!(registry.detect_format(one), None);
}

#[test]
fn registry_thread_safe() {
    use std::thread;
    
    // Spawn threads that all access the global registry
    let handles: Vec<_> = (0..4)
        .map(|_| {
            thread::spawn(|| {
                let registry = FormatRegistry::global();
                let names: Vec<_> = registry.format_names().collect();
                assert!(!names.is_empty());
                
                #[cfg(feature = "png")]
                assert!(registry.supports_extension("png"));
            })
        })
        .collect();
    
    for handle in handles {
        handle.join().expect("thread panicked");
    }
}
