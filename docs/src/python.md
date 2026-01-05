# Python API

vfx-rs provides Python bindings via PyO3 for pipeline integration.

## Installation

```bash
# Build from source
cd crates/vfx-rs-py
pip install maturin
maturin develop --release

# Or build wheel
maturin build --release
pip install target/wheels/vfx_rs-*.whl
```

## Quick Start

```python
import vfx

# Read image
img = vfx.read("render.exr")
print(f"Size: {img.width}x{img.height}, channels: {img.channels}")

# Write image
vfx.write("output.png", img)

# Open viewer
vfx.view("render.exr")
```

## API Reference

### Reading/Writing

```python
# Read any supported format
img = vfx.read("input.exr")
img = vfx.read("photo.jpg")

# Write with format auto-detection
vfx.write("output.png", img)
vfx.write("output.exr", img)
```

### Image Data

```python
img = vfx.read("input.exr")

# Properties
img.width      # int
img.height     # int
img.channels   # int
img.format     # str: "F32", "F16", "U8", "U16"

# Get numpy array (zero-copy when possible)
import numpy as np
arr = img.to_numpy()  # shape: (height, width, channels)

# Create from numpy
arr = np.random.rand(1080, 1920, 4).astype(np.float32)
img = vfx.ImageData.from_numpy(arr)
```

### Viewer

```python
# Simple view
vfx.view("render.exr")

# With options
vfx.view("render.exr", display="sRGB", view="ACES 1.0 - SDR Video")
```

## NumPy Integration

```python
import vfx
import numpy as np

# Load EXR as numpy array
img = vfx.read("render.exr")
arr = img.to_numpy()

# Process with numpy/scipy
from scipy.ndimage import gaussian_filter
blurred = gaussian_filter(arr, sigma=2)

# Save back
result = vfx.ImageData.from_numpy(blurred.astype(np.float32))
vfx.write("blurred.exr", result)
```

## Pipeline Example

```python
import vfx
import numpy as np
from pathlib import Path

def process_sequence(input_dir, output_dir):
    """Batch process EXR sequence with exposure adjustment."""
    input_dir = Path(input_dir)
    output_dir = Path(output_dir)
    output_dir.mkdir(exist_ok=True)
    
    for exr in sorted(input_dir.glob("*.exr")):
        img = vfx.read(str(exr))
        arr = img.to_numpy()
        
        # Exposure +1 stop
        arr *= 2.0
        
        result = vfx.ImageData.from_numpy(arr.astype(np.float32))
        vfx.write(str(output_dir / exr.name), result)
        print(f"Processed: {exr.name}")

process_sequence("renders/", "graded/")
```
