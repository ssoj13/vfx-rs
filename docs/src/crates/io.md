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
| AVIF | No | Yes | 8 | `avif` |
| HEIF | Yes | Yes | 8, 10 | `heif` |
| JP2 | Yes | No | 8, 12, 16 | `jp2` |
| PSD | Yes | No | 8, 16 | `psd` |
| DDS | Yes | No | various | `dds` |
| KTX2 | Yes | No | various | `ktx` |

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
use vfx_io::exr::{self, ExrWriter, ExrWriterOptions, Compression};

// Read single image
let image = exr::read("render.exr")?;

// Read all layers
let layered = exr::read_layers("render.exr")?;

// Read specific layer by index (0-based)
let layer = exr::read_layer("render.exr", 0, 0)?;

// Write with compression options
let writer = ExrWriter::with_options(ExrWriterOptions {
    compression: Compression::Zip,
    ..Default::default()
});
writer.write("output.exr", &image)?;
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

Requires system library (`libheif`) and feature flag:

```rust
use vfx_io::heif;

// Read with HDR info
let (image, hdr_info) = heif::read_heif("photo.heic")?;
if let Some(ref hdr) = hdr_info {
    println!("HDR: {:?}", hdr.transfer);
}

// Write with optional HDR metadata
heif::write_heif("output.heif", &image, hdr_info.as_ref())?;
```

## Multi-Layer Images

For EXR files with multiple layers:

```rust
use vfx_io::{LayeredImage, ImageLayer, ImageChannel};

// Read as layered (returns LayeredImage with all layers)
let layered = exr::read_layers("render.exr")?;

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
use vfx_io::sequence::{Sequence, scan_dir, FrameRange};

// Find sequences in directory
let seqs = scan_dir(Path::new("./frames/"))?;
for seq in seqs {
    println!("{}: frames {}-{}", seq.pattern(), seq.start, seq.end);
}

// Create sequence from pattern
let seq = Sequence::from_pattern("render.%04d.exr")?;
let range = FrameRange::new(1, 100);
for path in seq.paths(&range) {
    let image = read(&path)?;
    // process...
}
```

## UDIM Support

For texture tile sets:

```rust
use vfx_io::udim::UdimResolver;

// Load UDIM texture set
let resolver = UdimResolver::new("texture.<UDIM>.exr")?;

// Get available tiles
for (udim, path) in resolver.tiles() {
    let image = read(path)?;
    println!("Tile {}: {}x{}", udim, image.width, image.height);
}
```

## Streaming I/O

For very large images, use the streaming API which reads/writes in tiles:

```rust
use vfx_io::streaming::{open_streaming, StreamingSource};

// Open with auto-detection (tiled TIFF/EXR use true streaming)
let mut source = open_streaming("huge_image.tif")?;

println!("Image: {}x{}", source.width(), source.height());
println!("Channels: {}", source.source_channels());

// Read regions on demand
let region = source.read_region(0, 0, 512, 512)?;
// Region is always RGBA f32 for consistency

// Process in tiles
for ty in (0..source.height()).step_by(512) {
    for tx in (0..source.width()).step_by(512) {
        let w = 512.min(source.width() - tx);
        let h = 512.min(source.height() - ty);
        let tile = source.read_region(tx, ty, w, h)?;
        // Process tile...
    }
}
```

**Note:** True tile-by-tile streaming is supported for tiled TIFF and tiled EXR. Scanline EXR and other formats may cache the full image internally for region access.

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
- `vfx-exr` - OpenEXR implementation
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

Detection by magic bytes and extension:

```rust
use vfx_io::Format;

let format = Format::detect("file.exr")?;
// Checks magic bytes first, falls back to extension
```
