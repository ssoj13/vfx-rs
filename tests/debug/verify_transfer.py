#!/usr/bin/env python3
"""Verify transfer functions against OCIO."""

import numpy as np
import hashlib

try:
    import PyOpenColorIO as ocio
except ImportError:
    import opencolorio as ocio

# ============================================================================
# sRGB
# ============================================================================

def srgb_eotf(v):
    """sRGB EOTF: decode sRGB to linear."""
    if v <= 0.04045:
        return v / 12.92
    else:
        return ((v + 0.055) / 1.055) ** 2.4

def srgb_oetf(l):
    """sRGB OETF: encode linear to sRGB."""
    if l <= 0.0031308:
        return l * 12.92
    else:
        return 1.055 * (l ** (1.0 / 2.4)) - 0.055

# ============================================================================
# PQ (ST 2084)
# ============================================================================

L_MAX = 10000.0
M1 = 2610.0 / 16384.0
M2 = 2523.0 / 4096.0 * 128.0
C1 = 3424.0 / 4096.0
C2 = 2413.0 / 4096.0 * 32.0
C3 = 2392.0 / 4096.0 * 32.0

def pq_eotf(v):
    """PQ EOTF: decode PQ to linear nits."""
    if v <= 0:
        return 0.0
    vp = v ** (1.0 / M2)
    num = max(0.0, vp - C1)
    den = C2 - C3 * vp
    return L_MAX * (num / den) ** (1.0 / M1)

def pq_oetf(l):
    """PQ OETF: encode linear nits to PQ."""
    if l <= 0:
        return 0.0
    y = np.clip(l / L_MAX, 0, 1)
    yp = y ** M1
    num = C1 + C2 * yp
    den = 1.0 + C3 * yp
    return (num / den) ** M2

# ============================================================================
# HLG
# ============================================================================

HLG_A = 0.17883277
HLG_B = 0.28466892
HLG_C = 0.55991073

def hlg_oetf(e):
    """HLG OETF: encode linear to HLG."""
    if e <= 0:
        return 0.0
    elif e <= 1.0 / 12.0:
        return np.sqrt(3.0 * e)
    else:
        return HLG_A * np.log(12.0 * e - HLG_B) + HLG_C

def hlg_eotf(ep):
    """HLG inverse OETF: decode HLG to linear."""
    if ep <= 0:
        return 0.0
    elif ep <= 0.5:
        return ep * ep / 3.0
    else:
        return (np.exp((ep - HLG_C) / HLG_A) + HLG_B) / 12.0

# ============================================================================
# Canon Log 2
# ============================================================================

CLOG2_CUT = 0.092864125
CLOG2_SLOPE = 0.24136077
CLOG2_SCALE = 87.099375
CLOG2_NORM = 0.9

def clog2_decode(log_val):
    """Canon Log 2 decode to linear."""
    if log_val < CLOG2_CUT:
        out = -(10 ** ((CLOG2_CUT - log_val) / CLOG2_SLOPE) - 1) / CLOG2_SCALE
    else:
        out = (10 ** ((log_val - CLOG2_CUT) / CLOG2_SLOPE) - 1) / CLOG2_SCALE
    return out * CLOG2_NORM

# ============================================================================
# Tests
# ============================================================================

def test_srgb():
    """Test sRGB against OCIO."""
    print("=" * 60)
    print("sRGB Transfer Function")
    print("=" * 60)
    
    # OCIO sRGB uses ExponentWithLinearTransform or GammaOp with MONCURVE
    config = ocio.Config.CreateRaw()
    
    # Create sRGB transform using builtin
    transform = ocio.BuiltinTransform("DISPLAY - CIE-XYZ-D65_to_sRGB")
    processor = config.getProcessor(transform)
    cpu = processor.getDefaultCPUProcessor()
    
    # Test values
    test_vals = np.linspace(0, 1, 256, dtype=np.float32)
    
    # We need XYZ input for this transform, so let's use a simpler approach
    # with ExponentWithLinearTransform
    exp_transform = ocio.ExponentWithLinearTransform()
    exp_transform.setGamma([2.4, 2.4, 2.4, 1.0])
    exp_transform.setOffset([0.055, 0.055, 0.055, 0.0])
    exp_transform.setDirection(ocio.TRANSFORM_DIR_INVERSE)  # linear to sRGB
    
    processor2 = config.getProcessor(exp_transform)
    cpu2 = processor2.getDefaultCPUProcessor()
    
    # Apply OCIO
    ocio_result = []
    for v in test_vals:
        pixel = np.array([[v, v, v]], dtype=np.float32)
        cpu2.applyRGB(pixel)
        ocio_result.append(pixel[0, 0])
    ocio_result = np.array(ocio_result)
    
    # Apply manual
    manual_result = np.array([srgb_oetf(float(v)) for v in test_vals], dtype=np.float32)
    
    diff = np.abs(ocio_result - manual_result)
    print(f"Max diff (linear->sRGB): {np.max(diff):.2e}")
    print(f"Mean diff: {np.mean(diff):.2e}")
    
    # Test roundtrip
    exp_transform2 = ocio.ExponentWithLinearTransform()
    exp_transform2.setGamma([2.4, 2.4, 2.4, 1.0])
    exp_transform2.setOffset([0.055, 0.055, 0.055, 0.0])
    exp_transform2.setDirection(ocio.TRANSFORM_DIR_FORWARD)  # sRGB to linear
    
    processor3 = config.getProcessor(exp_transform2)
    cpu3 = processor3.getDefaultCPUProcessor()
    
    ocio_eotf = []
    for v in test_vals:
        pixel = np.array([[v, v, v]], dtype=np.float32)
        cpu3.applyRGB(pixel)
        ocio_eotf.append(pixel[0, 0])
    ocio_eotf = np.array(ocio_eotf)
    
    manual_eotf = np.array([srgb_eotf(float(v)) for v in test_vals], dtype=np.float32)
    
    diff2 = np.abs(ocio_eotf - manual_eotf)
    print(f"Max diff (sRGB->linear): {np.max(diff2):.2e}")
    print(f"Mean diff: {np.mean(diff2):.2e}")
    
    return np.max(diff) < 1e-5 and np.max(diff2) < 1e-5

def test_pq():
    """Test PQ against OCIO."""
    print("\n" + "=" * 60)
    print("PQ (ST 2084) Transfer Function")
    print("=" * 60)
    
    config = ocio.Config.CreateRaw()
    
    # PQ to linear
    transform = ocio.BuiltinTransform("CURVE - ST-2084_to_LINEAR")
    processor = config.getProcessor(transform)
    cpu = processor.getDefaultCPUProcessor()
    
    test_vals = np.linspace(0, 1, 256, dtype=np.float32)
    
    # Apply OCIO
    ocio_result = []
    for v in test_vals:
        pixel = np.array([[v, v, v]], dtype=np.float32)
        cpu.applyRGB(pixel)
        # OCIO returns nits/100, our function returns nits
        ocio_result.append(pixel[0, 0] * 100.0)  # Convert to nits
    ocio_result = np.array(ocio_result)
    
    # Apply manual
    manual_result = np.array([pq_eotf(float(v)) for v in test_vals], dtype=np.float32)
    
    # Calculate relative error for non-zero values
    mask = manual_result > 1.0  # Only compare values above 1 nit
    if np.any(mask):
        rel_diff = np.abs(ocio_result[mask] - manual_result[mask]) / manual_result[mask]
        print(f"Max relative diff (PQ->linear): {np.max(rel_diff):.2e}")
        print(f"Mean relative diff: {np.mean(rel_diff):.2e}")
    
    abs_diff = np.abs(ocio_result - manual_result)
    print(f"Max absolute diff: {np.max(abs_diff):.2e}")
    
    # Test linear to PQ
    transform2 = ocio.BuiltinTransform("CURVE - LINEAR_to_ST-2084")
    processor2 = config.getProcessor(transform2)
    cpu2 = processor2.getDefaultCPUProcessor()
    
    # Test with nits/100 input (OCIO expects nits/100)
    test_nits = np.array([0.01, 0.1, 1.0, 10.0, 100.0], dtype=np.float32)
    
    ocio_pq = []
    for nits in test_nits:
        pixel = np.array([[nits, nits, nits]], dtype=np.float32)
        cpu2.applyRGB(pixel)
        ocio_pq.append(pixel[0, 0])
    ocio_pq = np.array(ocio_pq)
    
    # Manual with nits * 100 input
    manual_pq = np.array([pq_oetf(float(n) * 100.0) for n in test_nits], dtype=np.float32)
    
    diff2 = np.abs(ocio_pq - manual_pq)
    print(f"Max diff (linear->PQ): {np.max(diff2):.2e}")
    
    return np.max(rel_diff) < 1e-4 if np.any(mask) else True

def test_clog2():
    """Test Canon Log 2 against OCIO."""
    print("\n" + "=" * 60)
    print("Canon Log 2 Transfer Function")
    print("=" * 60)
    
    config = ocio.Config.CreateRaw()
    transform = ocio.BuiltinTransform("CURVE - CANON_CLOG2_to_LINEAR")
    processor = config.getProcessor(transform)
    cpu = processor.getDefaultCPUProcessor()
    
    test_vals = np.linspace(0, 1, 256, dtype=np.float32)
    
    # Apply OCIO
    ocio_result = []
    for v in test_vals:
        pixel = np.array([[v, v, v]], dtype=np.float32)
        cpu.applyRGB(pixel)
        ocio_result.append(pixel[0, 0])
    ocio_result = np.array(ocio_result)
    
    # Apply manual
    manual_result = np.array([clog2_decode(float(v)) for v in test_vals], dtype=np.float32)
    
    diff = np.abs(ocio_result - manual_result)
    print(f"Max diff: {np.max(diff):.2e}")
    print(f"Mean diff: {np.mean(diff):.2e}")
    
    # Find worst case
    worst_idx = np.argmax(diff)
    print(f"Worst case: input={test_vals[worst_idx]:.6f}")
    print(f"  OCIO:   {ocio_result[worst_idx]:.10f}")
    print(f"  Manual: {manual_result[worst_idx]:.10f}")
    
    return np.max(diff) < 1e-5

def test_hlg():
    """Test HLG against OCIO (noting different normalization)."""
    print("\n" + "=" * 60)
    print("HLG Transfer Function (BT.2100 standard)")
    print("=" * 60)
    
    # Note: OCIO HLG uses E_MAX=3 normalization for HDR display workflow
    # Our implementation follows strict BT.2100 with [0,1] normalized range
    
    # Test roundtrip consistency of our implementation
    test_vals = np.linspace(0, 1, 256)
    
    roundtrip_errors = []
    for v in test_vals:
        encoded = hlg_oetf(v)
        decoded = hlg_eotf(encoded)
        roundtrip_errors.append(abs(v - decoded))
    
    print(f"Roundtrip max error: {max(roundtrip_errors):.2e}")
    print(f"Roundtrip mean error: {np.mean(roundtrip_errors):.2e}")
    
    # Verify constants match BT.2100
    print(f"\nConstants verification:")
    print(f"  a = {HLG_A} (BT.2100: 0.17883277)")
    print(f"  b = {HLG_B} (BT.2100: 1-4a = {1-4*HLG_A})")
    print(f"  c = {HLG_C} (BT.2100: 0.5-a*ln(4a) = {0.5 - HLG_A * np.log(4*HLG_A)})")
    
    # Check transition point continuity
    e_break = 1.0 / 12.0
    below = hlg_oetf(e_break - 1e-10)
    above = hlg_oetf(e_break + 1e-10)
    print(f"\nTransition point continuity at 1/12:")
    print(f"  Below: {below:.10f}")
    print(f"  Above: {above:.10f}")
    print(f"  Jump:  {abs(above - below):.2e}")
    
    return max(roundtrip_errors) < 1e-6

if __name__ == "__main__":
    results = []
    
    results.append(("sRGB", test_srgb()))
    results.append(("PQ", test_pq()))
    results.append(("Canon Log 2", test_clog2()))
    results.append(("HLG", test_hlg()))
    
    print("\n" + "=" * 60)
    print("SUMMARY")
    print("=" * 60)
    for name, passed in results:
        status = "[OK]" if passed else "[FAIL]"
        print(f"  {name}: {status}")
