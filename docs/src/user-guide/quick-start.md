# Quick Start

This guide will get you up and running with vfx-rs in minutes.

## Installation

### From crates.io

```bash
cargo install vfx-cli
```

### From Source

```bash
git clone https://github.com/vfx-rs/vfx-rs
cd vfx-rs
cargo build --release
```

The binary will be at `target/release/vfx`.

## Basic Usage

### View Image Information

```bash
# Basic info
vfx info image.exr

# Full statistics and metadata
vfx info image.exr --stats --all

# JSON output for scripting
vfx info image.exr --json
```

### Convert Formats

```bash
# Simple conversion
vfx convert input.exr output.png

# EXR with half-float and PIZ compression
vfx convert input.exr output.exr -d half -c piz

# JPEG with quality setting
vfx convert input.png output.jpg -q 90
```

### Resize Images

```bash
# Resize to specific dimensions
vfx resize input.exr -w 1920 -H 1080 -o output.exr

# Scale by factor
vfx resize input.exr --scale 0.5 -o half_size.exr

# Using different filter
vfx resize input.exr -w 4096 -o output.exr --filter lanczos
```

### Color Transforms

```bash
# Exposure adjustment (+1 stop)
vfx color input.exr -o output.exr --exposure 1.0

# Gamma correction
vfx color input.exr -o output.exr --gamma 2.2

# Saturation adjustment
vfx color input.exr -o output.exr --saturation 1.2

# Transfer function (linear to sRGB for display)
vfx color linear.exr -o display.png --transfer linear_to_srgb

# Color space conversion (gamut mapping)
vfx color input.exr -o output.exr --from ACEScg --to sRGB
vfx color render.exr -o display.exr --from ACEScg --to Rec2020
```

Supported color spaces: sRGB, linear_srgb, ACEScg, ACES2065, ACEScct, ACEScc, Rec709, Rec2020, DCI-P3, Display_P3.

### ACES Workflow

```bash
# Convert sRGB input to ACEScg (IDT)
vfx aces input.jpg -o linear.exr -t idt

# Apply RRT (Reference Rendering Transform)
vfx aces linear.exr -o tonemapped.exr -t rrt

# Full output transform (RRT + ODT to sRGB)
vfx aces linear.exr -o final.png -t rrt-odt
```

### Apply LUTs

```bash
# Apply .cube LUT (1D or 3D)
vfx lut input.exr -o output.exr -l film_look.cube

# Invert LUT
vfx lut graded.exr -o original.exr -l film_look.cube --invert
```

### Image Compositing

```bash
# Over composite (alpha blend)
vfx composite fg.exr bg.exr -o result.exr --mode over

# Additive blend
vfx composite light.exr base.exr -o result.exr --mode add

# Multiply blend
vfx composite mask.exr image.exr -o result.exr --mode multiply
```

### EXR Layer Operations

```bash
# List layers in EXR
vfx layers multi.exr

# Extract specific layer
vfx extract-layer multi.exr -o beauty.exr --layer beauty

# Merge multiple images into layers
vfx merge-layers beauty.exr diffuse.exr spec.exr -o combined.exr
```

### Batch Processing

```bash
# Convert all EXR files to PNG
vfx batch -i "*.exr" -o ./output -f png --op convert

# Resize all images to 50%
vfx batch -i "*.exr" -o ./resized --op resize --args scale=0.5
```

## Supported Formats

| Format | Read | Write | Extensions |
|--------|------|-------|------------|
| OpenEXR | Yes | Yes | .exr |
| JPEG | Yes | Yes | .jpg, .jpeg |
| PNG | Yes | Yes | .png |
| TIFF | Yes | Yes | .tif, .tiff |
| DPX | Yes | Yes | .dpx |
| HDR | Yes | Yes | .hdr |

**With optional features:**

| Format | Read | Write | Extensions | Feature |
|--------|------|-------|------------|---------|
| WebP | Yes | Yes | .webp | webp |
| AVIF | No | Yes | .avif | avif |
| JPEG2000 | Yes | No | .jp2, .j2k | jp2 |
| HEIF/HEIC | Yes | Yes | .heif, .heic | heif |

## Next Steps

- [CLI Reference](../cli/README.md) - Complete command documentation
- [ACES Workflows](../aces/README.md) - Understanding ACES color management
- [Programmer Guide](../programmer/README.md) - Using vfx-rs as a library
