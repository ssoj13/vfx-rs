# AGENTS.md

This document captures the current dataflow and codepaths in the VFX-RS workspace.
It is intended as a durable reference for future work.

## High-Level Dataflow

```
Files on disk
   │
   ▼
vfx-io::read  ──► ImageData (format-agnostic container)
   │
   ├─ metadata (colorspace, gamma)
   └─ pixel data (U8/U16/F32)
   │
   ▼
Color operations
   ├─ vfx-color (Pipeline + ColorProcessor)
   ├─ vfx-ocio (Config + Processor)
   └─ vfx-ops (resize/composite/filter)
   │
   ▼
vfx-io::write  ──► Output files
```

## OCIO Processing Flow

```
Config::from_file / from_yaml_str
   │
   ├─ parse roles, colorspaces, displays, looks, view_transforms
   └─ build internal structures
   │
   ▼
Config::processor(src, dst)
   │
   ├─ src colorspace → to_reference transform
   ├─ dst colorspace → from_reference transform
   └─ group transforms → Processor::from_transform
   │
   ▼
Processor::apply_rgb / apply_rgba
   │
   └─ Op list (Matrix/LUT/CDL/Range/Transfer/...)
```

## Display Pipeline (Intended)

```
Config::display_processor(src, display, view)
   │
   ├─ resolve display + view
   ├─ apply view transform (OCIO v2)
   ├─ apply view look(s)
   └─ convert to view colorspace
   │
   ▼
Processor::apply_rgb / apply_rgba
```

## CLI Codepaths

```
vfx (binary)
   │
   ├─ commands::info  -> vfx-io::read -> metadata dump
   ├─ commands::convert -> vfx-io::read -> vfx-io::write
   ├─ commands::lut -> load LUT -> CPU apply -> vfx-io::write
   ├─ commands::resize/blur/sharpen -> vfx-ops -> vfx-io::write
   └─ commands::color -> vfx-ocio / vfx-color -> vfx-io::write
```

## Crate Dependency Map (Simplified)

```
                     vfx-cli
                        │
        ┌───────────────┼────────────────┐
        ▼               ▼                ▼
     vfx-io         vfx-ops          vfx-ocio
        │               │                │
        └──────┬────────┴────────┬───────┘
               ▼                 ▼
           vfx-color           vfx-lut
               │                 │
               ├───── vfx-transfer
               ├───── vfx-primaries
               └───── vfx-math
                     │
                 vfx-core
```

## Key Data Structures

- `vfx-core::Image<C, T, N>`: typed image buffer with compile-time color space.
- `vfx-io::ImageData`: format-agnostic container for I/O.
- `vfx-ocio::Config`: parsed OCIO configuration (colorspaces, roles, displays).
- `vfx-ocio::Processor`: compiled transform op list.
- `vfx-color::Pipeline`: explicit sequence of per-RGB ops.

## Open Gaps to Track

- Two parallel image representations (`Image` vs `ImageData`).
- OCIO view transforms and looks not wired into display processors.
- File rules are not implemented per OCIO semantics (glob/regex/default rule).
- LUT domain handling is missing in OCIO 3D LUT ops.
