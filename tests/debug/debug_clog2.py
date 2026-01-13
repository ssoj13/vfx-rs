#!/usr/bin/env python3
"""Debug Canon C-Log2 comparison between OCIO and manual implementation."""

import numpy as np
import hashlib

try:
    import PyOpenColorIO as ocio
except ImportError:
    import opencolorio as ocio

# Constants (must match OCIO exactly)
CUT = 0.092864125
LOG_SLOPE = 0.24136077
LIN_SCALE = 87.099375
NORM = 0.9

def decode_manual(log_val):
    """Manual implementation of Canon C-Log2 decode."""
    if log_val < CUT:
        out = -(10 ** ((CUT - log_val) / LOG_SLOPE) - 1) / LIN_SCALE
    else:
        out = (10 ** ((log_val - CUT) / LOG_SLOPE) - 1) / LIN_SCALE
    return out * NORM

def decode_ocio(pixels):
    """Apply OCIO Canon C-Log2 decode."""
    config = ocio.Config.CreateRaw()
    transform = ocio.BuiltinTransform("CURVE - CANON_CLOG2_to_LINEAR")
    processor = config.getProcessor(transform)
    cpu = processor.getDefaultCPUProcessor()
    
    result = pixels.copy()
    if result.ndim == 1:
        rgb = np.column_stack([result, result, result])
        cpu.applyRGB(rgb)
        return rgb[:, 0]
    return result

def compute_hash(data, precision=5):
    """Compute hash matching Rust implementation."""
    quantized = np.round(data * (10 ** precision)).astype(np.int64)
    return hashlib.sha256(quantized.tobytes()).hexdigest()

# Generate test data
pixels = np.linspace(0.0, 1.0, 256, dtype=np.float32)
print(f"Input range: [{pixels[0]}, {pixels[-1]}]")
print(f"CUT = {CUT}")
print()

# Apply both
ocio_result = decode_ocio(pixels)
manual_result = np.array([decode_manual(v) for v in pixels], dtype=np.float32)

# Compare
diff = np.abs(ocio_result - manual_result)
print(f"Max diff: {np.max(diff):.10f}")
print(f"Mean diff: {np.mean(diff):.10f}")

# Find worst pixel
worst_idx = np.argmax(diff)
print(f"\nWorst pixel [{worst_idx}]:")
print(f"  Input: {pixels[worst_idx]:.10f}")
print(f"  OCIO:  {ocio_result[worst_idx]:.10f}")
print(f"  Manual: {manual_result[worst_idx]:.10f}")
print(f"  Diff:   {diff[worst_idx]:.10e}")

# Show around the cut point
print(f"\nAround cut point ({CUT}):")
for i, v in enumerate(pixels):
    if abs(v - CUT) < 0.02:
        d = abs(ocio_result[i] - manual_result[i])
        seg = "pos" if v >= CUT else "neg"
        print(f"  [{i:3}] {v:.8f} ({seg}) | OCIO={ocio_result[i]:.8f} | Manual={manual_result[i]:.8f} | {d:.2e}")

# Hashes
ocio_hash = compute_hash(ocio_result)
manual_hash = compute_hash(manual_result)
print(f"\nHashes:")
print(f"  OCIO:   {ocio_hash}")
print(f"  Manual: {manual_hash}")
print(f"  Match:  {ocio_hash == manual_hash}")

# Stats
print(f"\nStats:")
print(f"  OCIO: min={np.min(ocio_result):.6f}, max={np.max(ocio_result):.6f}")
print(f"  Manual: min={np.min(manual_result):.6f}, max={np.max(manual_result):.6f}")

# Expected from golden hashes.json
expected_hash = "47d736f333345d2f8424323dac45d59ad99178aac52fe06f57ec9a8962cbec82"
print(f"\nExpected golden hash: {expected_hash}")
print(f"  OCIO matches golden: {ocio_hash == expected_hash}")
