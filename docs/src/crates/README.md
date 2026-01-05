# Crate Reference

This section documents each crate in the vfx-rs workspace.

## Crate Overview

vfx-rs consists of 16 crates organized by functionality:

### Foundation

| Crate | Description |
|-------|-------------|
| [vfx-core](core.md) | Core types: Image, ColorSpace, Pixel |
| [vfx-math](math.md) | Matrices, vectors, interpolation |

### Color Science

| Crate | Description |
|-------|-------------|
| [vfx-transfer](transfer.md) | Transfer functions (OETF/EOTF) |
| [vfx-primaries](primaries.md) | Color primaries, RGB/XYZ matrices |
| [vfx-lut](lut.md) | 1D/3D LUT types and file formats |

### I/O and Compute

| Crate | Description |
|-------|-------------|
| [vfx-io](io.md) | Image file reading/writing |
| [vfx-compute](compute.md) | CPU/GPU compute backends |

### Color Management

| Crate | Description |
|-------|-------------|
| [vfx-color](color.md) | Unified color transformation API |
| [vfx-ocio](ocio.md) | OCIO config parsing and processing |
| [vfx-icc](icc.md) | ICC profile support |

### Image Processing

| Crate | Description |
|-------|-------------|
| [vfx-ops](ops.md) | Resize, blur, composite, warp |

### Applications

| Crate | Description |
|-------|-------------|
| [vfx-cli](cli.md) | Command-line tool |
| [vfx-view](view.md) | Image viewer with OCIO |
| [vfx-rs-py](python.md) | Python bindings |

### Testing

| Crate | Description |
|-------|-------------|
| [vfx-tests](tests.md) | Integration tests |
| [vfx-bench](bench.md) | Performance benchmarks |

## Dependency Hierarchy

```
vfx-cli / vfx-view / vfx-rs-py (Applications)
    │
    ├── vfx-ops (Image Processing)
    │       └── vfx-compute
    │
    ├── vfx-color (Color Management)
    │       ├── vfx-transfer
    │       ├── vfx-primaries
    │       └── vfx-lut
    │
    ├── vfx-ocio / vfx-icc (External Formats)
    │
    └── vfx-io (File I/O)
            │
            └── vfx-core (Foundation)
                    └── vfx-math
```

## Using Crates Individually

You can use any crate independently:

```toml
# Just image I/O
[dependencies]
vfx-io = "0.1"

# Just color math
[dependencies]
vfx-color = "0.1"

# Full stack
[dependencies]
vfx-core = "0.1"
vfx-io = "0.1"
vfx-ops = "0.1"
vfx-color = "0.1"
```
