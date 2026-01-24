#!/usr/bin/env python3
"""
Example 2: Color Grading

Apply exposure, saturation, contrast, and CDL grades to EXR images.
"""
import _bootstrap  # noqa: F401 - ensures venv Python
from pathlib import Path
import vfx_rs

TEST_DIR = Path(__file__).parent.parent
INPUT = TEST_DIR / "owl.exr"
OUTPUT_DIR = TEST_DIR / "out" / "examples"
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

# Load image
img = vfx_rs.read(INPUT)
proc = vfx_rs.Processor()

# Exposure adjustment (in stops)
work = img.copy()
proc.exposure(work, 1.0)  # +1 stop brighter
vfx_rs.write(OUTPUT_DIR / "grade_exposure_plus1.exr", work)

# Saturation
work = img.copy()
proc.saturation(work, 1.5)  # 150% saturation
vfx_rs.write(OUTPUT_DIR / "grade_saturated.exr", work)

work = img.copy()
proc.saturation(work, 0.0)  # Black & white
vfx_rs.write(OUTPUT_DIR / "grade_bw.exr", work)

# Contrast
work = img.copy()
proc.contrast(work, 1.3)  # Higher contrast
vfx_rs.write(OUTPUT_DIR / "grade_contrast.exr", work)

# CDL (Color Decision List) - professional color grading
work = img.copy()
proc.cdl(
    work,
    slope=[1.1, 1.0, 0.9],     # Warm tint
    offset=[0.02, 0.0, -0.02],  # Lift shadows warm
    power=[1.0, 1.0, 1.0]       # Gamma
)
vfx_rs.write(OUTPUT_DIR / "grade_cdl_warm.exr", work)

# Chain multiple operations for a "cinema" look
work = img.copy()
proc.exposure(work, 0.3)
proc.contrast(work, 1.15)
proc.saturation(work, 0.9)
proc.cdl(work, slope=[1.05, 1.0, 0.95], offset=[0.01, 0.0, -0.01])
vfx_rs.write(OUTPUT_DIR / "grade_cinema.exr", work)

print("Color grading examples written to:", OUTPUT_DIR)
