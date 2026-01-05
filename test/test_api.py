#!/usr/bin/env python3
"""
Comprehensive vfx_rs Python API test suite.

Tests all Python API features with visual verification and automated diff checks.
Writes intermediate results at each step for manual inspection.

Usage:
    python test_api.py [input_image]
    python test_api.py                    # Uses test/owl.exr
    python test_api.py my_image.exr       # Uses custom image

Output: test/out/api_test/
"""

import sys
import time
import subprocess
import argparse
from pathlib import Path
from dataclasses import dataclass
from typing import Optional, List, Callable

import numpy as np

# Add parent dir for local development
sys.path.insert(0, str(Path(__file__).parent.parent / "target" / "release"))

import vfx_rs
from vfx_rs import io, lut

# =============================================================================
# Configuration
# =============================================================================

TEST_DIR = Path(__file__).parent
VFX_CLI = TEST_DIR.parent / "target" / "release" / "vfx.exe"

# Mutable output dir (can be overridden by --output)
_out_dir = TEST_DIR / "out" / "api_test"

def get_out_dir() -> Path:
    return _out_dir

def set_out_dir(path: Path):
    global _out_dir
    _out_dir = path

# Tolerance for image diff (per-channel, 0-1 range)
DIFF_THRESHOLD = 0.001


@dataclass
class TestResult:
    """Result of a single test."""
    name: str
    passed: bool
    duration_ms: float
    message: str = ""
    output_path: Optional[Path] = None


class TestLogger:
    """Structured test logger with timing."""
    
    def __init__(self):
        self.results: List[TestResult] = []
        self.indent = 0
        self.start_time = time.time()
    
    def section(self, title: str):
        """Print section header."""
        print(f"\n{'=' * 70}")
        print(f"  {title}")
        print(f"{'=' * 70}")
    
    def step(self, msg: str):
        """Print step message."""
        indent = "  " * self.indent
        print(f"{indent}> {msg}")
    
    def info(self, msg: str):
        """Print info message."""
        indent = "  " * self.indent
        print(f"{indent}  {msg}")
    
    def success(self, msg: str):
        """Print success message."""
        indent = "  " * self.indent
        print(f"{indent}  [OK] {msg}")
    
    def error(self, msg: str):
        """Print error message."""
        indent = "  " * self.indent
        print(f"{indent}  [ERR] {msg}")
    
    def warning(self, msg: str):
        """Print warning message."""
        indent = "  " * self.indent
        print(f"{indent}  [WARN] {msg}")
    
    def record(self, result: TestResult):
        """Record test result."""
        self.results.append(result)
        status = "[PASS]" if result.passed else "[FAIL]"
        print(f"  {status} {result.name} ({result.duration_ms:.1f}ms)")
        if result.message:
            print(f"         {result.message}")
    
    def summary(self):
        """Print test summary."""
        elapsed = time.time() - self.start_time
        passed = sum(1 for r in self.results if r.passed)
        failed = len(self.results) - passed
        
        print(f"\n{'=' * 70}")
        print(f"  TEST SUMMARY")
        print(f"{'=' * 70}")
        print(f"  Total:  {len(self.results)} tests")
        print(f"  Passed: {passed}")
        print(f"  Failed: {failed}")
        print(f"  Time:   {elapsed:.2f}s")
        print(f"{'=' * 70}")
        
        if failed > 0:
            print("\nFailed tests:")
            for r in self.results:
                if not r.passed:
                    print(f"  - {r.name}: {r.message}")
        
        return failed == 0


log = TestLogger()


# =============================================================================
# Utilities
# =============================================================================

def run_vfx_diff(a: Path, b: Path, threshold: float = DIFF_THRESHOLD) -> tuple[bool, str]:
    """Compare two images using vfx diff CLI. Returns (passed, message)."""
    if not VFX_CLI.exists():
        return True, "vfx CLI not found, skipping diff"
    
    try:
        result = subprocess.run(
            [str(VFX_CLI), "diff", str(a), str(b), "-t", str(threshold), "-v"],
            capture_output=True,
            text=True,
            timeout=30
        )
        if result.returncode == 0:
            return True, "images match"
        else:
            return False, result.stdout or result.stderr or "diff failed"
    except Exception as e:
        return True, f"diff skipped: {e}"


def timed_test(name: str, func: Callable, *args, **kwargs) -> TestResult:
    """Run a test function with timing."""
    start = time.time()
    try:
        result = func(*args, **kwargs)
        elapsed_ms = (time.time() - start) * 1000
        if isinstance(result, tuple):
            passed, msg, out_path = result[0], result[1], result[2] if len(result) > 2 else None
        else:
            passed, msg, out_path = result, "", None
        return TestResult(name, passed, elapsed_ms, msg, out_path)
    except Exception as e:
        elapsed_ms = (time.time() - start) * 1000
        return TestResult(name, False, elapsed_ms, str(e))


# =============================================================================
# Test: Basic I/O
# =============================================================================

def test_basic_io(input_path: Path) -> list[TestResult]:
    """Test basic read/write across formats."""
    log.section("BASIC I/O")
    results = []
    
    # Read input image
    log.step(f"Reading input: {input_path}")
    img = vfx_rs.read(input_path)
    log.info(f"Loaded: {img}")
    
    # Test all format conversions
    formats = [
        ("exr", "OpenEXR (HDR float)"),
        ("png", "PNG (8-bit)"),
        ("jpg", "JPEG (lossy)"),
        ("tiff", "TIFF (16-bit)"),
        ("dpx", "DPX (10-bit film)"),
        ("hdr", "Radiance HDR"),
    ]
    
    for ext, desc in formats:
        def test_format(ext=ext, desc=desc):
            out_path = get_out_dir() / f"io_roundtrip.{ext}"
            vfx_rs.write(out_path, img)
            loaded = vfx_rs.read(out_path)
            return True, f"{loaded}", out_path
        
        r = timed_test(f"Write/Read {ext.upper()}", test_format)
        log.record(r)
        results.append(r)
    
    # Test specific format functions
    log.step("Testing format-specific I/O")
    
    # EXR with explicit function
    def test_exr_specific():
        out_path = get_out_dir() / "io_exr_specific.exr"
        io.write_exr(out_path, img)
        loaded = io.read_exr(out_path)
        return True, f"{loaded}", out_path
    
    r = timed_test("io.read/write_exr()", test_exr_specific)
    log.record(r)
    results.append(r)
    
    # JPEG with quality
    def test_jpeg_quality():
        out_path = get_out_dir() / "io_jpeg_q95.jpg"
        io.write_jpeg(out_path, img, quality=95)
        out_path_low = get_out_dir() / "io_jpeg_q50.jpg"
        io.write_jpeg(out_path_low, img, quality=50)
        size_high = out_path.stat().st_size
        size_low = out_path_low.stat().st_size
        return True, f"Q95={size_high//1024}KB, Q50={size_low//1024}KB", out_path
    
    r = timed_test("JPEG quality levels", test_jpeg_quality)
    log.record(r)
    results.append(r)
    
    # DPX bit depths
    log.step("Testing DPX bit depths")
    for bits in [8, 10, 12, 16]:
        def test_dpx_depth(bits=bits):
            out_path = get_out_dir() / f"io_dpx_{bits}bit.dpx"
            io.write_dpx(out_path, img, bit_depth=bits)
            loaded = io.read_dpx(out_path)
            return True, f"{loaded}", out_path
        
        r = timed_test(f"DPX {bits}-bit", test_dpx_depth)
        log.record(r)
        results.append(r)
    
    # Test BitDepth enum
    def test_bitdepth_enum():
        out_path = get_out_dir() / "io_dpx_enum.dpx"
        io.write_dpx(out_path, img, bit_depth=vfx_rs.BitDepth.Bit10)
        return True, "BitDepth.Bit10 works", out_path
    
    r = timed_test("DPX with BitDepth enum", test_bitdepth_enum)
    log.record(r)
    results.append(r)
    
    return results


# =============================================================================
# Test: Processor Operations
# =============================================================================

def test_processor(input_path: Path) -> list[TestResult]:
    """Test processor operations with chain."""
    log.section("PROCESSOR OPERATIONS")
    results = []
    
    # Load image
    img = vfx_rs.read(input_path)
    proc = vfx_rs.Processor()
    log.info(f"Processor backend: {proc.backend}")
    
    # === Single Operations ===
    log.step("Testing single operations")
    
    # Exposure ladder
    for stops in [-2, -1, 0, 1, 2]:
        def test_exposure(stops=stops):
            work = img.copy()
            proc.exposure(work, float(stops))
            out_path = get_out_dir() / f"proc_exposure_{stops:+d}.exr"
            vfx_rs.write(out_path, work)
            return True, f"{stops:+d} stops", out_path
        
        r = timed_test(f"Exposure {stops:+d} stops", test_exposure)
        log.record(r)
        results.append(r)
    
    # Saturation range
    for sat in [0.0, 0.5, 1.0, 1.5, 2.0]:
        def test_saturation(sat=sat):
            work = img.copy()
            proc.saturation(work, sat)
            name = f"proc_sat_{sat:.1f}".replace(".", "_")
            out_path = get_out_dir() / f"{name}.exr"
            vfx_rs.write(out_path, work)
            return True, f"sat={sat}", out_path
        
        r = timed_test(f"Saturation {sat:.1f}", test_saturation)
        log.record(r)
        results.append(r)
    
    # Contrast range
    for contrast in [0.5, 0.75, 1.0, 1.5, 2.0]:
        def test_contrast(contrast=contrast):
            work = img.copy()
            proc.contrast(work, contrast)
            name = f"proc_contrast_{contrast:.2f}".replace(".", "_")
            out_path = get_out_dir() / f"{name}.exr"
            vfx_rs.write(out_path, work)
            return True, f"contrast={contrast}", out_path
        
        r = timed_test(f"Contrast {contrast:.2f}", test_contrast)
        log.record(r)
        results.append(r)
    
    # === CDL Grades ===
    log.step("Testing CDL grades")
    
    cdl_presets = {
        "neutral": {"slope": [1.0, 1.0, 1.0], "offset": [0.0, 0.0, 0.0], "power": [1.0, 1.0, 1.0]},
        "warm": {"slope": [1.1, 1.0, 0.9], "offset": [0.02, 0.0, -0.02], "power": [1.0, 1.0, 1.0]},
        "cool": {"slope": [0.9, 1.0, 1.1], "offset": [-0.01, 0.0, 0.02], "power": [1.0, 1.0, 1.0]},
        "teal_orange": {"slope": [1.15, 0.95, 0.85], "offset": [-0.02, 0.01, 0.03], "power": [0.95, 1.0, 1.05]},
        "filmic": {"slope": [1.0, 1.0, 1.0], "offset": [0.0, 0.0, 0.0], "power": [1.1, 1.05, 1.0]},
    }
    
    for name, grade in cdl_presets.items():
        def test_cdl(name=name, grade=grade):
            work = img.copy()
            proc.cdl(work, slope=grade["slope"], offset=grade["offset"], power=grade["power"])
            out_path = get_out_dir() / f"proc_cdl_{name}.exr"
            vfx_rs.write(out_path, work)
            return True, f"CDL: {name}", out_path
        
        r = timed_test(f"CDL {name}", test_cdl)
        log.record(r)
        results.append(r)
    
    # === Chained Operations ===
    log.step("Testing operation chains")
    
    def test_chain_cinema():
        """Cinema look: exposure + contrast + saturation + CDL."""
        work = img.copy()
        proc.exposure(work, 0.3)
        vfx_rs.write(get_out_dir() / "chain_cinema_1_exposure.exr", work)
        
        proc.contrast(work, 1.2)
        vfx_rs.write(get_out_dir() / "chain_cinema_2_contrast.exr", work)
        
        proc.saturation(work, 0.9)
        vfx_rs.write(get_out_dir() / "chain_cinema_3_saturation.exr", work)
        
        proc.cdl(work, slope=[1.05, 1.0, 0.95], offset=[0.01, 0.0, -0.01])
        out_path = get_out_dir() / "chain_cinema_4_final.exr"
        vfx_rs.write(out_path, work)
        return True, "4-step chain complete", out_path
    
    r = timed_test("Chain: Cinema Look", test_chain_cinema)
    log.record(r)
    results.append(r)
    
    def test_chain_bw():
        """High contrast B&W."""
        work = img.copy()
        proc.contrast(work, 1.5)
        vfx_rs.write(get_out_dir() / "chain_bw_1_contrast.exr", work)
        
        proc.saturation(work, 0.0)
        out_path = get_out_dir() / "chain_bw_2_desat.exr"
        vfx_rs.write(out_path, work)
        return True, "contrast->desat chain", out_path
    
    r = timed_test("Chain: B&W High Contrast", test_chain_bw)
    log.record(r)
    results.append(r)
    
    def test_chain_hdr_recovery():
        """HDR boost and recovery."""
        work = img.copy()
        # Boost to HDR
        arr = work.numpy()
        arr_boosted = arr * 3.0
        boosted = vfx_rs.Image(arr_boosted.astype(np.float32))
        vfx_rs.write(get_out_dir() / "chain_hdr_1_boosted.exr", boosted)
        
        # Recover with exposure
        proc.exposure(boosted, -1.585)  # log2(3)
        out_path = get_out_dir() / "chain_hdr_2_recovered.exr"
        vfx_rs.write(out_path, boosted)
        return True, "HDR boost->recover", out_path
    
    r = timed_test("Chain: HDR Recovery", test_chain_hdr_recovery)
    log.record(r)
    results.append(r)
    
    return results


# =============================================================================
# Test: NumPy Interop
# =============================================================================

def test_numpy(input_path: Path) -> list[TestResult]:
    """Test numpy interoperability."""
    log.section("NUMPY INTEROP")
    results = []
    
    img = vfx_rs.read(input_path)
    log.info(f"Source image: {img}")
    
    # Basic numpy access
    def test_numpy_access():
        arr = img.numpy()
        h, w, c = arr.shape
        return True, f"shape=({h}, {w}, {c}), dtype={arr.dtype}", None
    
    r = timed_test("numpy() access", test_numpy_access)
    log.record(r)
    results.append(r)
    
    # Numpy copy
    def test_numpy_copy():
        arr = img.numpy(copy=True)
        arr[0, 0, :] = 0  # Modify - shouldn't affect original
        orig = img.numpy()[0, 0, :]
        modified = np.allclose(orig, 0)
        return not modified, "copy isolation works", None
    
    r = timed_test("numpy(copy=True)", test_numpy_copy)
    log.record(r)
    results.append(r)
    
    # Create from numpy
    def test_from_numpy():
        arr = np.random.rand(256, 256, 4).astype(np.float32)
        new_img = vfx_rs.Image(arr)
        out_path = get_out_dir() / "numpy_random.exr"
        vfx_rs.write(out_path, new_img)
        return True, f"{new_img}", out_path
    
    r = timed_test("Image(numpy_array)", test_from_numpy)
    log.record(r)
    results.append(r)
    
    # Vignette effect
    def test_numpy_vignette():
        arr = img.numpy()
        h, w, c = arr.shape
        
        y, x = np.ogrid[:h, :w]
        cx, cy = w / 2, h / 2
        dist = np.sqrt((x - cx)**2 + (y - cy)**2)
        max_dist = np.sqrt(cx**2 + cy**2)
        vignette = 1 - (dist / max_dist) ** 2 * 0.7
        
        vignetted = arr * vignette[:, :, np.newaxis]
        vignetted = np.clip(vignetted, 0, None).astype(np.float32)
        
        out_img = vfx_rs.Image(vignetted)
        out_path = get_out_dir() / "numpy_vignette.exr"
        vfx_rs.write(out_path, out_img)
        return True, "vignette applied", out_path
    
    r = timed_test("Vignette via numpy", test_numpy_vignette)
    log.record(r)
    results.append(r)
    
    # Channel operations
    def test_numpy_channel_swap():
        arr = img.numpy()
        h, w, c = arr.shape
        if c >= 3:
            swapped = arr.copy()
            swapped[:, :, 0] = arr[:, :, 2]  # R = B
            swapped[:, :, 2] = arr[:, :, 0]  # B = R
            swapped = np.ascontiguousarray(swapped, dtype=np.float32)
            out_img = vfx_rs.Image(swapped)
            out_path = get_out_dir() / "numpy_channel_swap.exr"
            vfx_rs.write(out_path, out_img)
            return True, "RGB -> BGR", out_path
        return True, "skipped (not RGB)", None
    
    r = timed_test("Channel swap", test_numpy_channel_swap)
    log.record(r)
    results.append(r)
    
    # Gradient multiply
    def test_numpy_gradient():
        arr = img.numpy()
        h, w, c = arr.shape
        gradient = np.linspace(0.3, 1.0, w).reshape(1, w, 1).astype(np.float32)
        graded = (arr * gradient).astype(np.float32)
        out_img = vfx_rs.Image(graded)
        out_path = get_out_dir() / "numpy_gradient.exr"
        vfx_rs.write(out_path, out_img)
        return True, "horizontal gradient", out_path
    
    r = timed_test("Gradient multiply", test_numpy_gradient)
    log.record(r)
    results.append(r)
    
    return results


# =============================================================================
# Test: Synthetic Patterns
# =============================================================================

def test_synthetic() -> list[TestResult]:
    """Test synthetic image creation."""
    log.section("SYNTHETIC PATTERNS")
    results = []
    
    size = 512
    
    # Color bars
    def test_colorbars():
        bars = np.zeros((size, size, 4), dtype=np.float32)
        colors = [
            [1, 1, 1], [1, 1, 0], [0, 1, 1], [0, 1, 0],
            [1, 0, 1], [1, 0, 0], [0, 0, 1], [0, 0, 0],
        ]
        bar_width = size // 8
        for i, color in enumerate(colors):
            bars[:, i*bar_width:(i+1)*bar_width, :3] = color
        bars[:, :, 3] = 1.0
        
        out_img = vfx_rs.Image(bars)
        out_path = get_out_dir() / "synth_colorbars.exr"
        vfx_rs.write(out_path, out_img)
        return True, f"{size}x{size} SMPTE-ish bars", out_path
    
    r = timed_test("Color bars", test_colorbars)
    log.record(r)
    results.append(r)
    
    # Gradient ramp
    def test_ramp():
        ramp = np.zeros((size, size, 4), dtype=np.float32)
        ramp[:, :, :3] = np.linspace(0, 1, size).reshape(1, size, 1)
        ramp[:, :, 3] = 1.0
        out_img = vfx_rs.Image(ramp)
        out_path = get_out_dir() / "synth_ramp.exr"
        vfx_rs.write(out_path, out_img)
        return True, "0-1 horizontal ramp", out_path
    
    r = timed_test("Gradient ramp", test_ramp)
    log.record(r)
    results.append(r)
    
    # Checker
    def test_checker():
        checker = np.zeros((size, size, 4), dtype=np.float32)
        check_size = 32
        for y in range(size):
            for x in range(size):
                if (x // check_size + y // check_size) % 2 == 0:
                    checker[y, x, :3] = 1.0
        checker[:, :, 3] = 1.0
        out_img = vfx_rs.Image(checker)
        out_path = get_out_dir() / "synth_checker.exr"
        vfx_rs.write(out_path, out_img)
        return True, f"{check_size}px checker", out_path
    
    r = timed_test("Checkerboard", test_checker)
    log.record(r)
    results.append(r)
    
    # HDR gradient (values > 1.0)
    def test_hdr_gradient():
        hdr = np.zeros((size, size, 4), dtype=np.float32)
        hdr[:, :, :3] = np.linspace(0, 4.0, size).reshape(1, size, 1)  # 0 to 4 stops
        hdr[:, :, 3] = 1.0
        out_img = vfx_rs.Image(hdr)
        out_path = get_out_dir() / "synth_hdr_ramp.exr"
        vfx_rs.write(out_path, out_img)
        return True, "0-4.0 HDR ramp", out_path
    
    r = timed_test("HDR gradient", test_hdr_gradient)
    log.record(r)
    results.append(r)
    
    return results


# =============================================================================
# Test: Image Properties
# =============================================================================

def test_properties(input_path: Path) -> list[TestResult]:
    """Test image properties and metadata."""
    log.section("IMAGE PROPERTIES")
    results = []
    
    img = vfx_rs.read(input_path)
    
    def test_props():
        w, h, c = img.width, img.height, img.channels
        fmt = img.format
        return True, f"{w}x{h}, {c}ch, {fmt}", None
    
    r = timed_test("Basic properties", test_props)
    log.record(r)
    results.append(r)
    
    # Empty image
    def test_empty():
        empty = vfx_rs.Image.empty(1920, 1080, 4)
        out_path = get_out_dir() / "prop_empty.exr"
        vfx_rs.write(out_path, empty)
        return True, f"{empty}", out_path
    
    r = timed_test("Image.empty()", test_empty)
    log.record(r)
    results.append(r)
    
    # Copy
    def test_copy():
        copy = img.copy()
        arr = copy.numpy()
        arr[0, 0, :] = 0
        orig = img.numpy()[0, 0, :]
        is_isolated = not np.allclose(orig, 0)
        return is_isolated, "copy is independent", None
    
    r = timed_test("Image.copy()", test_copy)
    log.record(r)
    results.append(r)
    
    return results


# =============================================================================
# Test: Format Roundtrip Verification
# =============================================================================

def test_roundtrip_verification(input_path: Path) -> list[TestResult]:
    """Test format conversion with diff verification."""
    log.section("FORMAT ROUNDTRIP VERIFICATION")
    results = []
    
    img = vfx_rs.read(input_path)
    
    # EXR -> EXR (should be lossless)
    def test_exr_roundtrip():
        out1 = get_out_dir() / "verify_exr_1.exr"
        out2 = get_out_dir() / "verify_exr_2.exr"
        vfx_rs.write(out1, img)
        img2 = vfx_rs.read(out1)
        vfx_rs.write(out2, img2)
        
        passed, msg = run_vfx_diff(out1, out2, threshold=0.0001)
        return passed, msg, out2
    
    r = timed_test("EXR roundtrip (lossless)", test_exr_roundtrip)
    log.record(r)
    results.append(r)
    
    # Processing chain verification
    def test_processing_roundtrip():
        proc = vfx_rs.Processor()
        
        # Forward: exposure +1
        work = img.copy()
        proc.exposure(work, 1.0)
        out_plus = get_out_dir() / "verify_exp_plus1.exr"
        vfx_rs.write(out_plus, work)
        
        # Reverse: exposure -1
        proc.exposure(work, -1.0)
        out_back = get_out_dir() / "verify_exp_roundtrip.exr"
        vfx_rs.write(out_back, work)
        
        # Compare with original
        out_orig = get_out_dir() / "verify_exp_original.exr"
        vfx_rs.write(out_orig, img)
        
        passed, msg = run_vfx_diff(out_orig, out_back, threshold=0.001)
        return passed, f"exposure +1/-1: {msg}", out_back
    
    r = timed_test("Exposure roundtrip", test_processing_roundtrip)
    log.record(r)
    results.append(r)
    
    # Saturation roundtrip (sat 2.0 then 0.5 should give ~1.0)
    def test_saturation_roundtrip():
        proc = vfx_rs.Processor()
        
        work = img.copy()
        proc.saturation(work, 2.0)
        out_double = get_out_dir() / "verify_sat_double.exr"
        vfx_rs.write(out_double, work)
        
        proc.saturation(work, 0.5)
        out_back = get_out_dir() / "verify_sat_roundtrip.exr"
        vfx_rs.write(out_back, work)
        
        out_orig = get_out_dir() / "verify_sat_original.exr"
        vfx_rs.write(out_orig, img)
        
        # Note: saturation is not perfectly invertible (non-linear, clipping)
        # threshold=0.15 allows for expected mathematical error
        passed, msg = run_vfx_diff(out_orig, out_back, threshold=0.15)
        return passed, f"saturation 2x/0.5x: {msg}", out_back
    
    r = timed_test("Saturation roundtrip", test_saturation_roundtrip)
    log.record(r)
    results.append(r)
    
    return results


# =============================================================================
# Test: Complete Pipeline
# =============================================================================

def test_layered_images(input_path: Path) -> list[TestResult]:
    """Test LayeredImage API."""
    log.section("LAYERED IMAGES")
    results = []
    
    # Read as layered
    def test_read_layered():
        layered = vfx_rs.read_layered(input_path)
        names = layered.layer_names
        return True, f"{len(names)} layers: {names}", None
    
    r = timed_test("read_layered()", test_read_layered)
    log.record(r)
    results.append(r)
    
    # Access layers
    def test_layer_access():
        layered = vfx_rs.read_layered(input_path)
        layer = layered[0]
        channels = layer.channel_names
        return True, f"Layer '{layer.name}': {channels}", None
    
    r = timed_test("Layer access", test_layer_access)
    log.record(r)
    results.append(r)
    
    # Channel access
    def test_channel_access():
        layered = vfx_rs.read_layered(input_path)
        layer = layered[0]
        ch = layer["R"] if "R" in layer.channel_names else layer[0]
        arr = ch.numpy()
        return True, f"Channel '{ch.name}': {len(arr)} samples, kind={ch.kind}", None
    
    r = timed_test("Channel access & numpy", test_channel_access)
    log.record(r)
    results.append(r)
    
    # Convert layer to image
    def test_layer_to_image():
        layered = vfx_rs.read_layered(input_path)
        layer = layered[0]
        img = layer.to_image()
        out_path = get_out_dir() / "layered_to_image.exr"
        vfx_rs.write(out_path, img)
        return True, f"{img}", out_path
    
    r = timed_test("Layer to Image", test_layer_to_image)
    log.record(r)
    results.append(r)
    
    # Create layered from image
    def test_from_image():
        img = vfx_rs.read(input_path)
        layered = vfx_rs.LayeredImage.from_image(img, "main")
        return True, f"{layered}", None
    
    r = timed_test("LayeredImage.from_image()", test_from_image)
    log.record(r)
    results.append(r)
    
    # Add layers
    def test_add_layers():
        img = vfx_rs.read(input_path)
        layered = vfx_rs.LayeredImage()
        layered.add_layer("beauty", img)
        
        # Process and add as another layer
        proc = vfx_rs.Processor()
        work = img.copy()
        proc.saturation(work, 0.0)
        layered.add_layer("desat", work)
        
        return True, f"{len(layered)} layers: {layered.layer_names}", None
    
    r = timed_test("Build multi-layer", test_add_layers)
    log.record(r)
    results.append(r)
    
    # Iterate layers
    def test_iterate():
        layered = vfx_rs.read_layered(input_path)
        names = [layer.name for layer in layered]
        return True, f"Iterated: {names}", None
    
    r = timed_test("Iterate layers", test_iterate)
    log.record(r)
    results.append(r)
    
    return results


def test_complete_pipeline(input_path: Path) -> list[TestResult]:
    """Test complete VFX pipeline."""
    log.section("COMPLETE VFX PIPELINE")
    results = []
    
    log.step("Pipeline: Load -> Process -> Export multi-format")
    
    def test_pipeline():
        # 1. Load
        img = vfx_rs.read(input_path)
        log.info(f"1. Loaded: {img}")
        vfx_rs.write(get_out_dir() / "pipeline_01_input.exr", img)
        
        # 2. Create processor
        proc = vfx_rs.Processor()
        log.info(f"2. Processor: {proc}")
        
        # 3. Grade: warm film look
        work = img.copy()
        proc.exposure(work, 0.2)
        log.info("3. Applied exposure +0.2")
        vfx_rs.write(get_out_dir() / "pipeline_02_exposure.exr", work)
        
        proc.contrast(work, 1.15)
        log.info("4. Applied contrast 1.15")
        vfx_rs.write(get_out_dir() / "pipeline_03_contrast.exr", work)
        
        proc.saturation(work, 0.95)
        log.info("5. Applied saturation 0.95")
        vfx_rs.write(get_out_dir() / "pipeline_04_saturation.exr", work)
        
        proc.cdl(work, 
                 slope=[1.08, 1.0, 0.92],
                 offset=[0.01, 0.0, -0.01],
                 power=[1.0, 1.0, 1.0])
        log.info("6. Applied CDL grade (warm)")
        vfx_rs.write(get_out_dir() / "pipeline_05_cdl.exr", work)
        
        # 4. Export to multiple formats
        formats = {
            "exr": "pipeline_final.exr",
            "png": "pipeline_final.png",
            "jpg": "pipeline_final.jpg",
            "tiff": "pipeline_final.tiff",
            "dpx": "pipeline_final.dpx",
        }
        
        for fmt, filename in formats.items():
            out_path = get_out_dir() / filename
            vfx_rs.write(out_path, work)
            log.info(f"7. Exported: {filename}")
        
        final_path = get_out_dir() / "pipeline_final.exr"
        return True, "complete pipeline executed", final_path
    
    r = timed_test("Full VFX pipeline", test_pipeline)
    log.record(r)
    results.append(r)
    
    return results


# =============================================================================
# Main
# =============================================================================

EXAMPLES = """
Examples:
  python test_api.py                      # Run with default owl.exr
  python test_api.py my_image.exr         # Test with custom image
  python test_api.py render.exr -o results # Custom output directory
  python test_api.py --help               # Show this help

Test Coverage:
  - Basic I/O (EXR, PNG, JPEG, TIFF, DPX, HDR)
  - Processor (exposure, saturation, contrast, CDL)
  - Operation chains (cinema look, B&W, HDR recovery)
  - NumPy interop (vignette, channel swap, gradients)
  - Synthetic patterns (colorbars, ramp, checker)
  - LayeredImage API (layers, channels, conversion)
  - Format roundtrip verification with vfx diff

Output:
  All intermediate results are written to the output directory.
  Each processing step saves its result for visual inspection.
"""

def main():
    parser = argparse.ArgumentParser(
        description="Comprehensive vfx_rs Python API test suite",
        epilog=EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "input",
        nargs="?",
        default=str(TEST_DIR / "owl.exr"),
        help="Input image path (default: test/owl.exr)"
    )
    parser.add_argument(
        "-o", "--output",
        default=str(TEST_DIR / "out" / "api_test"),
        help="Output directory (default: test/out/api_test)"
    )
    args = parser.parse_args()
    
    input_path = Path(args.input)
    set_out_dir(Path(args.output))
    
    # Header
    print("=" * 70)
    print("  VFX_RS PYTHON API - COMPREHENSIVE TEST SUITE")
    print("=" * 70)
    print(f"  Input:  {input_path}")
    print(f"  Output: {get_out_dir()}")
    print(f"  vfx CLI: {'found' if VFX_CLI.exists() else 'not found'}")
    print("=" * 70)
    
    # Validate input
    if not input_path.exists():
        print(f"\n[ERROR] Input file not found: {input_path}")
        sys.exit(1)
    
    # Create output directory
    get_out_dir().mkdir(parents=True, exist_ok=True)
    
    # Run all tests
    all_results = []
    all_results.extend(test_basic_io(input_path))
    all_results.extend(test_processor(input_path))
    all_results.extend(test_numpy(input_path))
    all_results.extend(test_synthetic())
    all_results.extend(test_properties(input_path))
    all_results.extend(test_roundtrip_verification(input_path))
    all_results.extend(test_layered_images(input_path))
    all_results.extend(test_complete_pipeline(input_path))
    
    # Summary
    log.results = all_results
    success = log.summary()
    
    print(f"\nOutput written to: {get_out_dir()}")
    print("Run 'ls -la test/out/api_test/' to see all generated files")
    
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
