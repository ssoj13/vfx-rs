# vfx-io

Image I/O for VFX pipelines.

## Purpose

Read and write image files in formats used in visual effects and film production. Designed as a Rust alternative to OpenImageIO.

## Quick Start

```rust
use vfx_io::{read, write};

// Auto-detect format from extension
let image = read("input.exr")?;
println!("{}x{}, {} channels", image.width, image.height, image.channels);

// Convert format
write("output.png", &image)?;
```

## Supported Formats

| Format | Read | Write | Bit Depths | Feature Flag |
|--------|------|-------|------------|--------------|
| EXR | Yes | Yes | f16, f32 | `exr` (default) |
| PNG | Yes | Yes | 8, 16 | `png` (default) |
| JPEG | Yes | Yes | 8 | `jpeg` (default) |
| TIFF | Yes | Yes | 8, 16, 32f | `tiff` (default) |
| DPX | Yes | Yes | 8, 10, 12, 16 | `dpx` (default) |
| HDR | Yes | Yes | 32f (RGBE) | `hdr` (default) |
| WebP | Yes | Yes | 8 | `webp` |
| AVIF | No | Yes | 8, 10 | `avif` |
| HEIF | Yes | Yes | 8, 10 | `heif` |
| JP2 | Yes | No | 8, 12, 16 | `jp2` |

## ImageData

The main container for image data:

```rust
use vfx_io::{ImageData, PixelFormat, PixelData};

// Create from dimensions
let img = ImageData::new(1920, 1080, 4, PixelFormat::F32);

// Create from data
let data = vec![0.5f32; 1920 * 1080 * 3];
let img = ImageData::from_f32(1920, 1080, 3, data);

// Convert formats
let f32_data = img.to_f32();   // For processing
let u8_data = img.to_u8();     // For display
let u16_data = img.to_u16();   // For 16-bit output
```

## Format-Specific Usage

### EXR (OpenEXR)

```rust
use vfx_io::exr;

// Read with layer info
let image = exr::read("render.exr")?;

// Read specific layer
let layer = exr::read_layer("render.exr", "diffuse")?;

// Write with compression
let opts = exr::WriteOptions {
    compression: exr::Compression::Zip,
    ..Default::default()
};
exr::write_with_options("output.exr", &image, &opts)?;
```

### DPX

```rust
use vfx_io::dpx::{DpxReader, DpxWriter, DpxWriterOptions, BitDepth};

// Read
let image = dpx::read("scan.0001.dpx")?;

// Write 10-bit (common for film)
let opts = DpxWriterOptions {
    bit_depth: BitDepth::Bit10,
    ..Default::default()
};
DpxWriter::with_options(opts).write("output.dpx", &image)?;
```

### HEIF/HEIC

Requires system library (`libheif`):

```rust
use vfx_io::heif;

// Read with HDR info
let (image, hdr_info) = heif::read_heif("photo.heic")?;
if let Some(hdr) = hdr_info {
    println!("HDR: {:?}", hdr.transfer);
}

// Write HDR
heif::write_heif("output.heif", &image, Some(&hdr_info))?;
```

## Multi-Layer Images

For EXR files with multiple layers:

```rust
use vfx_io::{LayeredImage, ImageLayer, ImageChannel};

// Read as layered
let layered = exr::read_layered("render.exr")?;

for layer in &layered.layers {
    println!("Layer: {} ({}x{})", layer.name, layer.width, layer.height);
    for ch in &layer.channels {
        println!("  Channel: {} ({:?})", ch.name, ch.kind);
    }
}

// Convert single layer to ImageData
let rgba = layered.layers[0].to_image_data()?;
```

## Metadata

Access format-specific metadata:

```rust
use vfx_io::Attrs;

let image = read("photo.jpg")?;

// String attributes
if let Some(make) = image.metadata.attrs.get_str("Make") {
    println!("Camera: {}", make);
}

// Numeric attributes
if let Some(iso) = image.metadata.attrs.get_u32("ISO") {
    println!("ISO: {}", iso);
}

// Color space
if let Some(cs) = &image.metadata.colorspace {
    println!("Color space: {}", cs);
}
```

## Image Sequences

Process numbered file sequences:

```rust
use vfx_io::sequence::{Sequence, find_sequences};

// Find sequences in directory
let seqs = find_sequences("./frames/")?;
for seq in seqs {
    println!("{}: frames {}-{}", seq.pattern, seq.start, seq.end);
}

// Iterate over sequence
let seq = Sequence::from_pattern("render.####.exr", 1, 100);
for path in seq.iter() {
    let image = read(&path)?;
    // process...
}
```

## UDIM Support

For texture tile sets:

```rust
use vfx_io::udim::{UdimSet, udim_pattern};

// Load UDIM texture set
let pattern = "texture.<UDIM>.exr";
let tiles = UdimSet::from_pattern(pattern)?;

for (udim, image) in tiles.iter() {
    println!("Tile {}: {}x{}", udim, image.width, image.height);
}
```

## Streaming I/O

For very large images:

```rust
use vfx_io::streaming::{StreamReader, StreamWriter};

// Read in tiles
let reader = StreamReader::open("huge.exr")?;
for tile in reader.tiles() {
    let data = tile.read()?;
    // Process tile...
}
```

## Channel Classification

Channels are classified by semantic meaning:

```rust
use vfx_io::ChannelKind;

match channel.kind {
    ChannelKind::Color => println!("RGB data"),
    ChannelKind::Alpha => println!("Transparency"),
    ChannelKind::Depth => println!("Z-depth"),
    ChannelKind::Id => println!("Object ID"),
    ChannelKind::Mask => println!("Matte"),
    ChannelKind::Generic => println!("Unknown"),
}
```

## Feature Flags

Enable only needed formats:

```toml
[dependencies]
vfx-io = { version = "0.1", default-features = false, features = ["exr", "png"] }
```

System library requirements:
- `heif` - libheif >= 1.17
- `jp2` - OpenJPEG

## Dependencies

- `vfx-core` - Core types
- `exr` - OpenEXR implementation
- `png`, `jpeg-decoder`, `tiff` - Format codecs
- `libheif-rs` - HEIF support (optional)
- `tracing` - Logging

## Design Decisions

### Why not use the `image` crate?

- Missing EXR, DPX, HDR metadata
- No multi-layer support
- Limited HDR/floating-point handling
- VFX workflows need more control

### Format auto-detection

Detection by extension and magic bytes:

```rust
use vfx_io::Format;

let format = Format::detect("file.exr")?;
// Checks extension first, then file header
```
