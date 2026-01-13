#!/usr/bin/env python3
"""Debug hash computation."""

import hashlib
import numpy as np

def compute_hash(data: np.ndarray, precision: int = 5) -> str:
    """Compute deterministic hash of float array."""
    quantized = np.round(data * (10 ** precision)).astype(np.int64)
    return hashlib.sha256(quantized.tobytes()).hexdigest()

# Simple test
test_data = np.array([0.0, 0.5, 1.0], dtype=np.float32)
print(f"Test data: {test_data}")
print(f"Test data dtype: {test_data.dtype}")

quantized = np.round(test_data * (10 ** 5)).astype(np.int64)
print(f"Quantized: {quantized}")
print(f"Quantized bytes: {quantized.tobytes().hex()}")
print(f"Hash: {compute_hash(test_data)}")

# Test with RGB cube first element
print("\n--- RGB Cube test ---")
rgb = np.array([0.0, 0.0, 0.0], dtype=np.float32)
print(f"RGB: {rgb}")
q = np.round(rgb * 1e5).astype(np.int64)
print(f"Quantized: {q}")
print(f"Bytes (hex): {q.tobytes().hex()}")

# Check byte order
print(f"\nNumPy byte order: {q.dtype.byteorder}")
print(f"System: {'little' if np.little_endian else 'big'} endian")

# More complex test
print("\n--- CDL Identity cube (first 3 pixels) ---")
cube = np.array([
    [0.0, 0.0, 0.0],
    [0.0, 0.0, 1/7],
    [0.0, 0.0, 2/7],
], dtype=np.float32)
print(f"Cube:\n{cube}")
flat = cube.flatten()
print(f"Flattened: {flat}")
q = np.round(flat * 1e5).astype(np.int64)
print(f"Quantized: {q}")
print(f"Bytes (hex): {q.tobytes().hex()}")
print(f"Hash: {hashlib.sha256(q.tobytes()).hexdigest()}")
