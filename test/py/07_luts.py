#!/usr/bin/env python3
"""
Example 7: LUT Processing

Load and apply 1D and 3D LUTs for color transforms.
"""
import _bootstrap  # noqa: F401 - ensures venv Python
from pathlib import Path
import numpy as np
import vfx_rs
from vfx_rs import lut

TEST_DIR = Path(__file__).parent.parent
INPUT = TEST_DIR / "owl.exr"
OUTPUT_DIR = TEST_DIR / "out" / "examples"
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

img = vfx_rs.read(INPUT)

# Create a simple contrast 1D LUT
lut_size = 65
lut_data = np.zeros((lut_size, 3), dtype=np.float32)
for i in range(lut_size):
    t = i / (lut_size - 1)
    # S-curve for contrast
    s = t ** 1.1  # Slight gamma
    lut_data[i] = [s, s, s]

lut1d = lut.Lut1D(lut_data)
result = lut.apply_lut1d(img, lut1d)
vfx_rs.write(OUTPUT_DIR / "lut_1d_contrast.exr", result)

# Create a simple 3D LUT (warm tint)
cube_size = 17
lut3d_data = np.zeros((cube_size, cube_size, cube_size, 3), dtype=np.float32)
for b in range(cube_size):
    for g in range(cube_size):
        for r in range(cube_size):
            rf = r / (cube_size - 1)
            gf = g / (cube_size - 1)
            bf = b / (cube_size - 1)
            # Add warm tint
            lut3d_data[b, g, r] = [
                min(1.0, rf * 1.05),  # Boost red
                gf,                    # Keep green
                bf * 0.95             # Reduce blue
            ]

lut3d = lut.Lut3D(lut3d_data)
result = lut.apply_lut3d(img, lut3d)
vfx_rs.write(OUTPUT_DIR / "lut_3d_warm.exr", result)

print("LUT examples written to:", OUTPUT_DIR)
print("Note: You can also load .cube files with lut.load_cube()")
