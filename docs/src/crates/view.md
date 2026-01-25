# vfx-view

GPU-accelerated image viewer with OCIO color management.

## Purpose

Interactive image viewer designed for VFX workflows. Displays images with proper color management, multi-layer EXR support, and essential review tools.

## Features

- Full OCIO display pipeline
- Multi-layer EXR with layer selection
- Channel isolation (R/G/B/A/Luminance)
- Exposure control with Ctrl+click reset
- Pan/zoom navigation (zoom to cursor)
- Keyboard shortcuts
- Persistent settings

## Quick Start

```rust
use vfx_view::{run, ViewerConfig};
use std::path::PathBuf;

let config = ViewerConfig::default();
let exit_code = run(PathBuf::from("image.exr"), config);
```

## Configuration

```rust
use vfx_view::ViewerConfig;

let config = ViewerConfig {
    ocio: Some("/path/to/config.ocio".into()),
    display: Some("sRGB".into()),
    view: Some("ACES 1.0 SDR".into()),
    colorspace: Some("ACEScg".into()),
    verbose: 1,
};
```

## OCIO Configuration

The viewer resolves OCIO config in this order:

1. `ViewerConfig::ocio` / `--ocio` CLI flag
2. `$OCIO` environment variable
3. Built-in ACES 1.3 config

### Display Pipeline

```
Input Image → Source CS → RRT (View Transform) → ODT (Display) → Screen
                 ↓
            (from metadata or --colorspace)
```

## Keyboard Shortcuts

### Navigation

| Key | Action |
|-----|--------|
| `F` | Fit image to window |
| `H` / `0` | Home (1:1 zoom, centered) |
| `+` / `=` | Zoom in |
| `-` | Zoom out |
| Mouse drag | Pan |
| Scroll | Zoom at cursor |

### Channels

| Key | Action |
|-----|--------|
| `C` | Color (RGB) |
| `R` | Red channel only |
| `G` | Green channel only |
| `B` | Blue channel only |
| `A` | Alpha channel |
| `L` | Luminance |

### File Operations

| Key | Action |
|-----|--------|
| `O` | Open file dialog |
| Double-click | Open file (no image) / Fit (with image) |
| Drag & drop | Open dropped file |

### General

| Key | Action |
|-----|--------|
| `Esc` | Exit |
| Ctrl+click on EV slider | Reset exposure to 0 |

## UI Labels

Controls are labeled with short names and detailed tooltips:

| Label | Full Name | Description |
|-------|-----------|-------------|
| **Src** | Source Colorspace | Input image colorspace |
| **RRT** | Reference Rendering Transform | View/tone-mapping transform |
| **ODT** | Output Device Transform | Display/monitor type |
| **EV** | Exposure Value | Brightness adjustment in stops |
| **Ch** | Channel | Display channel mode |
| **Layer** | Layer | EXR layer selection |

## Channel Modes

```rust
use vfx_view::ChannelMode;

ChannelMode::Color      // Full RGB
ChannelMode::Red        // Red as grayscale
ChannelMode::Green      // Green as grayscale
ChannelMode::Blue       // Blue as grayscale
ChannelMode::Alpha      // Alpha as grayscale
ChannelMode::Luminance  // Rec.709 luminance
```

## Layer Selection

For multi-layer EXR files, use the UI dropdown to select layers interactively.

**Note:** ViewerConfig does not include a `layer` field; layer selection is done through the UI at runtime.

## Persistence

Viewer saves state between sessions:

```rust
use vfx_view::ViewerPersistence;

// Stored in:
// Windows: %APPDATA%/vfx-rs/viewer/
// Linux: ~/.config/vfx-rs/viewer/
// macOS: ~/Library/Application Support/vfx-rs/viewer/

// Persisted settings:
// - Last opened file
// - Window size/position
// - Last display/view selection
// - Exposure
```

## Architecture

```
ViewerApp (eframe App)
    │
    ├── ViewerState (runtime parameters)
    │       zoom, pan, exposure, channel_mode...
    │
    ├── tx/rx channels (mpsc)
    │       ViewerMsg → Worker
    │       ViewerEvent ← Worker
    │
    └── Worker Thread (ViewerHandler)
            │
            ├── Image loading (vfx-io)
            ├── OCIO processing (vfx-ocio)
            └── Texture generation
```

## GPU Rendering

Uses `egui` with `wgpu` backend:

- Image displayed as GPU texture
- OCIO transforms baked to 3D LUT
- Display shader applies LUT + exposure
- Conditional repaint (saves CPU when idle)

## Supported Formats

All formats supported by `vfx-io`:
- EXR (including multi-layer)
- PNG, JPEG, TIFF
- DPX, HDR
- WebP, HEIF (if features enabled)

## Error Handling

```rust
let exit_code = run(path, config);
match exit_code {
    0 => println!("Viewer closed normally"),
    1 => println!("Error occurred"),
    _ => {}
}
```

## Dependencies

- `vfx-core`, `vfx-io` - Image loading
- `vfx-ocio` - Color management
- `egui` / `eframe` - GUI framework
- `wgpu` - GPU rendering
- `dirs` - Config paths
- `rfd` - File dialogs

## Feature Flag

The viewer is optional in `vfx-cli`:

```toml
[dependencies]
vfx-cli = { version = "0.1", features = ["viewer"] }  # default

# Without viewer (smaller binary)
vfx-cli = { version = "0.1", default-features = false }
```
