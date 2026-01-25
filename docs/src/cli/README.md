# CLI Reference

The `vfx` command provides comprehensive image processing for VFX workflows, combining functionality of OIIO tools (oiiotool, iinfo, idiff, iconvert, maketx).

## Global Options

```
-v, --verbose        Increase verbosity (-v info, -vv debug, -vvv trace)
-l, --log [PATH]     Write log to file (default: vfx.log)
--allow-non-color    Allow operations on non-color data (IDs, normals, masks)
```

## Commands

### Image Information

| Command | Alias | Description |
|---------|-------|-------------|
| [info](./info.md) | `i` | Display image metadata and statistics |
| [grep](./grep.md) | | Search metadata across files |
| [diff](./diff.md) | `d` | Compare two images |

### Format Conversion

| Command | Alias | Description |
|---------|-------|-------------|
| [convert](./convert.md) | `c` | Convert format, depth, compression |
| [maketx](./maketx.md) | `tx` | Create tiled/mipmapped textures |

### Geometry Operations

| Command | Alias | Description |
|---------|-------|-------------|
| [resize](./resize.md) | `r` | Scale image |
| [crop](./crop.md) | | Extract rectangular region |
| [transform](./transform.md) | | Flip, rotate 90°, transpose |
| [rotate](./rotate.md) | | Rotate by arbitrary angle |
| [warp](./warp.md) | | Lens distortion and effects |

### Color Operations

| Command | Alias | Description |
|---------|-------|-------------|
| [color](./color.md) | | Color space, exposure, saturation |
| [aces](./aces.md) | | ACES IDT/RRT/ODT transforms |
| [lut](./lut.md) | | Apply 1D/3D LUTs |
| [grade](./grade.md) | | ASC CDL color grading |
| [clamp](./clamp.md) | | Clamp values to range |
| [premult](./premult.md) | | Alpha premultiplication |

### Filters

| Command | Alias | Description |
|---------|-------|-------------|
| [blur](./blur.md) | | Gaussian/box blur |
| [sharpen](./sharpen.md) | | Unsharp mask sharpening |

### Compositing

| Command | Alias | Description |
|---------|-------|-------------|
| [composite](./composite.md) | `comp` | Blend operations (over, add, multiply) |
| [paste](./paste.md) | | Overlay at position |

### EXR Layers

| Command | Alias | Description |
|---------|-------|-------------|
| [layers](./layers.md) | `l` | List layers and channels |
| [extract-layer](./extract-layer.md) | `xl` | Extract single layer |
| [merge-layers](./merge-layers.md) | `ml` | Combine files into layers |

### Channel Operations

| Command | Alias | Description |
|---------|-------|-------------|
| [channel-shuffle](./channel-shuffle.md) | `cs` | Reorder channels by pattern |
| [channel-extract](./channel-extract.md) | `cx` | Extract named channels |

### Batch Processing

| Command | Alias | Description |
|---------|-------|-------------|
| [batch](./batch.md) | | Process multiple files |
| [udim](./udim.md) | | UDIM texture operations |

### Interactive

| Command | Alias | Description |
|---------|-------|-------------|
| [view](./view.md) | `v` | Image viewer with OCIO |

## Common Workflows

### Basic Pipeline

```bash
# Convert → Resize → Color correct → Output
vfx convert input.dpx -o temp.exr -d half
vfx resize temp.exr -o resized.exr -w 1920 -h 1080
vfx color resized.exr -o output.exr --exposure 0.5
```

### ACES Workflow

```bash
# Camera to ACEScg to sRGB output
vfx aces camera.dpx -o working.exr -t idt
vfx color working.exr -o graded.exr --exposure 0.3
vfx aces graded.exr -o final.png -t rrt-odt
```

### VFX Compositing

```bash
# Composite with color match
vfx composite fg.exr bg.exr -o comp.exr --mode over
vfx color comp.exr -o matched.exr --saturation 0.9
vfx aces matched.exr -o review.jpg -t rrt-odt
```

### Batch Processing

```bash
# Resize all files in directory
vfx batch -i "shots/*.exr" -o ./resized --op resize --args scale=0.5

# Convert format
vfx batch -i "*.exr" -o ./png --op convert -f png
```

### Debug Mode

```bash
# Verbose logging for troubleshooting
vfx info -vvv problem.exr
vfx convert input.exr output.exr -vvv --log=debug.log
```

## OIIO Equivalents

| vfx | OIIO |
|-----|------|
| `vfx info` | `iinfo` |
| `vfx convert` | `iconvert` |
| `vfx diff` | `idiff` |
| `vfx grep` | `igrep` |
| `vfx maketx` | `maketx` |
| `vfx view` | `iv` |
| `vfx resize` | `oiiotool --resize` |
| `vfx crop` | `oiiotool --crop` |
| `vfx composite` | `oiiotool --over` |
