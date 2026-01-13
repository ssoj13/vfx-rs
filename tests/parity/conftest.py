"""
Pytest configuration and fixtures for OCIO parity tests.

Prerequisites:
    pip install pytest numpy PyOpenColorIO
    cd crates/vfx-rs-py && maturin develop --release
"""

import pytest
import numpy as np
from pathlib import Path
import os
import sys

# Tolerance for float comparisons
RTOL = 1e-4  # relative tolerance
ATOL = 1e-6  # absolute tolerance (for values near zero)


def pytest_configure(config):
    """Configure pytest markers."""
    config.addinivalue_line("markers", "slow: marks tests as slow")
    config.addinivalue_line("markers", "transfer: transfer function tests")
    config.addinivalue_line("markers", "matrix: matrix transform tests")
    config.addinivalue_line("markers", "lut: LUT operation tests")
    config.addinivalue_line("markers", "ops: grading operation tests")
    config.addinivalue_line("markers", "processor: full processor chain tests")


@pytest.fixture(scope="session")
def project_root() -> Path:
    """Get project root directory."""
    return Path(__file__).parent.parent.parent


@pytest.fixture(scope="session")
def golden_dir(project_root) -> Path:
    """Get golden test data directory."""
    return project_root / "tests" / "golden"


@pytest.fixture(scope="session")
def reference_images_dir(golden_dir) -> Path:
    """Get reference images directory."""
    return golden_dir / "reference_images"


@pytest.fixture(scope="session")
def ocio_ref_dir(project_root) -> Path:
    """Get OCIO reference source directory."""
    return project_root / "_ref" / "OpenColorIO"


@pytest.fixture(scope="session")
def ocio_test_data(ocio_ref_dir) -> Path:
    """Get OCIO test data directory."""
    return ocio_ref_dir / "tests" / "data"


# ---------------------------------------------------------------------------
# OCIO fixtures
# ---------------------------------------------------------------------------

@pytest.fixture(scope="session")
def ocio():
    """Import PyOpenColorIO, skip if not available."""
    try:
        import PyOpenColorIO as OCIO
        return OCIO
    except ImportError:
        try:
            import opencolorio as OCIO
            return OCIO
        except ImportError:
            pytest.skip("OpenColorIO not installed (pip install opencolorio)")


@pytest.fixture(scope="session")
def ocio_builtin_config(ocio):
    """Get OCIO built-in config (studio-config-v2.1.0_aces-v1.3_ocio-v2.3)."""
    try:
        config = ocio.Config.CreateFromBuiltinConfig("studio-config-v2.1.0_aces-v1.3_ocio-v2.3")
        return config
    except Exception:
        # Fallback to default
        return ocio.Config.CreateRaw()


@pytest.fixture(scope="session")
def ocio_raw_config(ocio):
    """Get minimal raw OCIO config."""
    return ocio.Config.CreateRaw()


# ---------------------------------------------------------------------------
# vfx-rs fixtures
# ---------------------------------------------------------------------------

@pytest.fixture(scope="session")
def vfx():
    """Import vfx-rs Python bindings, skip if not available."""
    try:
        import vfx_rs
        return vfx_rs
    except ImportError:
        pytest.skip("vfx-rs-py not built. Run: cd crates/vfx-rs-py && maturin develop --release")


@pytest.fixture(scope="session")
def vfx_transfer(vfx):
    """Get vfx transfer module."""
    return vfx.transfer


@pytest.fixture(scope="session")
def vfx_color(vfx):
    """Get vfx color module."""
    return vfx.color


@pytest.fixture(scope="session")
def vfx_lut(vfx):
    """Get vfx lut module."""
    return vfx.lut


# ---------------------------------------------------------------------------
# Test image fixtures
# ---------------------------------------------------------------------------

@pytest.fixture(scope="session")
def gray_ramp_1d() -> np.ndarray:
    """1D gray ramp from 0 to 1, 256 values."""
    return np.linspace(0.0, 1.0, 256, dtype=np.float32)


@pytest.fixture(scope="session")
def gray_ramp_hdr() -> np.ndarray:
    """HDR gray ramp from 0 to 100, 256 values."""
    return np.linspace(0.0, 100.0, 256, dtype=np.float32)


@pytest.fixture(scope="session")
def gray_ramp_negative() -> np.ndarray:
    """Gray ramp with negative values, -1 to 2."""
    return np.linspace(-1.0, 2.0, 256, dtype=np.float32)


@pytest.fixture(scope="session")
def gray_ramp_2d() -> np.ndarray:
    """2D gray ramp image 16x16, single channel."""
    ramp = np.linspace(0.0, 1.0, 256, dtype=np.float32)
    return ramp.reshape(16, 16)


@pytest.fixture(scope="session")
def rgb_ramp_2d() -> np.ndarray:
    """2D RGB ramp image 16x16x3."""
    r = np.linspace(0.0, 1.0, 256, dtype=np.float32).reshape(16, 16)
    g = np.linspace(0.0, 1.0, 256, dtype=np.float32).reshape(16, 16)
    b = np.linspace(0.0, 1.0, 256, dtype=np.float32).reshape(16, 16)
    # Shift g and b to create color variation
    g = np.roll(g, 4, axis=0)
    b = np.roll(b, 8, axis=1)
    return np.stack([r, g, b], axis=-1)


@pytest.fixture(scope="session")
def color_cube_3d() -> np.ndarray:
    """3D color cube 8x8x8x3 for LUT testing."""
    size = 8
    r = np.linspace(0, 1, size, dtype=np.float32)
    g = np.linspace(0, 1, size, dtype=np.float32)
    b = np.linspace(0, 1, size, dtype=np.float32)
    rr, gg, bb = np.meshgrid(r, g, b, indexing='ij')
    return np.stack([rr, gg, bb], axis=-1)


@pytest.fixture
def random_rgb_image() -> np.ndarray:
    """Random RGB image 64x64x3 for stress testing."""
    np.random.seed(42)
    return np.random.rand(64, 64, 3).astype(np.float32)


# ---------------------------------------------------------------------------
# Comparison helpers
# ---------------------------------------------------------------------------

def assert_close(actual: np.ndarray, expected: np.ndarray, 
                 rtol: float = RTOL, atol: float = ATOL,
                 msg: str = ""):
    """Assert arrays are close within tolerance."""
    np.testing.assert_allclose(actual, expected, rtol=rtol, atol=atol, 
                               err_msg=msg)


def max_abs_diff(a: np.ndarray, b: np.ndarray) -> float:
    """Calculate maximum absolute difference."""
    return float(np.max(np.abs(a - b)))


def max_rel_diff(a: np.ndarray, b: np.ndarray, eps: float = 1e-10) -> float:
    """Calculate maximum relative difference."""
    return float(np.max(np.abs(a - b) / (np.abs(b) + eps)))


def psnr(a: np.ndarray, b: np.ndarray, max_val: float = 1.0) -> float:
    """Calculate PSNR between two images."""
    mse = np.mean((a - b) ** 2)
    if mse == 0:
        return float('inf')
    return 20 * np.log10(max_val / np.sqrt(mse))


# ---------------------------------------------------------------------------
# OCIO helpers
# ---------------------------------------------------------------------------

def apply_ocio_cpu(processor, pixels: np.ndarray) -> np.ndarray:
    """Apply OCIO processor to pixel array using CPU."""
    try:
        import PyOpenColorIO as ocio
    except ImportError:
        import opencolorio as ocio
    
    cpu = processor.getDefaultCPUProcessor()
    result = pixels.copy()
    
    if result.ndim == 1:
        # 1D array of single values - process as RGB
        rgb = np.column_stack([result, result, result])
        cpu.applyRGB(rgb)
        return rgb[:, 0]  # Return just one channel
    elif result.ndim == 2:
        # 2D grayscale - expand to RGB
        rgb = np.stack([result, result, result], axis=-1)
        flat = rgb.reshape(-1, 3)
        cpu.applyRGB(flat)
        return flat.reshape(result.shape + (3,))[..., 0]
    elif result.ndim == 3 and result.shape[-1] == 3:
        # RGB image
        flat = result.reshape(-1, 3).copy()
        cpu.applyRGB(flat)
        return flat.reshape(result.shape)
    elif result.ndim == 3 and result.shape[-1] == 4:
        # RGBA image
        flat = result.reshape(-1, 4).copy()
        cpu.applyRGBA(flat)
        return flat.reshape(result.shape)
    else:
        raise ValueError(f"Unsupported pixel shape: {result.shape}")


def ocio_builtin_transform(ocio, name: str, direction: str = "FORWARD"):
    """Create OCIO builtin transform."""
    direction_enum = (ocio.TransformDirection.TRANSFORM_DIR_FORWARD 
                      if direction == "FORWARD" 
                      else ocio.TransformDirection.TRANSFORM_DIR_INVERSE)
    transform = ocio.BuiltinTransform(name)
    transform.setDirection(direction_enum)
    return transform


# ---------------------------------------------------------------------------
# Hash helpers
# ---------------------------------------------------------------------------

def pixel_hash(pixels: np.ndarray) -> str:
    """Compute SHA256 hash of pixel data."""
    import hashlib
    # Quantize to avoid float precision issues
    quantized = (pixels * 1e6).astype(np.int32)
    return hashlib.sha256(quantized.tobytes()).hexdigest()


def save_golden_hash(name: str, pixels: np.ndarray, golden_dir: Path):
    """Save golden hash to JSON file."""
    import json
    
    hash_file = golden_dir / "hashes.json"
    if hash_file.exists():
        with open(hash_file) as f:
            hashes = json.load(f)
    else:
        hashes = {"version": "1.0", "tolerance": RTOL, "tests": {}}
    
    hashes["tests"][name] = {
        "hash": pixel_hash(pixels),
        "shape": list(pixels.shape),
        "dtype": str(pixels.dtype),
        "min": float(np.min(pixels)),
        "max": float(np.max(pixels)),
    }
    
    with open(hash_file, "w") as f:
        json.dump(hashes, f, indent=2)


def load_golden_hash(name: str, golden_dir: Path) -> dict:
    """Load golden hash from JSON file."""
    import json
    
    hash_file = golden_dir / "hashes.json"
    if not hash_file.exists():
        pytest.skip(f"Golden hashes not found: {hash_file}")
    
    with open(hash_file) as f:
        hashes = json.load(f)
    
    if name not in hashes.get("tests", {}):
        pytest.skip(f"Golden hash not found for: {name}")
    
    return hashes["tests"][name]
