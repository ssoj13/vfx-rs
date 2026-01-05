# PyO3 Python API Plan

## Overview

Python bindings для vfx-rs через PyO3 + numpy интеграцию.  
Цель: Pythonic API для VFX workflows, интероп с numpy/OpenCV/OIIO.

---

## TL;DR: Как это выглядит

### Простой пример: прочитать, обработать, сохранить
```python
import vfxpy

# Читаем EXR, применяем грейд, сохраняем
img = vfxpy.read("plate.exr")

proc = vfxpy.Processor()  # GPU если есть, иначе CPU
proc.exposure(img, 1.2)
proc.cdl(img, slope=[1.1, 1.0, 0.95], offset=[0.01, 0, -0.01], power=[1, 1, 1])
proc.apply_lut(img, vfxpy.lut.read_cube("show_lut.cube"))

vfxpy.write("graded.exr", img)
```

### numpy interop (zero-copy)
```python
import vfxpy
import numpy as np

# numpy -> vfxpy (view, не копия)
arr = np.zeros((1080, 1920, 4), dtype=np.float32)
img = vfxpy.Image(arr)

# vfxpy -> numpy (view)
img = vfxpy.read("input.exr")
arr = img.numpy()  # shape: (H, W, C)
arr[:, :, 0] *= 1.5  # Boost red channel - modifies img!
```

### Batch processing
```python
import vfxpy
from pathlib import Path

proc = vfxpy.Processor()
lut = vfxpy.lut.read_cube("grade.cube")

for exr in Path("plates").glob("*.exr"):
    img = vfxpy.read(exr)
    proc.exposure(img, 1.5).apply_lut(img, lut)
    vfxpy.write(f"graded/{exr.stem}.exr", img)
```

### Композитинг
```python
import vfxpy
from vfxpy import ops

fg = vfxpy.read("character.exr")   # RGBA with alpha
bg = vfxpy.read("background.exr")

# Porter-Duff over
comp = ops.over(fg, bg)

# Blend mode с opacity
result = ops.blend(fg, bg, mode="screen", opacity=0.7)
```

### Resize с фильтрами
```python
from vfxpy import ops

img = vfxpy.read("4k_plate.exr")
half = ops.resize(img, scale=0.5, filter="lanczos3")
hd = ops.resize(img, 1920, 1080, filter="mitchell")
```

### DPX sequence (10-bit log)
```python
import vfxpy
from vfxpy.io import dpx

# Читаем DPX scan
for i in range(1001, 1100):
    img = dpx.read(f"scan.{i:04d}.dpx")
    # ... process ...
    dpx.write(f"out.{i:04d}.dpx", img, bit_depth=10)
```

---

## Ключевые принципы API

| Принцип | Реализация |
|---------|------------|
| **Zero-copy numpy** | `img.numpy()` возвращает view, не копию |
| **GPU by default** | `Processor()` автовыбирает GPU если есть |
| **In-place ops** | `proc.exposure(img, 1.5)` модифицирует img |
| **Chainable** | `proc.exposure(img, 1.5).saturation(img, 1.2)` |
| **Format auto-detect** | `vfxpy.read("file.exr")` определяет формат сам |
| **Pythonic** | snake_case, kwargs, type hints |

## Architecture

```
vfx-py (PyO3 crate)
    ├── vfxpy.Image        ← ImageData wrapper + numpy buffer protocol
    ├── vfxpy.Processor    ← GPU/CPU compute wrapper
    ├── vfxpy.io           ← read/write functions
    ├── vfxpy.lut          ← Lut1D, Lut3D, CLF
    ├── vfxpy.color        ← CDL, ACES transforms
    └── vfxpy.ops          ← resize, composite, filter
```

## Phase 1: Core Types + I/O

### 1.1 Image Type
```python
import vfxpy
import numpy as np

# From numpy (zero-copy view or copy)
arr = np.random.rand(1080, 1920, 4).astype(np.float32)
img = vfxpy.Image(arr)           # RGBA f32
img = vfxpy.Image(arr, copy=True)  # Force copy

# To numpy (zero-copy)
arr = img.numpy()       # Returns view
arr = img.numpy(copy=True)

# Properties
img.width, img.height, img.channels
img.format  # 'u8', 'u16', 'f16', 'f32'
img.dtype   # numpy dtype
```

### 1.2 I/O Functions
```python
# Read any format (auto-detect)
img = vfxpy.read("input.exr")
img = vfxpy.read("input.dpx")

# Write with format-specific options
vfxpy.write("output.exr", img)
vfxpy.write("output.png", img, compression=9)
vfxpy.write("output.dpx", img, bit_depth=10)
vfxpy.write("output.jpg", img, quality=95)

# Format-specific modules
from vfxpy.io import exr, dpx, tiff
img = exr.read("hdr.exr")
dpx.write("scan.dpx", img, bit_depth=10, colorimetric="printing_density")
```

## Phase 2: Compute Processing

### 2.1 Processor
```python
from vfxpy import Processor

# Auto-select backend (GPU if available)
proc = Processor()
proc = Processor(backend="cpu")    # Force CPU
proc = Processor(backend="wgpu")   # Force GPU

# Check capabilities
print(proc.backend)      # "wgpu" or "cpu"
print(proc.device_name)  # "NVIDIA RTX 4090"

# Apply operations (in-place)
proc.exposure(img, 1.5)
proc.saturation(img, 1.2)
proc.contrast(img, 1.1, pivot=0.18)

# CDL
proc.cdl(img, slope=[1.1, 1.0, 0.9], offset=[0, 0, 0], power=[1, 1, 1])

# Chained (fluent)
proc.exposure(img, 1.5).saturation(img, 1.2).contrast(img, 1.1)
```

### 2.2 LUT Application
```python
from vfxpy.lut import Lut1D, Lut3D, read_cube, read_clf

# Load LUTs
lut3d = read_cube("grade.cube")
clf = read_clf("aces_rrt.clf")

# Apply
proc.apply_lut(img, lut3d)
proc.apply_clf(img, clf)

# Create LUTs
lut1d = Lut1D.gamma(1024, 2.2)
lut3d = Lut3D.identity(33)
```

## Phase 3: Image Operations

### 3.1 Resize
```python
from vfxpy import ops

# Resize with filter
resized = ops.resize(img, 1920, 1080, filter="lanczos3")
resized = ops.resize(img, scale=0.5, filter="mitchell")

# Filter options: nearest, bilinear, bicubic, mitchell, lanczos3
```

### 3.2 Composite
```python
# Porter-Duff
result = ops.over(fg, bg)
result = ops.under(fg, bg)

# Blend modes
result = ops.blend(a, b, mode="multiply")
result = ops.blend(a, b, mode="screen", opacity=0.5)
```

### 3.3 Transform
```python
# Geometric transforms
result = ops.rotate(img, angle=45, expand=True)
result = ops.flip(img, "horizontal")
result = ops.crop(img, x=100, y=100, width=500, height=500)
```

## Phase 4: Color Management

### 4.1 OCIO Integration (optional)
```python
from vfxpy import ocio

# Load config
config = ocio.Config("aces_1.2/config.ocio")

# Get processor
proc = config.processor("ACEScg", "sRGB - Display")
proc.apply(img)

# Role-based
display_proc = config.display_processor(view="sRGB")
```

### 4.2 Transfer Functions
```python
from vfxpy import transfer

# Apply transfer function
transfer.srgb_to_linear(img)
transfer.linear_to_srgb(img)
transfer.apply_pq(img, direction="decode")
transfer.apply_hlg(img, direction="encode")
```

## Implementation Details

### Cargo.toml (vfx-py)
```toml
[package]
name = "vfx-py"
version = "0.1.0"
edition = "2024"

[lib]
name = "vfxpy"
crate-type = ["cdylib"]

[dependencies]
pyo3 = { version = "0.23", features = ["extension-module"] }
numpy = "0.23"
vfx-core = { path = "../vfx-core" }
vfx-io = { path = "../vfx-io", features = ["all"] }
vfx-compute = { path = "../vfx-compute", features = ["io"] }
vfx-lut = { path = "../vfx-lut" }
vfx-ops = { path = "../vfx-ops" }
vfx-color = { path = "../vfx-color" }
```

### Module Structure
```
crates/vfx-py/
├── Cargo.toml
├── pyproject.toml          # maturin config
├── src/
│   ├── lib.rs              # PyO3 module entry
│   ├── image.rs            # Image wrapper
│   ├── processor.rs        # Processor wrapper
│   ├── io/
│   │   ├── mod.rs
│   │   ├── exr.rs
│   │   ├── dpx.rs
│   │   └── ...
│   ├── lut.rs              # LUT types
│   ├── ops.rs              # Image operations
│   └── color.rs            # Color transforms
└── python/
    └── vfxpy/
        ├── __init__.py     # Re-exports
        └── py.typed        # PEP 561
```

### numpy Buffer Protocol
```rust
use numpy::{PyArray3, PyArrayMethods};
use pyo3::prelude::*;

#[pyclass]
pub struct Image {
    inner: vfx_io::ImageData,
}

#[pymethods]
impl Image {
    /// Returns numpy array view (zero-copy when possible)
    fn numpy<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyArray3<f32>>> {
        let data = self.inner.to_f32();
        let shape = [self.inner.height as usize, self.inner.width as usize, self.inner.channels as usize];
        Ok(PyArray3::from_vec(py, data).reshape(shape)?)
    }
    
    /// Create from numpy array
    #[new]
    fn new(arr: &Bound<'_, PyArray3<f32>>) -> PyResult<Self> {
        // ...
    }
}
```

## Tasks

### P0: Foundation
- [ ] Setup vfx-py crate with maturin
- [ ] Image type with numpy interop
- [ ] Basic read/write functions
- [ ] CI: build wheels (manylinux, macOS, Windows)

### P1: Core API
- [ ] Processor wrapper (exposure, saturation, contrast, CDL)
- [ ] LUT types (Lut1D, Lut3D)
- [ ] LUT file loaders (CUBE, CLF, SPI)
- [ ] Format-specific I/O modules

### P2: Operations
- [ ] resize with filters
- [ ] composite (over, blend modes)
- [ ] transform (rotate, flip, crop)
- [ ] filter (blur, sharpen)

### P3: Advanced
- [ ] Streaming API for large images
- [ ] OCIO integration (optional feature)
- [ ] Async processing
- [ ] Type stubs (.pyi files)
- [ ] Sphinx documentation

## Build & Install

```bash
# Development
cd crates/vfx-py
maturin develop

# Build wheels
maturin build --release

# Publish to PyPI
maturin publish
```

## Testing

```python
# tests/test_basic.py
import vfxpy
import numpy as np

def test_roundtrip():
    arr = np.random.rand(100, 100, 4).astype(np.float32)
    img = vfxpy.Image(arr)
    assert img.width == 100
    assert img.height == 100
    
    out = img.numpy()
    np.testing.assert_array_almost_equal(arr, out)

def test_read_write(tmp_path):
    img = vfxpy.read("tests/fixtures/test.exr")
    vfxpy.write(tmp_path / "out.exr", img)
    loaded = vfxpy.read(tmp_path / "out.exr")
    assert img.width == loaded.width
```
