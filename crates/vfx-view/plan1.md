# Bug Hunt Report: vfx-view

**Date:** 2026-01-08  
**Scope:** `crates/vfx-view/src/` (5 files, ~1100 lines)

---

## Executive Summary

Viewer code is well-structured with clean separation: UI (app.rs), worker (handler.rs), messages, state. Found **2 functional bugs**, **5 performance issues**, and **8 code quality improvements**.

---

## 1. BUGS (Critical)

### 1.1 Zoom Center Ignored
**Location:** `handler.rs:415-420`
```rust
fn zoom(&mut self, factor: f32, _center: [f32; 2]) {  // <-- center ignored!
    self.zoom = (self.zoom * (1.0 + factor)).clamp(0.1, 100.0);
```
**Problem:** Zoom always happens relative to viewport center, not cursor position. The `center` parameter is passed from UI but never used.  
**Impact:** Poor UX - users expect zoom-to-cursor behavior like in Photoshop/Nuke.  
**Fix:** Implement proper zoom-to-point math.

### 1.2 Continuous Repaint Waste
**Location:** `app.rs:541`
```rust
ctx.request_repaint();  // Always called every frame!
```
**Problem:** Forces 60fps redraw even when idle. Wastes CPU/GPU.  
**Impact:** High CPU usage (~10-15%) even when doing nothing.  
**Fix:** Only repaint when events pending or animation in progress.

---

## 2. PERFORMANCE ISSUES

### 2.1 Unnecessary Vector Cloning Every Frame
**Location:** `app.rs:290, 311, 336, 381, 397`
```rust
for cs in self.state.colorspaces.clone() {  // Clone Vec every frame!
```
**Fix:** Use `self.state.colorspaces.iter()` or cache selections.

### 2.2 Repeated StateSync Sends
**Location:** `handler.rs:420, 427, 447, 454`
```rust
self.send(ViewerEvent::StateSync { zoom: self.zoom, pan: self.pan });
```
**Problem:** Same pattern repeated 4 times.  
**Fix:** Extract to `fn sync_state(&self)` helper.

### 2.3 Image Clone in Channel Mode
**Location:** `handler.rs:315`
```rust
ChannelMode::Color => img.clone(),  // Full image copy!
```
**Fix:** Return reference or cow pattern for Color mode.

---

## 3. CODE QUALITY

### 3.1 Magic Numbers
| Location | Value | Should Be |
|----------|-------|-----------|
| `handler.rs:443` | `0.95` | `const FIT_MARGIN: f32 = 0.95` |
| `handler.rs:354-355` | `0.2126, 0.7152, 0.0722` | `const REC709_R/G/B` |
| `state.rs:137` | `[1280.0, 720.0]` | `const DEFAULT_VIEWPORT` |
| `app.rs:273` | `0.0` | Use shared `DEFAULT_EXPOSURE` |

### 3.2 Inconsistent Default Exposure
**Locations:**
- `app.rs:273` - `const DEFAULT_EXPOSURE: f32 = 0.0` (local)
- `state.rs:82` - `exposure: 0.0` (inline)
- `handler.rs:51` - `exposure: 0.0` (inline)

**Fix:** Single source: `pub const DEFAULT_EXPOSURE: f32 = 0.0` in `state.rs`

### 3.3 Worker Thread Not Joined
**Location:** `app.rs:22`
```rust
_worker: JoinHandle<()>,  // Never joined!
```
**Problem:** On app close, worker thread may be orphaned.  
**Fix:** Implement `Drop` for `ViewerApp` that sends Close and joins.

### 3.4 Unused Import
**Location:** `app.rs:6`
```rust
use std::thread::{self, JoinHandle};  // `self` unused after spawn
```

---

## 4. MISSING FEATURES (Not bugs, but noted)

1. **Ctrl+O shortcut** for Open dialog
2. **Drag&drop visual feedback** (highlight on hover)
3. **Recent files** menu
4. **Pixel probe** (show values under cursor)

---

## 5. DATAFLOW DIAGRAM

```
┌─────────────────────────────────────────────────────────────────┐
│                         UI THREAD (app.rs)                       │
├─────────────────────────────────────────────────────────────────┤
│  ViewerApp                                                       │
│  ├── tx: Sender<ViewerMsg> ─────────────────────────────────┐   │
│  ├── rx: Receiver<ViewerEvent> <────────────────────────┐   │   │
│  ├── state: ViewerState                                 │   │   │
│  ├── texture: Option<TextureHandle>                     │   │   │
│  └── generation: u64                                    │   │   │
│                                                         │   │   │
│  [Input] → handle_input() → send(ViewerMsg)            │   │   │
│  [Frame] → process_events() ← rx.try_recv()            │   │   │
│  [Frame] → draw_controls/canvas/hints()                │   │   │
└─────────────────────────────────────────────────────────┼───┼───┘
                                                          │   │
                         ┌────────────────────────────────┘   │
                         │ ViewerEvent                        │ ViewerMsg
                         ▼                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                      WORKER THREAD (handler.rs)                  │
├─────────────────────────────────────────────────────────────────┤
│  ViewerHandler                                                   │
│  ├── rx: Receiver<ViewerMsg>                                    │
│  ├── tx: Sender<ViewerEvent>                                    │
│  ├── ocio_config: Option<Config>                                │
│  ├── image: Option<LayeredImage>                                │
│  └── generation: u64                                            │
│                                                                  │
│  run() loop:                                                     │
│    LoadImage → load_image() → regenerate_texture()              │
│    SetDisplay → set_display() → update_views() → regen          │
│    SetView/Exposure/Channel → set_*() → regenerate_texture()    │
│    Zoom/Pan/Fit/Home → update state → send(StateSync)           │
│                                                                  │
│  regenerate_texture():                                           │
│    1. Get layer from image                                       │
│    2. apply_channel_mode()                                       │
│    3. apply_ocio_pipeline() → Color32[]                         │
│    4. send(TextureReady)                                         │
└─────────────────────────────────────────────────────────────────┘
```

---

## 6. MESSAGE FLOW

```
ViewerMsg (UI → Worker):
  LoadImage(PathBuf)         → Triggers full reload + texture gen
  SetOcioConfig(Option)      → Reloads OCIO, updates displays/views
  SetDisplay(String)         → Updates views list + regen
  SetView(String)            → Regen texture
  SetInputColorspace(String) → Regen texture
  SetExposure(f32)           → Regen texture
  SetChannelMode(ChannelMode)→ Regen texture
  SetLayer(String)           → Regen texture
  Regenerate                 → Force regen
  SyncGeneration(u64)        → Sync generation counter
  Zoom/Pan/Fit/Home          → Update view state, send StateSync
  SetViewport([f32;2])       → Store for fit calculation
  Close                      → Exit worker loop

ViewerEvent (Worker → UI):
  ImageLoaded{...}           → Update UI state, title
  OcioConfigLoaded{...}      → Populate display/colorspace lists
  DisplayChanged{...}        → Populate views list
  TextureReady{...}          → Update texture for display
  StateSync{zoom,pan}        → Sync view state from worker
  Error(String)              → Show error message
```

---

## 7. ACTION PLAN

### Phase 1: Bug Fixes (Priority: HIGH)
- [ ] **1.1** Implement zoom-to-cursor in `handler.rs`
- [ ] **1.2** Conditional repaint in `app.rs`

### Phase 2: Performance (Priority: MEDIUM)
- [ ] **2.1** Replace `.clone()` with `.iter()` in ComboBox loops
- [ ] **2.2** Extract `sync_state()` helper in handler
- [ ] **2.3** Cow pattern for ChannelMode::Color

### Phase 3: Code Quality (Priority: LOW)
- [ ] **3.1** Extract magic numbers to constants
- [ ] **3.2** Single DEFAULT_EXPOSURE source
- [ ] **3.3** Implement Drop for proper worker cleanup
- [ ] **3.4** Clean unused imports

---

## 8. FILES SUMMARY

| File | Lines | Issues | Status |
|------|-------|--------|--------|
| `lib.rs` | 178 | 0 | ✅ Clean |
| `messages.rs` | 101 | 0 | ✅ Clean |
| `state.rs` | 188 | 2 | ⚠️ Magic numbers |
| `handler.rs` | 458 | 4 | ⚠️ Zoom bug, duplication |
| `app.rs` | 561 | 4 | ⚠️ Repaint bug, cloning |

**Total:** 1486 lines, 10 issues found

---

## APPROVAL REQUIRED

Waiting for approval to proceed with fixes. Recommend starting with Phase 1 (bugs).
