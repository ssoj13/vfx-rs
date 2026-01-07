# maketx - Texture Creation

Create tiled, mipmapped textures for rendering (like OIIO's maketx).

**Alias:** `tx`

## Synopsis

```bash
vfx maketx <INPUT> -o <OUTPUT> [-m] [-t <TILE>] [-f <FILTER>] [-w <WRAP>]
```

## Options

| Option | Description |
|--------|-------------|
| `-o, --output` | Output texture file (.tx, .exr) |
| `-m, --mipmap` | Generate mipmaps |
| `-t, --tile` | Tile size in pixels (default: 64) |
| `-f, --filter` | Mipmap filter: `box`, `bilinear`, `lanczos`, `mitchell` (default: lanczos) |
| `-w, --wrap` | Wrap mode: `black`, `clamp`, `periodic` (default: black) |

## Examples

### Basic Texture

```bash
# Create tiled texture
vfx maketx source.exr -o texture.tx -t 64
```

### With Mipmaps

```bash
# Create tiled, mipmapped texture
vfx maketx source.exr -o texture.tx -m -t 64
```

### Periodic Texture

```bash
# Tileable texture with wrap mode
vfx maketx tile.exr -o tile.tx -m -t 64 -w periodic
```

### High Quality Mipmaps

```bash
# Lanczos filter for best quality
vfx maketx hdri.exr -o hdri.tx -m -t 64 -f lanczos
```

## Texture Format

The `.tx` format is tiled EXR optimized for rendering:

```
┌─────┬─────┬─────┬─────┐
│Tile │Tile │Tile │Tile │  Level 0 (full res)
│ 0   │ 1   │ 2   │ 3   │
├─────┼─────┼─────┼─────┤
│Tile │Tile │Tile │Tile │
│ 4   │ 5   │ 6   │ 7   │
└─────┴─────┴─────┴─────┘

┌─────┬─────┐
│  0  │  1  │  Level 1 (1/2 res)
├─────┼─────┤
│  2  │  3  │
└─────┴─────┘

┌─────┐
│  0  │  Level 2 (1/4 res)
└─────┘
  ...
```

## Mipmap Filters

| Filter | Quality | Speed | Use Case |
|--------|---------|-------|----------|
| `box` | Low | Fast | Quick preview |
| `bilinear` | Medium | Fast | General use |
| `lanczos` | High | Slow | Final textures |
| `mitchell` | High | Medium | Balanced |

## Wrap Modes

| Mode | Behavior |
|------|----------|
| `black` | Black outside bounds |
| `clamp` | Repeat edge pixels |
| `periodic` | Tile infinitely |

## Use Cases

### Rendering Textures

```bash
# Prepare textures for Arnold/RenderMan
for f in textures/*.exr; do
    vfx maketx "$f" -o "${f%.exr}.tx" -m -t 64
done
```

### HDRI Environment

```bash
# Create environment texture
vfx maketx hdri_spherical.exr -o hdri.tx -m -t 64 -w clamp
```

### Tileable Textures

```bash
# Seamless texture with proper wrapping
vfx maketx seamless_wood.exr -o wood.tx -m -t 64 -w periodic
```

## Performance Tips

1. **Tile size 64** - Best for most renderers
2. **PIZ compression** - Good for most textures
3. **Half-float** - Sufficient for most textures
4. **Pre-convert** - Convert from other formats first

```bash
# Full preparation pipeline
vfx convert source.tif -o temp.exr -d half -c piz
vfx maketx temp.exr -o final.tx -m -t 64
rm temp.exr
```

## Technical Notes

- Output is always tiled EXR
- Mipmaps go down to 1x1
- Preserves color space metadata
- Parallel mipmap generation

## See Also

- [convert](./convert.md) - Format conversion
- [resize](./resize.md) - Image scaling
