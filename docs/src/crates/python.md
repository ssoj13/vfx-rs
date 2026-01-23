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

# Access as NumPy array (copies data)
data = img.numpy()  # Shape: (height, width, channels)

# Process
data *= 1.5  # Exposure adjustment

# Create from NumPy (copies data)
result = vfx_rs.Image(data)

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
arr = img.numpy()              # Returns np.ndarray (H, W, C), copies data
img = vfx_rs.Image(arr)        # From np.ndarray, copies data
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
```

## Color Transforms

```python
import vfx_rs.color as color

# Transfer functions
linear = color.srgb_eotf(0.5)
encoded = color.srgb_oetf(linear)

# Batch processing
data = img.numpy()
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
pixels = img.numpy().reshape(-1, 3)
result = pixels @ matrix.T
```

## LUT Processing

```python
import vfx_rs.lut as lut

# Load LUT
lut_3d = lut.read_cube_3d("grade.cube")
lut_1d = lut.read_cube_1d("gamma.cube")

# Apply
data = img.numpy()
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
data = img.numpy()
proc.apply(data)

# Display processor
proc = config.display_processor("ACEScg", "sRGB", "Film")
```

## NumPy Integration

### Data Types

```python
import numpy as np
import vfx_rs

# Only float32 is currently supported
data = np.zeros((1080, 1920, 3), dtype=np.float32)
img = vfx_rs.Image(data)
```

**Note:** Only `np.float32` dtype is currently supported. Other dtypes (uint8, uint16) are not yet implemented.

### Memory Layout

```python
# vfx-rs uses C-contiguous, interleaved layout
# Shape: (height, width, channels)
# Memory: RGBRGBRGB...

data = img.numpy()
assert data.flags['C_CONTIGUOUS']
```

### Copy Behavior

Data is always copied between Python and Rust:

```python
# img.numpy() always copies data
data = img.numpy()
data *= 2.0  # Does NOT modify original image

# To apply changes, create new Image
modified = vfx_rs.Image(data)
```

## Multi-Layer EXR

```python
import vfx_rs

# Read all layers as LayeredImage
layered = vfx_rs.read_layered("render.exr")

# Access layers by name
beauty = layered["beauty"]
depth = layered["depth"]

# List available layers
for name in layered.layer_names():
    layer = layered[name]
    print(f"{name}: {layer.width}x{layer.height}")
```

**Note:** `write_layers` and `read_layer` (single layer) are not yet implemented.

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

1. **Process as float32** - Native format
2. **Batch operations** - Minimize Python↔Rust crossings
3. **Use Image methods** - Operations like `blur()`, `resize()` stay in Rust

```python
# Good: single Rust call for entire image
vfx_rs.color.apply_srgb_eotf(data)

# Good: use Image methods (stays in Rust)
result = img.blur(sigma=2.0)

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
