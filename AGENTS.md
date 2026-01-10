# VFX-RS Project Status & Architecture

**Last Updated:** 2026-01-09
**Status:** Active Refactoring & Cleanup

---

## 🏗 Architecture & Data Flow

The project is structured as a workspace of crates, with `vfx-core` acting as the foundation.

```mermaid
graph TD
    %% Core Foundation
    Core[vfx-core] --> Math[vfx-math]
    Core --> Primaries[vfx-primaries]
    
    %% Color Science
    Math --> Transfer[vfx-transfer]
    Core --> Lut[vfx-lut]
    Transfer --> Color[vfx-color]
    Lut --> Color
    Primaries --> Color
    
    %% Execution & Compute
    Core --> Compute[vfx-compute]
    Color -.-> Compute %% Optional dependency for acceleration
    
    %% I/O and Formats
    Core --> Exr[vfx-exr]
    Exr --> IO[vfx-io]
    Color --> IO
    
    %% Application Layer
    Color --> Ops[vfx-ops]
    Compute --> Ops
    IO --> Ops
    
    %% Consumers
    Ops --> Cli[vfx-cli]
    Ops --> View[vfx-view]
    Ops --> Py[vfx-rs-py]
```

### Key Crates

| Crate | Responsibility | Status |
|-------|----------------|--------|
| `vfx-core` | Base types (`Image`, `Pixel`, `Rect`). Strongly typed. | Stable |
| `vfx-color` | Color science (Transforms, CDL, ACES). Source of truth for color logic. | Stable |
| `vfx-compute` | GPU/CPU executor. Runtime typed. **Duplication Risk**. | Needs Refactor |
| `vfx-exr` | OpenEXR implementation. | **High Debt** |
| `vfx-cli` | Command-line interface. Glue code. | Stable |

---

## 🛠 Technical Debt & Known Issues

### 1. Code Duplication (`vfx-compute`)
- **CDL:** `vfx-compute` redefines `Cdl` struct instead of using `vfx-color`.
- **Image:** `ComputeImage` uses `Vec<f32>` while `vfx-core` uses `Arc<Vec<T>>`. This prevents zero-copy sharing between CPU and GPU pipelines.

### 2. vfx-exr TODOs
The `vfx-exr` crate is feature-complete but contains ~200 `TODO` and `FIXME` markers:
- **Optimization:** Missing caching for level calculations and deep data blocks.
- **Safety:** Loose integer casting (needs `try_from`).
- **Cleanup:** Redundant clones and naming inconsistencies.

### 3. Missing Integrations
- `vfx-io`: Unpremult support is missing for OCIO transforms.
- `vfx-compute`: Shader caching is not fully integrated.

---

## 🔄 Refactoring Plan

See `plan1.md` for the current execution strategy.

1. **Unify Data Types:** Make `vfx-color` and `vfx-core` the single source of truth.
2. **Optimize Storage:** Align `ComputeImage` memory layout with `vfx-core` to enable zero-copy.
3. **Pay Down Debt:** Systematically address `vfx-exr` TODOs.

---

## Dataflow & Codepaths (High-Level)

### 1) CLI / Batch Pipeline

```
vfx-cli
  └─ commands/* -> vfx-io::read(...)
                   └─ format dispatch (exr/png/tiff/...)
                       └─ ImageBuf / ImageData
                          ├─ vfx-ops (filters, resize, composite, fft)
                          ├─ vfx-color (ACES, grading ops)
                          └─ vfx-ocio (Config + Processor)
                                   └─ ProcessorOp chain (CPU/GPU)
                   └─ vfx-io::write(...)
```

### 2) OCIO Processor Build/Apply

```
Config::processor(src, dst)
  └─ ColorSpace(src).to_reference()
  └─ ColorSpace(dst).from_reference() or inverse(to_reference)
  └─ GroupTransform -> Processor::from_transform
        └─ compile_transform()
             └─ ProcessorOp list (Matrix, Range, LUT, Transfer, ...)
        └─ apply_rgb() / apply()
```

### 3) EXR Deep Data Read Path

```
vfx-io::exr::read_*()
  └─ vfx-exr::image::read::ReadImage
       └─ meta::read_headers()
       └─ block::reader::ChunksReader
            └─ compression::decompress_*
                 └─ block::UncompressedBlock
                      └─ image::read::SpecificChannels/AnyChannels
                           └─ Image / Layer / Pixels
```

### 4) Viewer (vfx-view) Runtime Loop

```
UI thread (egui) <-> Worker thread (ViewerHandler)
  └─ load_image() -> vfx-io read -> layers -> apply_channel_mode()
       └─ ColorConfig::display_processor(...) -> apply_rgb()
            └─ upload texture -> draw_canvas()
```
