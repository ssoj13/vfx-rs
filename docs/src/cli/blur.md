# blur - Blur Filter

Apply blur effect to images.

## Synopsis

```bash
vfx blur <INPUT> -o <OUTPUT> [-r <RADIUS>] [-t <TYPE>]
```

## Options

| Option | Description |
|--------|-------------|
| `-o, --output` | Output file path |
| `-r, --radius` | Blur radius in pixels (default: 3) |
| `-t, --blur-type` | Blur algorithm: `box`, `gaussian` (default: gaussian) |
| `--layer` | Process only this layer (for multi-layer EXR) |

## Blur Types

### Gaussian Blur

Smooth, natural-looking blur with bell curve falloff.

```bash
vfx blur input.exr -o soft.exr -r 5 -t gaussian
```

### Box Blur

Simple averaging, faster but less smooth.

```bash
vfx blur input.exr -o box_blur.exr -r 5 -t box
```

## Examples

### Soft Focus Effect

```bash
# Light blur for soft focus
vfx blur portrait.exr -o soft_portrait.exr -r 2
```

### Background Defocus

```bash
# Heavy blur for background plate
vfx blur background.exr -o bg_defocused.exr -r 20 -t gaussian
```

### Multi-pass Blur

```bash
# Multiple light passes for smoother result
vfx blur input.exr -o pass1.exr -r 3
vfx blur pass1.exr -o pass2.exr -r 3
vfx blur pass2.exr -o final.exr -r 3
```

### Blur Specific Layer

```bash
# Blur only the diffuse pass
vfx blur render.exr -o render_soft.exr -r 5 --layer diffuse
```

## Non-Color Channels

By default, blur only processes color channels. For ID/mask channels:

```bash
# Force processing of non-color data
vfx --allow-non-color blur id_pass.exr -o id_smooth.exr -r 2
```

## Performance

| Radius | Approx. Time (1080p) |
|--------|---------------------|
| 1-5 | < 100ms |
| 10-20 | 100-500ms |
| 50+ | 1-5s |

## Technical Notes

- **Alpha channel is preserved** (only RGB is blurred)
- Uses 2D convolution (not separable, O(n²) per pixel)
- Edge pixels are clamped (no wrap)
- Output is always float32
- Single-threaded processing
- Works with RGBA (4ch) and grayscale+alpha (2ch) images

## See Also

- [sharpen](./sharpen.md) - Increase sharpness
- [color](./color.md) - Color adjustments
