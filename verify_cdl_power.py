#!/usr/bin/env python3
"""Compare standard powf vs OCIO ssePower polynomial approximation."""

import numpy as np

try:
    import PyOpenColorIO as ocio
except ImportError:
    import opencolorio as ocio

# OCIO polynomial coefficients from SSEMathFuncs.h
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

def fast_log2(x):
    """OCIO polynomial log2 approximation."""
    if x <= 0:
        return float('-inf')
    
    import struct
    bits = struct.unpack('I', struct.pack('f', x))[0]
    
    # Extract mantissa and set exponent to 0 (value in [1, 2))
    mantissa_bits = (bits & 0x007FFFFF) | 0x3F800000
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
    exponent = ((bits & 0x7F800000) >> 23) - 127
    
    return log2_mantissa + exponent

def fast_exp2(x):
    """OCIO polynomial exp2 approximation."""
    if x < -126:
        return 0.0
    if x >= 128:
        return float('inf')
    
    import struct
    import math
    
    # Floor with proper negative handling
    floor_x = int(math.floor(x))
    fraction = x - floor_x
    
    # exp2(fraction) using polynomial
    mexp = PNEXP0 + fraction * (
        PNEXP1 + fraction * (
            PNEXP2 + fraction * (
                PNEXP3 + fraction * PNEXP4
            )
        )
    )
    
    # exp2(floor_x) by directly setting exponent bits
    zf_bits = ((floor_x + 127) << 23) & 0xFFFFFFFF
    zf = struct.unpack('f', struct.pack('I', zf_bits))[0]
    
    return zf * mexp

def fast_pow(base, exp):
    """OCIO ssePower implementation."""
    if base <= 0:
        return 0.0
    return fast_exp2(exp * fast_log2(base))

def test_power_accuracy():
    """Test fast_pow vs standard pow."""
    print("Testing fast_pow (OCIO ssePower) vs numpy.power:")
    
    test_bases = np.linspace(0.1, 1.5, 20)
    test_exps = [0.8, 1.0, 1.2, 1.5, 2.0]
    
    max_diff = 0
    worst_case = None
    
    for base in test_bases:
        for exp in test_exps:
            standard = np.power(base, exp)
            fast = fast_pow(float(base), float(exp))
            diff = abs(standard - fast)
            
            if diff > max_diff:
                max_diff = diff
                worst_case = (base, exp, standard, fast, diff)
    
    print(f"Max diff: {max_diff:.2e}")
    if worst_case:
        print(f"Worst: {worst_case[0]:.4f}^{worst_case[1]} = {worst_case[2]:.8f} (standard) vs {worst_case[3]:.8f} (fast)")
    
    # Test CDL-typical values
    print("\nCDL typical case: 0.7571^1.2")
    base, exp = 0.7571428418, 1.2
    standard = np.power(base, exp)
    fast = fast_pow(base, exp)
    print(f"  Standard powf: {standard:.10f}")
    print(f"  OCIO fast_pow: {fast:.10f}")
    print(f"  Diff: {abs(standard - fast):.2e}")
    
    # Compare with OCIO actual output
    print("\nComparing with actual OCIO CDL:")
    config = ocio.Config.CreateRaw()
    cdl = ocio.CDLTransform()
    cdl.setSlope([1, 1, 1])
    cdl.setOffset([-0.1, -0.1, -0.1])
    cdl.setPower([1.2, 1.2, 1.2])
    cdl.setSat(1.0)
    
    processor = config.getProcessor(cdl)
    cpu = processor.getDefaultCPUProcessor()
    
    pixel = np.array([[0.8571428657, 0.0, 1.0]], dtype=np.float32)
    cpu.applyRGB(pixel)
    print(f"  OCIO result[0]: {pixel[0, 0]:.10f}")
    
    # Manual with fast_pow
    v = 0.8571428657 * 1.0 + (-0.1)  # 0.7571428657
    v = max(0, v)
    manual_fast = fast_pow(v, 1.2)
    manual_std = np.power(v, 1.2)
    print(f"  fast_pow:       {manual_fast:.10f}")
    print(f"  numpy.power:    {manual_std:.10f}")
    
    print(f"\n  OCIO == fast_pow: {abs(pixel[0,0] - manual_fast) < 1e-6}")
    print(f"  OCIO == numpy:    {abs(pixel[0,0] - manual_std) < 1e-6}")

if __name__ == "__main__":
    test_power_accuracy()
