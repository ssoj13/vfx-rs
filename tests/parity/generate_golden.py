#!/usr/bin/env python3
"""
Generate golden reference hashes from PyOpenColorIO.

This script creates reference data that can be used by Rust tests
to verify vfx-rs matches OCIO without requiring Python at test time.

Usage:
    python generate_golden.py [--output ../golden/hashes.json]
"""

import argparse
import hashlib
import json
import sys
from pathlib import Path
from datetime import datetime

import numpy as np

try:
    import PyOpenColorIO as ocio
    OCIO_VERSION = ocio.__version__
except ImportError:
    try:
        import opencolorio as ocio
        OCIO_VERSION = ocio.__version__
    except ImportError:
        print("ERROR: OpenColorIO not installed")
        print("Install with: pip install opencolorio")
        sys.exit(1)


# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

TOLERANCE = 1e-4
OUTPUT_FILE = Path(__file__).parent.parent / "golden" / "hashes.json"

# Test input arrays
GRAY_RAMP_256 = np.linspace(0.0, 1.0, 256, dtype=np.float32)
GRAY_RAMP_HDR = np.linspace(0.0, 100.0, 256, dtype=np.float32)
RGB_CUBE_8 = None  # Generated below


def generate_rgb_cube(size=8):
    """Generate RGB color cube."""
    r = np.linspace(0, 1, size, dtype=np.float32)
    g = np.linspace(0, 1, size, dtype=np.float32)
    b = np.linspace(0, 1, size, dtype=np.float32)
    rr, gg, bb = np.meshgrid(r, g, b, indexing='ij')
    return np.stack([rr.flatten(), gg.flatten(), bb.flatten()], axis=-1)


RGB_CUBE_8 = generate_rgb_cube(8)


# ---------------------------------------------------------------------------
# Hash helpers
# ---------------------------------------------------------------------------

def compute_hash(data: np.ndarray, precision: int = 5) -> str:
    """Compute deterministic hash of float array."""
    # Quantize to avoid float precision issues
    quantized = np.round(data * (10 ** precision)).astype(np.int64)
    return hashlib.sha256(quantized.tobytes()).hexdigest()


def compute_stats(data: np.ndarray) -> dict:
    """Compute statistics for validation."""
    return {
        "min": float(np.min(data)),
        "max": float(np.max(data)),
        "mean": float(np.mean(data)),
        "std": float(np.std(data)),
    }


# ---------------------------------------------------------------------------
# OCIO transforms
# ---------------------------------------------------------------------------

def apply_builtin_transform(name: str, pixels: np.ndarray, 
                            direction: str = "FORWARD") -> np.ndarray:
    """Apply OCIO builtin transform."""
    config = ocio.Config.CreateRaw()
    
    dir_enum = (ocio.TransformDirection.TRANSFORM_DIR_FORWARD 
                if direction == "FORWARD"
                else ocio.TransformDirection.TRANSFORM_DIR_INVERSE)
    
    transform = ocio.BuiltinTransform(name)
    transform.setDirection(dir_enum)
    
    processor = config.getProcessor(transform)
    cpu = processor.getDefaultCPUProcessor()
    
    result = pixels.copy()
    if result.ndim == 1:
        # Expand to RGB
        rgb = np.column_stack([result, result, result])
        cpu.applyRGB(rgb)
        return rgb[:, 0]
    elif result.ndim == 2 and result.shape[-1] == 3:
        flat = result.reshape(-1, 3).copy()
        cpu.applyRGB(flat)
        return flat.reshape(result.shape)
    else:
        raise ValueError(f"Unsupported shape: {result.shape}")


def apply_matrix_transform(matrix: list, pixels: np.ndarray) -> np.ndarray:
    """Apply OCIO matrix transform."""
    config = ocio.Config.CreateRaw()
    
    # OCIO expects 4x4 matrix as flat list
    if len(matrix) == 9:
        # Expand 3x3 to 4x4
        m = np.eye(4, dtype=np.float64)
        m[:3, :3] = np.array(matrix).reshape(3, 3)
        matrix = m.flatten().tolist()
    
    transform = ocio.MatrixTransform(matrix)
    processor = config.getProcessor(transform)
    cpu = processor.getDefaultCPUProcessor()
    
    result = pixels.copy()
    flat = result.reshape(-1, 3).copy()
    cpu.applyRGB(flat)
    return flat.reshape(result.shape)


def apply_cdl_transform(slope, offset, power, sat, pixels: np.ndarray,
                        style: str = "CDL_ASC") -> np.ndarray:
    """Apply OCIO CDL transform.
    
    Args:
        slope: RGB slope values
        offset: RGB offset values  
        power: RGB power values
        sat: Saturation value
        pixels: Input RGB pixels
        style: CDL style - "CDL_ASC" (clamped, default) or "CDL_NO_CLAMP"
    """
    config = ocio.Config.CreateRaw()
    
    transform = ocio.CDLTransform()
    transform.setSlope(slope)
    transform.setOffset(offset)
    transform.setPower(power)
    transform.setSat(sat)
    
    # Set CDL style - CDL_ASC matches ASC CDL v1.2 spec with clamping
    # This matches vfx-rs Cdl::apply() behavior
    cdl_style = getattr(ocio, style, ocio.CDL_ASC)
    transform.setStyle(cdl_style)
    
    processor = config.getProcessor(transform)
    cpu = processor.getDefaultCPUProcessor()
    
    result = pixels.copy()
    flat = result.reshape(-1, 3).copy()
    cpu.applyRGB(flat)
    return flat.reshape(result.shape)


# ---------------------------------------------------------------------------
# Generate golden data
# ---------------------------------------------------------------------------

def generate_transfer_hashes() -> dict:
    """Generate hashes for transfer function tests."""
    hashes = {}
    
    # OCIO builtin transform names (OCIO 2.x naming)
    transfers = [
        # Camera logs - to linear (using CURVE transforms where available)
        ("apple_log_to_linear", "CURVE - APPLE_LOG_to_LINEAR", GRAY_RAMP_256),
        ("canon_clog2_to_linear", "CURVE - CANON_CLOG2_to_LINEAR", GRAY_RAMP_256),
        ("canon_clog3_to_linear", "CURVE - CANON_CLOG3_to_LINEAR", GRAY_RAMP_256),
        
        # HDR
        ("pq_to_linear", "CURVE - ST-2084_to_LINEAR", GRAY_RAMP_256),
        ("hlg_oetf_inv", "CURVE - HLG-OETF-INVERSE", GRAY_RAMP_256),
        
        # ACES curves
        ("acescct_to_linear", "CURVE - ACEScct-LOG_to_LINEAR", GRAY_RAMP_256),
    ]
    
    for name, ocio_name, input_data in transfers:
        try:
            result = apply_builtin_transform(ocio_name, input_data)
            hashes[name] = {
                "ocio_transform": ocio_name,
                "input_type": "gray_ramp_256",
                "hash": compute_hash(result),
                "stats": compute_stats(result),
            }
            print(f"  [OK] {name}")
        except Exception as e:
            print(f"  [SKIP] {name}: {e}")
    
    return hashes


def generate_matrix_hashes() -> dict:
    """Generate hashes for matrix tests."""
    hashes = {}
    
    # Reference matrices
    SRGB_TO_XYZ = [
        0.4124564, 0.3575761, 0.1804375,
        0.2126729, 0.7151522, 0.0721750,
        0.0193339, 0.1191920, 0.9503041,
    ]
    
    AP0_TO_AP1 = [
        1.4514393161, -0.2365107469, -0.2149285693,
        -0.0765537734, 1.1762296998, -0.0996759264,
        0.0083161484, -0.0060324498, 0.9977163014,
    ]
    
    matrices = [
        ("srgb_to_xyz", SRGB_TO_XYZ),
        ("ap0_to_ap1", AP0_TO_AP1),
    ]
    
    for name, matrix in matrices:
        try:
            result = apply_matrix_transform(matrix, RGB_CUBE_8)
            hashes[name] = {
                "matrix": matrix,
                "input_type": "rgb_cube_8",
                "hash": compute_hash(result),
                "stats": compute_stats(result),
            }
            print(f"  [OK] {name}")
        except Exception as e:
            print(f"  [SKIP] {name}: {e}")
    
    return hashes


def generate_cdl_hashes() -> dict:
    """Generate hashes for CDL tests."""
    hashes = {}
    
    cdl_configs = [
        ("cdl_identity", [1, 1, 1], [0, 0, 0], [1, 1, 1], 1.0),
        ("cdl_warmup", [1.1, 1.0, 0.9], [0.02, 0, -0.02], [1, 1, 1], 1.1),
        ("cdl_contrast", [1, 1, 1], [-0.1, -0.1, -0.1], [1.2, 1.2, 1.2], 1.0),
    ]
    
    for name, slope, offset, power, sat in cdl_configs:
        try:
            result = apply_cdl_transform(slope, offset, power, sat, RGB_CUBE_8)
            hashes[name] = {
                "slope": slope,
                "offset": offset,
                "power": power,
                "saturation": sat,
                "input_type": "rgb_cube_8",
                "hash": compute_hash(result),
                "stats": compute_stats(result),
            }
            print(f"  [OK] {name}")
        except Exception as e:
            print(f"  [SKIP] {name}: {e}")
    
    return hashes


def generate_all_hashes(output_path: Path):
    """Generate all golden hashes."""
    print(f"Generating golden hashes with OCIO {OCIO_VERSION}")
    print(f"Output: {output_path}")
    print()
    
    golden = {
        "version": "1.0",
        "generated": datetime.now().isoformat(),
        "ocio_version": OCIO_VERSION,
        "tolerance": TOLERANCE,
        "input_data": {
            "gray_ramp_256": {
                "type": "linspace",
                "start": 0.0,
                "end": 1.0,
                "count": 256,
                "dtype": "float32",
            },
            "rgb_cube_8": {
                "type": "meshgrid_cube",
                "size": 8,
                "dtype": "float32",
            },
        },
        "tests": {},
    }
    
    # Generate each category
    print("Transfer functions:")
    golden["tests"]["transfers"] = generate_transfer_hashes()
    print()
    
    print("Matrix transforms:")
    golden["tests"]["matrices"] = generate_matrix_hashes()
    print()
    
    print("CDL operations:")
    golden["tests"]["cdl"] = generate_cdl_hashes()
    print()
    
    # Write output
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w") as f:
        json.dump(golden, f, indent=2)
    
    # Summary
    total = sum(len(v) for v in golden["tests"].values())
    print(f"Generated {total} golden hashes")
    print(f"Saved to: {output_path}")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="Generate golden reference hashes from PyOpenColorIO"
    )
    parser.add_argument(
        "--output", "-o",
        type=Path,
        default=OUTPUT_FILE,
        help=f"Output JSON file (default: {OUTPUT_FILE})"
    )
    
    args = parser.parse_args()
    generate_all_hashes(args.output)


if __name__ == "__main__":
    main()
