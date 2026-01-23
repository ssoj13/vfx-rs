# view - Image Viewer

Interactive image viewer with OCIO color management.

## Usage

```bash
vfx view [OPTIONS] [INPUT]
```

**Note:** Input is optional. If omitted, opens with no file (use File > Open or drag & drop).

## Options

| Option | Description |
|--------|-------------|
| `--display <NAME>` | OCIO display (sRGB, Rec.709, etc.) |
| `--view <NAME>` | OCIO view transform |
| `--colorspace <CS>` | Input colorspace |
| `--ocio <PATH>` | Custom OCIO config path |

## Features

- **HDR support** - Full float range display
- **Zoom/pan** - Mouse wheel (zoom to cursor) and drag
- **OCIO integration** - Built-in ACES 1.3 config
- **GPU accelerated** - wgpu-based rendering
- **Drag & drop** - Drop files to open
- **Multi-layer EXR** - Layer and channel selection

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `O` | Open file dialog |
| `F` | Fit to window |
| `H` / `0` | Home (1:1 zoom, centered) |
| `+` / `=` | Zoom in |
| `-` | Zoom out |
| `C` | Color (RGB) |
| `R` | Red channel |
| `G` | Green channel |
| `B` | Blue channel |
| `A` | Alpha channel |
| `L` | Luminance |
| `Esc` | Exit |

## Mouse Controls

| Action | Description |
|--------|-------------|
| Scroll | Zoom at cursor position |
| Drag | Pan image |
| Double-click | Fit to window |

## UI Controls

The viewer has labeled controls with detailed tooltips:

| Control | Label | Description |
|---------|-------|-------------|
| Source colorspace | **Src** | Input image colorspace (ACEScg, sRGB, etc.) |
| View transform | **RRT** | Reference Rendering Transform (tone mapping) |
| Display output | **ODT** | Output Device Transform (sRGB, Rec.709, etc.) |
| Exposure | **EV** | Exposure in stops (Ctrl+click to reset) |
| Channel | **Ch** | Channel display mode |
| Layer | **Layer** | EXR layer selection (if multi-layer) |

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

# Custom OCIO config
vfx view image.exr --ocio /path/to/config.ocio
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
