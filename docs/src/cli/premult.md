# premult - Alpha Premultiplication

Control alpha premultiplication state of RGBA images.

## Synopsis

```bash
vfx premult <INPUT> -o <OUTPUT> --premultiply|--unpremultiply [OPTIONS]
```

## Options

| Option | Description |
|--------|-------------|
| `-o, --output` | Output file path |
| `--premultiply` | Multiply RGB by alpha |
| `--unpremultiply` | Divide RGB by alpha |
| `--layer <NAME>` | Process only this layer (multi-layer EXR) |

**Note:** Must specify either `--premultiply` or `--unpremultiply`.

## Background

**Premultiplied alpha** (also called "associated alpha"):
- RGB values are multiplied by alpha: `(R*A, G*A, B*A, A)`
- Standard for compositing, EXR, and VFX pipelines
- Allows correct blending without edge artifacts

**Straight alpha** (also called "unassociated alpha"):
- RGB values are independent of alpha: `(R, G, B, A)`
- Common in PNG, some game engines
- Can show color fringing at edges when composited

## Examples

### Convert Between States

```bash
# Convert straight to premultiplied
vfx premult straight.png -o premult.exr --premultiply

# Convert premultiplied to straight
vfx premult premult.exr -o straight.png --unpremultiply
```

### Before/After Operations

```bash
# Unpremultiply, grade, then re-premultiply
vfx premult input.exr -o unp.exr --unpremultiply
vfx grade unp.exr -o graded.exr --saturation 1.2
vfx premult graded.exr -o final.exr --premultiply
```

### Multi-Layer EXR

```bash
# Fix premultiplication on beauty layer only
vfx premult render.exr -o fixed.exr --layer beauty --unpremultiply
```

## Notes

- Requires 4-channel (RGBA) images
- Unpremultiply protects against division by zero (alpha < 1e-6)
- Many color operations should be done on unpremultiplied data

## See Also

- [clamp](./clamp.md) - Value clamping
- [composite](./composite.md) - Image compositing
