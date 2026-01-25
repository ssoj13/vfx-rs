# Supported Formats

## Image Formats

### OpenEXR (.exr)

**Feature**: `exr` (default)

| Capability | Support |
|------------|---------|
| Read | ✓ |
| Write | ✓ |
| Half float (f16) | ✓ |
| Full float (f32) | ✓ |
| Multi-layer | ✓ |
| Deep data | ✓ (via read_deep/write_deep) |
| Tiled | ✓ |
| Compression | ZIP, PIZ, RLE, etc. |

**Metadata preserved**:
- Layer names
- Channel names
- Display/data windows
- Chromaticities
- Custom attributes

```bash
# List layers
vfx layers input.exr

# Extract specific layer
vfx extract-layer input.exr -o diffuse.png --layer diffuse
```

### PNG (.png)

**Feature**: `png` (default)

| Capability | Support |
|------------|---------|
| Read | ✓ |
| Write | ✓ |
| 8-bit | ✓ |
| 16-bit | ✓ |
| Grayscale | ✓ |
| RGB/RGBA | ✓ |
| Interlaced | ✓ |

**Notes**:
- sRGB gamma assumed on read
- Defaults to 8-bit output; use `PngWriter::with_options()` with `BitDepth::Sixteen` for 16-bit

### JPEG (.jpg, .jpeg)

**Feature**: `jpeg` (default)

| Capability | Support |
|------------|---------|
| Read | ✓ |
| Write | ✓ |
| Quality control | ✓ |
| Progressive | Read only (write uses baseline) |
| CMYK | ✓ (read, auto-converted to RGB) |

**Write options**:
```bash
vfx convert input.exr -o output.jpg --quality 95
```

### TIFF (.tif, .tiff)

**Feature**: `tiff`

| Capability | Support |
|------------|---------|
| Read | ✓ |
| Write | ✓ |
| 8/16/32-bit | ✓ |
| LZW compression | ✓ |
| Multi-page | partial (read-only, no page selection API) |

### HDR (.hdr)

**Feature**: `hdr`

| Capability | Support |
|------------|---------|
| Read | ✓ |
| Write | ✓ |
| RGBE encoding | ✓ |

**Notes**:
- Radiance HDR format
- Good for environment maps
- Lower precision than EXR

### PSD (.psd)

**Feature**: `psd`

| Capability | Support |
|------------|---------|
| Read | ✓ (flattened) |
| Write | ✗ |
| Layers | ✓ (via `read_layers()`) |
| 8/16-bit input | ✓ (output always 8-bit RGBA) |

### DPX (.dpx)

**Feature**: `dpx`

| Capability | Support |
|------------|---------|
| Read | ✓ |
| Write | ✓ |
| 10-bit log | ✓ |
| 16-bit | ✓ |
| Metadata | partial |

**Notes**:
- Film scanning format
- Log encoding common

## LUT Formats

### Cube (.cube)

Adobe/Resolve format.

| Capability | Support |
|------------|---------|
| 1D LUT | ✓ |
| 3D LUT | ✓ |
| Sizes | any |
| Domain | ✓ |

**Example**:
```
LUT_3D_SIZE 33
DOMAIN_MIN 0.0 0.0 0.0
DOMAIN_MAX 1.0 1.0 1.0
0.0 0.0 0.0
...
```

### CLF (.clf)

Academy Color Foundation format.

| Capability | Support |
|------------|---------|
| Read | ✓ |
| Write | ✓ |
| ProcessList | ✓ |
| LUT1D/3D | ✓ |
| Matrix | ✓ |
| CDL | ✓ |

**Notes**:
- XML-based
- Supports transform chains
- ACES-compatible

### SPI (.spi1d, .spi3d)

Sony Pictures Imageworks format.

| Capability | Support |
|------------|---------|
| 1D LUT (.spi1d) | ✓ |
| 3D LUT (.spi3d) | ✓ |

**Notes**:
- OpenColorIO native format
- Text-based

### CSP (.csp)

Rising Sun Pictures format.

| Capability | Support |
|------------|---------|
| Read | ✓ |
| Write | ✗ |
| 1D prelut | ✓ |
| 3D LUT | ✓ |

## ICC Profiles (.icc, .icm)

**Feature**: `icc`

| Version | Support |
|---------|---------|
| v2 | ✓ |
| v4 | ✓ |
| Display | ✓ |
| Input | ✓ |
| Output | ✓ |

**Intents**:
- Perceptual
- Relative colorimetric
- Saturation
- Absolute colorimetric

## OCIO Configs

OCIO support is provided by vfx-ocio crate (always available, no feature flag).

| Format | Support |
|--------|---------|
| OCIO v1 | ✓ |
| OCIO v2 | ✓ |

**Config discovery**:
1. `--config` flag
2. `$OCIO` environment variable
3. Default ACES config

## Format Detection

Formats are detected by file extension:

| Extension | Format |
|-----------|--------|
| `.exr` | OpenEXR |
| `.png` | PNG |
| `.jpg`, `.jpeg` | JPEG |
| `.tif`, `.tiff` | TIFF |
| `.hdr`, `.pic` | Radiance HDR |
| `.dpx` | DPX |
| `.heif`, `.heic` | HEIF/HEIC |
| `.webp` | WebP |
| `.avif` | AVIF |
| `.jp2` | JPEG2000 |

**Note:** LUT formats (.cube, .clf, .spi1d, .spi3d) and ICC profiles (.icc, .icm) are NOT auto-detected. PSD format requires explicit `psd` feature.

## Feature Flag Summary

```toml
[dependencies.vfx-io]
features = [
    "exr",    # OpenEXR (default)
    "png",    # PNG (default)
    "jpeg",   # JPEG (default)
    "tiff",   # TIFF (default)
    "dpx",    # DPX (default)
    "hdr",    # Radiance HDR (default)
    "psd",    # Photoshop (read-only)
    "dds",    # DirectDraw Surface
    "ktx",    # Khronos Texture
    "webp",   # WebP via image crate
    "avif",   # AVIF via image crate
    "jp2",    # JPEG2000 (requires OpenJPEG)
    "heif",   # HEIF/HEIC (requires libheif)
    "text",   # Text rendering
    "rayon",  # Parallel processing
]

# vfx-lut has no feature flags - all LUT formats are always available
[dependencies.vfx-lut]
```

## Recommended Formats

| Use Case | Format |
|----------|--------|
| VFX pipeline | EXR |
| Color grading | EXR, DPX |
| Web delivery | PNG, JPEG |
| HDR display | EXR, HDR |
| Print | TIFF (16-bit) |
| LUT interchange | CLF, Cube |
