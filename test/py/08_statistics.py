#!/usr/bin/env python3
"""
Example 8: Image Statistics

Analyze image data: min/max, mean, histogram, etc.
"""
import _bootstrap  # noqa: F401 - ensures venv Python
from pathlib import Path
import numpy as np
import vfx_rs

TEST_DIR = Path(__file__).parent.parent
INPUT = TEST_DIR / "owl.exr"

img = vfx_rs.read(INPUT)
arr = img.numpy()

print(f"Image: {INPUT.name}")
print(f"Size: {img.width}x{img.height}, {img.channels} channels")
print()

# Per-channel statistics using numpy
for i, name in enumerate(['R', 'G', 'B', 'A'][:img.channels]):
    ch = arr[:, :, i]
    print(f"Channel {name}:")
    print(f"  Min: {ch.min():.4f}")
    print(f"  Max: {ch.max():.4f}")
    print(f"  Mean: {ch.mean():.4f}")
    print(f"  Std: {ch.std():.4f}")
    print()

# Check for HDR values (> 1.0)
hdr_pixels = np.sum(arr[:, :, :3] > 1.0)
total_pixels = img.width * img.height * 3
print(f"HDR pixels (>1.0): {hdr_pixels} ({100*hdr_pixels/total_pixels:.2f}%)")

# Check for negative values
neg_pixels = np.sum(arr[:, :, :3] < 0.0)
print(f"Negative pixels: {neg_pixels}")

# Check for NaN/Inf
nan_count = np.sum(np.isnan(arr))
inf_count = np.sum(np.isinf(arr))
print(f"NaN pixels: {nan_count}")
print(f"Inf pixels: {inf_count}")

# Luminance
if img.channels >= 3:
    lum = 0.2126 * arr[:, :, 0] + 0.7152 * arr[:, :, 1] + 0.0722 * arr[:, :, 2]
    print(f"\nLuminance:")
    print(f"  Min: {lum.min():.4f}")
    print(f"  Max: {lum.max():.4f}")
    print(f"  Mean: {lum.mean():.4f}")
