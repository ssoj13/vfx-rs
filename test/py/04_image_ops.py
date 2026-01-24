#!/usr/bin/env python3
"""
Example 4: Image Operations

Resize, rotate, flip, blur, sharpen, and other image processing.
"""
import _bootstrap  # noqa: F401 - ensures venv Python
import math
from pathlib import Path
import vfx_rs
from vfx_rs import ops

TEST_DIR = Path(__file__).parent.parent
INPUT = TEST_DIR / "owl.exr"
OUTPUT_DIR = TEST_DIR / "out" / "examples"
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

img = vfx_rs.read(INPUT)
print(f"Original: {img.width}x{img.height}")

# Geometry transforms
flipped = ops.flip(img)  # Vertical flip
vfx_rs.write(OUTPUT_DIR / "ops_flip.exr", flipped)

flopped = ops.flop(img)  # Horizontal flip
vfx_rs.write(OUTPUT_DIR / "ops_flop.exr", flopped)

rotated = ops.rotate90(img)  # 90 degrees clockwise
vfx_rs.write(OUTPUT_DIR / "ops_rotate90.exr", rotated)

# Resize with different filters
half = ops.resize(img, img.width // 2, img.height // 2, ops.ResizeFilter.Lanczos3)
vfx_rs.write(OUTPUT_DIR / "ops_resize_half.exr", half)

# Arbitrary angle rotation (radians)
angled = ops.rotate(img, math.radians(15))
vfx_rs.write(OUTPUT_DIR / "ops_rotate_15deg.exr", angled)

# Filters
blurred = ops.blur(img, sigma=3.0)
vfx_rs.write(OUTPUT_DIR / "ops_blur.exr", blurred)

sharpened = ops.sharpen(img, amount=1.5)
vfx_rs.write(OUTPUT_DIR / "ops_sharpen.exr", sharpened)

edges = ops.sobel(img)  # Edge detection
vfx_rs.write(OUTPUT_DIR / "ops_sobel.exr", edges)

# Morphology
dilated = ops.dilate(img, size=5)
vfx_rs.write(OUTPUT_DIR / "ops_dilate.exr", dilated)

# Color operations
inverted = ops.invert(img)
vfx_rs.write(OUTPUT_DIR / "ops_invert.exr", inverted)

clamped = ops.clamp(img, 0.1, 0.9)  # Clamp to range
vfx_rs.write(OUTPUT_DIR / "ops_clamp.exr", clamped)

print("Image operations written to:", OUTPUT_DIR)
