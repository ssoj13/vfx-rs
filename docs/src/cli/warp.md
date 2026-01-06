# warp - Distortion Effects

Apply lens distortion and creative warp effects.

## Synopsis

```bash
vfx warp <INPUT> -o <OUTPUT> -t <TYPE> [-k <K1>] [--k2 <K2>] [-r <RADIUS>]
```

## Options

| Option | Description |
|--------|-------------|
| `-o, --output` | Output file path |
| `-t, --type` | Warp type |
| `-k, --k1` | Primary parameter (default: 0.2) |
| `--k2` | Secondary parameter (default: 0.0) |
| `-r, --radius` | Effect radius (default: 0.5) |

## Warp Types

### Lens Distortion

| Type | Effect |
|------|--------|
| `barrel` | Barrel distortion (wide-angle lens effect) |
| `pincushion` | Pincushion distortion (telephoto lens effect) |
| `fisheye` | Extreme barrel distortion |

### Creative Effects

| Type | Effect |
|------|--------|
| `twist` | Spiral/twirl effect |
| `wave` | Sinusoidal wave |
| `spherize` | Spherical bulge |
| `ripple` | Concentric ripples |

## Examples

### Barrel Distortion

```bash
# Add barrel distortion
vfx warp input.exr -o barrel.exr -t barrel -k 0.3
```

### Remove Lens Distortion

```bash
# Correct barrel distortion (use pincushion with same k)
vfx warp distorted.exr -o corrected.exr -t pincushion -k 0.2
```

### Fisheye Effect

```bash
# Strong fisheye look
vfx warp input.exr -o fisheye.exr -t fisheye -k 0.5
```

### Twist Effect

```bash
# Create spiral distortion
vfx warp input.exr -o twisted.exr -t twist -k 0.3 -r 0.8
```

### Wave Effect

```bash
# Horizontal wave distortion
vfx warp input.exr -o wavy.exr -t wave -k 10 --k2 0.1
# k1 = frequency, k2 = amplitude
```

### Spherize

```bash
# Spherical bulge at center
vfx warp input.exr -o bulge.exr -t spherize -k 0.5 -r 0.6
```

### Ripple Effect

```bash
# Concentric ripples
vfx warp input.exr -o ripple.exr -t ripple -k 20 --k2 0.05
# k1 = frequency, k2 = amplitude
```

## Distortion Formulas

### Barrel/Pincushion

```
r' = r * (1 + k1*r² + k2*r⁴)
```

Where r is distance from center (normalized).

- **k1 > 0**: Barrel distortion
- **k1 < 0**: Pincushion distortion

### Fisheye

```
r' = 2 * arctan(r * k) / (2 * arctan(k))
```

### Twist

```
angle = k * (1 - r/radius)²
x' = x*cos(angle) - y*sin(angle)
y' = x*sin(angle) + y*cos(angle)
```

### Wave

```
y' = y + amplitude * sin(2π * frequency * x)
```

## Use Cases

### Lens Correction

```bash
# Correct GoPro barrel distortion
vfx warp gopro.exr -o corrected.exr -t pincushion -k 0.35 --k2 0.1
```

### VFX Match

```bash
# Match lens distortion of plate
vfx warp cg_render.exr -o matched.exr -t barrel -k 0.15
```

### Title Effects

```bash
# Animated distortion for titles
for i in $(seq 0 0.02 0.5); do
    vfx warp title.exr -o "frame_${i}.exr" -t spherize -k $i
done
```

### Transition Effect

```bash
# Create ripple transition
vfx warp frame.exr -o distorted.exr -t ripple -k 30 --k2 0.1
```

## Technical Notes

- Uses bilinear interpolation
- Samples outside image return black/transparent
- Processes in parallel via rayon
- Preserves alpha channel

## See Also

- [rotate](./rotate.md) - Rotation
- [transform](./transform.md) - Basic transforms
- [resize](./resize.md) - Scaling
