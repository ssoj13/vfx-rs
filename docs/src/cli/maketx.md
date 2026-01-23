# maketx - Texture Preparation

Prepare images for texture use with tiled mipmapped EXR output.

**Alias:** `tx`

## Synopsis

```bash
vfx maketx <INPUT> -o <OUTPUT> [-m] [-f <FILTER>] [-t <TILE>]
```

## Options

| Option | Description |
|--------|-------------|
| `-o, --output` | Output file path |
| `-m, --mipmap` | Generate and embed mipmaps |
| `-f, --filter` | Mipmap filter: `box`, `bilinear`, `lanczos`, `mitchell` (default: lanczos) |
| `-t, --tile` | Tile size in pixels (default: 64) |
| `-w, --wrap` | Wrap mode hint (metadata only) |

## Features

### Mipmapped EXR Output

With `-m` flag and `.exr` output:
1. Loads input image
2. Generates mipmap chain using GPU backend
3. Writes tiled mipmapped EXR with all levels embedded
4. Uses ZIP16 compression for optimal quality/size

### GPU-Accelerated Generation

Mipmap generation uses the vfx-compute backend:
- Automatic GPU/CPU selection
- High-quality Lanczos filtering
- Parallel mip level generation

## Examples

### Create Mipmapped Texture

```bash
# Create production-ready texture with mipmaps
vfx maketx source.exr -o texture.exr -m -t 64

# Verbose output shows mip levels
vfx maketx source.exr -o texture.exr -m -v
```

Output:
```
Creating texture from source.exr
  Size: 2048x2048
  Tile size: 64
  Backend: wgpu
  Generating mipmaps...
    Level 1: 1024x1024
    Level 2: 512x512
    Level 3: 256x256
    ...
  Generated 12 mip levels
  Writing mipmapped tiled EXR...
Done.
```

### Different Filters

```bash
# Fast box filter
vfx maketx source.exr -o texture.exr -m -f box

# High quality lanczos
vfx maketx source.exr -o texture.exr -m -f lanczos

# Balanced mitchell
vfx maketx source.exr -o texture.exr -m -f mitchell
```

### Custom Tile Size

```bash
# Larger tiles (better compression, less random access)
vfx maketx source.exr -o texture.exr -m -t 128

# Smaller tiles (better random access)
vfx maketx source.exr -o texture.exr -m -t 32
```

## Mipmap Filters

| Filter | Quality | Speed | Use Case |
|--------|---------|-------|----------|
| `box` | Low | Fast | Quick preview |
| `bilinear` | Medium | Fast | General use |
| `mitchell` | High | Medium | Balanced |
| `lanczos` | Highest | Slower | Production |

## Output Format

### EXR with Mipmaps
- Tiled format with configurable tile size
- All mip levels embedded in single file
- ZIP16 compression
- Level mode: MipMap (inferred from data)

### Non-EXR Output
- Only base level saved (no mipmap embedding)
- Warning printed in verbose mode

## Technical Notes

- Uses vfx-exr for mipmapped EXR writing
- Mip sizes halve until 1x1
- RoundingMode::Down for size calculations
- Preserves all channels (RGB, RGBA, etc.)

## See Also

- [convert](./convert.md) - Format conversion
- [resize](./resize.md) - Image scaling
- [info](./info.md) - Inspect mipmaps
