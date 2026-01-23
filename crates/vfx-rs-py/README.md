# vfx_rs

High-performance VFX image processing library for Python.

## Installation

```bash
pip install vfx_rs
```

## Quick Start

```python
import vfx_rs
import numpy as np

# Read any supported format
img = vfx_rs.read("input.exr")

# GPU-accelerated processing
proc = vfx_rs.Processor()
proc.exposure(img, 1.5)
proc.saturation(img, 1.2)

# Save
vfx_rs.write("output.exr", img)

# numpy interop
arr = img.numpy()  # shape: (H, W, C), dtype: float32
```

## Supported Formats

- EXR (OpenEXR) - HDR/linear workflow
- PNG - lossless with alpha
- JPEG - lossy compression
- TIFF - print/archival
- DPX - film scanning (10-bit log)
- HDR - Radiance RGBE

## License

MIT OR Apache-2.0
