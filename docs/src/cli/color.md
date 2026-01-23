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

### Display-Referred

| Name | Description |
|------|-------------|
| `srgb` | Decode sRGB to linear |
| `linear_to_srgb` | Encode linear to sRGB |
| `rec709` | Decode Rec.709 OETF to linear |
| `linear_to_rec709` | Encode linear to Rec.709 OETF |

### HDR

| Name | Description |
|------|-------------|
| `pq` | Decode PQ (ST.2084) to linear nits |
| `linear_to_pq` | Encode linear to PQ |
| `hlg` | Decode HLG (BT.2100) to linear |
| `linear_to_hlg` | Encode linear to HLG |

### Camera Log

| Name | Description |
|------|-------------|
| `logc` | Decode ARRI LogC3 to linear |
| `linear_to_logc` | Encode linear to ARRI LogC3 |
| `logc4` | Decode ARRI LogC4 (ALEXA 35) to linear |
| `linear_to_logc4` | Encode linear to ARRI LogC4 |
| `slog3` | Decode Sony S-Log3 to linear |
| `linear_to_slog3` | Encode linear to Sony S-Log3 |
| `vlog` | Decode Panasonic V-Log to linear |
| `linear_to_vlog` | Encode linear to Panasonic V-Log |

## Examples

```bash
# Exposure adjustment
vfx color input.exr -o brighter.exr -e 1.5

# Gamma correction
vfx color input.exr -o corrected.exr -g 2.2

# Desaturate
vfx color input.exr -o mono.exr -s 0

# sRGB to linear (decode display image to linear)
vfx color photo.jpg -o linear.exr -t srgb

# Linear to sRGB display (encode linear for display)
vfx color linear.exr -o display.png -t linear_to_srgb

# Rec.709 decode and encode
vfx color video.dpx -o linear.exr -t rec709
vfx color linear.exr -o output.dpx -t linear_to_rec709

# HDR: PQ decode/encode
vfx color hdr10.exr -o linear.exr -t pq
vfx color linear.exr -o hdr_out.exr -t linear_to_pq

# HLG decode
vfx color hlg_source.exr -o linear.exr -t hlg

# Camera Log: ARRI LogC3 to linear
vfx color alexa.exr -o linear.exr -t logc

# Sony S-Log3 to linear
vfx color sony.mxf -o linear.exr -t slog3

# Combine adjustments
vfx color input.exr -o graded.exr -e 0.5 -s 1.2 -g 1.1
```

## Pipeline Example

```bash
# Full grade: exposure → saturation → gamma → encode for display
vfx color render.exr -o temp.exr -e 0.5 -s 1.1
vfx color temp.exr -o final.png -g 1.05 -t linear_to_srgb
```
