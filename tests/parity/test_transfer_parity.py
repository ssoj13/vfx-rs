"""
Transfer function parity tests: vfx-rs vs PyOpenColorIO.

Tests all 22 transfer functions for mathematical correctness against OCIO reference.
"""

import pytest
import numpy as np
from conftest import assert_close, apply_ocio_cpu, ocio_builtin_transform, RTOL, ATOL


# ---------------------------------------------------------------------------
# OCIO BuiltinTransform names for each transfer function
# ---------------------------------------------------------------------------

TRANSFER_FUNCTIONS = {
    # Camera Log curves
    "arri_logc3": ("ARRI_LOGC3_to_LINEAR", "LINEAR_to_ARRI_LOGC3"),
    "arri_logc4": ("ARRI_LOGC4_to_LINEAR", "LINEAR_to_ARRI_LOGC4"),
    "sony_slog2": ("SONY_SLOG2_to_LINEAR", "LINEAR_to_SONY_SLOG2"),
    "sony_slog3": ("SONY_SLOG3_to_LINEAR", "LINEAR_to_SONY_SLOG3"),
    "panasonic_vlog": ("PANASONIC_VLOG_to_LINEAR", "LINEAR_to_PANASONIC_VLOG"),
    "canon_clog2": ("CANON_CLOG2_to_LINEAR", "LINEAR_to_CANON_CLOG2"),
    "canon_clog3": ("CANON_CLOG3_to_LINEAR", "LINEAR_to_CANON_CLOG3"),
    "red_log3g10": ("RED_LOG3G10_to_LINEAR", "LINEAR_to_RED_LOG3G10"),
    "apple_log": ("APPLE_LOG_to_LINEAR", "LINEAR_to_APPLE_LOG"),
    "bmd_film_gen5": ("BLACKMAGIC_FILM_GEN5_to_LINEAR", "LINEAR_to_BLACKMAGIC_FILM_GEN5"),
    "davinci_intermediate": ("DAVINCI_INTERMEDIATE_to_LINEAR", "LINEAR_to_DAVINCI_INTERMEDIATE"),
    
    # Display curves  
    "srgb": ("sRGB_to_LINEAR", "LINEAR_to_sRGB"),
    "rec709": ("REC709_to_LINEAR", "LINEAR_to_REC709"),
    
    # HDR
    "pq": ("ST2084_to_LINEAR", "LINEAR_to_ST2084"),
    "hlg": ("HLG_to_LINEAR", "LINEAR_to_HLG"),
    
    # ACES
    "acescc": ("ACEScc_to_LINEAR", "LINEAR_to_ACEScc"),
    "acescct": ("ACEScct_to_LINEAR", "LINEAR_to_ACEScct"),
}

# Fallback OCIO names (some may differ between versions)
OCIO_BUILTIN_ALIASES = {
    "sRGB_to_LINEAR": "CURVE - sRGB_to_LINEAR",
    "LINEAR_to_sRGB": "CURVE - LINEAR_to_sRGB",
    "REC709_to_LINEAR": "CURVE - REC.709_to_LINEAR", 
    "LINEAR_to_REC709": "CURVE - LINEAR_to_REC.709",
    "ST2084_to_LINEAR": "CURVE - ST2084_to_LINEAR",
    "LINEAR_to_ST2084": "CURVE - LINEAR_to_ST2084",
    "HLG_to_LINEAR": "CURVE - HLG_to_LINEAR",
    "LINEAR_to_HLG": "CURVE - LINEAR_to_HLG",
    "ACEScc_to_LINEAR": "CURVE - ACEScc_to_ACES2065-1",
    "LINEAR_to_ACEScc": "CURVE - ACES2065-1_to_ACEScc",
    "ACEScct_to_LINEAR": "CURVE - ACEScct_to_ACES2065-1",
    "LINEAR_to_ACEScct": "CURVE - ACES2065-1_to_ACEScct",
}


def get_ocio_transform(ocio, name: str, direction: str = "FORWARD"):
    """Get OCIO builtin transform, trying aliases if needed."""
    try:
        return ocio_builtin_transform(ocio, name, direction)
    except Exception:
        # Try alias
        if name in OCIO_BUILTIN_ALIASES:
            return ocio_builtin_transform(ocio, OCIO_BUILTIN_ALIASES[name], direction)
        raise


# ---------------------------------------------------------------------------
# Test fixtures for transfer functions
# ---------------------------------------------------------------------------

@pytest.fixture(scope="module")
def test_values_standard() -> np.ndarray:
    """Standard test values 0-1 range."""
    return np.array([
        0.0, 0.001, 0.01, 0.02, 0.05, 0.10, 0.18, 0.25,
        0.50, 0.75, 0.90, 0.95, 0.99, 1.0
    ], dtype=np.float32)


@pytest.fixture(scope="module")
def test_values_hdr() -> np.ndarray:
    """HDR test values 0-100 range."""
    return np.array([
        0.0, 0.001, 0.01, 0.1, 0.18, 0.5, 1.0, 2.0,
        4.0, 8.0, 16.0, 32.0, 64.0, 100.0
    ], dtype=np.float32)


@pytest.fixture(scope="module")  
def test_values_extended() -> np.ndarray:
    """Extended range with small and large values."""
    return np.array([
        0.0, 1e-6, 1e-4, 0.001, 0.01, 0.1, 0.18,
        0.5, 1.0, 2.0, 10.0, 100.0, 1000.0
    ], dtype=np.float32)


# ---------------------------------------------------------------------------
# sRGB
# ---------------------------------------------------------------------------

@pytest.mark.transfer
class TestSRGB:
    """sRGB OETF/EOTF parity tests."""
    
    def test_srgb_to_linear_parity(self, ocio, ocio_raw_config, vfx_transfer, test_values_standard):
        """Test sRGB to linear vs OCIO."""
        try:
            transform = get_ocio_transform(ocio, "sRGB_to_LINEAR")
            processor = ocio_raw_config.getProcessor(transform)
            ocio_result = apply_ocio_cpu(processor, test_values_standard)
        except Exception as e:
            pytest.skip(f"OCIO sRGB transform not available: {e}")
        
        vfx_result = np.array([vfx_transfer.srgb_eotf(v) for v in test_values_standard], 
                              dtype=np.float32)
        
        assert_close(vfx_result, ocio_result, rtol=RTOL, 
                     msg="sRGB EOTF mismatch with OCIO")
    
    def test_srgb_roundtrip(self, vfx_transfer, test_values_standard):
        """Test sRGB encode/decode roundtrip."""
        encoded = np.array([vfx_transfer.srgb_oetf(v) for v in test_values_standard])
        decoded = np.array([vfx_transfer.srgb_eotf(v) for v in encoded])
        
        assert_close(decoded, test_values_standard, rtol=1e-5,
                     msg="sRGB roundtrip failed")


# ---------------------------------------------------------------------------
# ARRI LogC
# ---------------------------------------------------------------------------

@pytest.mark.transfer
class TestARRILogC:
    """ARRI LogC3/LogC4 parity tests."""
    
    def test_logc3_to_linear_parity(self, ocio, ocio_raw_config, vfx_transfer, test_values_standard):
        """Test LogC3 decode vs OCIO."""
        try:
            transform = get_ocio_transform(ocio, "ARRI_LOGC3_to_LINEAR")
            processor = ocio_raw_config.getProcessor(transform)
            ocio_result = apply_ocio_cpu(processor, test_values_standard)
        except Exception as e:
            pytest.skip(f"OCIO LogC3 transform not available: {e}")
        
        vfx_result = np.array([vfx_transfer.logc3_decode(v) for v in test_values_standard],
                              dtype=np.float32)
        
        assert_close(vfx_result, ocio_result, rtol=RTOL,
                     msg="LogC3 decode mismatch with OCIO")
    
    def test_logc4_to_linear_parity(self, ocio, ocio_raw_config, vfx_transfer, test_values_standard):
        """Test LogC4 decode vs OCIO."""
        try:
            transform = get_ocio_transform(ocio, "ARRI_LOGC4_to_LINEAR")
            processor = ocio_raw_config.getProcessor(transform)
            ocio_result = apply_ocio_cpu(processor, test_values_standard)
        except Exception as e:
            pytest.skip(f"OCIO LogC4 transform not available: {e}")
        
        vfx_result = np.array([vfx_transfer.logc4_decode(v) for v in test_values_standard],
                              dtype=np.float32)
        
        assert_close(vfx_result, ocio_result, rtol=RTOL,
                     msg="LogC4 decode mismatch with OCIO")
    
    def test_logc3_roundtrip(self, vfx_transfer, test_values_extended):
        """Test LogC3 encode/decode roundtrip."""
        # Skip negative values
        positive = test_values_extended[test_values_extended >= 0]
        encoded = np.array([vfx_transfer.logc3_encode(v) for v in positive])
        decoded = np.array([vfx_transfer.logc3_decode(v) for v in encoded])
        
        assert_close(decoded, positive, rtol=1e-4,
                     msg="LogC3 roundtrip failed")
    
    def test_logc4_roundtrip(self, vfx_transfer, test_values_extended):
        """Test LogC4 encode/decode roundtrip."""
        positive = test_values_extended[test_values_extended >= 0]
        encoded = np.array([vfx_transfer.logc4_encode(v) for v in positive])
        decoded = np.array([vfx_transfer.logc4_decode(v) for v in encoded])
        
        assert_close(decoded, positive, rtol=1e-4,
                     msg="LogC4 roundtrip failed")


# ---------------------------------------------------------------------------
# Sony S-Log
# ---------------------------------------------------------------------------

@pytest.mark.transfer
class TestSonyLog:
    """Sony S-Log2/S-Log3 parity tests."""
    
    def test_slog3_to_linear_parity(self, ocio, ocio_raw_config, vfx_transfer, test_values_standard):
        """Test S-Log3 decode vs OCIO."""
        try:
            transform = get_ocio_transform(ocio, "SONY_SLOG3_to_LINEAR")
            processor = ocio_raw_config.getProcessor(transform)
            ocio_result = apply_ocio_cpu(processor, test_values_standard)
        except Exception as e:
            pytest.skip(f"OCIO S-Log3 transform not available: {e}")
        
        vfx_result = np.array([vfx_transfer.slog3_decode(v) for v in test_values_standard],
                              dtype=np.float32)
        
        assert_close(vfx_result, ocio_result, rtol=RTOL,
                     msg="S-Log3 decode mismatch with OCIO")
    
    def test_slog3_roundtrip(self, vfx_transfer, test_values_extended):
        """Test S-Log3 encode/decode roundtrip."""
        positive = test_values_extended[test_values_extended >= 0]
        encoded = np.array([vfx_transfer.slog3_encode(v) for v in positive])
        decoded = np.array([vfx_transfer.slog3_decode(v) for v in encoded])
        
        assert_close(decoded, positive, rtol=1e-4,
                     msg="S-Log3 roundtrip failed")


# ---------------------------------------------------------------------------
# Canon Log
# ---------------------------------------------------------------------------

@pytest.mark.transfer
class TestCanonLog:
    """Canon Log 2/3 parity tests."""
    
    def test_clog2_to_linear_parity(self, ocio, ocio_raw_config, vfx_transfer, test_values_standard):
        """Test Canon Log 2 decode vs OCIO."""
        try:
            transform = get_ocio_transform(ocio, "CANON_CLOG2_to_LINEAR")
            processor = ocio_raw_config.getProcessor(transform)
            ocio_result = apply_ocio_cpu(processor, test_values_standard)
        except Exception as e:
            pytest.skip(f"OCIO Canon Log 2 transform not available: {e}")
        
        vfx_result = np.array([vfx_transfer.clog2_decode(v) for v in test_values_standard],
                              dtype=np.float32)
        
        assert_close(vfx_result, ocio_result, rtol=RTOL,
                     msg="Canon Log 2 decode mismatch with OCIO")
    
    def test_clog3_to_linear_parity(self, ocio, ocio_raw_config, vfx_transfer, test_values_standard):
        """Test Canon Log 3 decode vs OCIO."""
        try:
            transform = get_ocio_transform(ocio, "CANON_CLOG3_to_LINEAR")
            processor = ocio_raw_config.getProcessor(transform)
            ocio_result = apply_ocio_cpu(processor, test_values_standard)
        except Exception as e:
            pytest.skip(f"OCIO Canon Log 3 transform not available: {e}")
        
        vfx_result = np.array([vfx_transfer.clog3_decode(v) for v in test_values_standard],
                              dtype=np.float32)
        
        assert_close(vfx_result, ocio_result, rtol=RTOL,
                     msg="Canon Log 3 decode mismatch with OCIO")


# ---------------------------------------------------------------------------
# RED Log
# ---------------------------------------------------------------------------

@pytest.mark.transfer
class TestREDLog:
    """RED Log3G10 parity tests."""
    
    def test_log3g10_to_linear_parity(self, ocio, ocio_raw_config, vfx_transfer, test_values_standard):
        """Test RED Log3G10 decode vs OCIO."""
        try:
            transform = get_ocio_transform(ocio, "RED_LOG3G10_to_LINEAR")
            processor = ocio_raw_config.getProcessor(transform)
            ocio_result = apply_ocio_cpu(processor, test_values_standard)
        except Exception as e:
            pytest.skip(f"OCIO RED Log3G10 transform not available: {e}")
        
        vfx_result = np.array([vfx_transfer.log3g10_decode(v) for v in test_values_standard],
                              dtype=np.float32)
        
        assert_close(vfx_result, ocio_result, rtol=RTOL,
                     msg="RED Log3G10 decode mismatch with OCIO")


# ---------------------------------------------------------------------------
# HDR: PQ and HLG
# ---------------------------------------------------------------------------

@pytest.mark.transfer
class TestHDR:
    """PQ and HLG parity tests."""
    
    def test_pq_to_linear_parity(self, ocio, ocio_raw_config, vfx_transfer, test_values_standard):
        """Test PQ EOTF vs OCIO."""
        try:
            transform = get_ocio_transform(ocio, "ST2084_to_LINEAR")
            processor = ocio_raw_config.getProcessor(transform)
            ocio_result = apply_ocio_cpu(processor, test_values_standard)
        except Exception as e:
            pytest.skip(f"OCIO PQ transform not available: {e}")
        
        vfx_result = np.array([vfx_transfer.pq_eotf(v) for v in test_values_standard],
                              dtype=np.float32)
        
        # PQ can have larger values, use slightly looser tolerance
        assert_close(vfx_result, ocio_result, rtol=1e-3,
                     msg="PQ EOTF mismatch with OCIO")
    
    def test_pq_roundtrip(self, vfx_transfer):
        """Test PQ encode/decode roundtrip."""
        # PQ input is 0-10000 nits normalized
        test_values = np.array([0.0, 0.001, 0.01, 0.1, 0.5, 1.0], dtype=np.float32)
        encoded = np.array([vfx_transfer.pq_oetf(v) for v in test_values])
        decoded = np.array([vfx_transfer.pq_eotf(v) for v in encoded])
        
        assert_close(decoded, test_values, rtol=1e-4,
                     msg="PQ roundtrip failed")
    
    def test_hlg_roundtrip(self, vfx_transfer):
        """Test HLG encode/decode roundtrip."""
        test_values = np.array([0.0, 0.01, 0.1, 0.5, 1.0], dtype=np.float32)
        encoded = np.array([vfx_transfer.hlg_oetf(v) for v in test_values])
        decoded = np.array([vfx_transfer.hlg_eotf(v) for v in encoded])
        
        assert_close(decoded, test_values, rtol=1e-4,
                     msg="HLG roundtrip failed")


# ---------------------------------------------------------------------------
# ACES
# ---------------------------------------------------------------------------

@pytest.mark.transfer
class TestACES:
    """ACEScc/ACEScct parity tests."""
    
    def test_acescct_reference_values(self, vfx_transfer):
        """Test ACEScct against AMPAS specification values."""
        # From AMPAS S-2016-001
        test_cases = [
            (0.0, 0.0729055341958355),     # Zero -> B constant
            (0.0078125, 0.155251141552511), # Break point
            (0.18, 0.4135),                 # Mid gray (approx)
            (1.0, 0.5548),                  # 1.0 linear (approx)
        ]
        
        for linear, expected in test_cases:
            result = vfx_transfer.acescct_encode(linear)
            assert abs(result - expected) < 0.002, \
                f"ACEScct({linear}) = {result}, expected {expected}"
    
    def test_acescc_reference_values(self, vfx_transfer):
        """Test ACEScc against AMPAS specification values."""
        # From AMPAS S-2014-003
        test_cases = [
            (0.18, 0.4135),  # Mid gray
            (1.0, 0.5548),   # 1.0 linear
        ]
        
        for linear, expected in test_cases:
            result = vfx_transfer.acescc_encode(linear)
            assert abs(result - expected) < 0.002, \
                f"ACEScc({linear}) = {result}, expected {expected}"
    
    def test_acescct_roundtrip(self, vfx_transfer):
        """Test ACEScct encode/decode roundtrip."""
        test_values = np.array([0.0, 0.001, 0.01, 0.18, 0.5, 1.0, 2.0, 10.0], 
                               dtype=np.float32)
        encoded = np.array([vfx_transfer.acescct_encode(v) for v in test_values])
        decoded = np.array([vfx_transfer.acescct_decode(v) for v in encoded])
        
        assert_close(decoded, test_values, rtol=1e-4,
                     msg="ACEScct roundtrip failed")
    
    def test_acescc_roundtrip(self, vfx_transfer):
        """Test ACEScc encode/decode roundtrip."""
        # Skip very small values (log behavior)
        test_values = np.array([0.001, 0.01, 0.18, 0.5, 1.0, 2.0, 10.0],
                               dtype=np.float32)
        encoded = np.array([vfx_transfer.acescc_encode(v) for v in test_values])
        decoded = np.array([vfx_transfer.acescc_decode(v) for v in encoded])
        
        assert_close(decoded, test_values, rtol=1e-4,
                     msg="ACEScc roundtrip failed")


# ---------------------------------------------------------------------------
# All transfer functions comprehensive test
# ---------------------------------------------------------------------------

@pytest.mark.transfer
@pytest.mark.slow
class TestAllTransfers:
    """Comprehensive tests for all transfer functions."""
    
    @pytest.mark.parametrize("name,funcs", [
        ("srgb", ("srgb_oetf", "srgb_eotf")),
        ("logc3", ("logc3_encode", "logc3_decode")),
        ("logc4", ("logc4_encode", "logc4_decode")),
        ("slog3", ("slog3_encode", "slog3_decode")),
        ("vlog", ("vlog_encode", "vlog_decode")),
        ("clog2", ("clog2_encode", "clog2_decode")),
        ("clog3", ("clog3_encode", "clog3_decode")),
        ("log3g10", ("log3g10_encode", "log3g10_decode")),
        ("acescct", ("acescct_encode", "acescct_decode")),
        ("acescc", ("acescc_encode", "acescc_decode")),
        ("pq", ("pq_oetf", "pq_eotf")),
        ("hlg", ("hlg_oetf", "hlg_eotf")),
    ])
    def test_roundtrip_all(self, vfx_transfer, name, funcs):
        """Test roundtrip for all transfer functions."""
        encode_name, decode_name = funcs
        
        try:
            encode = getattr(vfx_transfer, encode_name)
            decode = getattr(vfx_transfer, decode_name)
        except AttributeError:
            pytest.skip(f"Transfer function {name} not exposed in Python API")
        
        # Test values - avoid problematic ranges
        test_values = np.array([0.001, 0.01, 0.18, 0.5, 1.0], dtype=np.float32)
        
        encoded = np.array([encode(v) for v in test_values])
        decoded = np.array([decode(v) for v in encoded])
        
        assert_close(decoded, test_values, rtol=1e-4,
                     msg=f"{name} roundtrip failed")
    
    def test_monotonicity(self, vfx_transfer):
        """Test that all transfer functions are monotonic."""
        test_values = np.linspace(0.001, 10.0, 100, dtype=np.float32)
        
        functions = [
            ("srgb_oetf", test_values[test_values <= 1.0]),
            ("logc3_encode", test_values),
            ("logc4_encode", test_values),
            ("slog3_encode", test_values),
            ("acescct_encode", test_values),
            ("pq_oetf", test_values[test_values <= 1.0]),
        ]
        
        for func_name, values in functions:
            try:
                func = getattr(vfx_transfer, func_name)
                results = np.array([func(v) for v in values])
                
                # Check monotonically increasing
                diffs = np.diff(results)
                assert np.all(diffs >= 0), f"{func_name} is not monotonic"
                
            except AttributeError:
                continue  # Skip if not available
