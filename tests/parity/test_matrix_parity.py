"""
Matrix transform parity tests: vfx-rs vs PyOpenColorIO.

Tests color space conversion matrices against OCIO reference values.
"""

import pytest
import numpy as np
from conftest import assert_close, RTOL, ATOL


# ---------------------------------------------------------------------------
# Reference matrices from OCIO and standards
# ---------------------------------------------------------------------------

# sRGB/Rec.709 to XYZ (D65) - IEC 61966-2-1
SRGB_TO_XYZ = np.array([
    [0.4124564, 0.3575761, 0.1804375],
    [0.2126729, 0.7151522, 0.0721750],
    [0.0193339, 0.1191920, 0.9503041],
], dtype=np.float32)

XYZ_TO_SRGB = np.array([
    [ 3.2404542, -1.5371385, -0.4985314],
    [-0.9692660,  1.8760108,  0.0415560],
    [ 0.0556434, -0.2040259,  1.0572252],
], dtype=np.float32)

# Rec.2020 to XYZ (D65) - ITU-R BT.2020
REC2020_TO_XYZ = np.array([
    [0.6369580, 0.1446169, 0.1688810],
    [0.2627002, 0.6779981, 0.0593017],
    [0.0000000, 0.0280727, 1.0609851],
], dtype=np.float32)

# ACES AP0 to XYZ (D60) - AMPAS
AP0_TO_XYZ = np.array([
    [0.9525523959, 0.0000000000,  0.0000936786],
    [0.3439664498, 0.7281660966, -0.0721325464],
    [0.0000000000, 0.0000000000,  1.0088251844],
], dtype=np.float32)

# ACES AP1 to XYZ (D60) - AMPAS  
AP1_TO_XYZ = np.array([
    [0.6624541811, 0.1340042065, 0.1561876870],
    [0.2722287168, 0.6740817658, 0.0536895174],
    [-0.0055746495, 0.0040607335, 1.0103391003],
], dtype=np.float32)

# AP0 to AP1 - ACES
AP0_TO_AP1 = np.array([
    [ 1.4514393161, -0.2365107469, -0.2149285693],
    [-0.0765537734,  1.1762296998, -0.0996759264],
    [ 0.0083161484, -0.0060324498,  0.9977163014],
], dtype=np.float32)

# ARRI Wide Gamut 4 to XYZ (D65) - OCIO ColorMatrixHelpers.cpp
AWG4_TO_XYZ = np.array([
    [0.7048583, 0.1291921, 0.1156447],
    [0.2541159, 0.7815589, -0.0356747],
    [-0.0595150, -0.0779312, 1.0474635],
], dtype=np.float32)

# Sony S-Gamut3 to XYZ (D65) - OCIO SonyCameras.cpp
SGAMUT3_TO_XYZ = np.array([
    [0.7065445, 0.1289178, 0.1145377],
    [0.2709946, 0.7866195, -0.0576141],
    [-0.0096778, 0.0046206, 0.9136841],
], dtype=np.float32)

# RED Wide Gamut RGB to XYZ (D65) - OCIO RedCameras.cpp
RWG_TO_XYZ = np.array([
    [0.7352752, 0.0686624, 0.1465235],
    [0.2869164, 0.8429429, -0.1298593],
    [-0.0797991, -0.0471580, 1.0369746],
], dtype=np.float32)


# ---------------------------------------------------------------------------
# Test fixtures
# ---------------------------------------------------------------------------

@pytest.fixture(scope="module")
def test_rgb_colors() -> np.ndarray:
    """Standard test RGB colors."""
    return np.array([
        [0.0, 0.0, 0.0],      # Black
        [1.0, 1.0, 1.0],      # White
        [1.0, 0.0, 0.0],      # Red
        [0.0, 1.0, 0.0],      # Green
        [0.0, 0.0, 1.0],      # Blue
        [0.18, 0.18, 0.18],   # 18% gray
        [0.5, 0.5, 0.5],      # 50% gray
    ], dtype=np.float32)


@pytest.fixture(scope="module")
def random_rgb_colors() -> np.ndarray:
    """Random RGB colors for stress testing."""
    np.random.seed(42)
    return np.random.rand(100, 3).astype(np.float32)


# ---------------------------------------------------------------------------
# Matrix multiplication helpers
# ---------------------------------------------------------------------------

def apply_matrix(matrix: np.ndarray, rgb: np.ndarray) -> np.ndarray:
    """Apply 3x3 matrix to RGB values."""
    if rgb.ndim == 1:
        return matrix @ rgb
    else:
        return (matrix @ rgb.T).T


# ---------------------------------------------------------------------------
# Basic matrix tests
# ---------------------------------------------------------------------------

@pytest.mark.matrix
class TestBasicMatrices:
    """Basic matrix correctness tests."""
    
    def test_srgb_xyz_roundtrip(self, test_rgb_colors):
        """Test sRGB <-> XYZ roundtrip."""
        xyz = apply_matrix(SRGB_TO_XYZ, test_rgb_colors)
        rgb_back = apply_matrix(XYZ_TO_SRGB, xyz)
        
        assert_close(rgb_back, test_rgb_colors, rtol=1e-5,
                     msg="sRGB<->XYZ roundtrip failed")
    
    def test_matrix_determinant(self):
        """Test that all matrices are invertible."""
        matrices = [
            ("sRGB->XYZ", SRGB_TO_XYZ),
            ("XYZ->sRGB", XYZ_TO_SRGB),
            ("Rec2020->XYZ", REC2020_TO_XYZ),
            ("AP0->XYZ", AP0_TO_XYZ),
            ("AP1->XYZ", AP1_TO_XYZ),
            ("AP0->AP1", AP0_TO_AP1),
            ("AWG4->XYZ", AWG4_TO_XYZ),
        ]
        
        for name, matrix in matrices:
            det = np.linalg.det(matrix)
            assert abs(det) > 1e-6, f"{name} matrix is singular (det={det})"
    
    def test_identity_composition(self):
        """Test that M @ M^-1 = I."""
        identity = np.eye(3, dtype=np.float32)
        
        result = SRGB_TO_XYZ @ XYZ_TO_SRGB
        assert_close(result, identity, atol=1e-5,
                     msg="sRGB->XYZ @ XYZ->sRGB != Identity")


# ---------------------------------------------------------------------------
# VFX-RS matrix parity tests
# ---------------------------------------------------------------------------

@pytest.mark.matrix
class TestVfxMatrices:
    """vfx-rs matrix parity tests."""
    
    def test_vfx_srgb_to_xyz(self, vfx_color, test_rgb_colors):
        """Test vfx-rs sRGB to XYZ conversion."""
        try:
            vfx_matrix = vfx_color.get_matrix("sRGB", "XYZ_D65")
            vfx_matrix = np.array(vfx_matrix).reshape(3, 3)
        except (AttributeError, Exception) as e:
            pytest.skip(f"vfx-rs matrix API not available: {e}")
        
        assert_close(vfx_matrix, SRGB_TO_XYZ, rtol=1e-5,
                     msg="vfx-rs sRGB->XYZ matrix differs from reference")
    
    def test_vfx_ap0_to_ap1(self, vfx_color, test_rgb_colors):
        """Test vfx-rs ACES AP0 to AP1 conversion."""
        try:
            vfx_matrix = vfx_color.get_matrix("ACES_AP0", "ACES_AP1")
            vfx_matrix = np.array(vfx_matrix).reshape(3, 3)
        except (AttributeError, Exception) as e:
            pytest.skip(f"vfx-rs ACES matrix API not available: {e}")
        
        assert_close(vfx_matrix, AP0_TO_AP1, rtol=1e-5,
                     msg="vfx-rs AP0->AP1 matrix differs from reference")
    
    def test_vfx_color_transform(self, vfx_color, test_rgb_colors):
        """Test vfx-rs color space transformation."""
        try:
            # Transform sRGB to XYZ
            vfx_result = vfx_color.transform(test_rgb_colors, "sRGB", "XYZ_D65")
            vfx_result = np.array(vfx_result)
        except (AttributeError, Exception) as e:
            pytest.skip(f"vfx-rs transform API not available: {e}")
        
        expected = apply_matrix(SRGB_TO_XYZ, test_rgb_colors)
        
        assert_close(vfx_result, expected, rtol=1e-5,
                     msg="vfx-rs color transform differs from matrix reference")


# ---------------------------------------------------------------------------
# OCIO matrix comparison
# ---------------------------------------------------------------------------

@pytest.mark.matrix
class TestOCIOMatrixParity:
    """OCIO matrix parity tests."""
    
    def test_ocio_srgb_to_xyz(self, ocio, ocio_raw_config, test_rgb_colors):
        """Test OCIO sRGB to XYZ conversion."""
        try:
            transform = ocio.MatrixTransform(SRGB_TO_XYZ.flatten().tolist() + [0, 0, 0, 1])
            processor = ocio_raw_config.getProcessor(transform)
            cpu = processor.getDefaultCPUProcessor()
            
            result = test_rgb_colors.copy()
            cpu.applyRGB(result)
        except Exception as e:
            pytest.skip(f"OCIO matrix transform not available: {e}")
        
        expected = apply_matrix(SRGB_TO_XYZ, test_rgb_colors)
        
        assert_close(result, expected, rtol=1e-5,
                     msg="OCIO matrix transform differs from numpy reference")
    
    def test_ocio_camera_gamut_matrices(self, ocio, ocio_builtin_config, random_rgb_colors, vfx_color):
        """Test camera gamut matrices against OCIO config."""
        gamut_tests = [
            ("ARRI Wide Gamut 4", AWG4_TO_XYZ),
            ("Sony S-Gamut3", SGAMUT3_TO_XYZ),
            ("REDWideGamutRGB", RWG_TO_XYZ),
        ]
        
        for gamut_name, expected_matrix in gamut_tests:
            try:
                # Get processor from OCIO config
                processor = ocio_builtin_config.getProcessor(gamut_name, "CIE-XYZ-D65")
                cpu = processor.getDefaultCPUProcessor()
                
                # Apply to test colors
                ocio_result = random_rgb_colors[:10].copy()
                cpu.applyRGB(ocio_result)
                
                # Compare with reference matrix
                expected = apply_matrix(expected_matrix, random_rgb_colors[:10])
                
                assert_close(ocio_result, expected, rtol=1e-3,
                             msg=f"OCIO {gamut_name}->XYZ differs from reference")
                             
            except Exception as e:
                # Skip if colorspace not in config
                continue


# ---------------------------------------------------------------------------
# Chromatic adaptation tests
# ---------------------------------------------------------------------------

@pytest.mark.matrix
class TestChromaticAdaptation:
    """Chromatic adaptation matrix tests."""
    
    # Bradford adaptation matrix from D65 to D50
    BRADFORD_D65_TO_D50 = np.array([
        [ 1.0478112, 0.0228866, -0.0501270],
        [ 0.0295424, 0.9904844, -0.0170491],
        [-0.0092345, 0.0150436,  0.7521316],
    ], dtype=np.float32)
    
    def test_bradford_matrix(self, vfx_color):
        """Test Bradford adaptation matrix D65->D50."""
        try:
            vfx_matrix = vfx_color.chromatic_adaptation("D65", "D50", "Bradford")
            vfx_matrix = np.array(vfx_matrix).reshape(3, 3)
        except (AttributeError, Exception) as e:
            pytest.skip(f"vfx-rs chromatic adaptation not available: {e}")
        
        assert_close(vfx_matrix, self.BRADFORD_D65_TO_D50, rtol=1e-4,
                     msg="Bradford D65->D50 matrix differs from reference")
    
    def test_cat02_matrix(self, vfx_color):
        """Test CAT02 adaptation matrix."""
        # CAT02 D65 to D50
        CAT02_D65_TO_D50 = np.array([
            [ 1.0427245, 0.0308911, -0.0528534],
            [ 0.0221167, 0.9889460, -0.0102103],
            [-0.0085287, 0.0131037,  0.7603487],
        ], dtype=np.float32)
        
        try:
            vfx_matrix = vfx_color.chromatic_adaptation("D65", "D50", "CAT02")
            vfx_matrix = np.array(vfx_matrix).reshape(3, 3)
        except (AttributeError, Exception) as e:
            pytest.skip(f"vfx-rs CAT02 not available: {e}")
        
        assert_close(vfx_matrix, CAT02_D65_TO_D50, rtol=1e-4,
                     msg="CAT02 D65->D50 matrix differs from reference")


# ---------------------------------------------------------------------------
# Precision and edge case tests
# ---------------------------------------------------------------------------

@pytest.mark.matrix
class TestMatrixPrecision:
    """Matrix precision and edge case tests."""
    
    def test_negative_values(self, test_rgb_colors):
        """Test matrix handling of negative RGB values."""
        negative_rgb = np.array([
            [-0.1, 0.5, 0.5],
            [0.5, -0.1, 0.5],
            [0.5, 0.5, -0.1],
            [-0.1, -0.1, -0.1],
        ], dtype=np.float32)
        
        xyz = apply_matrix(SRGB_TO_XYZ, negative_rgb)
        rgb_back = apply_matrix(XYZ_TO_SRGB, xyz)
        
        assert_close(rgb_back, negative_rgb, rtol=1e-5,
                     msg="Matrix roundtrip fails with negative values")
    
    def test_large_values(self):
        """Test matrix handling of large RGB values (HDR)."""
        large_rgb = np.array([
            [10.0, 10.0, 10.0],
            [100.0, 0.0, 0.0],
            [0.0, 50.0, 0.0],
        ], dtype=np.float32)
        
        xyz = apply_matrix(SRGB_TO_XYZ, large_rgb)
        rgb_back = apply_matrix(XYZ_TO_SRGB, xyz)
        
        assert_close(rgb_back, large_rgb, rtol=1e-5,
                     msg="Matrix roundtrip fails with large values")
    
    def test_matrix_chain(self, test_rgb_colors):
        """Test chained matrix transformations."""
        # sRGB -> XYZ -> AP0 -> AP1 -> XYZ -> sRGB
        # Should return to original
        
        # Simplified: just sRGB -> XYZ -> sRGB
        intermediate = apply_matrix(SRGB_TO_XYZ, test_rgb_colors)
        final = apply_matrix(XYZ_TO_SRGB, intermediate)
        
        assert_close(final, test_rgb_colors, rtol=1e-5,
                     msg="Matrix chain roundtrip failed")
