# vfx-exr

OpenEXR image format support with full deep data capabilities.

## Overview

`vfx-exr` is a fork of [exrs](https://github.com/johannesvollmer/exrs) extended with complete deep data support. It provides:

- Full OpenEXR 2.x specification compliance
- Deep scanline and tiled images
- All compression methods (except DWA/DWAB)
- Multi-layer and multi-part files
- Mip/rip map levels

## Features

| Feature | Status |
|---------|--------|
| Scanline images | ✅ Read/Write |
| Tiled images | ✅ Read/Write |
| Deep scanline | ✅ Read/Write |
| Deep tiled | ✅ Read/Write |
| Multi-layer | ✅ Read/Write |
| Multi-part | ✅ Read/Write |
| Mip/rip maps | ✅ Read/Write |

### Compression Support

| Method | Read | Write | Type |
|--------|------|-------|------|
| None | ✅ | ✅ | Uncompressed |
| RLE | ✅ | ✅ | Lossless |
| ZIP | ✅ | ✅ | Lossless |
| ZIPS | ✅ | ✅ | Lossless (single scanline) |
| PIZ | ✅ | ✅ | Lossless (wavelet) |
| PXR24 | ✅ | ✅ | Lossy (24-bit float) |
| B44 | ✅ | ✅ | Lossy (fixed rate) |
| B44A | ✅ | ✅ | Lossy (adaptive) |
| DWAA | ❌ | ❌ | Help wanted |
| DWAB | ❌ | ❌ | Help wanted |

## Usage

### Reading EXR Files

```rust
use vfx_exr::prelude::*;

// Read with automatic type detection
let image = read_first_rgba_layer_from_file(
    "render.exr",
    |resolution, _| vec![vec![(0.0f32, 0.0, 0.0, 1.0); resolution.width()]; resolution.height()],
    |buffer, position, pixel| buffer[position.y()][position.x()] = pixel
)?;
```

### Writing EXR Files

```rust
use vfx_exr::prelude::*;

let size = Vec2(1920, 1080);
let pixels: Vec<(f32, f32, f32, f32)> = vec![(0.5, 0.5, 0.5, 1.0); 1920 * 1080];

Image::from_encoded_channels(
    size,
    SpecificChannels::rgba(|pos| pixels[pos.y() * 1920 + pos.x()])
).write().to_file("output.exr")?;
```

### Deep Data

```rust
use vfx_exr::image::deep::*;

// Read deep EXR
let deep_image = read_deep_image("deep_render.exr")?;

// Access per-pixel samples
for pixel in deep_image.pixels() {
    println!("Pixel has {} samples", pixel.sample_count());
    for i in 0..pixel.sample_count() {
        let z = pixel.get::<f32>("Z", i);
        let a = pixel.get::<f32>("A", i);
    }
}
```

## Code Quality

As of 2026-01-14, all critical issues have been addressed:

- ✅ PIZ Huffman overflow protection (saturating_sub)
- ✅ Safe integer casting in compression
- ✅ Wrapping arithmetic in wavelet transforms
- ✅ Proper deep tile handling

## License

BSD-3-Clause (inherited from exrs)
