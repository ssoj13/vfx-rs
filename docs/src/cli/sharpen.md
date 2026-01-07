# sharpen - Image Sharpening

Enhance image sharpness using unsharp masking.

## Synopsis

```bash
vfx sharpen <INPUT> -o <OUTPUT> [-a <AMOUNT>]
```

## Options

| Option | Description |
|--------|-------------|
| `-o, --output` | Output file path |
| `-a, --amount` | Sharpen amount (0.0-10.0, default: 1.0) |
| `--layer` | Process only this layer (for multi-layer EXR) |

## Amount Guide

| Amount | Effect |
|--------|--------|
| 0.0-0.5 | Subtle sharpening |
| 0.5-1.5 | Normal sharpening |
| 1.5-3.0 | Strong sharpening |
| 3.0-10.0 | Extreme (may cause halos) |

## Examples

### Basic Sharpening

```bash
# Standard sharpen
vfx sharpen input.exr -o sharp.exr -a 1.0
```

### Subtle Enhancement

```bash
# Light sharpen for web images
vfx sharpen photo.jpg -o photo_web.jpg -a 0.3
```

### Strong Sharpening

```bash
# Heavy sharpen for textures
vfx sharpen texture.exr -o texture_sharp.exr -a 2.5
```

### Output Sharpening

```bash
# Final sharpen after resize
vfx resize input.exr -w 1920 -h 1080 -o resized.exr
vfx sharpen resized.exr -o final.exr -a 0.5
```

## Algorithm

Unsharp masking formula:

```
sharpened = original + amount * (original - blurred)
```

The `amount` parameter controls how much of the edge enhancement is added.

## Non-Color Channels

By default, sharpen only processes color channels:

```bash
# Force processing of non-color data
vfx --allow-non-color sharpen mask.exr -o mask_sharp.exr -a 1.0
```

## Best Practices

1. **Sharpen last** - Apply after all other processing
2. **Use subtly** - Over-sharpening causes halos
3. **Check at 100%** - View at actual pixels
4. **Match output** - Different amounts for web vs print

## Comparison with Filters

| Tool | Use Case |
|------|----------|
| `sharpen` | General sharpening |
| ImageBufAlgo::laplacian | Edge detection |
| ImageBufAlgo::unsharp_mask | Fine control over radius |

## See Also

- [blur](./blur.md) - Blur images
- [resize](./resize.md) - Scale with filter
