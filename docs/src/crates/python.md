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
print(img.format)     # "f32", "f16", "u8", "u16", "u32"

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
import vfx_rs
import vfx_rs.ops as ops

# Image-level color transforms (returns new Image)
img = vfx_rs.read("input.exr")
linear = img.srgb_to_linear()  # sRGB -> linear
encoded = linear.linear_to_srgb()  # linear -> sRGB

# Using ops module
linear = ops.srgb_to_linear(img)
encoded = ops.linear_to_srgb(linear)

# OCIO color conversion
converted = img.colorconvert("ACEScg", "sRGB")
```

## Color Spaces

```python
import vfx_rs
import vfx_rs.ocio as ocio

# Use Image method for color space conversion
img = vfx_rs.read("input.exr")
converted = img.colorconvert("ACEScg", "sRGB")

# Or use the ocio module directly
converted = ocio.colorconvert(img, "ACEScg", "sRGB")

# With specific config
config = ocio.ColorConfig.from_file("/path/to/config.ocio")
converted = ocio.colorconvert(img, "ACEScg", "sRGB", config=config)
```

## LUT Processing

```python
import vfx_rs.lut as lut

# Load LUT
lut_3d = lut.read_cube_3d("grade.cube")
lut_1d = lut.read_cube_1d("gamma.cube")

# Apply to single pixel
rgb_out = lut_3d.apply([0.5, 0.3, 0.2])  # [f32; 3] -> [f32; 3]
val_out = lut_1d.apply(0.5)  # f32 -> f32

# For bulk processing, iterate over pixels
data = img.numpy()
for y in range(data.shape[0]):
    for x in range(data.shape[1]):
        data[y, x, :3] = lut_3d.apply(data[y, x, :3].tolist())
```

## Image Operations

```python
import vfx_rs.ops as ops
from vfx_rs.ops import ResizeFilter

# Resize (width and height are required, filter is optional enum)
resized = ops.resize(img, 1920, 1080)  # default: Bilinear
resized = ops.resize(img, 1920, 1080, ResizeFilter.Lanczos3)

# Blur (sigma parameter, not radius)
blurred = ops.blur(img, sigma=2.0)  # Gaussian blur

# Or use Image methods directly
blurred = img.blur(sigma=2.0)
resized = img.resize(1920, 1080)

# Composite (separate functions for each blend mode)
result = ops.over(foreground, background)
result = ops.multiply(a, b)
result = ops.screen(a, b)
```

## OCIO Integration

```python
import vfx_rs.ocio as ocio

# Load config (ColorConfig class, not Config)
config = ocio.ColorConfig.from_file("/path/to/config.ocio")

# Or use built-in ACES 1.3
config = ocio.ColorConfig.aces_1_3()

# Use module-level functions for transforms (no Processor class)
# colorconvert: convert between color spaces
result = ocio.colorconvert(img, "ACEScg", "sRGB")

# ociodisplay: apply display/view transform
result = ocio.ociodisplay(img, display="sRGB", view="ACES 1.0 SDR")

# ociolook: apply look
result = ocio.ociolook(img, look_name="Film", input_space="ACEScg")

# list available color spaces
colorspaces = ocio.list_colorspaces()  # with default config
colorspaces = ocio.list_colorspaces(config)  # with specific config
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

Data is always copied between Python and Rust. There is no zero-copy mode:

```python
# img.numpy() always copies data (allocates new Vec<f32>)
data = img.numpy()
data *= 2.0  # Does NOT modify original image

# To apply changes, create new Image (also copies)
modified = vfx_rs.Image(data)
```

**Note:** For best performance, minimize Python↔Rust crossings by using Image methods that stay in Rust.

## Multi-Layer EXR

```python
import vfx_rs

# Read all layers as LayeredImage
layered = vfx_rs.read_layered("render.exr")

# Access layers by name or index
beauty = layered["beauty"]
depth = layered["depth"]
first = layered[0]

# List available layers (layer_names is a property, not a method)
for name in layered.layer_names:  # no parentheses!
    layer = layered[name]
    print(f"{name}: {layer.width}x{layer.height}")

# Convert layer to flat Image
img = beauty.to_image()
```

**Note:** Only `read_layered()` is currently available. Writing multi-layer EXR is done through the Rust API.

## Error Handling

```python
import vfx_rs

try:
    img = vfx_rs.read("missing.exr")
except IOError as e:
    print(f"I/O error: {e}")
except ValueError as e:
    print(f"Value error: {e}")
```

**Note:** vfx_rs uses standard Python exceptions (`IOError`, `ValueError`) rather than custom exception types.

## Performance Tips

1. **Process as float32** - Native format
2. **Batch operations** - Minimize Python↔Rust crossings
3. **Use Image methods** - Operations like `blur()`, `resize()` stay in Rust

```python
# Good: single Rust call for entire image
linear = img.srgb_to_linear()

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
