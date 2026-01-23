# Processing Pipeline

How image data flows through vfx-rs during typical operations.

## Overview

```
┌──────────┐     ┌──────────┐     ┌───────────┐     ┌──────────┐
│  Input   │────►│  Decode  │────►│  Process  │────►│  Encode  │
│  File    │     │  to f32  │     │  (ops)    │     │  & Save  │
└──────────┘     └──────────┘     └───────────┘     └──────────┘
```

## Stage 1: File Reading

### Format Detection

```rust
// vfx_io::read internally:
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    let format = Format::detect(path)?;  // Extension + magic bytes
    
    match format {
        Format::Exr => exr::read(path),
        Format::Png => png::read(path),
        // ...
    }
}
```

### Format-Specific Decoding

Each format produces `ImageData`:

```rust
// EXR: Native f16/f32, preserves precision
let data = exr::read("scene.exr")?;
// data.format = PixelFormat::F16 or F32

// PNG: Decoded to u8/u16
let data = png::read("image.png")?;
// data.format = PixelFormat::U8 or U16

// DPX: Decoded from 10/12-bit packed to u16
let data = dpx::read("scan.dpx")?;
// data.format = PixelFormat::U16
```

## Stage 2: Working Format Conversion

Most operations require f32:

```rust
// ImageData::to_f32() normalizes to 0.0-1.0 for integer types
let working = image.to_f32();

// Conversion rules:
// u8:  v / 255.0
// u16: v / 65535.0
// f16: direct cast
// f32: clone
```

## Stage 3: Processing

### Color Transform Chain

```rust
use vfx_transfer::srgb;
use vfx_color::aces;

// Typical ACES workflow:
let mut data = image.to_f32();
let channels = image.channels as usize;

// 1. Apply transfer function (decode sRGB to linear)
for pixel in data.chunks_exact_mut(channels) {
    let rgb = srgb::eotf_rgb([pixel[0], pixel[1], pixel[2]]);
    pixel[0] = rgb[0];
    pixel[1] = rgb[1];
    pixel[2] = rgb[2];
}

// 2. Apply ACES RRT+ODT (returns new Vec)
let display = aces::apply_rrt_odt_srgb(&data, channels);
```

### Parallel Processing

Operations parallelize over pixels:

```rust
// Row-parallel (cache-friendly)
(0..height).into_par_iter().for_each(|y| {
    let row_start = y * width * channels;
    let row = &mut data[row_start..row_start + width * channels];
    
    for pixel in row.chunks_exact_mut(channels) {
        // Transform pixel
    }
});
```

### In-Place vs. Allocating

```rust
// In-place (preferred for simple transforms)
fn apply_exposure(data: &mut [f32], stops: f32) {
    let factor = 2.0f32.powf(stops);
    for v in data.iter_mut() {
        *v *= factor;
    }
}

// Allocating (when dimensions change)
fn resize(data: &[f32], ...) -> Vec<f32> {
    let mut result = Vec::with_capacity(new_size);
    // Fill result...
    result
}
```

## Stage 4: Output Encoding

### Bit Depth Conversion

```rust
// f32 → u8 (display/web)
let u8_data: Vec<u8> = working.iter()
    .map(|&v| (v.clamp(0.0, 1.0) * 255.0) as u8)
    .collect();

// f32 → u16 (print/archive)
let u16_data: Vec<u16> = working.iter()
    .map(|&v| (v.clamp(0.0, 1.0) * 65535.0) as u16)
    .collect();

// f32 → EXR (keeps native)
// No conversion needed
```

### Format-Specific Encoding

```rust
// PNG: Applies compression, optional bit depth
png::write(path, &image)?;

// EXR: Applies chosen compression (ZIP, PIZ, etc.)
exr::write_with_options(path, &image, &ExrWriteOptions {
    compression: Compression::Zip,
    ..Default::default()
})?;
```

## Data Layout

### Interleaved (Standard)

```
Memory: R0 G0 B0 R1 G1 B1 R2 G2 B2 ...

Index formula:
sample = data[y * width * channels + x * channels + channel]
```

### Planar (EXR Layers)

```
Memory: R0 R1 R2 ... G0 G1 G2 ... B0 B1 B2 ...

Index formula:
sample = data[channel * width * height + y * width + x]
```

Conversion between layouts:

```rust
// Interleaved → Planar
fn deinterleave(data: &[f32], w: usize, h: usize, c: usize) -> Vec<Vec<f32>> {
    let mut planes = vec![vec![0.0; w * h]; c];
    for y in 0..h {
        for x in 0..w {
            for ch in 0..c {
                planes[ch][y * w + x] = data[(y * w + x) * c + ch];
            }
        }
    }
    planes
}
```

## Memory Management

### Avoiding Copies

```rust
// Bad: Multiple allocations
let temp1 = apply_op1(&data);
let temp2 = apply_op2(&temp1);
let result = apply_op3(&temp2);

// Good: In-place chain
let mut data = image.to_f32();
apply_op1(&mut data);
apply_op2(&mut data);
apply_op3(&mut data);
```

### Pre-allocation

```rust
// Known size
let mut result = Vec::with_capacity(width * height * channels);

// Unknown size (rare)
let mut result = Vec::new();
result.reserve(estimated_size);
```

## Error Propagation

```rust
fn process_image(path: &Path) -> Result<ImageData> {
    // Each ? propagates errors up the chain
    let image = vfx_io::read(path)?;
    let data = image.to_f32();
    let processed = some_operation(&data)?;
    let result = ImageData::from_f32(..., processed);
    Ok(result)
}
```

## Tracing Integration

Operations emit tracing events:

```rust
use tracing::{trace, debug, info};

pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    trace!(path = %path.as_ref().display(), "vfx_io::read");
    
    let format = Format::detect(&path)?;
    debug!(format = ?format, "Detected format");
    
    let result = match format {
        // ...
    };
    
    info!(
        w = result.width,
        h = result.height,
        ch = result.channels,
        "Image loaded"
    );
    
    Ok(result)
}
```
