# vfx-view

GPU-accelerated image viewer with OCIO color management.

## Purpose

Interactive image viewer designed for VFX workflows. Displays images with proper color management, multi-layer EXR support, and essential review tools.

## Features

- Full OCIO display pipeline
- Multi-layer EXR with layer selection
- Channel isolation (R/G/B/A/Luminance)
- Exposure control
- Pan/zoom navigation
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
    layer: Some("beauty".into()),
    exposure: 0.0,
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
Input Image → Color Space → Display Transform → View → Screen
                   ↓
              (from image metadata or config file rules)
```

## Keyboard Shortcuts

### Navigation

| Key | Action |
|-----|--------|
| `F` | Fit image to window |
| `H` | Home (1:1 zoom, centered) |
| `+` / `=` | Zoom in |
| `-` | Zoom out |
| Mouse drag | Pan |
| Scroll | Zoom |

### Channels

| Key | Action |
|-----|--------|
| `C` | Color (RGB) |
| `R` | Red channel only |
| `G` | Green channel only |
| `B` | Blue channel only |
| `A` | Alpha channel |
| `L` | Luminance |

### Exposure

| Key | Action |
|-----|--------|
| `[` | Decrease exposure |
| `]` | Increase exposure |
| `0` | Reset exposure |

### General

| Key | Action |
|-----|--------|
| `Esc` / `Q` | Exit |
| `Space` | Toggle UI overlay |

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

For multi-layer EXR files:

```rust
let config = ViewerConfig {
    layer: Some("diffuse".into()),  // Show specific layer
    ..Default::default()
};
```

Or select interactively via UI dropdown.

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

## ViewerState

Runtime state:

```rust
use vfx_view::ViewerState;

// Access current state
let state = app.state();
println!("Zoom: {}", state.zoom);
println!("Exposure: {}", state.exposure);
println!("Channel: {:?}", state.channel_mode);
```

## CLI Integration

The viewer is integrated into `vfx` CLI:

```bash
# View image
vfx view image.exr

# With options
vfx view image.exr --display sRGB --view "ACES 1.0 SDR"

# Specific layer
vfx view render.exr --layer diffuse
```

## Architecture

```
ViewerApp (main application)
    │
    ├── ViewerState (current view parameters)
    │
    ├── Handler (input handling)
    │
    └── Renderer (GPU display)
            │
            ├── Image texture
            ├── OCIO LUT texture
            └── Display shader
```

## GPU Rendering

Uses `egui` with `wgpu` backend:

- Image displayed as GPU texture
- OCIO transforms baked to 3D LUT
- Display shader applies LUT + exposure
- 60 FPS pan/zoom

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

## Feature Flag

The viewer is optional in `vfx-cli`:

```toml
[dependencies]
vfx-cli = { version = "0.1", features = ["viewer"] }  # default

# Without viewer (smaller binary)
vfx-cli = { version = "0.1", default-features = false }
```
