#!/usr/bin/env python3
"""Debug RGB cube generation between Python and Rust."""

import numpy as np
import hashlib

def generate_rgb_cube_python(size=8):
    """Generate RGB cube using Python/NumPy meshgrid."""
    r = np.linspace(0, 1, size, dtype=np.float32)
    g = np.linspace(0, 1, size, dtype=np.float32)
    b = np.linspace(0, 1, size, dtype=np.float32)
    rr, gg, bb = np.meshgrid(r, g, b, indexing='ij')
    return np.stack([rr.flatten(), gg.flatten(), bb.flatten()], axis=-1)

def generate_rgb_cube_rust_style(size=8):
    """Generate RGB cube using Rust-style nested loops."""
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

def compute_hash(data, precision=5):
    """Compute hash."""
    quantized = np.round(data.flatten() * (10 ** precision)).astype(np.int64)
    return hashlib.sha256(quantized.tobytes()).hexdigest()

# Generate both
python_cube = generate_rgb_cube_python(8)
rust_cube = generate_rgb_cube_rust_style(8)

print(f"Python shape: {python_cube.shape}")
print(f"Rust shape: {rust_cube.shape}")
print()

# Check if they're the same
diff = np.abs(python_cube - rust_cube)
print(f"Max diff: {np.max(diff)}")
print(f"Arrays equal: {np.allclose(python_cube, rust_cube)}")
print()

# Show first few entries
print("First 10 entries:")
print("Index | Python               | Rust")
print("-" * 60)
for i in range(10):
    print(f"{i:5} | {python_cube[i]} | {rust_cube[i]}")

print()

# Check hashes
python_hash = compute_hash(python_cube)
rust_hash = compute_hash(rust_cube)
print(f"Python cube hash: {python_hash}")
print(f"Rust cube hash:   {rust_hash}")
print(f"Hashes match: {python_hash == rust_hash}")

# Check a few key indices
print("\nKey indices check:")
# Index 0 should be (0,0,0)
# Index 7 should be (0,0,1) in rust order, (0,1,0) in numpy 'ij' order??
print(f"Index 0: Python={python_cube[0]}, Rust={rust_cube[0]}")
print(f"Index 1: Python={python_cube[1]}, Rust={rust_cube[1]}")
print(f"Index 7: Python={python_cube[7]}, Rust={rust_cube[7]}")
print(f"Index 8: Python={python_cube[8]}, Rust={rust_cube[8]}")
print(f"Index 63: Python={python_cube[63]}, Rust={rust_cube[63]}")
print(f"Index 64: Python={python_cube[64]}, Rust={rust_cube[64]}")
