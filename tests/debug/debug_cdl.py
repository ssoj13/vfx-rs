#!/usr/bin/env python3
"""Debug CDL comparison between OCIO and expected values."""

import numpy as np

try:
    import PyOpenColorIO as ocio
except ImportError:
    import opencolorio as ocio

def generate_rgb_cube(size=8):
    """Generate RGB color cube - must match Rust exactly."""
    r = np.linspace(0, 1, size, dtype=np.float32)
    g = np.linspace(0, 1, size, dtype=np.float32)
    b = np.linspace(0, 1, size, dtype=np.float32)
    rr, gg, bb = np.meshgrid(r, g, b, indexing='ij')
    return np.stack([rr.flatten(), gg.flatten(), bb.flatten()], axis=-1)

def apply_cdl_ocio(slope, offset, power, sat, pixels):
    """Apply CDL using OCIO with CDL_ASC style."""
    config = ocio.Config.CreateRaw()
    
    transform = ocio.CDLTransform()
    transform.setSlope(slope)
    transform.setOffset(offset)
    transform.setPower(power)
    transform.setSat(sat)
    transform.setStyle(ocio.CDL_ASC)  # Clamped mode
    
    processor = config.getProcessor(transform)
    cpu = processor.getDefaultCPUProcessor()
    
    result = pixels.copy()
    flat = result.reshape(-1, 3).copy()
    cpu.applyRGB(flat)
    return flat.reshape(result.shape)

def apply_cdl_manual(slope, offset, power, sat, pixels):
    """Apply CDL manually (ASC CDL v1.2 spec)."""
    result = pixels.copy()
    
    for i in range(len(result)):
        rgb = result[i].copy()
        
        # SOP per channel
        for c in range(3):
            v = rgb[c] * slope[c] + offset[c]
            v = np.clip(v, 0, 1)  # Clamp BEFORE power
            rgb[c] = v ** power[c]
        
        # Saturation
        if abs(sat - 1.0) > 1e-6:
            luma = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]
            for c in range(3):
                rgb[c] = luma + (rgb[c] - luma) * sat
        
        # Final clamp
        result[i] = np.clip(rgb, 0, 1)
    
    return result

# Test with contrast CDL
print("Testing CDL contrast:")
slope = [1.0, 1.0, 1.0]
offset = [-0.1, -0.1, -0.1]
power = [1.2, 1.2, 1.2]
sat = 1.0

pixels = generate_rgb_cube(8)
print(f"Input shape: {pixels.shape}")
print(f"Input first 5: {pixels[:5]}")

ocio_result = apply_cdl_ocio(slope, offset, power, sat, pixels)
manual_result = apply_cdl_manual(slope, offset, power, sat, pixels)

print(f"\nOCIO result first 5: {ocio_result[:5]}")
print(f"Manual result first 5: {manual_result[:5]}")

diff = np.abs(ocio_result - manual_result)
print(f"\nMax diff: {np.max(diff)}")
print(f"Mean diff: {np.mean(diff)}")

# Check specific value
idx = 0  # (0,0,0)
print(f"\nPixel 0 (0,0,0):")
print(f"  Input: {pixels[idx]}")
print(f"  OCIO:  {ocio_result[idx]}")
print(f"  Manual: {manual_result[idx]}")

# Expected: 
# v = 0 * 1 + (-0.1) = -0.1
# clamp to [0,1]: 0
# power: 0^1.2 = 0
# sat=1.0: no change
# final clamp: 0
expected = [0.0, 0.0, 0.0]
print(f"  Expected: {expected}")

# Check another value - middle gray
idx = 256  # roughly middle
print(f"\nPixel {idx}:")
print(f"  Input: {pixels[idx]}")
print(f"  OCIO:  {ocio_result[idx]}")
print(f"  Manual: {manual_result[idx]}")
