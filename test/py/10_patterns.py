#!/usr/bin/env python3
"""
Example 10: Pattern Generation

Create synthetic test patterns: gradients, checkers, noise.
"""
import _bootstrap  # noqa: F401 - ensures venv Python
from pathlib import Path
import numpy as np
import vfx_rs
from vfx_rs import ops

OUTPUT_DIR = Path(__file__).parent.parent / "out" / "examples"
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

SIZE = 512

# Checkerboard pattern
checker = ops.checker(
    SIZE, SIZE,
    check_width=32, check_height=32,
    color1=[0.2, 0.2, 0.2, 1.0],
    color2=[0.8, 0.8, 0.8, 1.0]
)
vfx_rs.write(OUTPUT_DIR / "pattern_checker.exr", checker)

# Solid color fill
red = ops.fill(SIZE, SIZE, [1.0, 0.0, 0.0, 1.0])
vfx_rs.write(OUTPUT_DIR / "pattern_red.exr", red)

# Noise patterns
uniform_noise = ops.noise(SIZE, SIZE, noise_type="uniform", a=0.0, b=1.0)
vfx_rs.write(OUTPUT_DIR / "pattern_noise_uniform.exr", uniform_noise)

gaussian_noise = ops.noise(SIZE, SIZE, noise_type="gaussian", a=0.5, b=0.15)
vfx_rs.write(OUTPUT_DIR / "pattern_noise_gaussian.exr", gaussian_noise)

# SMPTE-style color bars
bars = np.zeros((SIZE, SIZE, 4), dtype=np.float32)
colors = [
    [1, 1, 1], [1, 1, 0], [0, 1, 1], [0, 1, 0],
    [1, 0, 1], [1, 0, 0], [0, 0, 1], [0, 0, 0],
]
bar_width = SIZE // 8
for i, color in enumerate(colors):
    bars[:, i*bar_width:(i+1)*bar_width, :3] = color
bars[:, :, 3] = 1.0
vfx_rs.write(OUTPUT_DIR / "pattern_colorbars.exr", vfx_rs.Image(bars))

# HDR gradient (values 0 to 4)
hdr_ramp = np.zeros((SIZE, SIZE, 4), dtype=np.float32)
hdr_ramp[:, :, :3] = np.linspace(0, 4.0, SIZE).reshape(1, SIZE, 1)
hdr_ramp[:, :, 3] = 1.0
vfx_rs.write(OUTPUT_DIR / "pattern_hdr_ramp.exr", vfx_rs.Image(hdr_ramp))

print("Pattern examples written to:", OUTPUT_DIR)
