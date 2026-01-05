# vfx-rs-py

Python bindings for vfx-rs.

## Purpose

PyO3-based Python module providing access to vfx-rs functionality from Python. Integrates with NumPy for efficient data exchange.

## Installation

### From PyPI (when published)

```bash
pip install vfx-rs
```

### From Source

```bash
cd crates/vfx-rs-py
maturin develop --release
```

Or build wheel:

```bash
maturin build --release
pip install target/wheels/vfx_rs-*.whl
```

## Quick Start

```python
import vfx_rs
import numpy as np

# Read image
img = vfx_rs.read("input.exr")
print(f"Size: {img.width}x{img.height}, {img.channels} channels")

# Access as NumPy array
data = img.to_numpy()  # Shape: (height, width, channels)

# Process
data *= 1.5  # Exposure adjustment

# Create from NumPy
result = vfx_rs.Image.from_numpy(data)

# Write
vfx_rs.write("output.exr", result)
```

## Image Class

```python
import vfx_rs

# Properties
img = vfx_rs.read("image.exr")
print(img.width)      # int
print(img.height)     # int
print(img.channels)   # int
print(img.format)     # "f32", "f16", "u8", "u16"

# NumPy conversion
arr = img.to_numpy()           # Returns np.ndarray (H, W, C)
img = vfx_rs.Image.from_numpy(arr)  # From np.ndarray

# Data access
data = img.data()              # Raw bytes
```

## I/O Functions

```python
import vfx_rs

# Read (auto-detect format)
img = vfx_rs.read("input.exr")
img = vfx_rs.read("photo.jpg")

# Write (format from extension)
vfx_rs.write("output.png", img)
vfx_rs.write("output.exr", img)

# With options
vfx_rs.write("output.jpg", img, quality=90)
vfx_rs.write("output.exr", img, compression="zip")
```

## Color Transforms

```python
import vfx_rs.color as color

# Transfer functions
linear = color.srgb_eotf(0.5)
encoded = color.srgb_oetf(linear)

# Batch processing
data = img.to_numpy()
color.apply_srgb_eotf(data)  # In-place

# ACES
color.apply_rrt_odt_srgb(data)
color.apply_inverse_odt_srgb(data)
```

## Color Spaces

```python
import vfx_rs.color as color
import numpy as np

# Get conversion matrix
matrix = color.rgb_to_rgb_matrix("sRGB", "ACEScg")

# Apply to pixels
pixels = img.to_numpy().reshape(-1, 3)
result = pixels @ matrix.T
```

## LUT Processing

```python
import vfx_rs.lut as lut

# Load LUT
lut_3d = lut.read_cube_3d("grade.cube")
lut_1d = lut.read_cube_1d("gamma.cube")

# Apply
data = img.to_numpy()
lut.apply_3d(data, lut_3d)
lut.apply_1d(data, lut_1d)
```

## Image Operations

```python
import vfx_rs.ops as ops

# Resize
resized = ops.resize(img, width=1920, height=1080, filter="lanczos")
resized = ops.resize(img, scale=0.5)

# Blur
blurred = ops.blur(img, radius=5, type="gaussian")

# Composite
result = ops.over(foreground, background)
result = ops.blend(a, b, mode="multiply")
```

## OCIO Integration

```python
import vfx_rs.ocio as ocio

# Load config
config = ocio.Config.from_file("/path/to/config.ocio")

# Or use built-in
config = ocio.builtin_aces_1_3()

# Create processor
proc = config.processor("ACEScg", "sRGB")

# Apply
data = img.to_numpy()
proc.apply(data)

# Display processor
proc = config.display_processor("ACEScg", "sRGB", "Film")
```

## NumPy Integration

### Data Types

```python
import numpy as np
import vfx_rs

# Float32 (default for processing)
data = np.zeros((1080, 1920, 3), dtype=np.float32)
img = vfx_rs.Image.from_numpy(data)

# Uint8 (for display)
data = np.zeros((1080, 1920, 3), dtype=np.uint8)
img = vfx_rs.Image.from_numpy(data)

# Uint16
data = np.zeros((1080, 1920, 3), dtype=np.uint16)
img = vfx_rs.Image.from_numpy(data)
```

### Memory Layout

```python
# vfx-rs uses C-contiguous, interleaved layout
# Shape: (height, width, channels)
# Memory: RGBRGBRGB...

data = img.to_numpy()
assert data.flags['C_CONTIGUOUS']
```

### Zero-Copy (when possible)

```python
# to_numpy() returns a view when data is compatible
data = img.to_numpy()
data *= 2.0  # Modifies original!

# Force copy
data = img.to_numpy().copy()
```

## Multi-Layer EXR

```python
import vfx_rs

# Read all layers
layers = vfx_rs.read_layers("render.exr")
for name, layer in layers.items():
    print(f"{name}: {layer.width}x{layer.height}")

# Read specific layer
beauty = vfx_rs.read_layer("render.exr", "beauty")

# Write layers
vfx_rs.write_layers("output.exr", {
    "beauty": beauty_img,
    "diffuse": diffuse_img,
    "specular": specular_img,
})
```

## Error Handling

```python
import vfx_rs

try:
    img = vfx_rs.read("missing.exr")
except vfx_rs.IoError as e:
    print(f"I/O error: {e}")
except vfx_rs.FormatError as e:
    print(f"Format error: {e}")
```

## Performance Tips

1. **Use in-place operations** - Avoid copies
2. **Process as float32** - Native format
3. **Batch operations** - Minimize Python↔Rust crossings
4. **Use NumPy views** - Zero-copy when possible

```python
# Good: single Rust call for entire image
vfx_rs.color.apply_srgb_eotf(data)

# Bad: Python loop with per-pixel calls
for y in range(height):
    for x in range(width):
        data[y, x] = vfx_rs.color.srgb_eotf(data[y, x])
```

## Dependencies

- `vfx-core`, `vfx-io`, `vfx-color`, `vfx-lut`, `vfx-ops`
- `pyo3` - Python bindings
- `numpy` - NumPy integration

## Building

Requires:
- Rust toolchain
- Python 3.8+
- maturin (`pip install maturin`)

```bash
# Development (editable install)
maturin develop

# Release wheel
maturin build --release
```
