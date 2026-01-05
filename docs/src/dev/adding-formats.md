# Adding Image Formats

This guide covers adding new image format support to `vfx-io`.

## Architecture Overview

Format support in `vfx-io` uses a trait-based design:

```rust
// Core traits in vfx-io/src/traits.rs
pub trait ImageReader {
    fn read(path: &Path) -> Result<ImageData>;
    fn supports_extension(ext: &str) -> bool;
}

pub trait ImageWriter {
    fn write(path: &Path, data: &ImageData) -> Result<()>;
    fn supports_extension(ext: &str) -> bool;
}
```

## Step-by-Step: Adding TIFF Support

### 1. Add Dependency

In `vfx-io/Cargo.toml`:

```toml
[dependencies]
tiff = { version = "0.9", optional = true }

[features]
default = ["exr", "png"]
tiff = ["dep:tiff"]
```

### 2. Create Format Module

Create `vfx-io/src/formats/tiff.rs`:

```rust
//! TIFF image format support.

use crate::{ImageData, Result};
use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use anyhow::bail;

/// Read TIFF image.
pub fn read(path: &Path) -> Result<ImageData> {
    let file = File::open(path)?;
    let mut decoder = tiff::decoder::Decoder::new(BufReader::new(file))?;
    
    let (width, height) = decoder.dimensions()?;
    let color_type = decoder.colortype()?;
    
    let channels = match color_type {
        tiff::ColorType::Gray(_) => 1,
        tiff::ColorType::RGB(_) => 3,
        tiff::ColorType::RGBA(_) => 4,
        _ => bail!("Unsupported TIFF color type: {:?}", color_type),
    };
    
    let image = decoder.read_image()?;
    
    let data = match image {
        tiff::decoder::DecodingResult::U8(bytes) => {
            bytes.iter().map(|&b| b as f32 / 255.0).collect()
        }
        tiff::decoder::DecodingResult::U16(shorts) => {
            shorts.iter().map(|&s| s as f32 / 65535.0).collect()
        }
        tiff::decoder::DecodingResult::F32(floats) => floats,
        _ => bail!("Unsupported TIFF bit depth"),
    };
    
    Ok(ImageData::from_f32(width, height, channels, data))
}

/// Write TIFF image.
pub fn write(path: &Path, data: &ImageData) -> Result<()> {
    use tiff::encoder::{TiffEncoder, colortype};
    
    let file = File::create(path)?;
    let mut encoder = TiffEncoder::new(file)?;
    
    let pixels = data.to_f32();
    
    // Convert to 16-bit for TIFF output
    let pixels_u16: Vec<u16> = pixels
        .iter()
        .map(|&v| (v.clamp(0.0, 1.0) * 65535.0) as u16)
        .collect();
    
    match data.channels {
        1 => encoder.write_image::<colortype::Gray16>(
            data.width, data.height, &pixels_u16
        )?,
        3 => encoder.write_image::<colortype::RGB16>(
            data.width, data.height, &pixels_u16
        )?,
        4 => encoder.write_image::<colortype::RGBA16>(
            data.width, data.height, &pixels_u16
        )?,
        _ => bail!("TIFF supports 1, 3, or 4 channels"),
    }
    
    Ok(())
}

/// Check if extension is TIFF.
pub fn supports(ext: &str) -> bool {
    matches!(ext.to_lowercase().as_str(), "tif" | "tiff")
}
```

### 3. Register in Format Dispatch

In `vfx-io/src/formats/mod.rs`:

```rust
#[cfg(feature = "tiff")]
pub mod tiff;

pub fn read_image(path: &Path) -> Result<ImageData> {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    
    #[cfg(feature = "tiff")]
    if tiff::supports(ext) {
        return tiff::read(path);
    }
    
    #[cfg(feature = "exr")]
    if exr::supports(ext) {
        return exr::read(path);
    }
    
    // ... other formats
    
    bail!("Unsupported format: {}", ext)
}
```

### 4. Add Tests

In `vfx-io/src/formats/tiff.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_tiff_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.tiff");
        
        // Create test image
        let original = ImageData::from_f32(
            64, 64, 3,
            (0..64*64*3).map(|i| (i % 256) as f32 / 255.0).collect()
        );
        
        write(&path, &original).unwrap();
        let loaded = read(&path).unwrap();
        
        assert_eq!(original.width, loaded.width);
        assert_eq!(original.height, loaded.height);
        assert_eq!(original.channels, loaded.channels);
        
        // Check pixels (allowing for 16-bit quantization)
        let orig_data = original.to_f32();
        let load_data = loaded.to_f32();
        for (a, b) in orig_data.iter().zip(load_data.iter()) {
            assert!((a - b).abs() < 0.001);
        }
    }
}
```

### 5. Update Documentation

Add to `docs/src/crates/io.md`:

```markdown
### TIFF Support

Enable with `tiff` feature:
- Read: 8/16/32-bit grayscale, RGB, RGBA
- Write: 16-bit output
```

### 6. Add CLI Support

If format needs special handling in CLI, update `vfx-cli/src/commands/convert.rs`:

```rust
// Usually automatic via vfx-io, but if special options needed:
if output_ext == "tiff" {
    // TIFF-specific options
}
```

## Format Checklist

When adding a new format:

- [ ] Add dependency in `Cargo.toml` (optional)
- [ ] Create feature flag
- [ ] Implement `read()` function
- [ ] Implement `write()` function
- [ ] Implement `supports()` function
- [ ] Register in dispatch
- [ ] Add unit tests
- [ ] Add roundtrip test
- [ ] Update documentation
- [ ] Add test images to `test/images/`

## Handling Metadata

For formats with metadata:

```rust
pub struct TiffMetadata {
    pub compression: Compression,
    pub photometric: Photometric,
    pub resolution: Option<(f32, f32)>,
    // etc.
}

pub fn read_with_metadata(path: &Path) -> Result<(ImageData, TiffMetadata)> {
    // ...
}
```

## Multi-Layer Formats

For formats supporting multiple images (like TIFF pages):

```rust
pub fn read_layer(path: &Path, layer: usize) -> Result<ImageData> {
    let file = File::open(path)?;
    let mut decoder = Decoder::new(BufReader::new(file))?;
    
    // Seek to requested page
    for _ in 0..layer {
        decoder.next_image()?;
    }
    
    // Read that page
    // ...
}

pub fn list_layers(path: &Path) -> Result<Vec<String>> {
    // Return layer names/indices
}
```

## Performance Considerations

- Use streaming/tiled reading for large images
- Implement parallel decoding where possible
- Consider memory-mapped files for huge images
- Profile with realistic test data

```rust
// Example: tiled reading
pub fn read_tiled(path: &Path, tile_callback: impl FnMut(Tile)) -> Result<()> {
    // Read tiles without loading entire image
}
```
