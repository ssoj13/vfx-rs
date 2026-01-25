# grade - CDL Color Grading

Apply ASC CDL (Color Decision List) color grading operations.

## Synopsis

```bash
vfx grade <INPUT> -o <OUTPUT> [OPTIONS]
```

## Options

| Option | Description |
|--------|-------------|
| `-o, --output` | Output file path |
| `--slope <R,G,B>` | Multiplier before offset (default: 1,1,1) |
| `--offset <R,G,B>` | Value added after slope (default: 0,0,0) |
| `--power <R,G,B>` | Gamma/power applied last (default: 1,1,1) |
| `--saturation <F>` | Saturation multiplier (default: 1.0) |
| `--layer <NAME>` | Process only this layer (multi-layer EXR) |

## CDL Formula

The ASC CDL formula applied per-pixel:

```
output = (input * slope + offset) ^ power
```

If saturation is not 1.0, an additional saturation adjustment is applied using Rec.709 luminance coefficients.

## Examples

### Lift/Gamma/Gain Style

```bash
# Darken shadows (reduce offset)
vfx grade input.exr -o dark.exr --offset -0.1,-0.1,-0.1

# Brighten midtones (power < 1)
vfx grade input.exr -o bright.exr --power 0.8,0.8,0.8

# Increase overall contrast (slope > 1)
vfx grade input.exr -o contrast.exr --slope 1.2,1.2,1.2
```

### Color Correction

```bash
# Warm up (add red, reduce blue)
vfx grade input.exr -o warm.exr --slope 1.1,1.0,0.9

# Cool down
vfx grade input.exr -o cool.exr --slope 0.9,1.0,1.1

# Desaturate
vfx grade input.exr -o desat.exr --saturation 0.5

# Oversaturate
vfx grade input.exr -o sat.exr --saturation 1.5
```

### Combined Adjustments

```bash
# Full CDL grade
vfx grade input.exr -o graded.exr \
    --slope 1.1,1.0,0.95 \
    --offset 0.02,0.01,0.0 \
    --power 1.1,1.0,0.95 \
    --saturation 1.1
```

### Multi-Layer EXR

```bash
# Grade only beauty pass
vfx grade render.exr -o graded.exr --layer beauty \
    --slope 1.2,1.1,1.0
```

## Notes

- Negative values are handled with sign-preserving power function
- Saturation uses Rec.709 luminance weights (0.2126, 0.7152, 0.0722)
- Works with any channel count (grayscale, RGB, RGBA)

## See Also

- [color](./color.md) - Color space conversions
- [lut](./lut.md) - LUT application
