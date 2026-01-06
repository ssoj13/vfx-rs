# crop - Image Cropping

Extract a rectangular region from an image.

## Synopsis

```bash
vfx crop <INPUT> -x <X> -y <Y> -w <WIDTH> -H <HEIGHT> -o <OUTPUT>
```

## Options

| Option | Description |
|--------|-------------|
| `-x <X>` | X offset (left edge) |
| `-y <Y>` | Y offset (top edge) |
| `-w <WIDTH>` | Width of crop region |
| `-H <HEIGHT>` | Height of crop region |
| `-o, --output` | Output file path |
| `--layer` | Process only this layer (for multi-layer EXR) |

## Examples

### Basic Crop

```bash
# Crop 1920x1080 region starting at (100, 50)
vfx crop input.exr -x 100 -y 50 -w 1920 -H 1080 -o cropped.exr
```

### Center Crop

```bash
# For 4K image (3840x2160), crop center 1920x1080
vfx crop 4k.exr -x 960 -y 540 -w 1920 -H 1080 -o center.exr
```

### Crop Specific Layer

```bash
# Crop only the beauty layer from multi-layer EXR
vfx crop multilayer.exr -x 0 -y 0 -w 1920 -H 1080 \
    -o beauty_crop.exr --layer beauty
```

## Coordinate System

```
(0,0)─────────────────────────▶ X
  │
  │    (x,y)┌─────────────┐
  │         │             │
  │         │  Crop Area  │
  │         │  (w × H)    │
  │         │             │
  │         └─────────────┘
  │
  ▼
  Y
```

## Notes

- Coordinates are in pixels, origin at top-left
- Crop region must be within image bounds
- Preserves pixel format and color space
- Metadata is preserved in output

## See Also

- [resize](./resize.md) - Scale images
- [paste](./paste.md) - Overlay images
