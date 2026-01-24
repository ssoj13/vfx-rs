#!/usr/bin/env python3
"""
Example 1: Basic EXR I/O

Read EXR files, inspect properties, and write to various formats.
"""
import _bootstrap  # noqa: F401 - ensures venv Python
from pathlib import Path
import vfx_rs

# Paths
TEST_DIR = Path(__file__).parent.parent
INPUT = TEST_DIR / "owl.exr"
OUTPUT_DIR = TEST_DIR / "out" / "examples"
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

# Read an EXR file
img = vfx_rs.read(INPUT)
print(f"Loaded: {INPUT.name}")
print(f"  Size: {img.width}x{img.height}")
print(f"  Channels: {img.channels}")
print(f"  Format: {img.format}")

# Write to different formats
vfx_rs.write(OUTPUT_DIR / "output.exr", img)   # OpenEXR (HDR)
vfx_rs.write(OUTPUT_DIR / "output.png", img)   # PNG (8-bit)
vfx_rs.write(OUTPUT_DIR / "output.jpg", img)   # JPEG (lossy)
vfx_rs.write(OUTPUT_DIR / "output.tiff", img)  # TIFF (16-bit)
vfx_rs.write(OUTPUT_DIR / "output.dpx", img)   # DPX (10-bit film)
vfx_rs.write(OUTPUT_DIR / "output.hdr", img)   # Radiance HDR

print(f"\nWritten to: {OUTPUT_DIR}")
