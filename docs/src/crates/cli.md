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
| `layers` | EXR layer operations | `oiiotool --ch` |
| `lut` | Apply LUT files | `oiiotool --ociolook` |
| `batch` | Parallel processing | - |
| `view` | Image viewer | `iv` |

## Global Options

```bash
vfx [OPTIONS] <COMMAND>

Options:
  -v, --verbose    Increase verbosity (-v, -vv, -vvv)
  -q, --quiet      Suppress output
  --log <FILE>     Write logs to file
  -h, --help       Print help
  -V, --version    Print version
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

# JSON output
vfx info image.exr --json

# Show layers
vfx info render.exr --layers
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
vfx resize input.exr output.exr --width 1920 --height 1080

# By single dimension (preserve aspect)
vfx resize input.exr output.exr --width 1920

# By scale factor
vfx resize input.exr output.exr --scale 0.5

# With filter
vfx resize input.exr output.exr --width 1920 --filter lanczos
```

Filters: `nearest`, `bilinear`, `bicubic`, `lanczos`

## color

Color adjustments:

```bash
# Exposure (stops)
vfx color input.exr output.exr --exposure 1.0

# Gamma
vfx color input.exr output.exr --gamma 2.2

# Saturation
vfx color input.exr output.exr --saturation 1.2

# Transfer function
vfx color input.exr output.exr --transfer srgb

# Combined
vfx color input.exr output.exr --exposure 0.5 --gamma 1.1 --saturation 1.1
```

## aces

ACES color transforms:

```bash
# IDT: sRGB → ACEScg
vfx aces input.jpg output.exr --transform idt

# RRT only (tonemap)
vfx aces input.exr output.exr --transform rrt

# ODT: ACEScg → sRGB
vfx aces input.exr output.png --transform odt

# Full RRT+ODT (most common)
vfx aces input.exr output.png --transform rrt-odt
```

## composite

Layer compositing:

```bash
# A over B
vfx composite fg.exr bg.exr output.exr --mode over

# Blend modes
vfx composite a.exr b.exr output.exr --mode multiply
vfx composite a.exr b.exr output.exr --mode screen
vfx composite a.exr b.exr output.exr --mode add
```

## layers

EXR layer operations:

```bash
# List layers
vfx layers input.exr --list

# Extract layer
vfx layers input.exr output.exr --extract beauty

# Merge layers
vfx layers base.exr overlay.exr output.exr --merge

# Rename layer
vfx layers input.exr output.exr --rename "diffuse:diff"
```

## lut

Apply LUT files:

```bash
# Apply 3D LUT
vfx lut input.exr output.exr --lut grade.cube

# Apply 1D LUT
vfx lut input.exr output.exr --lut gamma.cube

# Invert LUT
vfx lut input.exr output.exr --lut grade.cube --inverse
```

## batch

Parallel processing:

```bash
# Process all EXR files
vfx batch "*.exr" --output "./converted/{name}.png" --op convert

# With operation
vfx batch "render.*.exr" --output "./graded/{name}.exr" \
    --op "color --exposure 0.5"

# Parallel jobs
vfx batch "*.exr" --output "./{name}.png" --jobs 8
```

Pattern variables:
- `{name}` - Filename without extension
- `{ext}` - Original extension
- `{dir}` - Original directory
- `{frame}` - Frame number (for sequences)

## view

Interactive viewer:

```bash
# View image
vfx view image.exr

# With OCIO
vfx view image.exr --ocio /path/to/config.ocio

# Specific display/view
vfx view image.exr --display sRGB --view "ACES 1.0 SDR"

# Specific layer
vfx view render.exr --layer diffuse
```

## Layer Flag

Many commands support `--layer` for EXR:

```bash
# Process specific layer
vfx color render.exr output.exr --layer diffuse --exposure 1.0

# Resize specific layer
vfx resize render.exr output.exr --layer beauty --width 1920
```

## Logging

```bash
# Log to file
vfx -vv convert input.exr output.png --log process.log

# Structured logging (with tracing)
RUST_LOG=vfx_cli=debug vfx convert input.exr output.png
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |

## Shell Completion

Generate completions:

```bash
# Bash
vfx --generate-completion bash > /etc/bash_completion.d/vfx

# Zsh
vfx --generate-completion zsh > ~/.zsh/completions/_vfx

# Fish
vfx --generate-completion fish > ~/.config/fish/completions/vfx.fish

# PowerShell
vfx --generate-completion powershell > vfx.ps1
```

## Dependencies

- `vfx-core`, `vfx-io`, `vfx-ops`, `vfx-color`, `vfx-lut`
- `vfx-view` (optional)
- `clap` - Argument parsing
- `glob` - File patterns
- `rayon` - Parallel batch
- `tracing` - Logging
