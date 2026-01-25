# clamp - Value Range Clamping

Clamp pixel values to a specified range.

## Synopsis

```bash
vfx clamp <INPUT> -o <OUTPUT> [OPTIONS]
```

## Options

| Option | Description |
|--------|-------------|
| `-o, --output` | Output file path |
| `--min <VALUE>` | Minimum value (default: 0.0) |
| `--max <VALUE>` | Maximum value (default: 1.0) |
| `--negatives` | Clamp only negative values to 0 |
| `--fireflies` | Clamp only values > 1.0 |
| `--layer <NAME>` | Process only this layer (multi-layer EXR) |

## Examples

### Standard Range Clamping

```bash
# Clamp to standard [0, 1] range
vfx clamp input.exr -o clamped.exr

# Custom range
vfx clamp input.exr -o hdr.exr --min 0.0 --max 10.0
```

### Convenience Modes

```bash
# Remove negative values (common in compositing)
vfx clamp input.exr -o clean.exr --negatives

# Remove fireflies (bright outliers)
vfx clamp render.exr -o clean.exr --fireflies
```

### Multi-Layer EXR

```bash
# Clamp beauty pass only
vfx clamp render.exr -o clean.exr --layer beauty --negatives
```

## Use Cases

### Cleaning Render Passes

```bash
# Remove negative values from diffuse pass
vfx clamp render.exr -o diffuse_clean.exr \
    --layer diffuse --negatives

# Clamp fireflies in specular
vfx clamp render.exr -o spec_clean.exr \
    --layer specular --fireflies
```

### Preparing for Export

```bash
# Clamp HDR to SDR range before JPEG export
vfx clamp hdr.exr -o sdr.exr --min 0 --max 1
vfx convert sdr.exr -o output.jpg
```

## Notes

- Clamps all channels (including alpha if present)
- `--negatives` and `--fireflies` are mutually exclusive with `--min/--max`
- For alpha-aware clamping, use `premult` before/after

## See Also

- [premult](./premult.md) - Alpha premultiplication
- [grade](./grade.md) - CDL grading
