#!/usr/bin/env python3
"""
Example 5: Multi-layer EXR

Read and manipulate multi-layer EXR files (render passes).
"""
import _bootstrap  # noqa: F401 - ensures venv Python
from pathlib import Path
import vfx_rs

TEST_DIR = Path(__file__).parent.parent
INPUT = TEST_DIR / "owl.exr"
OUTPUT_DIR = TEST_DIR / "out" / "examples"
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

# Read as layered image
layered = vfx_rs.read_layered(INPUT)
print(f"Layer count: {len(layered)}")
print(f"Layer names: {layered.layer_names}")

# Access first layer
first_layer = layered[0]
print(f"\nFirst layer: '{first_layer.name}'")
print(f"  Channels: {first_layer.channel_names}")

# Convert layer to regular image
img = first_layer.to_image()
print(f"  Image: {img.width}x{img.height}, {img.channels} channels")

# Access individual channels
for ch_name in first_layer.channel_names[:4]:  # First 4 channels
    ch = first_layer[ch_name]
    arr = ch.numpy()
    print(f"  Channel '{ch_name}': min={arr.min():.3f}, max={arr.max():.3f}")

# Create a new layered image with multiple passes
new_layered = vfx_rs.LayeredImage()
proc = vfx_rs.Processor()

# Add beauty pass
new_layered.add_layer("beauty", img)

# Add desaturated version as a pass
desat = img.copy()
proc.saturation(desat, 0.0)
new_layered.add_layer("desaturated", desat)

# Add exposure variants
for stops in [-1, 0, 1]:
    variant = img.copy()
    proc.exposure(variant, float(stops))
    new_layered.add_layer(f"exposure_{stops:+d}", variant)

print(f"\nCreated layered image with {len(new_layered)} layers")
print(f"Layers: {new_layered.layer_names}")
