# color - Color Transforms

Apply exposure, gamma, saturation, and transfer function adjustments.

## Usage

```bash
vfx color [OPTIONS] <INPUT> -o <OUTPUT>
```

## Options

| Option | Description |
|--------|-------------|
| `-e, --exposure <STOPS>` | Exposure adjustment in stops |
| `-g, --gamma <VALUE>` | Gamma correction |
| `-s, --saturation <VALUE>` | Saturation (0=mono, 1=normal, 2=boost) |
| `-t, --transfer <TF>` | Apply transfer function |
| `--layer <NAME>` | Process specific EXR layer |

## Transfer Functions

| Name | Description |
|------|-------------|
| `srgb` | sRGB EOTF (linear → display) |
| `srgb-inv` | Inverse sRGB (display → linear) |
| `rec709` | Rec.709 gamma |
| `pq` | PQ/ST.2084 (HDR) |
| `pq-inv` | Inverse PQ |
| `hlg` | Hybrid Log-Gamma |
| `hlg-inv` | Inverse HLG |
| `log` | Cineon log |
| `log-inv` | Inverse log |

## Examples

```bash
# Exposure adjustment
vfx color input.exr -o brighter.exr -e 1.5

# Gamma correction
vfx color input.exr -o corrected.exr -g 2.2

# Desaturate
vfx color input.exr -o mono.exr -s 0

# Linear to sRGB display
vfx color linear.exr -o display.png -t srgb

# sRGB to linear
vfx color photo.jpg -o linear.exr -t srgb-inv

# HDR PQ encode
vfx color hdr.exr -o hdr10.exr -t pq

# Combine adjustments
vfx color input.exr -o graded.exr -e 0.5 -s 1.2 -g 1.1
```

## Pipeline Example

```bash
# Full grade: exposure → saturation → gamma → sRGB
vfx color render.exr -o temp.exr -e 0.5 -s 1.1
vfx color temp.exr -o final.png -g 1.05 -t srgb
```
