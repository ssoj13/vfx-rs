# vfx-view Architecture

## Module Structure

```
vfx-view/src/
├── lib.rs       # Entry points: run(), run_opt()
├── app.rs       # UI thread: ViewerApp, egui rendering
├── handler.rs   # Worker thread: image loading, OCIO processing
├── messages.rs  # ViewerMsg (UI→Worker), ViewerEvent (Worker→UI)
└── state.rs     # ViewerState, ViewerPersistence, ChannelMode
```

---

## Thread Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                        MAIN THREAD                            │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                    eframe/egui                          │  │
│  │  - Window management                                    │  │
│  │  - Input handling (keyboard, mouse, drag&drop)          │  │
│  │  - UI rendering (panels, controls, canvas)              │  │
│  │  - Texture display (GPU upload via wgpu)                │  │
│  └────────────────────────────────────────────────────────┘  │
│                            │                                  │
│                     ViewerApp                                 │
│                     ├── tx ──────────────┐                   │
│                     └── rx ◄─────────┐   │                   │
└─────────────────────────────────────┼───┼────────────────────┘
                                      │   │
              mpsc::channel           │   │  mpsc::channel
              ViewerEvent             │   │  ViewerMsg
                                      │   │
┌─────────────────────────────────────┼───┼────────────────────┐
│                        WORKER THREAD │   │                    │
│                     ┌────────────────┘   ▼                   │
│                     │            ViewerHandler               │
│  ┌──────────────────┴─────────────────────────────────────┐  │
│  │  - Image loading (vfx-io)                               │  │
│  │  - OCIO config management (vfx-ocio)                    │  │
│  │  - Channel isolation (R/G/B/A/Luma)                     │  │
│  │  - Exposure adjustment                                  │  │
│  │  - Display transform (OCIO processor)                   │  │
│  │  - Pixel conversion (f32 → Color32)                     │  │
│  │  - View state (zoom/pan) calculations                   │  │
│  └─────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

---

## Data Flow: Image Loading

```
User Action                UI Thread                    Worker Thread
───────────────────────────────────────────────────────────────────────
                                                        
[Drop file]  ──►  handle_dropped_files()                
       or         open_file_dialog()                    
[Open btn]        │                                     
                  ▼                                     
              send(LoadImage(path))  ──────────────►  load_image()
                                                        │
                                                        ▼
                                                      vfx_io::exr::read_layers()
                                                        │ or vfx_io::read()
                                                        ▼
                                                      self.image = Some(layered)
                                                        │
                                                        ▼
              process_events()  ◄──────────────────  send(ImageLoaded{...})
              │                                         │
              ▼                                         ▼
              state.image_dims = dims               regenerate_texture()
              state.layers = layers                     │
              update window title                       ▼
                                                      apply_channel_mode()
                                                        │
                                                        ▼
                                                      apply_ocio_pipeline()
                                                        │
                                                        ▼
              process_events()  ◄──────────────────  send(TextureReady{...})
              │
              ▼
              texture = ctx.load_texture(pixels)
              │
              ▼
              draw_canvas() displays texture
```

---

## Data Flow: OCIO Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                    regenerate_texture()                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. GET LAYER                                                    │
│     image.layers.find(|l| l.name == self.layer)                 │
│     └── Layer { width, height, channels: HashMap<String, Vec> } │
│                                                                  │
│  2. CONVERT TO ImageData                                         │
│     layer.to_image_data() → ImageData { width, height, data }   │
│                                                                  │
│  3. APPLY CHANNEL MODE                                           │
│     ┌─────────────────────────────────────────────────────────┐ │
│     │ Color     → pass through (clone)                        │ │
│     │ Red       → [R,R,R] grayscale                           │ │
│     │ Green     → [G,G,G] grayscale                           │ │
│     │ Blue      → [B,B,B] grayscale                           │ │
│     │ Alpha     → [A,A,A] grayscale                           │ │
│     │ Luminance → [L,L,L] where L = 0.2126R+0.7152G+0.0722B   │ │
│     └─────────────────────────────────────────────────────────┘ │
│                                                                  │
│  4. APPLY OCIO PIPELINE                                          │
│     ┌─────────────────────────────────────────────────────────┐ │
│     │ a) Apply exposure: pixel *= 2^EV                        │ │
│     │                                                         │ │
│     │ b) Determine input colorspace:                          │ │
│     │    - self.input_colorspace if set                       │ │
│     │    - else scene_linear role                             │ │
│     │    - else "ACEScg" fallback                             │ │
│     │                                                         │ │
│     │ c) Create display processor:                            │ │
│     │    config.display_processor(input, display, view)       │ │
│     │                                                         │ │
│     │ d) Apply transform:                                     │ │
│     │    processor.apply_rgb(&mut pixels)                     │ │
│     └─────────────────────────────────────────────────────────┘ │
│                                                                  │
│  5. CONVERT TO Color32                                           │
│     f32 [0..1] → u8 [0..255] → Color32::from_rgba_unmultiplied  │
│                                                                  │
│  6. SEND TO UI                                                   │
│     send(TextureReady { generation, width, height, pixels })    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Data Flow: View Controls (Zoom/Pan)

```
User Action              UI Thread                   Worker Thread
──────────────────────────────────────────────────────────────────

[Scroll wheel]  ──►  handle_input()
                     │
                     ▼
                     send(Zoom{factor, center})  ───►  zoom()
                                                        │
                                                        ▼
                                                       self.zoom *= (1+factor)
                                                       self.zoom.clamp(0.1, 100.0)
                                                        │
                     process_events()  ◄────────────  send(StateSync{zoom,pan})
                     │
                     ▼
                     state.zoom = zoom
                     │
                     ▼
                     draw_canvas() uses new zoom


[Mouse drag]    ──►  draw_canvas()
                     response.dragged()
                     │
                     ▼
                     send(Pan{delta})  ─────────────►  pan()
                                                        │
                                                        ▼
                                                       self.pan += delta/zoom
                                                        │
                     process_events()  ◄────────────  send(StateSync{zoom,pan})
                     │
                     ▼
                     state.pan = pan
```

---

## State Synchronization

```
┌─────────────────────┐              ┌─────────────────────┐
│     UI State        │              │   Worker State      │
│   (ViewerState)     │              │  (ViewerHandler)    │
├─────────────────────┤              ├─────────────────────┤
│ display             │◄────sync────►│ display             │
│ view                │◄────sync────►│ view                │
│ input_colorspace    │◄────sync────►│ input_colorspace    │
│ exposure            │◄────sync────►│ exposure            │
│ channel_mode        │◄────sync────►│ channel_mode        │
│ layer               │◄────sync────►│ layer               │
│ zoom                │◄────sync────►│ zoom                │
│ pan                 │◄────sync────►│ pan                 │
│ viewport_size       │─────────────►│ viewport            │
├─────────────────────┤              ├─────────────────────┤
│ displays (list)     │◄─────────────│ (from config)       │
│ views (list)        │◄─────────────│ (from config)       │
│ colorspaces (list)  │◄─────────────│ (from config)       │
│ layers (list)       │◄─────────────│ (from image)        │
│ image_dims          │◄─────────────│ (from image)        │
├─────────────────────┤              ├─────────────────────┤
│ texture             │◄─────────────│ (generated pixels)  │
│ error               │◄─────────────│ (error messages)    │
└─────────────────────┘              └─────────────────────┘

Sync Direction:
  ──► UI changes setting, sends to Worker
  ◄── Worker broadcasts state back to UI
```

---

## Generation Counter (Stale Result Rejection)

```
Problem: User rapidly changes settings → multiple regen requests
         Old results may arrive after new ones

Solution: Generation counter

UI Thread:                          Worker Thread:
─────────────────────────────────────────────────────────
generation = 0

[User changes view]
generation = 1
send(SyncGeneration(1))  ────────►  self.generation = 1
send(SetView("ACES"))    ────────►  regenerate_texture()
                                    ... processing ...

[User changes view again]                  
generation = 2
send(SyncGeneration(2))  ────────►  self.generation = 2
send(SetView("Filmic"))  ────────►  regenerate_texture()
                                    ... processing ...

                         ◄────────  TextureReady{gen=1, ...}
process_events():                   
  if gen(1) < self.gen(2):
    continue;  // SKIP stale result!

                         ◄────────  TextureReady{gen=2, ...}
process_events():
  if gen(2) >= self.gen(2):
    texture = load_texture(...)  // USE this result
```

---

## Persistence Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                         STARTUP                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. run_opt() / run()                                           │
│     │                                                            │
│     ▼                                                            │
│  2. load_persistence()                                           │
│     └── dirs::config_dir()/vfx-rs/viewer/app.ron                │
│         │                                                        │
│         ▼                                                        │
│     ViewerPersistence {                                          │
│       last_file, ocio_path, display, view, exposure, channel    │
│     }                                                            │
│     │                                                            │
│     ▼                                                            │
│  3. ViewerState::from_persistence()                              │
│     └── Merges persistence + CLI overrides                       │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                         SHUTDOWN                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. eframe::App::save() called automatically                     │
│     │                                                            │
│     ▼                                                            │
│  2. state.to_persistence()                                       │
│     │                                                            │
│     ▼                                                            │
│  3. eframe::set_value(storage, "vfx_viewer_state", &persistence)│
│     └── Saved to app.ron                                         │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Key Types

```rust
// messages.rs
enum ViewerMsg { LoadImage, SetOcioConfig, SetDisplay, SetView, ... }
enum ViewerEvent { ImageLoaded, TextureReady, StateSync, Error, ... }

// state.rs
enum ChannelMode { Color, Red, Green, Blue, Alpha, Luminance }
struct ViewerPersistence { last_file, ocio_path, display, view, exposure, ... }
struct ViewerState { displays, views, colorspaces, zoom, pan, ... }
const DEFAULT_EXPOSURE: f32 = 0.0;
const DEFAULT_VIEWPORT: [f32; 2] = [1280.0, 720.0];

// app.rs
struct ViewerApp { tx, rx, worker: Option<JoinHandle>, texture, state, ... }
struct ViewerConfig { ocio, display, view, colorspace, verbose }
impl Drop for ViewerApp { ... }  // Proper worker cleanup

// handler.rs
struct ViewerHandler { rx, tx, ocio_config, image, display, view, ... }
const FIT_MARGIN: f32 = 0.95;
const ZOOM_MIN: f32 = 0.1;
const ZOOM_MAX: f32 = 100.0;
const LUMA_R/G/B: f32 = ...;  // Rec.709 coefficients
```

---

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
| Scroll | Zoom at cursor |
| Drag | Pan image |
| Double-click | Fit / Open (if no image) |
| Ctrl+click EV | Reset exposure |

---

## UI Labels

| Label | Full Name | Description |
|-------|-----------|-------------|
| **Src** | Source Colorspace | Input colorspace |
| **RRT** | Reference Rendering Transform | View/tone-mapping |
| **ODT** | Output Device Transform | Display type |
| **EV** | Exposure Value | Brightness (stops) |
| **Ch** | Channel | Display mode |
| **Layer** | Layer | EXR layer |

---

## Implementation Notes

### Conditional Repaint
- `process_events()` returns `bool` indicating if events were processed
- `request_repaint()` only called when events received
- Saves CPU when viewer is idle

### Zoom-to-Cursor
```rust
fn zoom(&mut self, factor: f32, center: [f32; 2]) {
    let old_zoom = self.zoom;
    let new_zoom = (old_zoom * (1.0 + factor)).clamp(ZOOM_MIN, ZOOM_MAX);
    // Adjust pan so point under cursor stays fixed
    let scale_diff = 1.0 / new_zoom - 1.0 / old_zoom;
    self.pan[0] += center[0] * scale_diff;
    self.pan[1] += center[1] * scale_diff;
    self.zoom = new_zoom;
    self.sync_view_state();
}
```

### Worker Cleanup (Drop)
```rust
impl Drop for ViewerApp {
    fn drop(&mut self) {
        let _ = self.tx.send(ViewerMsg::Close);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
```
