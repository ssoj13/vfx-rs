# EXR Implementation

OpenEXR reading and writing internals.

## Overview

vfx-rs uses the pure-Rust `exr` crate for OpenEXR support. This provides:

- Full OpenEXR 2.x compatibility
- Multi-layer/multi-part support
- All compression methods
- Deep image support (read-only)

## Reading EXR

### Basic Flow

```rust
// vfx_io::exr::read internally:
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    let file = std::fs::File::open(path)?;
    let reader = exr::prelude::read()
        .no_deep_data()
        .largest_resolution_level()
        .all_channels()
        .first_valid_layer()
        .all_attributes();
    
    let image = reader.from_buffered(BufReader::new(file))?;
    
    convert_to_image_data(image)
}
```

### Channel Handling

EXR stores channels in arbitrary order:

```rust
// Standard channel mapping
fn find_channel_index(channels: &[Channel], name: &str) -> Option<usize> {
    // Try exact match first
    if let Some(i) = channels.iter().position(|c| c.name == name) {
        return Some(i);
    }
    
    // Try case-insensitive
    let lower = name.to_lowercase();
    channels.iter().position(|c| c.name.to_lowercase() == lower)
}

// Reorder to RGBA
let r_idx = find_channel_index(&channels, "R")?;
let g_idx = find_channel_index(&channels, "G")?;
let b_idx = find_channel_index(&channels, "B")?;
let a_idx = find_channel_index(&channels, "A");
```

### Data Type Conversion

```rust
// EXR sample types → Rust
match sample_type {
    SampleType::F16 => {
        // half::f16 → f32
        let f16_data: Vec<f16> = read_channel_f16(...);
        let f32_data: Vec<f32> = f16_data.iter().map(|v| v.to_f32()).collect();
    }
    SampleType::F32 => {
        // Direct
        let f32_data: Vec<f32> = read_channel_f32(...);
    }
    SampleType::U32 => {
        // Keep as u32 (for ID passes)
        let u32_data: Vec<u32> = read_channel_u32(...);
    }
}
```

## Multi-Layer Support

### Layer Discovery

```rust
pub fn read_layers<P: AsRef<Path>>(path: P) -> IoResult<LayeredImage> {
    let meta = exr::meta::read_meta_from_path(path)?;
    
    let mut layers = Vec::new();
    for header in &meta.headers {
        let layer_name = header.name.clone().unwrap_or_default();
        let channels = parse_channels(&header.channels);
        
        layers.push(ImageLayer {
            name: layer_name,
            width: header.width(),
            height: header.height(),
            channels,
        });
    }
    
    LayeredImage { layers, metadata }
}
```

### Layer Naming Conventions

EXR uses dot-separated layer names:

```
beauty.R, beauty.G, beauty.B, beauty.A
diffuse.R, diffuse.G, diffuse.B
specular.R, specular.G, specular.B
depth.Z
```

Parsing:

```rust
fn parse_layer_channel(name: &str) -> (&str, &str) {
    if let Some(dot_pos) = name.rfind('.') {
        let layer = &name[..dot_pos];
        let channel = &name[dot_pos + 1..];
        (layer, channel)
    } else {
        ("", name)  // Default layer
    }
}
```

## Writing EXR

### Basic Flow

```rust
pub fn write<P: AsRef<Path>>(path: P, image: &ImageData) -> IoResult<()> {
    let layer = image.to_layer("main");
    
    let exr_channels = layer.channels.iter()
        .map(|ch| {
            let samples = match &ch.samples {
                ChannelSamples::F32(data) => {
                    AnyChannels::Flat(FlatSamples::F32(data.clone()))
                }
                ChannelSamples::U32(data) => {
                    AnyChannels::Flat(FlatSamples::U32(data.clone()))
                }
            };
            (ch.name.clone(), samples)
        })
        .collect();
    
    let exr_image = Image::from_layer(Layer::new(
        image.width,
        image.height,
        exr_channels,
    ));
    
    exr_image.write().to_path(path)?;
    Ok(())
}
```

### Compression

```rust
pub fn write_with_options<P: AsRef<Path>>(
    path: P,
    image: &ImageData,
    options: &ExrWriteOptions,
) -> IoResult<()> {
    let compression = match options.compression {
        Compression::None => exr::prelude::Compression::Uncompressed,
        Compression::Rle => exr::prelude::Compression::RLE,
        Compression::Zip => exr::prelude::Compression::ZIP16,
        Compression::Zips => exr::prelude::Compression::ZIP1,
        Compression::Piz => exr::prelude::Compression::PIZ,
        Compression::Pxr24 => exr::prelude::Compression::PXR24,
        Compression::B44 => exr::prelude::Compression::B44,
        Compression::B44a => exr::prelude::Compression::B44A,
        Compression::Dwaa => exr::prelude::Compression::DWAA(None),
        Compression::Dwab => exr::prelude::Compression::DWAB(None),
    };
    
    // Apply compression...
}
```

### Compression Recommendations

| Use Case | Compression | Notes |
|----------|-------------|-------|
| Archival | ZIP | Lossless, good ratio |
| Interchange | PIZ | Fast, good for random access |
| Real-time | None | Fastest read |
| Lossy OK | DWAA | Best ratio, slight loss |
| Film scans | B44A | Good for noisy images |

## Metadata Handling

### Standard Attributes

```rust
// Read standard EXR attributes
fn read_metadata(header: &Header) -> Metadata {
    let mut meta = Metadata::default();
    
    // Color space (ACES standard attribute)
    if let Some(cs) = header.get_text("acesImageContainerFlag") {
        meta.colorspace = Some("ACES".into());
    }
    
    // Chromaticities
    if let Some(chroma) = header.get_chromaticities() {
        // Store primaries info
    }
    
    // Custom attributes
    for (name, value) in &header.custom_attributes {
        meta.attrs.set(name, value.clone());
    }
    
    meta
}
```

### Custom Attributes

```rust
// Write custom attributes
fn set_custom_attributes(image: &mut ExrImage, attrs: &Attrs) {
    for (key, value) in attrs.iter() {
        match value {
            AttrValue::String(s) => {
                image.attributes.insert(key.clone(), Text(s.clone()));
            }
            AttrValue::Float(f) => {
                image.attributes.insert(key.clone(), Float(*f));
            }
            // ...
        }
    }
}
```

## Performance Considerations

### Parallel Decoding

The `exr` crate supports parallel decompression:

```rust
// Enable parallel reading
let reader = exr::prelude::read()
    .parallel()  // Uses rayon internally
    .from_file(path)?;
```

### Scanline vs. Tiled

```rust
// Tiled is better for partial reads
if header.is_tiled() {
    // Read specific tiles
    for tile in tiles_in_roi(roi) {
        let data = read_tile(tile)?;
    }
} else {
    // Must read full scanlines
    for scanline in scanlines_in_roi(roi) {
        let data = read_scanline(scanline)?;
    }
}
```

### Memory Mapping (Future)

For very large files:

```rust
// Potential future optimization
let mmap = memmap2::Mmap::map(&file)?;
let reader = exr::prelude::read().from_slice(&mmap[..])?;
```

## Error Handling

```rust
// Convert exr crate errors to vfx-io errors
impl From<exr::Error> for IoError {
    fn from(e: exr::Error) -> Self {
        match e {
            exr::Error::NotSupported(msg) => IoError::UnsupportedFormat(msg),
            exr::Error::Invalid(msg) => IoError::DecodeError(msg),
            exr::Error::Io(io_err) => IoError::Io(io_err),
        }
    }
}
```
