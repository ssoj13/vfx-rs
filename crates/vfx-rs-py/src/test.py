#!/usr/bin/env python3
"""
Visual quality tests for vfx_rs Python bindings.

Run: python test.py
Output: C:/projects/projects.rust/_vfx-rs/test/out/
"""

import vfx_rs
from vfx_rs import io, lut
import numpy as np
from pathlib import Path

# Paths
TEST_DIR = Path(r"C:\projects\projects.rust\_vfx-rs\test")
OUT_DIR = TEST_DIR / "out"
OUT_DIR.mkdir(exist_ok=True)


def test_exposure_ladder():
    """Create exposure ladder: -2, -1, 0, +1, +2 stops."""
    print("\n=== Exposure Ladder ===")
    
    img = vfx_rs.read(TEST_DIR / "owl.exr")
    proc = vfx_rs.Processor()
    
    stops = [-2, -1, 0, 1, 2]
    for stop in stops:
        work = img.copy()
        proc.exposure(work, float(stop))
        out_path = OUT_DIR / f"exposure_{stop:+d}.exr"
        vfx_rs.write(out_path, work)
        print(f"  Written: {out_path.name}")


def test_saturation_range():
    """Saturation range: 0 (grayscale) to 2 (oversaturated)."""
    print("\n=== Saturation Range ===")
    
    img = vfx_rs.read(TEST_DIR / "owl.exr")
    proc = vfx_rs.Processor()
    
    levels = [0.0, 0.5, 1.0, 1.5, 2.0]
    for sat in levels:
        work = img.copy()
        proc.saturation(work, sat)
        name = f"saturation_{sat:.1f}".replace(".", "_")
        out_path = OUT_DIR / f"{name}.exr"
        vfx_rs.write(out_path, work)
        print(f"  Written: {out_path.name}")


def test_contrast_range():
    """Contrast range: 0.5 (flat) to 2.0 (punchy)."""
    print("\n=== Contrast Range ===")
    
    img = vfx_rs.read(TEST_DIR / "owl.exr")
    proc = vfx_rs.Processor()
    
    levels = [0.5, 0.75, 1.0, 1.5, 2.0]
    for contrast in levels:
        work = img.copy()
        proc.contrast(work, contrast)
        name = f"contrast_{contrast:.2f}".replace(".", "_")
        out_path = OUT_DIR / f"{name}.exr"
        vfx_rs.write(out_path, work)
        print(f"  Written: {out_path.name}")


def test_cdl_grades():
    """CDL color grades - warm, cool, teal-orange."""
    print("\n=== CDL Grades ===")
    
    img = vfx_rs.read(TEST_DIR / "owl.exr")
    proc = vfx_rs.Processor()
    
    grades = {
        "neutral": {"slope": [1.0, 1.0, 1.0], "offset": [0.0, 0.0, 0.0], "power": [1.0, 1.0, 1.0]},
        "warm": {"slope": [1.1, 1.0, 0.9], "offset": [0.02, 0.0, -0.02], "power": [1.0, 1.0, 1.0]},
        "cool": {"slope": [0.9, 1.0, 1.1], "offset": [-0.01, 0.0, 0.02], "power": [1.0, 1.0, 1.0]},
        "teal_orange": {"slope": [1.15, 0.95, 0.85], "offset": [-0.02, 0.01, 0.03], "power": [0.95, 1.0, 1.05]},
        "filmic": {"slope": [1.0, 1.0, 1.0], "offset": [0.0, 0.0, 0.0], "power": [1.1, 1.05, 1.0]},
        "bleach_bypass": {"slope": [1.0, 1.0, 1.0], "offset": [0.0, 0.0, 0.0], "power": [1.2, 1.2, 1.2], "saturation": 0.6},
    }
    
    for name, grade in grades.items():
        work = img.copy()
        proc.cdl(
            work,
            slope=grade.get("slope"),
            offset=grade.get("offset"),
            power=grade.get("power"),
            saturation=grade.get("saturation", 1.0)
        )
        out_path = OUT_DIR / f"cdl_{name}.exr"
        vfx_rs.write(out_path, work)
        print(f"  Written: {out_path.name}")


def test_combined_grade():
    """Combined look: exposure + contrast + saturation + CDL."""
    print("\n=== Combined Grade ===")
    
    img = vfx_rs.read(TEST_DIR / "owl.exr")
    proc = vfx_rs.Processor()
    
    # Cinema look
    work = img.copy()
    proc.exposure(work, 0.3)
    proc.contrast(work, 1.2)
    proc.saturation(work, 0.9)
    proc.cdl(work, slope=[1.05, 1.0, 0.95], offset=[0.01, 0.0, -0.01], power=[1.0, 1.0, 1.0])
    
    vfx_rs.write(OUT_DIR / "combined_cinema.exr", work)
    print(f"  Written: combined_cinema.exr")
    
    # High contrast B&W
    work = img.copy()
    proc.contrast(work, 1.5)
    proc.saturation(work, 0.0)
    
    vfx_rs.write(OUT_DIR / "combined_bw.exr", work)
    print(f"  Written: combined_bw.exr")


def test_numpy_manipulation():
    """Direct numpy pixel manipulation."""
    print("\n=== Numpy Manipulation ===")
    
    img = vfx_rs.read(TEST_DIR / "owl.exr")
    arr = img.numpy()
    
    h, w, c = arr.shape
    print(f"  Image shape: {w}x{h}, {c} channels")
    
    # Vignette effect
    y, x = np.ogrid[:h, :w]
    cx, cy = w / 2, h / 2
    dist = np.sqrt((x - cx)**2 + (y - cy)**2)
    max_dist = np.sqrt(cx**2 + cy**2)
    vignette = 1 - (dist / max_dist) ** 2 * 0.7
    vignette = vignette[:, :, np.newaxis]
    
    vignetted = arr * vignette
    vignetted = np.clip(vignetted, 0, None).astype(np.float32)
    
    vig_img = vfx_rs.Image(vignetted)
    vfx_rs.write(OUT_DIR / "numpy_vignette.exr", vig_img)
    print(f"  Written: numpy_vignette.exr")
    
    # Channel swap (RGB -> BRG)
    swapped = arr[:, :, [2, 0, 1, 3]] if c == 4 else arr[:, :, [2, 0, 1]]
    swapped = np.ascontiguousarray(swapped, dtype=np.float32)
    swap_img = vfx_rs.Image(swapped)
    vfx_rs.write(OUT_DIR / "numpy_channel_swap.exr", swap_img)
    print(f"  Written: numpy_channel_swap.exr")
    
    # Horizontal gradient multiply
    gradient = np.linspace(0.3, 1.0, w).reshape(1, w, 1).astype(np.float32)
    graded = arr * gradient
    grad_img = vfx_rs.Image(graded.astype(np.float32))
    vfx_rs.write(OUT_DIR / "numpy_gradient.exr", grad_img)
    print(f"  Written: numpy_gradient.exr")


def test_format_roundtrip():
    """Test format conversions: EXR -> PNG -> JPEG -> TIFF."""
    print("\n=== Format Roundtrip ===")
    
    img = vfx_rs.read(TEST_DIR / "owl.exr")
    print(f"  Original: {img}")
    
    # EXR -> PNG
    vfx_rs.write(OUT_DIR / "roundtrip.png", img)
    png = vfx_rs.read(OUT_DIR / "roundtrip.png")
    print(f"  PNG: {png}")
    
    # PNG -> JPEG
    io.write_jpeg(OUT_DIR / "roundtrip.jpg", png, quality=95)
    jpg = io.read_jpeg(OUT_DIR / "roundtrip.jpg")
    print(f"  JPEG: {jpg}")
    
    # JPEG -> TIFF
    io.write_tiff(OUT_DIR / "roundtrip.tiff", jpg)
    tif = io.read_tiff(OUT_DIR / "roundtrip.tiff")
    print(f"  TIFF: {tif}")
    
    # TIFF -> EXR
    vfx_rs.write(OUT_DIR / "roundtrip_final.exr", tif)
    final = vfx_rs.read(OUT_DIR / "roundtrip_final.exr")
    print(f"  Final EXR: {final}")


def test_dpx_bitdepths():
    """Test DPX at different bit depths."""
    print("\n=== DPX Bit Depths ===")
    
    img = vfx_rs.read(TEST_DIR / "owl.exr")
    
    for depth in [8, 10, 12, 16]:
        out_path = OUT_DIR / f"dpx_{depth}bit.dpx"
        io.write_dpx(out_path, img, bit_depth=depth)
        
        # Read back and check
        loaded = io.read_dpx(out_path)
        print(f"  {depth}-bit: written and read back as {loaded}")


def test_synthetic_patterns():
    """Generate synthetic test patterns."""
    print("\n=== Synthetic Patterns ===")
    
    size = 512
    
    # Color bars
    bars = np.zeros((size, size, 4), dtype=np.float32)
    colors = [
        [1, 1, 1],  # white
        [1, 1, 0],  # yellow
        [0, 1, 1],  # cyan
        [0, 1, 0],  # green
        [1, 0, 1],  # magenta
        [1, 0, 0],  # red
        [0, 0, 1],  # blue
        [0, 0, 0],  # black
    ]
    bar_width = size // 8
    for i, color in enumerate(colors):
        bars[:, i*bar_width:(i+1)*bar_width, :3] = color
    bars[:, :, 3] = 1.0
    
    bars_img = vfx_rs.Image(bars)
    vfx_rs.write(OUT_DIR / "pattern_colorbars.exr", bars_img)
    print(f"  Written: pattern_colorbars.exr")
    
    # Gradient ramp
    ramp = np.zeros((size, size, 4), dtype=np.float32)
    ramp[:, :, :3] = np.linspace(0, 1, size).reshape(1, size, 1)
    ramp[:, :, 3] = 1.0
    
    ramp_img = vfx_rs.Image(ramp)
    vfx_rs.write(OUT_DIR / "pattern_ramp.exr", ramp_img)
    print(f"  Written: pattern_ramp.exr")
    
    # Checker
    checker = np.zeros((size, size, 4), dtype=np.float32)
    check_size = 32
    for y in range(size):
        for x in range(size):
            if (x // check_size + y // check_size) % 2 == 0:
                checker[y, x, :3] = 1.0
    checker[:, :, 3] = 1.0
    
    checker_img = vfx_rs.Image(checker)
    vfx_rs.write(OUT_DIR / "pattern_checker.exr", checker_img)
    print(f"  Written: pattern_checker.exr")


def test_hdr_range():
    """Test HDR value handling (values > 1.0)."""
    print("\n=== HDR Range ===")
    
    img = vfx_rs.read(TEST_DIR / "owl.exr")
    arr = img.numpy()
    
    # Boost to HDR range
    hdr = arr * 3.0
    hdr_img = vfx_rs.Image(hdr.astype(np.float32))
    vfx_rs.write(OUT_DIR / "hdr_boosted.exr", hdr_img)
    print(f"  Written: hdr_boosted.exr (3x brightness)")
    
    # Exposure down to bring back
    proc = vfx_rs.Processor()
    work = hdr_img.copy()
    proc.exposure(work, -1.585)  # log2(3) ≈ 1.585
    vfx_rs.write(OUT_DIR / "hdr_recovered.exr", work)
    print(f"  Written: hdr_recovered.exr (exposure corrected)")


def main():
    print("=" * 60)
    print("VFX_RS Visual Quality Tests")
    print("=" * 60)
    print(f"Input: {TEST_DIR}")
    print(f"Output: {OUT_DIR}")
    
    test_exposure_ladder()
    test_saturation_range()
    test_contrast_range()
    test_cdl_grades()
    test_combined_grade()
    test_numpy_manipulation()
    test_format_roundtrip()
    test_dpx_bitdepths()
    test_synthetic_patterns()
    test_hdr_range()
    
    print("\n" + "=" * 60)
    print("All tests complete!")
    print(f"Check output in: {OUT_DIR}")
    print("=" * 60)


if __name__ == "__main__":
    main()
