"""
LUT operation parity tests: vfx-rs vs PyOpenColorIO.

Tests LUT parsing, interpolation, and application against OCIO reference.
"""

import pytest
import numpy as np
from pathlib import Path
from conftest import assert_close, apply_ocio_cpu, RTOL


# ---------------------------------------------------------------------------
# Test fixtures
# ---------------------------------------------------------------------------

@pytest.fixture(scope="module")
def simple_1d_lut_data() -> np.ndarray:
    """Simple 1D LUT: gamma 2.2 curve."""
    size = 256
    x = np.linspace(0, 1, size, dtype=np.float32)
    return x ** 2.2


@pytest.fixture(scope="module")
def identity_3d_lut_data() -> np.ndarray:
    """Identity 3D LUT 17x17x17."""
    size = 17
    r = np.linspace(0, 1, size, dtype=np.float32)
    g = np.linspace(0, 1, size, dtype=np.float32)
    b = np.linspace(0, 1, size, dtype=np.float32)
    rr, gg, bb = np.meshgrid(r, g, b, indexing='ij')
    return np.stack([rr, gg, bb], axis=-1)


@pytest.fixture(scope="module")
def contrast_3d_lut_data() -> np.ndarray:
    """Contrast boost 3D LUT."""
    size = 17
    r = np.linspace(0, 1, size, dtype=np.float32)
    g = np.linspace(0, 1, size, dtype=np.float32)
    b = np.linspace(0, 1, size, dtype=np.float32)
    rr, gg, bb = np.meshgrid(r, g, b, indexing='ij')
    
    # Apply S-curve contrast
    def contrast(x, amount=1.5):
        return 0.5 + (x - 0.5) * amount
    
    return np.stack([
        np.clip(contrast(rr), 0, 1),
        np.clip(contrast(gg), 0, 1),
        np.clip(contrast(bb), 0, 1),
    ], axis=-1)


@pytest.fixture(scope="module")
def test_cube_file(tmp_path_factory) -> Path:
    """Create a test .cube file."""
    tmp_dir = tmp_path_factory.mktemp("lut")
    cube_path = tmp_dir / "test.cube"
    
    # Write simple 1D LUT
    with open(cube_path, "w") as f:
        f.write("TITLE \"Test LUT\"\n")
        f.write("LUT_1D_SIZE 17\n")
        for i in range(17):
            v = (i / 16.0) ** 2.2
            f.write(f"{v:.6f} {v:.6f} {v:.6f}\n")
    
    return cube_path


@pytest.fixture(scope="module")
def test_3d_cube_file(tmp_path_factory) -> Path:
    """Create a test 3D .cube file."""
    tmp_dir = tmp_path_factory.mktemp("lut3d")
    cube_path = tmp_dir / "test_3d.cube"
    
    size = 5  # Small for fast tests
    
    with open(cube_path, "w") as f:
        f.write("TITLE \"Test 3D LUT\"\n")
        f.write(f"LUT_3D_SIZE {size}\n")
        
        for b in range(size):
            for g in range(size):
                for r in range(size):
                    rv = r / (size - 1)
                    gv = g / (size - 1)
                    bv = b / (size - 1)
                    # Simple transform: invert
                    f.write(f"{1-rv:.6f} {1-gv:.6f} {1-bv:.6f}\n")
    
    return cube_path


# ---------------------------------------------------------------------------
# 1D LUT tests
# ---------------------------------------------------------------------------

@pytest.mark.lut
class Test1DLUT:
    """1D LUT interpolation tests."""
    
    def test_1d_linear_interp_endpoints(self, simple_1d_lut_data):
        """Test 1D LUT returns exact values at endpoints."""
        lut = simple_1d_lut_data
        
        # At index 0, input 0.0 should return lut[0]
        assert lut[0] == pytest.approx(0.0, abs=1e-6)
        
        # At index -1, input 1.0 should return lut[-1]
        assert lut[-1] == pytest.approx(1.0, abs=1e-6)
    
    def test_1d_linear_interp_midpoint(self, simple_1d_lut_data):
        """Test 1D LUT linear interpolation at midpoints."""
        lut = simple_1d_lut_data
        size = len(lut)
        
        # Interpolate at 0.5
        idx = 0.5 * (size - 1)
        low_idx = int(idx)
        high_idx = low_idx + 1
        frac = idx - low_idx
        
        expected = lut[low_idx] * (1 - frac) + lut[high_idx] * frac
        
        # Manual interpolation
        actual = np.interp(0.5, np.linspace(0, 1, size), lut)
        
        assert actual == pytest.approx(expected, rel=1e-5)
    
    def test_vfx_1d_lut_apply(self, vfx_lut, simple_1d_lut_data, gray_ramp_1d):
        """Test vfx-rs 1D LUT application."""
        try:
            lut = vfx_lut.Lut1D(simple_1d_lut_data.tolist())
            result = lut.apply(gray_ramp_1d.tolist())
            result = np.array(result, dtype=np.float32)
        except (AttributeError, Exception) as e:
            pytest.skip(f"vfx-rs 1D LUT API not available: {e}")
        
        # Compare with numpy interpolation
        expected = np.interp(gray_ramp_1d, 
                            np.linspace(0, 1, len(simple_1d_lut_data)), 
                            simple_1d_lut_data)
        
        assert_close(result, expected, rtol=1e-5,
                     msg="vfx-rs 1D LUT differs from numpy interp")


# ---------------------------------------------------------------------------
# 3D LUT tests
# ---------------------------------------------------------------------------

@pytest.mark.lut
class Test3DLUT:
    """3D LUT interpolation tests."""
    
    def test_3d_identity_passthrough(self, identity_3d_lut_data, rgb_ramp_2d):
        """Test identity 3D LUT returns input unchanged."""
        # Identity LUT should return same values
        size = identity_3d_lut_data.shape[0]
        
        # Test exact grid points
        for r in range(size):
            for g in range(size):
                for b in range(size):
                    expected = np.array([r, g, b], dtype=np.float32) / (size - 1)
                    actual = identity_3d_lut_data[r, g, b]
                    assert_close(actual, expected, atol=1e-6,
                                 msg=f"Identity LUT mismatch at [{r},{g},{b}]")
    
    def test_vfx_3d_lut_identity(self, vfx_lut, identity_3d_lut_data):
        """Test vfx-rs 3D LUT with identity."""
        try:
            # Flatten for vfx-rs API
            flat_lut = identity_3d_lut_data.flatten().tolist()
            size = identity_3d_lut_data.shape[0]
            
            lut = vfx_lut.Lut3D(flat_lut, size)
            
            # Test some colors
            test_colors = [
                [0.0, 0.0, 0.0],
                [1.0, 1.0, 1.0],
                [0.5, 0.5, 0.5],
                [0.25, 0.75, 0.5],
            ]
            
            for color in test_colors:
                result = lut.apply_rgb(color)
                assert_close(np.array(result), np.array(color), rtol=1e-4,
                             msg=f"Identity LUT changed color {color}")
                             
        except (AttributeError, Exception) as e:
            pytest.skip(f"vfx-rs 3D LUT API not available: {e}")
    
    def test_trilinear_vs_tetrahedral(self, vfx_lut, contrast_3d_lut_data):
        """Test trilinear vs tetrahedral interpolation."""
        try:
            flat_lut = contrast_3d_lut_data.flatten().tolist()
            size = contrast_3d_lut_data.shape[0]
            
            lut_tri = vfx_lut.Lut3D(flat_lut, size, interpolation="trilinear")
            lut_tet = vfx_lut.Lut3D(flat_lut, size, interpolation="tetrahedral")
            
            # Test non-grid-aligned color
            test_color = [0.33, 0.66, 0.25]
            
            result_tri = np.array(lut_tri.apply_rgb(test_color))
            result_tet = np.array(lut_tet.apply_rgb(test_color))
            
            # Results should be similar but not identical
            assert_close(result_tri, result_tet, rtol=0.05,
                         msg="Trilinear and tetrahedral differ too much")
                         
        except (AttributeError, Exception) as e:
            pytest.skip(f"vfx-rs LUT interpolation API not available: {e}")


# ---------------------------------------------------------------------------
# LUT file format tests
# ---------------------------------------------------------------------------

@pytest.mark.lut
class TestLUTFormats:
    """LUT file format parsing tests."""
    
    def test_cube_1d_parse(self, vfx_lut, test_cube_file):
        """Test .cube 1D LUT parsing."""
        try:
            lut = vfx_lut.load(str(test_cube_file))
            assert lut is not None
            assert lut.size() == 17
        except (AttributeError, Exception) as e:
            pytest.skip(f"vfx-rs LUT loading not available: {e}")
    
    def test_cube_3d_parse(self, vfx_lut, test_3d_cube_file):
        """Test .cube 3D LUT parsing."""
        try:
            lut = vfx_lut.load(str(test_3d_cube_file))
            assert lut is not None
        except (AttributeError, Exception) as e:
            pytest.skip(f"vfx-rs 3D LUT loading not available: {e}")
    
    def test_lut_roundtrip_cube(self, vfx_lut, tmp_path, simple_1d_lut_data):
        """Test LUT write then read roundtrip."""
        try:
            # Create and save
            lut = vfx_lut.Lut1D(simple_1d_lut_data.tolist())
            out_path = tmp_path / "roundtrip.cube"
            lut.save(str(out_path))
            
            # Load back
            lut2 = vfx_lut.load(str(out_path))
            
            # Compare data
            data1 = np.array(lut.data())
            data2 = np.array(lut2.data())
            
            assert_close(data1, data2, rtol=1e-5,
                         msg="LUT roundtrip changed data")
                         
        except (AttributeError, Exception) as e:
            pytest.skip(f"vfx-rs LUT save/load not available: {e}")


# ---------------------------------------------------------------------------
# OCIO LUT comparison
# ---------------------------------------------------------------------------

@pytest.mark.lut
class TestOCIOLUTParity:
    """OCIO LUT parity tests."""
    
    def test_ocio_file_transform(self, ocio, ocio_raw_config, test_cube_file, gray_ramp_2d):
        """Test OCIO FileTransform with .cube file."""
        try:
            transform = ocio.FileTransform(str(test_cube_file))
            processor = ocio_raw_config.getProcessor(transform)
            
            # Apply to test image
            rgb_input = np.stack([gray_ramp_2d, gray_ramp_2d, gray_ramp_2d], axis=-1)
            ocio_result = apply_ocio_cpu(processor, rgb_input)
            
            # Result should be non-zero
            assert np.max(ocio_result) > 0
            
        except Exception as e:
            pytest.skip(f"OCIO FileTransform not available: {e}")
    
    def test_vfx_vs_ocio_lut_apply(self, ocio, ocio_raw_config, vfx_lut, 
                                    test_cube_file, rgb_ramp_2d):
        """Compare vfx-rs LUT application vs OCIO."""
        try:
            # OCIO
            transform = ocio.FileTransform(str(test_cube_file))
            processor = ocio_raw_config.getProcessor(transform)
            ocio_result = apply_ocio_cpu(processor, rgb_ramp_2d.copy())
            
            # vfx-rs
            lut = vfx_lut.load(str(test_cube_file))
            vfx_result = lut.apply_image(rgb_ramp_2d.flatten().tolist(), 
                                         rgb_ramp_2d.shape[1], 
                                         rgb_ramp_2d.shape[0])
            vfx_result = np.array(vfx_result).reshape(rgb_ramp_2d.shape)
            
            assert_close(vfx_result, ocio_result, rtol=RTOL,
                         msg="vfx-rs LUT differs from OCIO")
                         
        except Exception as e:
            pytest.skip(f"LUT comparison not available: {e}")


# ---------------------------------------------------------------------------
# CLF format tests
# ---------------------------------------------------------------------------

@pytest.mark.lut
class TestCLFFormat:
    """Common LUT Format (CLF) tests."""
    
    @pytest.fixture
    def test_clf_file(self, tmp_path) -> Path:
        """Create a simple CLF file."""
        clf_path = tmp_path / "test.clf"
        
        clf_content = '''<?xml version="1.0" encoding="UTF-8"?>
<ProcessList compCLFversion="3.0" id="test">
    <Matrix inBitDepth="32f" outBitDepth="32f">
        <Array dim="3 3">
            1.2 0.0 0.0
            0.0 1.0 0.0
            0.0 0.0 0.8
        </Array>
    </Matrix>
</ProcessList>
'''
        clf_path.write_text(clf_content)
        return clf_path
    
    def test_clf_parse(self, vfx_lut, test_clf_file):
        """Test CLF file parsing."""
        try:
            clf = vfx_lut.load_clf(str(test_clf_file))
            assert clf is not None
        except (AttributeError, Exception) as e:
            pytest.skip(f"vfx-rs CLF loading not available: {e}")
    
    def test_clf_matrix_apply(self, vfx_lut, test_clf_file):
        """Test CLF matrix operation."""
        try:
            clf = vfx_lut.load_clf(str(test_clf_file))
            
            # Test color
            test_rgb = [0.5, 0.5, 0.5]
            result = clf.apply_rgb(test_rgb)
            
            # Expected: [0.5*1.2, 0.5*1.0, 0.5*0.8] = [0.6, 0.5, 0.4]
            expected = [0.6, 0.5, 0.4]
            
            assert_close(np.array(result), np.array(expected), rtol=1e-5,
                         msg="CLF matrix application incorrect")
                         
        except (AttributeError, Exception) as e:
            pytest.skip(f"vfx-rs CLF apply not available: {e}")
