#!/usr/bin/env python3
"""Compare OCIO CDL output with expected Rust output."""

import numpy as np
import subprocess
import sys

try:
    import PyOpenColorIO as ocio
except ImportError:
    import opencolorio as ocio

def generate_rgb_cube(size=8):
    """Generate RGB color cube - must match Rust exactly."""
    cube = []
    for r in range(size):
        for g in range(size):
            for b in range(size):
                cube.append([
                    r / (size - 1),
                    g / (size - 1),
                    b / (size - 1),
                ])
    return np.array(cube, dtype=np.float32)

def apply_cdl_ocio(slope, offset, power, sat, pixels, style=ocio.CDL_ASC):
    """Apply CDL using OCIO."""
    config = ocio.Config.CreateRaw()
    
    transform = ocio.CDLTransform()
    transform.setSlope(slope)
    transform.setOffset(offset)
    transform.setPower(power)
    transform.setSat(sat)
    transform.setStyle(style)
    
    processor = config.getProcessor(transform)
    cpu = processor.getDefaultCPUProcessor()
    
    result = pixels.copy()
    cpu.applyRGB(result)
    return result

def apply_cdl_rust_style(slope, offset, power, sat, pixels):
    """Apply CDL using Rust-style implementation."""
    result = pixels.copy()
    
    for i in range(len(result)):
        rgb = result[i].copy()
        
        # SOP per channel
        for c in range(3):
            v = rgb[c] * slope[c] + offset[c]
            # Clamp to [0,1] BEFORE power (ASC CDL v1.2)
            v = np.clip(v, 0, 1)
            rgb[c] = v ** power[c]
        
        # Saturation
        if abs(sat - 1.0) > 1e-6:
            luma = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]
            for c in range(3):
                rgb[c] = luma + (rgb[c] - luma) * sat
        
        # Final clamp after saturation
        result[i] = np.clip(rgb, 0, 1)
    
    return result

def compare_cdl(name, slope, offset, power, sat):
    """Compare OCIO and Rust-style CDL."""
    print(f"\n{'='*60}")
    print(f"CDL: {name}")
    print(f"  slope={slope}, offset={offset}, power={power}, sat={sat}")
    print(f"{'='*60}")
    
    pixels = generate_rgb_cube(8)
    
    ocio_result = apply_cdl_ocio(slope, offset, power, sat, pixels)
    rust_result = apply_cdl_rust_style(slope, offset, power, sat, pixels)
    
    diff = np.abs(ocio_result - rust_result)
    
    print(f"\nMax diff: {np.max(diff):.10f}")
    print(f"Mean diff: {np.mean(diff):.10f}")
    
    # Find worst pixel
    worst_idx = np.unravel_index(np.argmax(diff), diff.shape)
    pixel_idx = worst_idx[0]
    
    if np.max(diff) > 1e-6:
        print(f"\nWorst pixel [{pixel_idx}]:")
        print(f"  Input:  {pixels[pixel_idx]}")
        print(f"  OCIO:   {ocio_result[pixel_idx]}")
        print(f"  Rust:   {rust_result[pixel_idx]}")
        
        # Step by step
        rgb_in = pixels[pixel_idx].copy()
        print(f"\n  Step-by-step for channel {worst_idx[1]}:")
        c = worst_idx[1]
        v = rgb_in[c]
        print(f"    Input: {v:.10f}")
        v_slope = v * slope[c]
        print(f"    After slope: {v_slope:.10f}")
        v_offset = v_slope + offset[c]
        print(f"    After offset: {v_offset:.10f}")
        v_clamp = np.clip(v_offset, 0, 1)
        print(f"    After clamp [0,1]: {v_clamp:.10f}")
        v_power = v_clamp ** power[c]
        print(f"    After power: {v_power:.10f}")
    else:
        print("\nPerfect match!")
    
    # Hash comparison
    import hashlib
    def compute_hash(data):
        q = np.round(data.flatten() * 1e5).astype(np.int64)
        return hashlib.sha256(q.tobytes()).hexdigest()
    
    print(f"\nHashes:")
    print(f"  OCIO: {compute_hash(ocio_result)}")
    print(f"  Rust: {compute_hash(rust_result)}")

# Test all CDL configs
compare_cdl("identity", [1,1,1], [0,0,0], [1,1,1], 1.0)
compare_cdl("warmup", [1.1, 1.0, 0.9], [0.02, 0, -0.02], [1,1,1], 1.1)
compare_cdl("contrast", [1,1,1], [-0.1, -0.1, -0.1], [1.2, 1.2, 1.2], 1.0)
