# vfx-cli

Unified VFX image processing CLI.

## Purpose

Command-line tool for image processing, designed as a Rust alternative to `oiiotool`. Provides a single `vfx` binary with subcommands for common operations.

## Installation

```bash
cargo install vfx-cli
```

Or build from source:

```bash
cargo build --release -p vfx-cli
```

Binary is at `target/release/vfx` (or `vfx.exe` on Windows).

## Commands Overview

| Command | Description | OIIO Equivalent |
|---------|-------------|-----------------|
| `info` | Display image metadata | `iinfo` |
| `convert` | Convert between formats | `iconvert` |
| `resize` | Scale images | `oiiotool --resize` |
| `color` | Color adjustments | `oiiotool --exposure` |
| `aces` | ACES color transforms | - |
| `composite` | Layer compositing | `oiiotool --over` |
| `layers` | List EXR layers | `oiiotool --ch` |
| `extract-layer` | Extract single EXR layer | - |
| `merge-layers` | Merge EXR layers | - |
| `lut` | Apply LUT files | `oiiotool --ociolook` |
| `batch` | Parallel processing | - |
| `view` | Image viewer | `iv` |

## Global Options

```bash
vfx [OPTIONS] <COMMAND>

Options:
  -v, --verbose...       Increase verbosity (-v, -vv, -vvv)
  -l, --log [PATH]       Write logs to file (default: vfx.log)
  -j, --threads <NUM>    Number of threads (0 = auto)
  --allow-non-color      Allow processing non-color data (IDs, normals)
  -h, --help             Print help
  -V, --version          Print version
```

### Verbosity Levels

| Flag | Level | Output |
|------|-------|--------|
| (none) | ERROR | Errors only |
| `-v` | INFO | Operations performed |
| `-vv` | DEBUG | Detailed progress |
| `-vvv` | TRACE | Everything |

## info

Display image information:

```bash
# Basic info
vfx info image.exr

# Show detailed stats
vfx info image.exr --stats

# Show all metadata
vfx info image.exr --all

# JSON output
vfx info image.exr --json
```

Output:
```
image.exr: 1920x1080, 4 channels (RGBA), f16
  Compression: ZIP
  Color space: ACEScg
```

## convert

Convert between formats:

```bash
# Basic conversion
vfx convert input.exr output.png

# Specify bit depth
vfx convert input.exr output.png --depth 16

# EXR compression
vfx convert input.png output.exr --compression piz
```

## resize

Scale images:

```bash
# By dimensions
vfx resize input.exr -o output.exr --width 1920 --height 1080

# By single dimension (preserve aspect)
vfx resize input.exr -o output.exr --width 1920

# By scale factor
vfx resize input.exr -o output.exr --scale 0.5

# With filter
vfx resize input.exr -o output.exr --width 1920 --filter lanczos
```

Filters: `nearest`, `bilinear`, `bicubic`, `lanczos`

## color

Color adjustments:

```bash
# Exposure (stops)
vfx color input.exr -o output.exr --exposure 1.0

# Gamma
vfx color input.exr -o output.exr --gamma 2.2

# Saturation
vfx color input.exr -o output.exr --saturation 1.2

# Transfer function
vfx color input.exr -o output.exr --transfer srgb

# Combined
vfx color input.exr -o output.exr --exposure 0.5 --gamma 1.1 --saturation 1.1
```

## aces

ACES color transforms:

```bash
# IDT: sRGB -> ACEScg
vfx aces input.jpg -o output.exr --transform idt

# RRT only (tonemap)
vfx aces input.exr -o output.exr --transform rrt

# ODT: ACEScg -> sRGB
vfx aces input.exr -o output.png --transform odt

# Full RRT+ODT (most common)
vfx aces input.exr -o output.png --transform rrt-odt
```

## composite

Layer compositing:

```bash
# A over B
vfx composite fg.exr bg.exr -o output.exr --mode over

# Blend modes
vfx composite a.exr b.exr -o output.exr --mode multiply
vfx composite a.exr b.exr -o output.exr --mode screen
vfx composite a.exr b.exr -o output.exr --mode add
```

## layers

List EXR layers:

```bash
# List all layers in EXR file(s)
vfx layers render.exr

# JSON output
vfx layers render.exr --json

# Multiple files
vfx layers *.exr
```

## extract-layer

Extract a single layer from EXR:

```bash
# Extract by name
vfx extract-layer render.exr -o beauty.exr --layer beauty

# Extract by index
vfx extract-layer render.exr -o layer0.exr --layer 0
```

## merge-layers

Merge EXR layers:

```bash
# Merge two EXR files into one with multiple layers
vfx merge-layers beauty.exr diffuse.exr -o combined.exr
```

## lut

Apply LUT files:

```bash
# Apply 3D LUT
vfx lut input.exr -o output.exr --lut grade.cube

# Apply 1D LUT
vfx lut input.exr -o output.exr --lut gamma.cube

# Invert LUT
vfx lut input.exr -o output.exr --lut grade.cube --inverse
```

## batch

Parallel processing:

```bash
# Process all EXR files with an operation
vfx batch --input "*.exr" --output-dir ./converted --op convert --format png

# With operation arguments
vfx batch --input "render.*.exr" --output-dir ./graded --op color --args "exposure=0.5"
```

**Note:** The batch command uses `--input` (glob pattern), `--output-dir` (directory), `--op` (operation name), `--args` (key=value arguments), and `--format` (output extension).

## view

Interactive viewer (requires `viewer` feature):

```bash
# View image
vfx view image.exr

# With OCIO config
vfx view image.exr --ocio /path/to/config.ocio

# Specific display/view
vfx view image.exr --display sRGB --view "ACES 1.0 SDR"

# Override input colorspace
vfx view image.exr --colorspace "ACEScg"
```

## Logging

```bash
# Log to default file (vfx.log)
vfx -l convert input.exr output.png

# Log to custom file
vfx -l process.log convert input.exr output.png

# Verbose + logging
vfx -vv -l convert input.exr output.png

# Environment variable logging
RUST_LOG=vfx_cli=debug vfx convert input.exr output.png
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |

## Dependencies

- `vfx-core`, `vfx-io`, `vfx-ops`, `vfx-color`, `vfx-lut`
- `vfx-view` (optional, via `viewer` feature)
- `clap` - Argument parsing
- `glob` - File patterns
- `rayon` - Parallel batch
- `tracing` - Logging
