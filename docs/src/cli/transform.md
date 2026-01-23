# transform - Flip/Rotate

Apply geometric transformations: flip, rotate 90°, transpose.

## Synopsis

```bash
vfx transform <INPUT> -o <OUTPUT> [OPTIONS]
```

## Options

| Option | Description |
|--------|-------------|
| `-o, --output` | Output file path |
| `--flip-h` | Flip horizontal (mirror) |
| `--flip-v` | Flip vertical |
| `-r, --rotate` | Rotate: 90, 180, 270 degrees |
| `--transpose` | Transpose (swap X and Y axes) |

## Examples

### Horizontal Flip

```bash
# Mirror image left-right
vfx transform input.exr -o mirrored.exr --flip-h
```

### Vertical Flip

```bash
# Flip image upside-down
vfx transform input.exr -o flipped.exr --flip-v
```

### Rotate 90°

```bash
# Rotate 90° clockwise
vfx transform input.exr -o rotated.exr -r 90
```

### Rotate 180°

```bash
# Rotate 180° (same as --flip-h --flip-v)
vfx transform input.exr -o upside_down.exr -r 180
```

### Rotate 270°

```bash
# Rotate 270° clockwise (or 90° counter-clockwise)
vfx transform input.exr -o rotated.exr -r 270
```

### Transpose

```bash
# Swap X and Y axes
vfx transform input.exr -o transposed.exr --transpose
```

### Combined Operations

```bash
# Flip both horizontal and vertical
vfx transform input.exr -o both.exr --flip-h --flip-v

# Flip and rotate
vfx transform input.exr -o result.exr --flip-h -r 90
```

## Transform Visualization

```
Original:        --flip-h:        --flip-v:        -r 90:
┌─────────┐      ┌─────────┐      ┌─────────┐      ┌─────┐
│ A     B │      │ B     A │      │ C     D │      │ B D │
│         │      │         │      │         │      │     │
│ C     D │      │ D     C │      │ A     B │      │ A C │
└─────────┘      └─────────┘      └─────────┘      └─────┘

-r 180:          -r 270:          --transpose:
┌─────────┐      ┌─────┐          ┌─────┐
│ D     C │      │ C A │          │ A C │
│         │      │     │          │     │
│ B     A │      │ D B │          │ B D │
└─────────┘      └─────┘          └─────┘
```

## Use Cases

### Fix Orientation

```bash
# Camera was upside down
vfx transform shot.exr -o fixed.exr -r 180
```

### Mirror for Compositing

```bash
# Create mirrored element
vfx transform element.exr -o element_mirror.exr --flip-h
```

### Aspect Ratio Change

```bash
# Transpose for vertical to horizontal
vfx transform portrait.exr -o landscape.exr --transpose
```

## Performance

All transforms are:
- O(n) single-pass operations
- Memory-efficient (streaming where possible)
- Parallel via rayon

## Notes

- Rotation is lossless (exact pixel mapping)
- 90° rotation is clockwise
- Works on any image format
- Multi-layer EXR: first layer only (use `--layer` on other commands for specific layers)
- Output is always float32

## See Also

- [rotate](./rotate.md) - Arbitrary angle rotation
- [resize](./resize.md) - Scale images
- [crop](./crop.md) - Extract regions
