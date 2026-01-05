# Data Flow

This page describes how image data flows through vfx-rs during typical operations.

## Core Data Structure

All image data flows through `vfx_io::ImageData`:

```rust
pub struct ImageData {
    pub width: u32,
    pub height: u32,
    pub channels: u32,
    pub data: ImageBuffer,  // u8, u16, f16, or f32
}
```

The `ImageBuffer` enum supports multiple bit depths:

```rust
pub enum ImageBuffer {
    U8(Vec<u8>),
    U16(Vec<u16>),
    F16(Vec<half::f16>),
    F32(Vec<f32>),
}
```

## Read-Process-Write Pipeline

A typical CLI operation follows this pattern:

```
┌─────────────┐     ┌───────────────┐     ┌───────────────┐     ┌─────────────┐
│  Load Image │ ──► │ Convert to    │ ──► │   Process     │ ──► │ Save Image  │
│  (vfx-io)   │     │ f32 working   │     │  (vfx-ops,    │     │ (vfx-io)    │
│             │     │ format        │     │   vfx-color)  │     │             │
└─────────────┘     └───────────────┘     └───────────────┘     └─────────────┘
```

### Step 1: Load Image

```rust
// vfx_io::read() dispatches to format-specific loader
let image = vfx_io::read("input.exr")?;

// EXR: preserves f16/f32, multi-layer
// PNG/JPEG: loads as u8, converted later
// DPX: loads as u16 (10-bit packed to 16-bit)
```

### Step 2: Convert to Working Format

Most processing requires f32 linear data:

```rust
let data = image.to_f32();  // Vec<f32>
```

Conversion handles:
- u8 → f32: divide by 255.0
- u16 → f32: divide by 65535.0
- f16 → f32: direct conversion

### Step 3: Process

Operations work on `&mut [f32]` or return new `Vec<f32>`:

```rust
// Color transform
let result = vfx_color::aces::apply_rrt_odt_srgb(&data, channels);

// Image operation
let blurred = vfx_ops::filter::box_blur(&data, w, h, c, radius)?;

// Resize
let resized = vfx_ops::resize::resize_f32(&data, sw, sh, c, dw, dh, filter)?;
```

### Step 4: Save Image

Output format determines final bit depth:

```rust
let output = ImageData::from_f32(width, height, channels, result);
vfx_io::write("output.png", &output)?;  // Converts to u8 for PNG
vfx_io::write("output.exr", &output)?;  // Keeps f32 for EXR
```

## Color Transform Pipeline

ACES workflow example:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            ACES Color Pipeline                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌────────────┐     ┌─────────────┐     ┌─────────┐     ┌───────────────┐  │
│  │   Input    │ ──► │     IDT     │ ──► │   RRT   │ ──► │     ODT       │  │
│  │   (sRGB)   │     │ sRGB→ACEScg │     │ tonemap │     │ ACEScg→sRGB   │  │
│  └────────────┘     └─────────────┘     └─────────┘     └───────────────┘  │
│                                                                             │
│  Inverse:                                                                   │
│                                                                             │
│  ┌────────────┐     ┌─────────────┐                                        │
│  │   sRGB     │ ◄── │ Inverse ODT │ ◄── (for reading display-referred)     │
│  │   Display  │     │ sRGB→ACEScg │                                        │
│  └────────────┘     └─────────────┘                                        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

Code flow:

```rust
// IDT: sRGB gamma → ACEScg linear
apply_inverse_odt_srgb(&data, channels)

// RRT: Reference Rendering Transform (tonemap)
rrt(r, g, b, &params)

// ODT: ACEScg → sRGB display
acescg_to_srgb(r, g, b)
srgb::oetf(value)  // Apply gamma

// Combined RRT+ODT (most common)
apply_rrt_odt_srgb(&data, channels)
```

## Parallel Processing

Operations use Rayon for multi-threaded processing:

```rust
// Row-parallel blur
(0..height).into_par_iter().for_each(|y| {
    // Each thread processes independent rows
});

// Chunk-parallel pixel operations
data.par_chunks_mut(chunk_size).for_each(|chunk| {
    for pixel in chunk.chunks_exact_mut(channels) {
        // Process RGB
    }
});
```

## GPU Compute Path

When GPU features are enabled:

```
┌─────────────┐     ┌───────────────┐     ┌─────────────┐     ┌─────────────┐
│  CPU Data   │ ──► │ Upload to GPU │ ──► │ GPU Compute │ ──► │ Download    │
│  Vec<f32>   │     │   Buffer      │     │   Shader    │     │ to CPU      │
└─────────────┘     └───────────────┘     └─────────────┘     └─────────────┘
```

GPU is typically faster for:
- Large images (4K+)
- Batch processing
- Complex operations (3D LUT apply, convolutions)

CPU is typically faster for:
- Small images
- Simple operations
- When GPU upload/download overhead dominates

## Multi-Layer EXR Flow

EXR files can contain multiple named layers:

```
input.exr
├── beauty (RGBA)
├── diffuse (RGB)
├── specular (RGB)
├── depth (Z)
└── normal (XYZ)
```

Layer-aware processing:

```rust
// Load specific layer
let image = load_image_layer(&path, Some("diffuse"))?;

// Process only that layer
let processed = apply_color_transform(&image)?;

// Save back (preserves other layers)
save_image_layer(&path, &processed, Some("diffuse"))?;
```

## Memory Layout

All image data is stored in planar-interleaved format:

```
For RGB image (3 channels):
[R0, G0, B0, R1, G1, B1, R2, G2, B2, ...]

Index formula:
pixel_index = y * width + x
channel_offset = pixel_index * num_channels + channel
```

This matches standard image file formats and GPU texture layouts.

## Error Handling Flow

Errors propagate through the call stack:

```
vfx_io::read()
    └── FormatError
        └── ExrError / PngError / etc.

vfx_ops::resize()
    └── FilterError
        └── InvalidDimension / UnsupportedFilter

vfx_cli::run()
    └── anyhow::Error (collects all)
```

The CLI uses `anyhow` for ergonomic error handling, while library crates use `thiserror` for typed errors.
