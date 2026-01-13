#!/usr/bin/env python3
"""Final CDL verification: compare OCIO CDL output with our fast_pow implementation."""

import numpy as np
import struct

try:
    import PyOpenColorIO as ocio
except ImportError:
    import opencolorio as ocio

# OCIO polynomial coefficients (identical to sse_math.rs)
PNLOG5 = 4.487361286440374006195e-2
PNLOG4 = -4.165637071209677112635e-1
PNLOG3 = 1.631148826119436277100
PNLOG2 = -3.550793018041176193407
PNLOG1 = 5.091710879305474367557
PNLOG0 = -2.800364054395965731506

PNEXP4 = 1.353416792833547468620e-2
PNEXP3 = 5.201146058412685018921e-2
PNEXP2 = 2.414427569091865207710e-1
PNEXP1 = 6.930038344665415134202e-1
PNEXP0 = 1.000002593370603213644

EXP_MASK = 0x7F800000
EXP_BIAS = 127
EXP_SHIFT = 23

def fast_log2(x):
    """OCIO polynomial log2 approximation (scalar)."""
    if x <= 0:
        return float('-inf')
    
    bits = struct.unpack('I', struct.pack('f', x))[0]
    
    # Extract mantissa in [1, 2)
    mantissa_bits = (bits & ~EXP_MASK) | (EXP_BIAS << EXP_SHIFT)
    mantissa = struct.unpack('f', struct.pack('I', mantissa_bits))[0]
    
    # Polynomial evaluation
    log2_mantissa = PNLOG0 + mantissa * (
        PNLOG1 + mantissa * (
            PNLOG2 + mantissa * (
                PNLOG3 + mantissa * (
                    PNLOG4 + mantissa * PNLOG5
                )
            )
        )
    )
    
    # Extract exponent
    exponent = ((bits & EXP_MASK) >> EXP_SHIFT) - EXP_BIAS
    
    return log2_mantissa + exponent

def fast_exp2(x):
    """OCIO polynomial exp2 approximation (scalar)."""
    if x < -126:
        return 0.0
    if x >= 128:
        return float('inf')
    
    import math
    
    # floor with proper negative handling
    if x >= 0:
        floor_x = int(x)
    else:
        floor_x = int(x) - 1
    
    fraction = x - floor_x
    
    # exp2(fraction) using polynomial
    mexp = PNEXP0 + fraction * (
        PNEXP1 + fraction * (
            PNEXP2 + fraction * (
                PNEXP3 + fraction * PNEXP4
            )
        )
    )
    
    # exp2(floor_x) by setting exponent bits
    zf_bits = ((floor_x + EXP_BIAS) << EXP_SHIFT) & 0xFFFFFFFF
    zf = struct.unpack('f', struct.pack('I', zf_bits))[0]
    
    return zf * mexp

def fast_pow(base, exp):
    """OCIO ssePower implementation."""
    if base <= 0:
        return 0.0
    return fast_exp2(exp * fast_log2(base))

# Luma weights (Rec.709)
LUMA_R = np.float32(0.2126)
LUMA_G = np.float32(0.7152)
LUMA_B = np.float32(0.0722)

def apply_cdl_manual(pixels, slope, offset, power, sat):
    """Apply CDL using our fast_pow (matching Rust implementation)."""
    result = pixels.copy()
    
    for i in range(len(result)):
        rgb = result[i].copy()
        
        # SOP per channel
        for c in range(3):
            v = rgb[c] * slope[c] + offset[c]
            v = max(0.0, min(1.0, v))  # clamp [0,1]
            if power[c] != 1.0:
                rgb[c] = fast_pow(v, power[c])
            else:
                rgb[c] = v
        
        # Saturation - OCIO-compatible order: multiply then sum
        if abs(sat - 1.0) > 1e-9:
            src = rgb.copy()  # save original
            wr = np.float32(src[0]) * LUMA_R
            wg = np.float32(src[1]) * LUMA_G
            wb = np.float32(src[2]) * LUMA_B
            luma = wr + wg + wb
            rgb[0] = luma + sat * (src[0] - luma)
            rgb[1] = luma + sat * (src[1] - luma)
            rgb[2] = luma + sat * (src[2] - luma)
        
        # Final clamp
        result[i] = np.clip(rgb, 0, 1)
    
    return result

def apply_cdl_ocio(pixels, slope, offset, power, sat):
    """Apply CDL using OCIO."""
    config = ocio.Config.CreateRaw()
    cdl = ocio.CDLTransform()
    cdl.setSlope(slope)
    cdl.setOffset(offset)
    cdl.setPower(power)
    cdl.setSat(sat)
    cdl.setStyle(ocio.CDL_ASC)
    
    processor = config.getProcessor(cdl)
    cpu = processor.getDefaultCPUProcessor()
    
    result = pixels.copy()
    cpu.applyRGB(result)
    return result

def generate_test_cube(size=8):
    """Generate RGB test cube."""
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

def test_cdl(name, slope, offset, power, sat):
    """Test CDL configuration."""
    print(f"\n{'='*60}")
    print(f"CDL: {name}")
    print(f"  slope={slope}, offset={offset}")
    print(f"  power={power}, sat={sat}")
    print(f"{'='*60}")
    
    pixels = generate_test_cube(8)
    
    ocio_result = apply_cdl_ocio(pixels, slope, offset, power, sat)
    manual_result = apply_cdl_manual(pixels, slope, offset, power, sat)
    
    diff = np.abs(ocio_result - manual_result)
    max_diff = np.max(diff)
    mean_diff = np.mean(diff)
    
    print(f"\nMax diff:  {max_diff:.2e}")
    print(f"Mean diff: {mean_diff:.2e}")
    
    # Count exact matches
    exact = np.sum(ocio_result == manual_result)
    total = ocio_result.size
    print(f"Exact matches: {exact}/{total} ({100*exact/total:.1f}%)")
    
    # Find worst pixel
    if max_diff > 0:
        worst_flat = np.argmax(diff.flatten())
        worst_pixel = worst_flat // 3
        worst_channel = worst_flat % 3
        
        print(f"\nWorst case:")
        print(f"  Pixel {worst_pixel}, channel {worst_channel}")
        print(f"  Input:  {pixels[worst_pixel]}")
        print(f"  OCIO:   {ocio_result[worst_pixel]}")
        print(f"  Manual: {manual_result[worst_pixel]}")
        print(f"  Diff:   {diff[worst_pixel]}")
    
    # Bit-level analysis
    bits_diff = []
    for i in range(len(ocio_result)):
        for c in range(3):
            o_bits = struct.unpack('I', struct.pack('f', ocio_result[i, c]))[0]
            m_bits = struct.unpack('I', struct.pack('f', manual_result[i, c]))[0]
            if o_bits != m_bits:
                bit_diff = abs(o_bits - m_bits)
                bits_diff.append(bit_diff)
    
    if bits_diff:
        print(f"\nBit-level analysis:")
        print(f"  Non-identical values: {len(bits_diff)}/{total}")
        print(f"  Max ULP diff: {max(bits_diff)}")
        print(f"  Mean ULP diff: {sum(bits_diff)/len(bits_diff):.1f}")
    else:
        print(f"\nBit-perfect match!")
    
    return max_diff < 1e-5  # acceptable threshold

# Run tests
print("CDL Final Verification: OCIO vs fast_pow implementation")
print("="*60)

all_pass = True
all_pass &= test_cdl("identity", [1,1,1], [0,0,0], [1,1,1], 1.0)
all_pass &= test_cdl("warmup", [1.1, 1.0, 0.9], [0.02, 0, -0.02], [1,1,1], 1.1)
all_pass &= test_cdl("contrast", [1,1,1], [-0.1, -0.1, -0.1], [1.2, 1.2, 1.2], 1.0)
all_pass &= test_cdl("full_cdl", [1.2, 0.9, 1.1], [0.05, -0.03, 0.02], [1.1, 1.3, 0.9], 1.15)
all_pass &= test_cdl("extreme_power", [1,1,1], [0,0,0], [2.2, 0.45, 1.8], 1.0)

print(f"\n{'='*60}")
print(f"OVERALL: {'PASS' if all_pass else 'FAIL'}")
print(f"{'='*60}")
