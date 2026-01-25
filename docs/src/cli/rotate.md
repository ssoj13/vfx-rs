# rotate - Arbitrary Rotation

Rotate image by any angle (not just 90° increments).

## Synopsis

```bash
vfx rotate <INPUT> -o <OUTPUT> -a <ANGLE> [--bg-color <COLOR>]
```

## Options

| Option | Description |
|--------|-------------|
| `-o, --output` | Output file path |
| `-a, --angle` | Rotation angle in degrees (counter-clockwise) |
| `--bg-color` | Background color for exposed areas (R,G,B or R,G,B,A) |

## Examples

### Basic Rotation

```bash
# Rotate 45 degrees counter-clockwise
vfx rotate input.exr -o rotated.exr -a 45
```

### Clockwise Rotation

```bash
# Rotate 30 degrees clockwise (use negative)
vfx rotate input.exr -o rotated.exr -a -30
```

### Small Adjustment

```bash
# Straighten slightly tilted image
vfx rotate input.exr -o straight.exr -a 2.5
```

### Custom Background

```bash
# Rotate with white background
vfx rotate input.exr -o rotated.exr -a 45 --bg-color 1,1,1

# Rotate with transparent background
vfx rotate input.exr -o rotated.exr -a 45 --bg-color 0,0,0,0
```

## Rotation Visualization

```
Original:                  Rotated 45°:
┌─────────────┐                  ◢◣
│             │                ◢   ◣
│             │              ◢       ◣
│             │            ◢    ⬜     ◣
│             │              ◣       ◢
└─────────────┘                ◣   ◢
                                 ◥◤
```

## Notes on Output Size

The output image may be larger than the input to accommodate the rotated content:

```
For a W×H image rotated by angle θ:
new_width  = W * |cos(θ)| + H * |sin(θ)|
new_height = W * |sin(θ)| + H * |cos(θ)|
```

Example: 1920×1080 rotated 45° becomes ~2121×2121

## Use Cases

### Horizon Correction

```bash
# Fix tilted camera shot
vfx rotate tilted.exr -o fixed.exr -a 1.5
vfx crop fixed.exr -x 20 -y 20 -w 1880 -H 1040 -o final.exr
```

### Creative Effect

```bash
# Dutch angle for dramatic effect
vfx rotate scene.exr -o dutch.exr -a 15
```

### Animation Frame

```bash
# Rotate element for animation
for i in $(seq 0 10 360); do
    vfx rotate element.exr -o "frame_${i}.exr" -a $i --bg-color 0,0,0,0
done
```

## Performance

Arbitrary rotation requires resampling every pixel:
- Uses bilinear interpolation
- Sequential processing (single-threaded)
- Larger angles create larger outputs

For 90° increments, use [transform](./transform.md) (lossless, faster).

## Comparison: rotate vs transform

| Feature | rotate | transform |
|---------|--------|-----------|
| Arbitrary angles | Yes | No (90° only) |
| Resampling | Yes | No |
| Quality loss | Minimal | None |
| Speed | Slower | Faster |
| Output size | May change | Same |

## See Also

- [transform](./transform.md) - Lossless 90° rotation
- [warp](./warp.md) - Distortion effects
