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
| Deep data | ✗ |
| Tiled | ✓ |
| Compression | ZIP, PIZ, RLE, etc. |

**Metadata preserved**:
- Layer names
- Channel names
- Display/data windows
- Chromaticities
- Custom attributes

```bash
# Read specific layer
vfx info input.exr --layers
vfx convert input.exr -o output.png --layer "diffuse"
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
- 16-bit output when source is float

### JPEG (.jpg, .jpeg)

**Feature**: `jpeg` (default)

| Capability | Support |
|------------|---------|
| Read | ✓ |
| Write | ✓ |
| Quality control | ✓ |
| Progressive | ✓ |
| CMYK | ✗ |

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
| Multi-page | partial |

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
| Layers | ✗ |
| 8/16-bit | ✓ |

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

**Feature**: `ocio`

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
| `.hdr` | Radiance HDR |
| `.dpx` | DPX |
| `.psd` | Photoshop |
| `.cube` | Cube LUT |
| `.clf` | CLF |
| `.spi1d`, `.spi3d` | SPI |
| `.icc`, `.icm` | ICC Profile |

## Feature Flag Summary

```toml
[dependencies.vfx-io]
features = [
    "exr",    # OpenEXR (default)
    "png",    # PNG (default)
    "jpeg",   # JPEG (default)
    "tiff",   # TIFF
    "hdr",    # Radiance HDR
    "dpx",    # DPX
    "psd",    # Photoshop (read-only)
]

[dependencies.vfx-lut]
features = [
    "cube",   # .cube (default)
    "clf",    # .clf
    "spi",    # .spi1d/.spi3d
]
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
