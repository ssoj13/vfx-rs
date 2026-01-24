"""
Bootstrap module for vfx-rs Python examples.

Ensures we're using the project's venv Python which has vfx_rs installed.
Import this at the top of any example script before importing vfx_rs.
"""
import os
import sys
from pathlib import Path

# Find the venv relative to this file (test/py/_bootstrap.py -> .venv)
_THIS_DIR = Path(__file__).resolve().parent
_PROJECT_ROOT = _THIS_DIR.parent.parent
_VENV_DIR = _PROJECT_ROOT / ".venv"
VENV_PYTHON = _VENV_DIR / "bin" / "python"

# Check if we're already running from the project's venv
_current_venv = Path(sys.prefix).resolve()
_target_venv = _VENV_DIR.resolve()

# Re-exec with venv Python if we're not already using it
if VENV_PYTHON.exists() and _current_venv != _target_venv:
    os.execv(str(VENV_PYTHON), [str(VENV_PYTHON)] + sys.argv)
