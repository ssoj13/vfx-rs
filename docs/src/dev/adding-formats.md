# Adding Image Formats

Guide for adding new image format support to `vfx-io`.

## Architecture Overview

Format support in `vfx-io` follows the OIIO plugin pattern:

```
vfx-io/src/
    traits.rs       <- FormatReader/FormatWriter traits
    registry.rs     <- FormatRegistry for dynamic dispatch
    tiff.rs         <- TIFF format (TiffReader, TiffWriter)
    exr.rs          <- EXR format (ExrReader, ExrWriter)
    dpx.rs          <- DPX format (DpxReader, DpxWriter)
    ...
```

Each format implements:
- `FormatReader<O>` trait for reading with options
- `FormatWriter<O>` trait for writing with options
- Convenience functions (`read`, `write`)
- Registration in `FormatRegistry`

### Core Traits

```rust
// In vfx-io/src/traits.rs
pub trait FormatReader<O = ()>: Default {
    /// Read image from file path.
    fn read(&self, path: impl AsRef<Path>) -> IoResult<ImageData>;
    
    /// Read image from memory buffer.
    fn read_from_memory(&self, data: &[u8]) -> IoResult<ImageData>;
    
    /// Check if format supports given capability.
    fn supports(&self, capability: FormatCapability) -> bool;
}

pub trait FormatWriter<O = ()>: Default {
    /// Write image to file path.
    fn write(&self, path: impl AsRef<Path>, image: &ImageData) -> IoResult<()>;
    
    /// Write image to memory buffer.
    fn write_to_memory(&self, image: &ImageData) -> IoResult<Vec<u8>>;
}
```

## Step-by-Step: Adding SGI Format

### 1. Add Dependency

In `vfx-io/Cargo.toml`:

```toml
[dependencies]
sgi = { version = "0.1", optional = true }

[features]
# Current defaults include multiple formats
default = ["exr", "png", "jpeg", "tiff", "dpx", "hdr"]
sgi = ["dep:sgi"]
```

### 2. Create Format Module

Create `vfx-io/src/sgi.rs`:

```rust
//! SGI/RGB image format support.
//!
//! The SGI format (also known as IRIS RGB) was developed by Silicon Graphics
//! for their workstations. It supports 8-bit and 16-bit images with RLE compression.

use crate::{ImageData, IoResult, IoError, FormatReader, FormatWriter, FormatCapability};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write, Seek};
use std::path::Path;

// ============================================================================
// Reader
// ============================================================================

/// SGI format reader options.
#[derive(Debug, Clone, Default)]
pub struct SgiReaderOptions {
    // Add format-specific options here
}

/// SGI format reader.
#[derive(Debug, Clone, Default)]
pub struct SgiReader {
    options: SgiReaderOptions,
}

impl SgiReader {
    /// Create reader with options.
    pub fn with_options(options: SgiReaderOptions) -> Self {
        Self { options }
    }
}

impl FormatReader<SgiReaderOptions> for SgiReader {
    fn read(&self, path: impl AsRef<Path>) -> IoResult<ImageData> {
        let file = File::open(path.as_ref())?;
        let mut reader = BufReader::new(file);
        read_sgi(&mut reader)
    }
    
    fn read_from_memory(&self, data: &[u8]) -> IoResult<ImageData> {
        let mut cursor = std::io::Cursor::new(data);
        read_sgi(&mut cursor)
    }
    
    fn supports(&self, cap: FormatCapability) -> bool {
        matches!(cap, FormatCapability::IoProxy)
    }
}

/// Read SGI from reader.
fn read_sgi<R: Read + Seek>(reader: &mut R) -> IoResult<ImageData> {
    // Read SGI header (512 bytes)
    let mut header = [0u8; 512];
    reader.read_exact(&mut header)?;
    
    // Check magic number (0x01DA)
    if header[0] != 0x01 || header[1] != 0xDA {
        return Err(IoError::InvalidFormat("Not an SGI file".into()));
    }
    
    // Parse header
    let storage = header[2];  // 0 = uncompressed, 1 = RLE
    let bpc = header[3] as usize;  // 1 or 2 bytes per channel
    let dimension = u16::from_be_bytes([header[4], header[5]]);
    let width = u16::from_be_bytes([header[6], header[7]]) as usize;
    let height = u16::from_be_bytes([header[8], header[9]]) as usize;
    let channels = u16::from_be_bytes([header[10], header[11]]) as usize;
    
    // Read pixel data
    let pixel_count = width * height * channels;
    let mut data = vec![0.0f32; pixel_count];
    
    if storage == 0 {
        // Uncompressed
        if bpc == 1 {
            let mut bytes = vec![0u8; pixel_count];
            reader.read_exact(&mut bytes)?;
            for (i, &b) in bytes.iter().enumerate() {
                data[i] = b as f32 / 255.0;
            }
        } else {
            let mut shorts = vec![0u8; pixel_count * 2];
            reader.read_exact(&mut shorts)?;
            for i in 0..pixel_count {
                let v = u16::from_be_bytes([shorts[i*2], shorts[i*2+1]]);
                data[i] = v as f32 / 65535.0;
            }
        }
    } else {
        // RLE compressed - implement RLE decoding
        // ...
    }
    
    // SGI stores channels in planes, need to interleave
    let mut interleaved = vec![0.0f32; pixel_count];
    for y in 0..height {
        for x in 0..width {
            for c in 0..channels {
                let src_idx = c * width * height + y * width + x;
                let dst_idx = (y * width + x) * channels + c;
                interleaved[dst_idx] = data[src_idx];
            }
        }
    }
    
    Ok(ImageData::from_f32(width, height, channels, interleaved))
}

// ============================================================================
// Writer
// ============================================================================

/// SGI format writer options.
#[derive(Debug, Clone)]
pub struct SgiWriterOptions {
    /// Use RLE compression.
    pub compress: bool,
    /// Output bit depth (1 or 2 bytes per channel).
    pub bytes_per_channel: u8,
}

impl Default for SgiWriterOptions {
    fn default() -> Self {
        Self {
            compress: true,
            bytes_per_channel: 1,
        }
    }
}

/// SGI format writer.
#[derive(Debug, Clone, Default)]
pub struct SgiWriter {
    options: SgiWriterOptions,
}

impl SgiWriter {
    /// Create writer with options.
    pub fn with_options(options: SgiWriterOptions) -> Self {
        Self { options }
    }
}

impl FormatWriter<SgiWriterOptions> for SgiWriter {
    fn write(&self, path: impl AsRef<Path>, image: &ImageData) -> IoResult<()> {
        let file = File::create(path.as_ref())?;
        let mut writer = BufWriter::new(file);
        write_sgi(&mut writer, image, &self.options)
    }
    
    fn write_to_memory(&self, image: &ImageData) -> IoResult<Vec<u8>> {
        let mut buffer = Vec::new();
        write_sgi(&mut std::io::Cursor::new(&mut buffer), image, &self.options)?;
        Ok(buffer)
    }
}

fn write_sgi<W: Write + Seek>(
    writer: &mut W,
    image: &ImageData,
    options: &SgiWriterOptions,
) -> IoResult<()> {
    // Write SGI header
    let mut header = [0u8; 512];
    header[0] = 0x01;
    header[1] = 0xDA;
    header[2] = if options.compress { 1 } else { 0 };
    header[3] = options.bytes_per_channel;
    // ... fill rest of header
    
    writer.write_all(&header)?;
    
    // Write pixel data
    let pixels = image.to_f32();
    // ... convert and write
    
    Ok(())
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Read SGI image from path.
pub fn read(path: impl AsRef<Path>) -> IoResult<ImageData> {
    SgiReader::default().read(path)
}

/// Write SGI image to path.
pub fn write(path: impl AsRef<Path>, image: &ImageData) -> IoResult<()> {
    SgiWriter::default().write(path, image)
}

/// Check if header matches SGI format.
pub fn can_read(header: &[u8]) -> bool {
    header.len() >= 2 && header[0] == 0x01 && header[1] == 0xDA
}

/// Supported file extensions.
pub const EXTENSIONS: &[&str] = &["sgi", "rgb", "rgba", "bw"];

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.sgi");
        
        let original = ImageData::from_f32(
            64, 64, 3,
            (0..64*64*3).map(|i| (i % 256) as f32 / 255.0).collect()
        );
        
        write(&path, &original).unwrap();
        let loaded = read(&path).unwrap();
        
        assert_eq!(original.width, loaded.width);
        assert_eq!(original.height, loaded.height);
        assert_eq!(original.channels, loaded.channels);
    }
    
    #[test]
    fn test_can_read() {
        // Valid SGI magic
        assert!(can_read(&[0x01, 0xDA, 0x00, 0x01]));
        // Invalid
        assert!(!can_read(&[0x89, 0x50, 0x4E, 0x47]));  // PNG
    }
}
```

### 3. Register in lib.rs

In `vfx-io/src/lib.rs`:

```rust
#[cfg(feature = "sgi")]
pub mod sgi;
```

### 4. Add to FormatRegistry

In `vfx-io/src/registry.rs`, add to the `init_builtin_formats()` function:

```rust
fn init_builtin_formats() -> FormatRegistry {
    let mut reg = FormatRegistry::new();
    
    // ... existing formats ...
    
    #[cfg(feature = "sgi")]
    reg.register(FormatInfo {
        name: "SGI",
        extensions: crate::sgi::EXTENSIONS,
        can_read: crate::sgi::can_read,
        read_path: |p| crate::sgi::read(p),
        read_memory: |d| crate::sgi::SgiReader::default().read_from_memory(d),
        read_subimage_path: None,
        num_subimages: None,
        num_miplevels: None,
        write_path: Some(|p, i| crate::sgi::write(p, i)),
        write_memory: Some(|i| crate::sgi::SgiWriter::default().write_to_memory(i)),
        capabilities: &[FormatCapability::IoProxy],
        read_deep_path: None,
    });
    
    reg
}
```

### 5. Update Documentation

Add to `docs/src/crates/io.md`:

```markdown
### SGI/RGB

Enable with `sgi` feature:
- Read: 8/16-bit, RLE compressed
- Write: 8-bit with optional RLE
- Extensions: .sgi, .rgb, .rgba, .bw
```

## Format Checklist

When adding a new format:

- [ ] Create `vfx-io/src/format.rs` with Reader/Writer structs
- [ ] Implement `FormatReader<Options>` trait
- [ ] Implement `FormatWriter<Options>` trait  
- [ ] Add `can_read()` function for magic byte detection
- [ ] Define `EXTENSIONS` constant
- [ ] Add convenience `read()`/`write()` functions
- [ ] Register in `lib.rs` with feature flag
- [ ] Register in `registry.rs` `init_builtin_formats()`
- [ ] Add feature flag to `Cargo.toml`
- [ ] Add unit tests (roundtrip, can_read)
- [ ] Add test images to `test/images/`
- [ ] Update documentation

## Multi-Subimage Formats

For formats supporting multiple images (EXR, TIFF):

```rust
impl SgiReader {
    /// Read specific subimage.
    pub fn read_subimage(&self, path: impl AsRef<Path>, subimage: usize) -> IoResult<ImageData> {
        // ...
    }
    
    /// Get number of subimages.
    pub fn num_subimages(&self, path: impl AsRef<Path>) -> IoResult<usize> {
        // ...
    }
}
```

Register the additional functions:

```rust
FormatInfo {
    read_subimage_path: Some(|p, s, m| reader.read_subimage(p, s)),
    num_subimages: Some(|p| reader.num_subimages(p)),
    num_miplevels: Some(|p, s| reader.num_miplevels(p, s)),
    // ...
}
```

## Deep Data Formats

For formats supporting deep pixels (EXR):

```rust
impl ExrReader {
    /// Read deep data from file.
    pub fn read_deep(&self, path: impl AsRef<Path>) -> IoResult<DeepData> {
        // ...
    }
}
```

Register:

```rust
FormatInfo {
    read_deep_path: Some(|p| crate::exr::ExrReader::default().read_deep(p)),
    capabilities: &[FormatCapability::DeepData, ...],
    // ...
}
```

## Performance Tips

- Use `BufReader`/`BufWriter` for file I/O
- Implement streaming/tiled reading for large images
- Use rayon for parallel row processing
- Profile with realistic test data
