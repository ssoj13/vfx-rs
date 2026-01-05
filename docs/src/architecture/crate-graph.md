# Crate Dependency Graph

This page shows how vfx-rs crates depend on each other.

## ASCII Diagram

```
                                    ┌─────────────┐
                                    │   vfx-cli   │
                                    │   (binary)  │
                                    └──────┬──────┘
                                           │
              ┌────────────────────────────┼───────────────────────────┐
              │                            │                           │
              ▼                            ▼                           ▼
        ┌──────────┐               ┌───────────────┐            ┌───────────┐
        │  vfx-io  │               │   vfx-color   │            │  vfx-ops  │
        │ (I/O)    │               │ (color mgmt)  │            │ (filters) │
        └────┬─────┘               └───────┬───────┘            └─────┬─────┘
             │                             │                          │
             │                    ┌────────┴────────┐                 │
             │                    │                 │                 │
             │                    ▼                 ▼                 │
             │           ┌────────────────┐  ┌────────────┐           │
             │           │  vfx-transfer  │  │vfx-primaries│          │
             │           │   (OETF/EOTF)  │  │ (gamuts)   │           │
             │           └───────┬────────┘  └─────┬──────┘           │
             │                   │                 │                  │
             │                   │     ┌───────────┘                  │
             │                   │     │                              │
             │                   ▼     ▼                              │
             │                ┌───────────┐                           │
             │                │  vfx-lut  │                           │
             │                │ (1D/3D)   │                           │
             │                └─────┬─────┘                           │
             │                      │                                 │
             │    ┌─────────────────┼──────────────────┐              │
             │    │                 │                  │              │
             ▼    ▼                 ▼                  ▼              ▼
        ┌──────────────┐    ┌────────────┐    ┌─────────────┐    ┌──────────┐
        │  vfx-compute │    │  vfx-math  │    │  vfx-ocio   │    │  vfx-icc │
        │  (CPU/GPU)   │    │ (matrices) │    │ (configs)   │    │ (lcms2)  │
        └──────┬───────┘    └─────┬──────┘    └──────┬──────┘    └────┬─────┘
               │                  │                  │                │
               └──────────────────┴─────────┬────────┴────────────────┘
                                            │
                                            ▼
                                     ┌────────────┐
                                     │  vfx-core  │
                                     │  (types)   │
                                     └────────────┘
```

## Dependency Details

### vfx-core (Foundation)

No internal dependencies. Uses:
- `half` - 16-bit floats
- `thiserror` - error types
- `rayon` - parallelism

### vfx-math

```
vfx-math
    └── vfx-core
```

Uses `glam` for matrix math and `wide` for SIMD.

### vfx-transfer, vfx-lut

```
vfx-transfer          vfx-lut
    └── vfx-core          ├── vfx-core
                          └── quick-xml (CLF parsing)
```

### vfx-primaries

```
vfx-primaries
    ├── vfx-core
    └── vfx-math
```

Uses `glam` for chromatic adaptation matrices.

### vfx-io

```
vfx-io
    └── vfx-core
```

All format codecs are optional features:
- `exr` - OpenEXR via `exr` crate
- `png`, `jpeg`, `tiff` - respective crates
- `dpx` - native implementation
- `heif` - via `libheif-rs` (requires system library)

### vfx-compute

```
vfx-compute
    ├── vfx-core
    └── vfx-io (optional, for testing)
```

GPU backends via features:
- `wgpu` - WebGPU (Vulkan, Metal, DX12)
- `cuda` - NVIDIA CUDA via `cudarc`

### vfx-color

```
vfx-color
    ├── vfx-core
    ├── vfx-math
    ├── vfx-transfer
    ├── vfx-primaries
    ├── vfx-lut
    └── vfx-compute (optional, gpu feature)
```

This is the main color management crate, combining all color science.

### vfx-ocio

```
vfx-ocio
    ├── vfx-core
    ├── vfx-math
    ├── vfx-lut
    ├── vfx-transfer
    └── vfx-primaries
```

Uses `saphyr` for YAML parsing (OCIO config format).

### vfx-icc

```
vfx-icc
    └── vfx-core
```

Uses `lcms2` bindings for ICC profile handling.

### vfx-ops

```
vfx-ops
    ├── vfx-core
    ├── vfx-io
    ├── vfx-math
    └── vfx-compute
```

Image processing operations with optional parallel/FFT features.

### vfx-cli

```
vfx-cli
    ├── vfx-core
    ├── vfx-io
    ├── vfx-ops
    ├── vfx-color
    ├── vfx-lut
    └── vfx-view (optional)
```

The command-line application pulling everything together.

## External Dependencies

Key external crates used across the workspace:

| Crate | Purpose |
|-------|---------|
| `glam` | Fast vector/matrix math |
| `half` | f16 (half-precision) floats |
| `rayon` | Parallel iterators |
| `clap` | CLI argument parsing |
| `exr` | OpenEXR reading/writing |
| `lcms2` | ICC profile transforms |
| `wgpu` | Cross-platform GPU compute |
| `pyo3` | Python bindings |
