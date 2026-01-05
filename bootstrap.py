#!/usr/bin/env python3
"""
bootstrap.py - Unified build/test/python script for vfx-rs.

Cross-platform Python script for common development tasks.
Single-file solution with no external dependencies (stdlib only).

Commands:
    build         Build Rust workspace
    test          Run Rust tests
    bench         Run benchmarks
    python        Build Python wheel via maturin
    python-reqs   Install Python dev dependencies
    clean         Clean build artifacts
    check         Run clippy and fmt check

Usage:
    python bootstrap.py build
    python bootstrap.py test
    python bootstrap.py python --release --install
"""

from __future__ import annotations

import argparse
import os
import platform
import shutil
import subprocess
import sys
import time
from pathlib import Path

# ============================================================
# CONSTANTS & CONFIG
# ============================================================

ROOT_DIR = Path(__file__).parent.resolve()

# Crate groups for selective testing
CRATE_GROUPS = {
    "core": ["vfx-core", "vfx-math", "vfx-primaries"],
    "color": ["vfx-color", "vfx-transfer", "vfx-lut"],
    "io": ["vfx-io"],
    "compute": ["vfx-compute", "vfx-ops"],
    "all": [],  # Empty means all
}

# ANSI colors
class Colors:
    """ANSI color codes."""
    
    RESET = "\033[0m"
    RED = "\033[91m"
    GREEN = "\033[92m"
    YELLOW = "\033[93m"
    CYAN = "\033[96m"
    WHITE = "\033[97m"
    DARK_GRAY = "\033[90m"
    
    @classmethod
    def init(cls) -> None:
        """Enable ANSI on Windows."""
        if platform.system() == "Windows":
            os.system("")


# ============================================================
# UTILITY FUNCTIONS
# ============================================================

def fmt_time(ms: float) -> str:
    """Format milliseconds nicely."""
    if ms < 1000:
        return f"{ms:.0f}ms"
    elif ms < 60000:
        return f"{ms/1000:.1f}s"
    else:
        mins = int(ms // 60000)
        secs = (ms % 60000) / 1000
        return f"{mins}m{secs:.0f}s"


def print_header(text: str) -> None:
    """Print section header."""
    line = "=" * 60
    print()
    print(f"{Colors.CYAN}{line}")
    print(text)
    print(f"{line}{Colors.RESET}")


def print_step(text: str) -> None:
    """Print step indicator."""
    print(f"  {Colors.WHITE}{text}{Colors.RESET}")


def print_success(text: str) -> None:
    """Print success message."""
    print(f"  {Colors.GREEN}{text}{Colors.RESET}")


def print_error(text: str) -> None:
    """Print error message."""
    print(f"  {Colors.RED}{text}{Colors.RESET}")


def print_warning(text: str) -> None:
    """Print warning message."""
    print(f"  {Colors.YELLOW}{text}{Colors.RESET}")


def run_cmd(args: list[str], cwd: Path | None = None, 
            capture: bool = False) -> tuple[int, str, float]:
    """Run command and return (exit_code, output, time_ms)."""
    start = time.perf_counter()
    result = subprocess.run(
        args,
        cwd=cwd or ROOT_DIR,
        capture_output=capture,
        text=True,
    )
    elapsed_ms = (time.perf_counter() - start) * 1000
    output = (result.stdout or "") + (result.stderr or "") if capture else ""
    return result.returncode, output, elapsed_ms


def which(cmd: str) -> Path | None:
    """Find executable in PATH."""
    result = shutil.which(cmd)
    return Path(result) if result else None


# ============================================================
# BUILD COMMAND
# ============================================================

def run_build(args: argparse.Namespace) -> int:
    """Build Rust workspace."""
    print_header("BUILD")
    
    cmd = ["cargo", "build"]
    
    if args.release:
        cmd.append("--release")
        print_step("Mode: release")
    else:
        print_step("Mode: debug")
    
    if args.features:
        cmd.extend(["--features", args.features])
        print_step(f"Features: {args.features}")
    
    print()
    print_step("Building workspace...")
    
    exit_code, _, elapsed = run_cmd(cmd)
    
    print()
    if exit_code == 0:
        print_success(f"Build successful ({fmt_time(elapsed)})")
    else:
        print_error("Build failed")
    print()
    
    return exit_code


# ============================================================
# TEST COMMAND
# ============================================================

def run_test(args: argparse.Namespace) -> int:
    """Run Rust tests."""
    print_header("TEST")
    
    cmd = ["cargo", "test"]
    
    # Select crates
    group = args.group
    if group and group != "all" and group in CRATE_GROUPS:
        crates = CRATE_GROUPS[group]
        print_step(f"Group: {group} ({len(crates)} crates)")
        for crate in crates:
            cmd.extend(["-p", crate])
    else:
        print_step("Group: all")
    
    if args.release:
        cmd.append("--release")
    
    # Pass test arguments
    if args.nocapture:
        cmd.extend(["--", "--nocapture"])
    
    print()
    print_step("Running tests...")
    print()
    
    exit_code, _, elapsed = run_cmd(cmd)
    
    print()
    if exit_code == 0:
        print_success(f"All tests passed ({fmt_time(elapsed)})")
    else:
        print_error("Some tests failed")
    print()
    
    return exit_code


# ============================================================
# BENCH COMMAND
# ============================================================

def run_bench(args: argparse.Namespace) -> int:
    """Run benchmarks."""
    print_header("BENCHMARK")
    
    bench_crate = ROOT_DIR / "crates" / "vfx-bench"
    if not bench_crate.exists():
        print_error("vfx-bench crate not found")
        return 1
    
    cmd = ["cargo", "run", "--release", "-p", "vfx-bench"]
    
    if args.filter:
        cmd.extend(["--", args.filter])
        print_step(f"Filter: {args.filter}")
    
    print()
    print_step("Running benchmarks...")
    print()
    
    exit_code, _, elapsed = run_cmd(cmd)
    
    print()
    if exit_code == 0:
        print_success(f"Benchmarks complete ({fmt_time(elapsed)})")
    else:
        print_error("Benchmarks failed")
    print()
    
    return exit_code


# ============================================================
# CHECK COMMAND
# ============================================================

def run_check(args: argparse.Namespace) -> int:
    """Run clippy and fmt check."""
    print_header("CHECK")
    
    all_ok = True
    
    # Clippy
    print_step("Running clippy...")
    exit_code, _, elapsed = run_cmd(["cargo", "clippy", "--workspace", "--", "-D", "warnings"])
    if exit_code == 0:
        print_success(f"Clippy OK ({fmt_time(elapsed)})")
    else:
        print_error("Clippy found issues")
        all_ok = False
    
    print()
    
    # Format check
    print_step("Checking formatting...")
    exit_code, _, elapsed = run_cmd(["cargo", "fmt", "--check"])
    if exit_code == 0:
        print_success(f"Format OK ({fmt_time(elapsed)})")
    else:
        print_warning("Format issues found. Run: cargo fmt")
        all_ok = False
    
    print()
    
    if all_ok:
        print_success("All checks passed!")
    else:
        print_error("Some checks failed")
    print()
    
    return 0 if all_ok else 1


# ============================================================
# BOOK COMMAND
# ============================================================

def run_book(args: argparse.Namespace) -> int:
    """Build mdbook documentation."""
    print_header("DOCUMENTATION")
    
    docs_dir = ROOT_DIR / "docs"
    if not docs_dir.exists():
        print_error("docs directory not found")
        return 1
    
    # Check mdbook
    if not which("mdbook"):
        print_error("mdbook not found")
        print_warning("Install: cargo install mdbook")
        return 1
    
    print_step("Building documentation...")
    print()
    
    exit_code, _, elapsed = run_cmd(["mdbook", "build"], cwd=docs_dir)
    
    print()
    if exit_code == 0:
        print_success(f"Documentation built ({fmt_time(elapsed)})")
        print_step(f"Open: {docs_dir / 'book' / 'index.html'}")
    else:
        print_error("Build failed")
    print()
    
    return exit_code


# ============================================================
# CLEAN COMMAND
# ============================================================

def run_clean(args: argparse.Namespace) -> int:
    """Clean build artifacts."""
    print_header("CLEAN")
    
    print_step("Running cargo clean...")
    exit_code, _, _ = run_cmd(["cargo", "clean"])
    
    # Also clean Python build artifacts
    py_crate = ROOT_DIR / "crates" / "vfx-rs-py"
    for pattern in ["*.so", "*.pyd", "*.egg-info"]:
        for f in py_crate.glob(pattern):
            print_step(f"Removing {f.name}")
            if f.is_dir():
                shutil.rmtree(f)
            else:
                f.unlink()
    
    print()
    print_success("Clean complete")
    print()
    
    return exit_code


# ============================================================
# PYTHON BUILD
# ============================================================

def run_python_reqs(args: argparse.Namespace) -> int:
    """Install Python dev dependencies."""
    print_header("PYTHON DEPENDENCIES")
    print()
    
    packages = ["maturin", "numpy", "pytest"]
    
    # Try uv first (faster)
    if which("uv"):
        print_step("Installing with uv...")
        result = subprocess.run(["uv", "pip", "install"] + packages)
        if result.returncode == 0:
            print()
            print_success("Done!")
            print()
            return 0
        print_warning("uv failed, trying pip...")
    
    # Fallback to pip
    print_step("Installing with pip...")
    result = subprocess.run([sys.executable, "-m", "pip", "install"] + packages)
    
    print()
    if result.returncode == 0:
        print_success("Done!")
    else:
        print_error("Failed to install dependencies")
        return 1
    print()
    return 0


def run_python_build(args: argparse.Namespace) -> int:
    """Build Python wheel via maturin."""
    print_header("PYTHON BUILD")
    print()
    
    # Check maturin
    if not which("maturin"):
        print_error("maturin not found")
        print_warning("Run: python bootstrap.py python-reqs")
        return 1
    
    py_crate = ROOT_DIR / "crates" / "vfx-rs-py"
    if not py_crate.exists():
        print_error("vfx-rs-py crate not found")
        return 1
    
    build_type = "release" if args.release else "debug"
    print_step(f"Mode: {build_type}")
    print_step(f"Install: {args.install}")
    print()
    
    start = time.perf_counter()
    
    if args.install:
        cmd = ["maturin", "develop"]
        if args.release:
            cmd.append("--release")
        msg = f"Building and installing ({build_type})..."
    else:
        cmd = ["maturin", "build"]
        if args.release:
            cmd.append("--release")
        msg = f"Building wheel ({build_type})..."
    
    print_step(msg)
    result = subprocess.run(cmd, cwd=py_crate)
    
    elapsed_ms = (time.perf_counter() - start) * 1000
    
    print()
    if result.returncode == 0:
        print_success(f"Done! ({fmt_time(elapsed_ms)})")
        
        if not args.install:
            wheel_dir = ROOT_DIR / "target" / "wheels"
            if wheel_dir.exists():
                wheels = sorted(wheel_dir.glob("*.whl"), 
                              key=lambda p: p.stat().st_mtime, reverse=True)
                if wheels:
                    print_step(f"Wheel: {wheels[0]}")
    else:
        print_error("Build failed")
        return 1
    print()
    return 0


def run_python_test(args: argparse.Namespace) -> int:
    """Run Python tests."""
    print_header("PYTHON TESTS")
    print()
    
    py_crate = ROOT_DIR / "crates" / "vfx-rs-py"
    test_dir = py_crate / "tests"
    
    if not test_dir.exists():
        print_warning("No Python tests found")
        return 0
    
    # Check pytest
    if not which("pytest"):
        print_error("pytest not found")
        print_warning("Run: python bootstrap.py python-reqs")
        return 1
    
    print_step("Running pytest...")
    print()
    
    result = subprocess.run(["pytest", "-v", str(test_dir)])
    
    print()
    if result.returncode == 0:
        print_success("All Python tests passed!")
    else:
        print_error("Some Python tests failed")
    print()
    
    return result.returncode


# ============================================================
# HELP
# ============================================================

HELP_TEXT = """
 VFX-RS BUILD SYSTEM
 
 COMMANDS
   build         Build Rust workspace
   test          Run Rust tests
   bench         Run benchmarks
   check         Run clippy and fmt check
   clean         Clean build artifacts
   python        Build Python wheel via maturin
   python-reqs   Install Python dev dependencies
   python-test   Run Python tests
   book          Build mdbook documentation
 
 BUILD OPTIONS
   --release     Build in release mode
   --features    Enable features (e.g., cuda,wgpu)
 
 TEST OPTIONS
   --group       Test group: core|color|io|compute|all
   --nocapture   Show test output
   --release     Test release build
 
 PYTHON OPTIONS
   --release     Build release wheel
   --install     Install in current virtualenv
 
 EXAMPLES
   python bootstrap.py build                    # Debug build
   python bootstrap.py build --release          # Release build
   python bootstrap.py test                     # Run all tests
   python bootstrap.py test --group core        # Test core crates only
   python bootstrap.py check                    # Clippy + fmt
   python bootstrap.py python --release --install  # Build & install Python
   python bootstrap.py python-test              # Run Python tests
"""


# ============================================================
# MAIN
# ============================================================

def main() -> int:
    Colors.init()
    
    parser = argparse.ArgumentParser(
        description="VFX-RS build system",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    
    parser.add_argument(
        "command",
        nargs="?",
        choices=["build", "test", "bench", "check", "clean", 
                 "python", "python-reqs", "python-test", "book", "help"],
        default="help",
        help="Command to run",
    )
    
    # Build options
    parser.add_argument(
        "--release",
        action="store_true",
        help="Build/test in release mode",
    )
    
    parser.add_argument(
        "--features",
        help="Cargo features to enable",
    )
    
    # Test options
    parser.add_argument(
        "--group", "-g",
        choices=list(CRATE_GROUPS.keys()),
        help="Test group",
    )
    
    parser.add_argument(
        "--nocapture",
        action="store_true",
        help="Show test output",
    )
    
    # Bench options
    parser.add_argument(
        "--filter", "-f",
        help="Benchmark filter",
    )
    
    # Python options
    parser.add_argument(
        "--install",
        action="store_true",
        help="Install in current virtualenv",
    )
    
    args = parser.parse_args()
    
    # Dispatch
    if args.command == "help" or args.command is None:
        print(HELP_TEXT)
        return 0
    elif args.command == "build":
        return run_build(args)
    elif args.command == "test":
        return run_test(args)
    elif args.command == "bench":
        return run_bench(args)
    elif args.command == "check":
        return run_check(args)
    elif args.command == "clean":
        return run_clean(args)
    elif args.command == "python":
        return run_python_build(args)
    elif args.command == "python-reqs":
        return run_python_reqs(args)
    elif args.command == "python-test":
        return run_python_test(args)
    elif args.command == "book":
        return run_book(args)
    
    return 0


if __name__ == "__main__":
    sys.exit(main())
