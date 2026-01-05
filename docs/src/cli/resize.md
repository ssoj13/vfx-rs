# resize - Image Scaling

Scale images using high-quality resampling filters.

## Usage

```bash
vfx resize [OPTIONS] <INPUT> -o <OUTPUT>
```

## Options

| Option | Description |
|--------|-------------|
| `-w, --width <W>` | Target width |
| `-h, --height <H>` | Target height |
| `-s, --scale <S>` | Scale factor (0.5 = half) |
| `-f, --filter <F>` | Filter: nearest, bilinear, bicubic, lanczos |
| `--layer <NAME>` | Process specific EXR layer |

## Examples

```bash
# Resize to specific dimensions
vfx resize input.exr -w 1920 -h 1080 -o output.exr

# Scale by factor
vfx resize input.exr -s 0.5 -o half.exr

# Width only (preserve aspect ratio)
vfx resize input.exr -w 1920 -o output.exr

# High-quality downscale
vfx resize input.exr -s 0.25 -f lanczos -o proxy.exr

# Fast preview
vfx resize input.exr -s 0.1 -f nearest -o thumb.exr
```

## Filters

| Filter | Quality | Speed | Best For |
|--------|---------|-------|----------|
| `nearest` | Low | Fastest | Pixel art, IDs |
| `bilinear` | Medium | Fast | Previews |
| `bicubic` | High | Medium | General use |
| `lanczos` | Highest | Slow | Final output, downscaling |

## GPU Acceleration

Resize automatically uses GPU when available via wgpu. Falls back to CPU if:
- No GPU detected
- Image too large for GPU memory
- wgpu initialization fails

Check with verbose mode:
```bash
vfx resize -vv input.exr -s 0.5 -o out.exr
# DEBUG: Attempting GPU resize
# DEBUG: Using GPU backend: Vulkan
```
