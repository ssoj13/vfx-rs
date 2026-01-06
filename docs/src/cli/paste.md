# paste - Image Overlay

Paste/overlay one image onto another at a specific position.

## Synopsis

```bash
vfx paste <BACKGROUND> <FOREGROUND> -o <OUTPUT> [-x <X>] [-y <Y>] [-b]
```

## Options

| Option | Description |
|--------|-------------|
| `<BACKGROUND>` | Background image |
| `<FOREGROUND>` | Foreground image to paste |
| `-o, --output` | Output file path |
| `-x` | X offset (default: 0, can be negative) |
| `-y` | Y offset (default: 0, can be negative) |
| `-b, --blend` | Use alpha blending (if foreground has alpha) |

## Examples

### Basic Paste

```bash
# Paste at top-left corner
vfx paste background.exr overlay.exr -o result.exr
```

### Paste at Position

```bash
# Paste at specific coordinates
vfx paste background.exr logo.exr -o result.exr -x 100 -y 50
```

### Paste with Alpha Blend

```bash
# Use foreground alpha for blending
vfx paste background.exr element.exr -o result.exr -x 200 -y 150 --blend
```

### Negative Offset

```bash
# Paste partially off-screen
vfx paste background.exr overlay.exr -o result.exr -x -50 -y -25
```

### Center Paste

```bash
# Calculate center position (for 1920x1080 bg, 100x100 fg)
# x = (1920 - 100) / 2 = 910
# y = (1080 - 100) / 2 = 490
vfx paste background.exr logo.exr -o result.exr -x 910 -y 490 --blend
```

## Coordinate System

```
Background Image:
(0,0)─────────────────────────────▶ X
  │
  │  (x,y)┌──────────────┐
  │       │              │
  │       │  Foreground  │
  │       │              │
  │       └──────────────┘
  │
  ▼
  Y
```

## Blend Modes

### Without --blend

```
result = foreground (where fg exists)
       = background (elsewhere)
```

Foreground completely replaces background.

### With --blend

```
result = fg * fg_alpha + bg * (1 - fg_alpha)
```

Uses foreground alpha for smooth compositing.

## Use Cases

### Add Logo/Watermark

```bash
# Add watermark to bottom-right
# For 1920x1080 image, 200x50 watermark, 10px margin
vfx paste frame.exr watermark.exr -o branded.exr \
    -x 1710 -y 1020 --blend
```

### Composite Elements

```bash
# Overlay VFX element
vfx paste plate.exr vfx_element.exr -o comp.exr \
    -x 500 -y 300 --blend
```

### Patch Image

```bash
# Replace region with patch
vfx crop source.exr -x 100 -y 100 -w 200 -H 200 -o patch.exr
# ... edit patch ...
vfx paste source.exr edited_patch.exr -o fixed.exr -x 100 -y 100
```

### Create Contact Sheet

```bash
# Manually arrange thumbnails
vfx paste canvas.exr thumb1.exr -o temp1.exr -x 0 -y 0
vfx paste temp1.exr thumb2.exr -o temp2.exr -x 110 -y 0
vfx paste temp2.exr thumb3.exr -o temp3.exr -x 220 -y 0
vfx paste temp3.exr thumb4.exr -o sheet.exr -x 0 -y 110
```

## Notes

- Foreground can extend beyond background bounds (clipped)
- Preserves background format and color space
- Works with any pixel format
- For complex compositing, use [composite](./composite.md)

## See Also

- [composite](./composite.md) - Advanced compositing modes
- [crop](./crop.md) - Extract regions
