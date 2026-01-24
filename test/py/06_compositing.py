#!/usr/bin/env python3
"""
Example 6: Compositing

Blend modes, alpha compositing, and layer merging.
"""
import _bootstrap  # noqa: F401 - ensures venv Python
from pathlib import Path
import numpy as np
import vfx_rs
from vfx_rs import ops

TEST_DIR = Path(__file__).parent.parent
INPUT = TEST_DIR / "owl.exr"
OUTPUT_DIR = TEST_DIR / "out" / "examples"
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

# Load base image
img = vfx_rs.read(INPUT)

# Create a colored overlay
overlay_arr = np.zeros((img.height, img.width, 4), dtype=np.float32)
overlay_arr[:, :, 0] = 0.8  # Orange tint
overlay_arr[:, :, 1] = 0.4
overlay_arr[:, :, 2] = 0.1
overlay_arr[:, :, 3] = 0.3  # Semi-transparent
overlay = vfx_rs.Image(overlay_arr)

# Porter-Duff compositing
result = ops.over(overlay, img)  # Overlay over background
vfx_rs.write(OUTPUT_DIR / "comp_over.exr", result)

# Blend modes
screen_blend = ops.screen(img, overlay)
vfx_rs.write(OUTPUT_DIR / "comp_screen.exr", screen_blend)

multiply_blend = ops.multiply(img, overlay)
vfx_rs.write(OUTPUT_DIR / "comp_multiply.exr", multiply_blend)

overlay_blend = ops.overlay(img, overlay)
vfx_rs.write(OUTPUT_DIR / "comp_overlay.exr", overlay_blend)

softlight_blend = ops.softlight(img, overlay)
vfx_rs.write(OUTPUT_DIR / "comp_softlight.exr", softlight_blend)

# Create difference mask between two images
proc = vfx_rs.Processor()
modified = img.copy()
proc.exposure(modified, 0.5)
diff = ops.absdiff(img, modified)
vfx_rs.write(OUTPUT_DIR / "comp_difference.exr", diff)

# Additive blend (good for light effects)
glow = img.copy()
proc.exposure(glow, -2.0)  # Dim it
additive = ops.add_blend(img, glow)
vfx_rs.write(OUTPUT_DIR / "comp_additive.exr", additive)

print("Compositing examples written to:", OUTPUT_DIR)
