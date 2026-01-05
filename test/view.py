#!/usr/bin/env python3
"""Test script for vfx-view image viewer."""

import subprocess
import sys
from pathlib import Path

# Paths
ROOT = Path(__file__).parent.parent
VFX_EXE = ROOT / "target" / "release" / "vfx.exe"
TEST_IMAGE = Path(__file__).parent / "owl.exr"


def main():
    # Check executable exists
    if not VFX_EXE.exists():
        print(f"Error: {VFX_EXE} not found")
        print("Run: cargo build --release -p vfx-cli")
        return 1

    # Use provided arg or default test image
    image = Path(sys.argv[1]) if len(sys.argv) > 1 else TEST_IMAGE
    
    if not image.exists():
        print(f"Error: {image} not found")
        return 1

    print(f"Opening: {image}")
    result = subprocess.run([str(VFX_EXE), "view", str(image)])
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
