#!/usr/bin/env python3
"""
Launch the vfx-rs image viewer.

Usage:
    python view.py                  # View test/owl.exr
    python view.py image.exr        # View specific file
    python view.py *.exr            # View multiple files
"""
import _bootstrap  # noqa: F401 - ensures venv Python
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parents[2]
VFX_EXE = ROOT / "target" / "release" / "vfx.exe"
if not VFX_EXE.exists():
    VFX_EXE = ROOT / "target" / "release" / "vfx"

TEST_DIR = Path(__file__).parent.parent
DEFAULT_IMAGE = TEST_DIR / "owl.exr"


def main():
    if not VFX_EXE.exists():
        print(f"Error: vfx CLI not found at {VFX_EXE}")
        print("Build it with: cargo build --release -p vfx-cli")
        return 1

    # Get image paths from args or use default
    if len(sys.argv) > 1:
        images = [Path(arg) for arg in sys.argv[1:]]
    else:
        images = [DEFAULT_IMAGE]

    # Verify files exist
    for img in images:
        if not img.exists():
            print(f"Error: {img} not found")
            return 1

    # Launch viewer
    for img in images:
        print(f"Opening: {img}")
        result = subprocess.run([str(VFX_EXE), "view", str(img)])
        if result.returncode != 0:
            return result.returncode

    return 0


if __name__ == "__main__":
    sys.exit(main())
