# convert - Format Conversion

Convert between image formats with optional bit depth and compression settings. Equivalent to OIIO's `iconvert`.

## Usage

```bash
vfx convert [OPTIONS] <INPUT> <OUTPUT>
```

## Options

| Option | Description |
|--------|-------------|
| `-d, --depth <DEPTH>` | Target bit depth: 8, 16, 32, half |
| `-c, --compression <COMP>` | EXR compression: none, rle, zip, piz, dwaa, dwab |

## Examples

```bash
# EXR to PNG (auto 8-bit)
vfx convert render.exr output.png

# EXR to EXR with different compression
vfx convert input.exr output.exr -c dwaa

# Convert to half-float
vfx convert input.exr output.exr -d half

# DPX to EXR (10-bit log to linear)
vfx convert scan.dpx output.exr

# JPEG to PNG
vfx convert photo.jpg photo.png
```

## Format Auto-Detection

The output format is detected from file extension:

| Extension | Format |
|-----------|--------|
| `.exr` | OpenEXR |
| `.png` | PNG |
| `.jpg`, `.jpeg` | JPEG |
| `.tif`, `.tiff` | TIFF |
| `.dpx` | DPX |
| `.hdr` | Radiance RGBE |
| `.heic`, `.heif` | HEIF (if enabled) |
| `.webp` | WebP (if enabled) |

## EXR-to-EXR Preservation

When both input and output are EXR, all layers and channels are preserved:

```bash
# Preserves all layers
vfx convert multilayer.exr recompressed.exr -c piz
```

## Bit Depth Reference

| Depth | Format | Use Case |
|-------|--------|----------|
| `8` | U8 | Display, web |
| `16` | U16 | Print, DPX |
| `half` | F16 | VFX, HDR |
| `32` | F32 | Deep compositing |
