#!/usr/bin/env python3
"""
Example 9: Color Space Conversions

Convert between sRGB, linear, and other color spaces.
"""
import _bootstrap  # noqa: F401 - ensures venv Python
from pathlib import Path
import vfx_rs
from vfx_rs import ops

TEST_DIR = Path(__file__).parent.parent
INPUT = TEST_DIR / "owl.exr"
OUTPUT_DIR = TEST_DIR / "out" / "examples"
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

img = vfx_rs.read(INPUT)

# Convert linear to sRGB (for display)
srgb = ops.linear_to_srgb(img)
vfx_rs.write(OUTPUT_DIR / "color_srgb.exr", srgb)

# Convert sRGB to linear (for compositing)
linear = ops.srgb_to_linear(srgb)
vfx_rs.write(OUTPUT_DIR / "color_linear.exr", linear)

# Apply color matrix transform (example: RGB to YUV-ish)
# Row-major 4x4 matrix
yuv_matrix = [
    0.299, 0.587, 0.114, 0.0,   # Y
    -0.14713, -0.28886, 0.436, 0.0,  # U
    0.615, -0.51499, -0.10001, 0.0,  # V
    0.0, 0.0, 0.0, 1.0  # A passthrough
]
yuv = ops.colormatrixtransform(img, yuv_matrix)
vfx_rs.write(OUTPUT_DIR / "color_yuv.exr", yuv)

# HDR range compression (useful for viewing HDR in LDR)
compressed = ops.rangecompress(img)
vfx_rs.write(OUTPUT_DIR / "color_rangecompress.exr", compressed)

# And expand back
expanded = ops.rangeexpand(compressed)
vfx_rs.write(OUTPUT_DIR / "color_rangeexpand.exr", expanded)

# Color map (false color visualization)
gray = img.copy()
vfx_rs.Processor().saturation(gray, 0.0)
heatmap = ops.color_map(gray, map_name=ops.ColorMapName.Inferno)
vfx_rs.write(OUTPUT_DIR / "color_heatmap.exr", heatmap)

print("Color space examples written to:", OUTPUT_DIR)
