#!/usr/bin/env python3
"""Test hash computation for RGB cube."""
import numpy as np
import hashlib

def generate_rgb_cube(size=8):
    """Generate RGB color cube."""
    r = np.linspace(0, 1, size, dtype=np.float32)
    g = np.linspace(0, 1, size, dtype=np.float32)
    b = np.linspace(0, 1, size, dtype=np.float32)
    rr, gg, bb = np.meshgrid(r, g, b, indexing='ij')
    return np.stack([rr.flatten(), gg.flatten(), bb.flatten()], axis=-1)

def compute_hash(data, precision=5):
    """Compute hash."""
    flat = data.flatten()
    quantized = np.round(flat * (10 ** precision)).astype(np.int64)
    return hashlib.sha256(quantized.tobytes()).hexdigest()

cube = generate_rgb_cube(8)
print(f"Cube shape: {cube.shape}")
print(f"Cube dtype: {cube.dtype}")
print(f"First: {cube[0]}")
print(f"Last: {cube[-1]}")

# Flatten and check
flat = cube.flatten()
print(f"Flat shape: {flat.shape}")
print(f"First 9 flat values: {flat[:9]}")

# Quantize
quantized = np.round(flat * 1e5).astype(np.int64)
print(f"First 9 quantized: {quantized[:9]}")

# Check byte representation of first value
first_val = quantized[0]
print(f"First quantized value: {first_val}")
print(f"Bytes (little endian): {first_val.tobytes().hex()}")

# Full hash
hash_val = hashlib.sha256(quantized.tobytes()).hexdigest()
print(f"\nHash: {hash_val}")

# Expected
expected = "cff6e617bdcb51b048e7518086a4d1f2d78f0a0259e73047be195b50010885cd"
print(f"Expected: {expected}")
print(f"Match: {hash_val == expected}")
