#!/usr/bin/env python3
"""Debug Apple Log comparison between OCIO and manual implementation."""

import numpy as np
import hashlib

try:
    import PyOpenColorIO as ocio
except ImportError:
    import opencolorio as ocio

# Constants (must match OCIO exactly)
R_0 = -0.05641088
R_T = 0.01
C = 47.28711236
BETA = 0.00964052
GAMMA = 0.08550479
DELTA = 0.69336945
BASE = 2.0

P_T = C * (R_T - R_0) ** 2

def decode_manual(log_val):
    """Manual implementation of Apple Log decode."""
    if log_val >= P_T:
        # Log segment
        return BASE ** ((log_val - DELTA) / GAMMA) - BETA
    elif log_val >= 0.0:
        # Gamma segment
        return np.sqrt(log_val / C) + R_0
    else:
        # Below zero
        return R_0

def decode_ocio(pixels):
    """Apply OCIO Apple Log decode."""
    config = ocio.Config.CreateRaw()
    transform = ocio.BuiltinTransform("CURVE - APPLE_LOG_to_LINEAR")
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

# Generate test data (must match Rust gray_ramp_256)
pixels = np.linspace(0.0, 1.0, 256, dtype=np.float32)
print(f"Input range: [{pixels[0]}, {pixels[-1]}]")
print(f"P_t = {P_T}")
print()

# Apply OCIO
ocio_result = decode_ocio(pixels)

# Apply manual implementation
manual_result = np.array([decode_manual(v) for v in pixels], dtype=np.float32)

# Compare
diff = np.abs(ocio_result - manual_result)
print(f"Max diff: {np.max(diff):.10f}")
print(f"Mean diff: {np.mean(diff):.10f}")

# Show first few values
print("\nFirst 10 values:")
print("Index | Input      | OCIO       | Manual     | Diff")
print("-" * 60)
for i in range(10):
    d = abs(ocio_result[i] - manual_result[i])
    print(f"{i:5} | {pixels[i]:.8f} | {ocio_result[i]:.8f} | {manual_result[i]:.8f} | {d:.2e}")

# Show values around P_t
print(f"\nValues around P_t ({P_T:.6f}):")
for i, v in enumerate(pixels):
    if abs(v - P_T) < 0.02:
        d = abs(ocio_result[i] - manual_result[i])
        seg = "log" if v >= P_T else "gamma"
        print(f"{i:5} | {v:.8f} ({seg:5}) | {ocio_result[i]:.8f} | {manual_result[i]:.8f} | {d:.2e}")

# Show last values
print("\nLast 5 values:")
for i in range(251, 256):
    d = abs(ocio_result[i] - manual_result[i])
    print(f"{i:5} | {pixels[i]:.8f} | {ocio_result[i]:.8f} | {manual_result[i]:.8f} | {d:.2e}")

# Compute hashes
ocio_hash = compute_hash(ocio_result)
manual_hash = compute_hash(manual_result)

print(f"\nHashes:")
print(f"  OCIO:   {ocio_hash}")
print(f"  Manual: {manual_hash}")
print(f"  Match:  {ocio_hash == manual_hash}")

# Stats
print(f"\nStats:")
print(f"  OCIO min: {np.min(ocio_result):.10f}")
print(f"  OCIO max: {np.max(ocio_result):.10f}")
print(f"  Manual min: {np.min(manual_result):.10f}")
print(f"  Manual max: {np.max(manual_result):.10f}")
