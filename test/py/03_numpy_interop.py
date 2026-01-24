#!/usr/bin/env python3
"""
Example 3: NumPy Interoperability

Access EXR pixel data as NumPy arrays and create images from arrays.
"""
import _bootstrap  # noqa: F401 - ensures venv Python
from pathlib import Path
import numpy as np
import vfx_rs

TEST_DIR = Path(__file__).parent.parent
INPUT = TEST_DIR / "owl.exr"
OUTPUT_DIR = TEST_DIR / "out" / "examples"
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

# Read EXR and get numpy array
img = vfx_rs.read(INPUT)
arr = img.numpy()  # Shape: (height, width, channels), dtype: float32
print(f"Array shape: {arr.shape}, dtype: {arr.dtype}")

# Modify pixel data directly
# Example: apply a vignette effect
h, w, c = arr.shape
y, x = np.ogrid[:h, :w]
cx, cy = w / 2, h / 2
dist = np.sqrt((x - cx)**2 + (y - cy)**2)
max_dist = np.sqrt(cx**2 + cy**2)
vignette = 1 - (dist / max_dist) ** 2 * 0.6  # Darken edges

vignetted = (arr * vignette[:, :, np.newaxis]).astype(np.float32)
vfx_rs.write(OUTPUT_DIR / "numpy_vignette.exr", vfx_rs.Image(vignetted))

# Create synthetic image from scratch
gradient = np.zeros((512, 512, 4), dtype=np.float32)
gradient[:, :, 0] = np.linspace(0, 1, 512).reshape(1, 512)  # R: horizontal
gradient[:, :, 1] = np.linspace(0, 1, 512).reshape(512, 1)  # G: vertical
gradient[:, :, 2] = 0.5  # B: constant
gradient[:, :, 3] = 1.0  # A: opaque
vfx_rs.write(OUTPUT_DIR / "numpy_gradient.exr", vfx_rs.Image(gradient))

# Channel manipulation: swap R and B
swapped = arr.copy()
swapped[:, :, 0], swapped[:, :, 2] = arr[:, :, 2].copy(), arr[:, :, 0].copy()
vfx_rs.write(OUTPUT_DIR / "numpy_rgb_to_bgr.exr", vfx_rs.Image(swapped))

print("NumPy examples written to:", OUTPUT_DIR)
