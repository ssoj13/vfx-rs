# view - Image Viewer

Interactive image viewer with OCIO color management.

## Usage

```bash
vfx view [OPTIONS] <INPUT>
```

## Options

| Option | Description |
|--------|-------------|
| `--display <NAME>` | OCIO display (sRGB, Rec.709, etc.) |
| `--view <NAME>` | OCIO view transform |
| `--colorspace <CS>` | Input colorspace |

## Features

- **HDR support** - Full float range display
- **Zoom/pan** - Mouse wheel and drag
- **OCIO integration** - Built-in ACES 1.3 config
- **GPU accelerated** - wgpu-based rendering
- **Drag & drop** - Drop files to open

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `1` | Fit to window |
| `0` | 1:1 pixel view |
| `R` | Reset view |
| `G` | Toggle RGB/Alpha |
| `Esc` | Exit |

## Examples

```bash
# Open with default sRGB display
vfx view render.exr

# Specify display and view
vfx view render.exr --display "sRGB" --view "ACES 1.0 - SDR Video"

# View in Rec.709
vfx view broadcast.exr --display "Rec.709"

# Raw view (no color transform)
vfx view data.exr --view "Raw"
```

## Built-in Displays

The viewer includes an ACES 1.3 configuration:

| Display | Views |
|---------|-------|
| sRGB | ACES 1.0 - SDR Video, Un-tone-mapped, Log, Raw |
| Rec.709 | ACES 1.0 - SDR Video, Un-tone-mapped, Log, Raw |
| Display P3 | ACES 1.0 - SDR Video, Un-tone-mapped, Log, Raw |
| Rec.2020 | ACES 1.0 - SDR Video, Un-tone-mapped, Log, Raw |

## Python API

```python
import vfx

# Open viewer from Python
vfx.view("render.exr")
```
