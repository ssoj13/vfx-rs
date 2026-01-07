#!/usr/bin/env python3
"""
Tests for demosaic, make_texture, fillholes, and render_text.

These tests verify the Python bindings for the newly implemented functions.
"""

import sys
from pathlib import Path

import numpy as np

# Add parent dir for local development
sys.path.insert(0, str(Path(__file__).parent.parent / "target" / "release"))

import vfx_rs
from vfx_rs import ops

# =============================================================================
# Test Utilities
# =============================================================================

def create_test_image(width: int, height: int, channels: int = 4) -> vfx_rs.Image:
    """Create a test image with gradient pattern."""
    data = np.zeros((height, width, channels), dtype=np.float32)
    for y in range(height):
        for x in range(width):
            u = x / max(width - 1, 1)
            v = y / max(height - 1, 1)
            for c in range(min(channels, 3)):
                data[y, x, c] = (u + v) / 2
            if channels == 4:
                data[y, x, 3] = 1.0  # Alpha
    return vfx_rs.Image(data)


def create_bayer_image(width: int, height: int) -> vfx_rs.Image:
    """Create a simulated Bayer pattern image (single channel)."""
    data = np.zeros((height, width, 1), dtype=np.float32)
    for y in range(height):
        for x in range(width):
            # RGGB pattern
            if y % 2 == 0:
                if x % 2 == 0:
                    data[y, x, 0] = 1.0  # R
                else:
                    data[y, x, 0] = 0.5  # G
            else:
                if x % 2 == 0:
                    data[y, x, 0] = 0.5  # G
                else:
                    data[y, x, 0] = 0.25  # B
    return vfx_rs.Image(data)


def create_image_with_holes(width: int, height: int) -> vfx_rs.Image:
    """Create an RGBA image with holes (zero alpha)."""
    data = np.zeros((height, width, 4), dtype=np.float32)
    for y in range(height):
        for x in range(width):
            is_hole = ((x // 4) + (y // 4)) % 2 == 0
            if is_hole:
                data[y, x] = [0.0, 0.0, 0.0, 0.0]  # Hole
            else:
                data[y, x] = [1.0, 0.5, 0.25, 1.0]  # Valid
    return vfx_rs.Image(data)


def run_test(name: str, test_fn):
    """Run a test and print result."""
    try:
        test_fn()
        print(f"  [PASS] {name}")
        return True
    except Exception as e:
        print(f"  [FAIL] {name}: {e}")
        return False


# =============================================================================
# Demosaic Tests
# =============================================================================

def test_demosaic_functional_api():
    """Test demosaic via functional API (ops module)."""
    bayer = create_bayer_image(16, 16)
    
    rgb = ops.demosaic(bayer, ops.BayerPattern.RGGB, ops.DemosaicAlgorithm.Bilinear)
    
    assert rgb.width == 16
    assert rgb.height == 16
    assert rgb.channels == 3


def test_demosaic_object_api():
    """Test demosaic via object API (Image method)."""
    bayer = create_bayer_image(16, 16)
    
    rgb = bayer.demosaic(ops.BayerPattern.RGGB, ops.DemosaicAlgorithm.Bilinear)
    
    assert rgb.width == 16
    assert rgb.height == 16
    assert rgb.channels == 3


def test_demosaic_vng():
    """Test VNG demosaic algorithm."""
    bayer = create_bayer_image(32, 32)
    
    rgb = ops.demosaic(bayer, ops.BayerPattern.RGGB, ops.DemosaicAlgorithm.VNG)
    
    assert rgb.channels == 3


def test_demosaic_all_patterns():
    """Test all Bayer patterns."""
    bayer = create_bayer_image(16, 16)
    
    for pattern in [ops.BayerPattern.RGGB, ops.BayerPattern.BGGR, 
                    ops.BayerPattern.GRBG, ops.BayerPattern.GBRG]:
        rgb = ops.demosaic(bayer, pattern)
        assert rgb.channels == 3, f"Pattern {pattern} failed"


# =============================================================================
# Mipmap/Texture Tests
# =============================================================================

def test_make_texture_functional():
    """Test make_texture via functional API."""
    img = create_test_image(64, 64, 3)
    
    mips = ops.make_texture(img)
    
    assert len(mips) == 7  # 64, 32, 16, 8, 4, 2, 1
    assert mips[0].width == 64
    assert mips[1].width == 32
    assert mips[-1].width == 1


def test_make_texture_object():
    """Test make_texture via object API."""
    img = create_test_image(64, 64, 3)
    
    mips = img.make_texture()
    
    assert len(mips) >= 1
    assert mips[0].width == 64


def test_make_mip_level():
    """Test generating specific mip level."""
    img = create_test_image(128, 128, 3)
    
    mip0 = img.make_mip_level(0)
    mip1 = img.make_mip_level(1)
    mip3 = img.make_mip_level(3)
    
    assert mip0.width == 128
    assert mip1.width == 64
    assert mip3.width == 16


def test_mipmap_options():
    """Test mipmap with options."""
    img = create_test_image(32, 32, 4)
    
    opts = ops.MipmapOptions()
    opts.filter = ops.MipmapFilter.Lanczos
    
    mips = ops.make_texture(img, opts)
    
    assert len(mips) > 1


def test_mip_utilities():
    """Test mip utility functions."""
    assert ops.mip_level_count(256, 256) == 9
    assert ops.mip_dimensions(256, 256, 0) == (256, 256)
    assert ops.mip_dimensions(256, 256, 1) == (128, 128)


# =============================================================================
# Fillholes Tests
# =============================================================================

def test_has_holes():
    """Test hole detection."""
    img_with_holes = create_image_with_holes(32, 32)
    img_solid = create_test_image(32, 32, 4)
    
    assert ops.has_holes(img_with_holes) == True
    assert ops.has_holes(img_solid) == False


def test_has_holes_object():
    """Test hole detection via object API."""
    img = create_image_with_holes(32, 32)
    
    assert img.has_holes() == True


def test_count_holes():
    """Test hole counting."""
    img = create_image_with_holes(16, 16)
    
    count = ops.count_holes(img)
    
    assert count > 0
    assert count < 16 * 16


def test_fillholes_basic():
    """Test basic hole filling."""
    img = create_image_with_holes(32, 32)
    
    assert ops.has_holes(img)
    
    filled = ops.fillholes_pushpull(img)
    
    assert filled.width == 32
    assert filled.height == 32
    assert not ops.has_holes(filled)


def test_fillholes_object():
    """Test hole filling via object API."""
    img = create_image_with_holes(32, 32)
    
    filled = img.fillholes()
    
    assert not filled.has_holes()


def test_fillholes_options():
    """Test hole filling with options."""
    img = create_image_with_holes(16, 16)
    
    opts = ops.FillHolesOptions()
    opts.dilate = True
    opts.alpha_threshold = 0.01
    
    filled = ops.fillholes_pushpull(img, opts)
    
    assert not ops.has_holes(filled)


# =============================================================================
# Text Rendering Tests (if feature enabled)
# =============================================================================

def test_text_rendering():
    """Test text rendering (requires 'text' feature)."""
    try:
        from vfx_rs.drawing import render_text, render_text_into, TextStyle, TextAlign
    except ImportError:
        print("  [SKIP] Text rendering not available (feature 'text' not enabled)")
        return
    
    style = TextStyle(font_size=32.0, color=[1.0, 0.0, 0.0, 1.0])
    img = render_text("Hello World", style, 256, 64)
    
    assert img.width == 256
    assert img.height == 64
    assert img.channels == 4


def test_text_into_image():
    """Test rendering text into existing image."""
    try:
        from vfx_rs.drawing import render_text_into, TextStyle
    except ImportError:
        print("  [SKIP] Text rendering not available")
        return
    
    bg = create_test_image(256, 128, 4)
    style = TextStyle(font_size=24.0)
    
    result = render_text_into(bg, "Test", 10, 10, style)
    
    assert result.width == 256
    assert result.height == 128


def test_text_alignments():
    """Test text alignments."""
    try:
        from vfx_rs.drawing import render_text, TextStyle, TextAlign
    except ImportError:
        print("  [SKIP] Text rendering not available")
        return
    
    for align in [TextAlign.Left, TextAlign.Center, TextAlign.Right]:
        style = TextStyle(font_size=24.0, align=align)
        img = render_text("Aligned", style, 200, 50)
        assert img.width == 200


# =============================================================================
# Main
# =============================================================================

def main():
    print("=" * 60)
    print("vfx_rs New Functions Test Suite")
    print("=" * 60)
    
    all_tests = [
        # Demosaic
        ("Demosaic - Functional API", test_demosaic_functional_api),
        ("Demosaic - Object API", test_demosaic_object_api),
        ("Demosaic - VNG Algorithm", test_demosaic_vng),
        ("Demosaic - All Patterns", test_demosaic_all_patterns),
        
        # Mipmaps
        ("Mipmap - Functional API", test_make_texture_functional),
        ("Mipmap - Object API", test_make_texture_object),
        ("Mipmap - Specific Level", test_make_mip_level),
        ("Mipmap - With Options", test_mipmap_options),
        ("Mipmap - Utility Functions", test_mip_utilities),
        
        # Fillholes
        ("Fillholes - Has Holes Detection", test_has_holes),
        ("Fillholes - Has Holes Object", test_has_holes_object),
        ("Fillholes - Count Holes", test_count_holes),
        ("Fillholes - Basic", test_fillholes_basic),
        ("Fillholes - Object API", test_fillholes_object),
        ("Fillholes - With Options", test_fillholes_options),
        
        # Text (optional)
        ("Text - Basic Rendering", test_text_rendering),
        ("Text - Into Existing Image", test_text_into_image),
        ("Text - Alignments", test_text_alignments),
    ]
    
    passed = 0
    failed = 0
    
    print("\nRunning tests...\n")
    
    for name, test_fn in all_tests:
        if run_test(name, test_fn):
            passed += 1
        else:
            failed += 1
    
    print("\n" + "=" * 60)
    print(f"Results: {passed} passed, {failed} failed")
    print("=" * 60)
    
    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
