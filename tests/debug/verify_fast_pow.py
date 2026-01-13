#!/usr/bin/env python3
"""Verify fast_pow matches OCIO ssePower exactly."""

import numpy as np
import struct

try:
    import PyOpenColorIO as ocio
except ImportError:
    import opencolorio as ocio

# OCIO polynomial coefficients (from SSE.h)
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
    """OCIO sseLog2 polynomial approximation (scalar)."""
    if x <= 0:
        return float('-inf')
    
    # Get float bits
    bits = struct.unpack('I', struct.pack('f', float(x)))[0]
    
    # Extract mantissa and set exponent to 0 (value in [1, 2))
    mantissa_bits = (bits & ~EXP_MASK) | (EXP_BIAS << EXP_SHIFT)
    mantissa = struct.unpack('f', struct.pack('I', mantissa_bits))[0]
    
    # Polynomial evaluation (Horner's method, same order as OCIO)
    log2 = PNLOG0 + mantissa * (
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
    
    return log2 + exponent

def fast_exp2(x):
    """OCIO sseExp2 polynomial approximation (scalar)."""
    if x < -126:
        return 0.0
    if x >= 128:
        return float('inf')
    
    import math
    
    # Floor with proper negative handling (matches OCIO)
    floor_x = int(math.floor(x))
    fraction = x - floor_x
    
    # Polynomial for exp2(fraction)
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

def test_against_ocio_cdl():
    """Compare fast_pow with OCIO CDL power."""
    print("=" * 60)
    print("Testing fast_pow vs OCIO CDL")
    print("=" * 60)
    
    config = ocio.Config.CreateRaw()
    
    # CDL with power != 1
    cdl = ocio.CDLTransform()
    cdl.setSlope([1, 1, 1])
    cdl.setOffset([0, 0, 0])
    cdl.setPower([1.2, 1.2, 1.2])
    cdl.setSat(1.0)
    
    processor = config.getProcessor(cdl)
    cpu = processor.getDefaultCPUProcessor()
    
    # Test values
    test_vals = np.linspace(0.01, 1.0, 100, dtype=np.float32)
    
    max_diff = 0
    worst_case = None
    
    for v in test_vals:
        # OCIO result
        pixel = np.array([[v, v, v]], dtype=np.float32)
        cpu.applyRGB(pixel)
        ocio_result = pixel[0, 0]
        
        # fast_pow result  
        fast_result = fast_pow(float(v), 1.2)
        
        diff = abs(ocio_result - fast_result)
        if diff > max_diff:
            max_diff = diff
            worst_case = (v, ocio_result, fast_result, diff)
    
    print(f"Max diff: {max_diff:.2e}")
    if worst_case:
        print(f"Worst: {worst_case[0]:.6f}^1.2")
        print(f"  OCIO:     {worst_case[1]:.10f}")
        print(f"  fast_pow: {worst_case[2]:.10f}")
        print(f"  diff:     {worst_case[3]:.2e}")
    
    # Test specific value from earlier
    print("\nSpecific test: 0.7571428418^1.2")
    base = 0.7571428418
    
    pixel = np.array([[base, base, base]], dtype=np.float32)
    cpu.applyRGB(pixel)
    ocio_v = pixel[0, 0]
    
    fast_v = fast_pow(base, 1.2)
    std_v = base ** 1.2
    
    print(f"  OCIO:       {ocio_v:.10f}")
    print(f"  fast_pow:   {fast_v:.10f}")
    print(f"  std powf:   {std_v:.10f}")
    print(f"  diff OCIO-fast: {abs(ocio_v - fast_v):.2e}")
    print(f"  diff OCIO-std:  {abs(ocio_v - std_v):.2e}")
    
    return max_diff

def test_different_powers():
    """Test various power values."""
    print("\n" + "=" * 60)
    print("Testing different power values")
    print("=" * 60)
    
    config = ocio.Config.CreateRaw()
    
    powers = [0.5, 0.8, 1.0, 1.2, 1.5, 2.0, 2.4]
    base = 0.5
    
    for power in powers:
        cdl = ocio.CDLTransform()
        cdl.setSlope([1, 1, 1])
        cdl.setOffset([0, 0, 0])
        cdl.setPower([power, power, power])
        cdl.setSat(1.0)
        
        processor = config.getProcessor(cdl)
        cpu = processor.getDefaultCPUProcessor()
        
        pixel = np.array([[base, base, base]], dtype=np.float32)
        cpu.applyRGB(pixel)
        ocio_v = pixel[0, 0]
        
        fast_v = fast_pow(base, power)
        
        diff = abs(ocio_v - fast_v)
        status = "[OK]" if diff < 1e-6 else f"[DIFF: {diff:.2e}]"
        print(f"  {base}^{power}: OCIO={ocio_v:.8f}, fast={fast_v:.8f} {status}")

if __name__ == "__main__":
    max_diff = test_against_ocio_cdl()
    test_different_powers()
    
    print("\n" + "=" * 60)
    if max_diff < 1e-6:
        print("SUCCESS: fast_pow matches OCIO ssePower!")
    else:
        print(f"WARNING: max_diff = {max_diff:.2e}")
