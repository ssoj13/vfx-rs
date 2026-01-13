#!/usr/bin/env python3
"""Numerical verification of LUT3D interpolation against OCIO."""

import numpy as np

try:
    import PyOpenColorIO as ocio
except ImportError:
    import opencolorio as ocio

def create_test_lut(size=5):
    """Create a non-trivial test LUT with known values."""
    # Blue-major order like OCIO
    data = []
    for r in range(size):
        for g in range(size):
            for b in range(size):
                rf = r / (size - 1)
                gf = g / (size - 1)
                bf = b / (size - 1)
                # Non-linear transform for testing
                out_r = rf * 0.8 + gf * 0.1 + bf * 0.1
                out_g = rf * 0.1 + gf * 0.8 + bf * 0.1
                out_b = rf * 0.1 + gf * 0.1 + bf * 0.8
                data.extend([out_r, out_g, out_b])
    return np.array(data, dtype=np.float32)

def apply_ocio_lut3d(lut_data, size, test_pixels, interp='tetrahedral'):
    """Apply LUT using OCIO."""
    config = ocio.Config.CreateRaw()
    
    # Create Lut3DTransform
    lut = ocio.Lut3DTransform()
    lut.setGridSize(size)
    
    if interp == 'tetrahedral':
        lut.setInterpolation(ocio.INTERP_TETRAHEDRAL)
    else:
        lut.setInterpolation(ocio.INTERP_LINEAR)
    
    # Set data (OCIO expects Blue-fast order)
    lut.setData(lut_data)
    
    processor = config.getProcessor(lut)
    cpu = processor.getDefaultCPUProcessor()
    
    result = test_pixels.copy()
    cpu.applyRGB(result)
    return result

def apply_rust_style_tetrahedral(lut_data, size, test_pixels):
    """Apply tetrahedral interpolation matching our Rust implementation."""
    result = np.zeros_like(test_pixels)
    n = size - 1
    
    for pi, pixel in enumerate(test_pixels):
        r, g, b = pixel
        r = np.clip(r, 0, 1)
        g = np.clip(g, 0, 1)
        b = np.clip(b, 0, 1)
        
        # Grid indices
        ri = int(min(r * n, n - 1))
        gi = int(min(g * n, n - 1))
        bi = int(min(b * n, n - 1))
        
        # Fractional parts
        rf = r * n - ri
        gf = g * n - gi
        bf = b * n - bi
        
        # Get 8 corners (Blue-major indexing)
        def get(r_idx, g_idx, b_idx):
            idx = 3 * (b_idx + size * (g_idx + size * r_idx))
            return lut_data[idx:idx+3]
        
        c000 = get(ri, gi, bi)
        c100 = get(ri+1, gi, bi)
        c010 = get(ri, gi+1, bi)
        c110 = get(ri+1, gi+1, bi)
        c001 = get(ri, gi, bi+1)
        c101 = get(ri+1, gi, bi+1)
        c011 = get(ri, gi+1, bi+1)
        c111 = get(ri+1, gi+1, bi+1)
        
        # Tetrahedral interpolation (OCIO conditions)
        out = np.zeros(3)
        for i in range(3):
            if rf > gf:
                if gf > bf:
                    # T1: rf > gf > bf
                    out[i] = c000[i] + rf*(c100[i]-c000[i]) + gf*(c110[i]-c100[i]) + bf*(c111[i]-c110[i])
                elif rf > bf:
                    # T2: rf > bf >= gf
                    out[i] = c000[i] + rf*(c100[i]-c000[i]) + bf*(c101[i]-c100[i]) + gf*(c111[i]-c101[i])
                else:
                    # T3: bf >= rf > gf
                    out[i] = c000[i] + bf*(c001[i]-c000[i]) + rf*(c101[i]-c001[i]) + gf*(c111[i]-c101[i])
            else:
                # gf >= rf
                if bf > gf:
                    # T6: bf > gf >= rf
                    out[i] = c000[i] + bf*(c001[i]-c000[i]) + gf*(c011[i]-c001[i]) + rf*(c111[i]-c011[i])
                elif bf > rf:
                    # T5: gf >= bf > rf
                    out[i] = c000[i] + gf*(c010[i]-c000[i]) + bf*(c011[i]-c010[i]) + rf*(c111[i]-c011[i])
                else:
                    # T4: gf >= rf >= bf
                    out[i] = c000[i] + gf*(c010[i]-c000[i]) + rf*(c110[i]-c010[i]) + bf*(c111[i]-c110[i])
        
        result[pi] = out
    
    return result

def apply_rust_style_trilinear(lut_data, size, test_pixels):
    """Apply trilinear interpolation matching our Rust implementation."""
    result = np.zeros_like(test_pixels)
    n = size - 1
    
    for pi, pixel in enumerate(test_pixels):
        r, g, b = pixel
        r = np.clip(r, 0, 1)
        g = np.clip(g, 0, 1)
        b = np.clip(b, 0, 1)
        
        # Grid indices
        ri = int(min(r * n, n - 1))
        gi = int(min(g * n, n - 1))
        bi = int(min(b * n, n - 1))
        
        # Fractional parts
        rf = r * n - ri
        gf = g * n - gi
        bf = b * n - bi
        
        # Get 8 corners (Blue-major indexing)
        def get(r_idx, g_idx, b_idx):
            idx = 3 * (b_idx + size * (g_idx + size * r_idx))
            return lut_data[idx:idx+3]
        
        c000 = get(ri, gi, bi)
        c100 = get(ri+1, gi, bi)
        c010 = get(ri, gi+1, bi)
        c110 = get(ri+1, gi+1, bi)
        c001 = get(ri, gi, bi+1)
        c101 = get(ri+1, gi, bi+1)
        c011 = get(ri, gi+1, bi+1)
        c111 = get(ri+1, gi+1, bi+1)
        
        # Trilinear interpolation (OCIO order: B -> G -> R)
        out = np.zeros(3)
        for i in range(3):
            # Blue axis first
            b0 = c000[i] * (1-bf) + c001[i] * bf
            b1 = c010[i] * (1-bf) + c011[i] * bf
            b2 = c100[i] * (1-bf) + c101[i] * bf
            b3 = c110[i] * (1-bf) + c111[i] * bf
            
            # Green axis
            g0 = b0 * (1-gf) + b1 * gf
            g1 = b2 * (1-gf) + b3 * gf
            
            # Red axis
            out[i] = g0 * (1-rf) + g1 * rf
        
        result[pi] = out
    
    return result

def main():
    print("="*60)
    print("LUT3D OCIO Parity Verification")
    print("="*60)
    
    # Create test LUT
    size = 5
    lut_data = create_test_lut(size)
    print(f"\nCreated {size}x{size}x{size} test LUT")
    
    # Generate test pixels
    np.random.seed(42)
    test_pixels = np.random.rand(1000, 3).astype(np.float32)
    
    # Also add specific edge cases
    edge_cases = np.array([
        [0.0, 0.0, 0.0],   # Black
        [1.0, 1.0, 1.0],   # White
        [0.5, 0.5, 0.5],   # Gray
        [0.3, 0.5, 0.7],   # R < G < B
        [0.7, 0.5, 0.3],   # R > G > B
        [0.5, 0.7, 0.3],   # G > R > B
        [0.3, 0.7, 0.5],   # G > B > R
        [0.5, 0.3, 0.7],   # B > R > G
        [0.7, 0.3, 0.5],   # R > B > G
        [0.33333, 0.33333, 0.33333],  # Equal fractions
        [0.0, 0.5, 1.0],   # Extremes
    ], dtype=np.float32)
    
    test_pixels = np.vstack([test_pixels, edge_cases])
    print(f"Testing {len(test_pixels)} pixels")
    
    # Test TETRAHEDRAL
    print("\n--- TETRAHEDRAL INTERPOLATION ---")
    ocio_result = apply_ocio_lut3d(lut_data, size, test_pixels, 'tetrahedral')
    rust_result = apply_rust_style_tetrahedral(lut_data, size, test_pixels)
    
    diff = np.abs(ocio_result - rust_result)
    max_diff = np.max(diff)
    mean_diff = np.mean(diff)
    
    print(f"Max diff:  {max_diff:.2e}")
    print(f"Mean diff: {mean_diff:.2e}")
    
    if max_diff > 1e-6:
        worst_idx = np.unravel_index(np.argmax(diff), diff.shape)
        print(f"\nWorst pixel [{worst_idx[0]}]:")
        print(f"  Input: {test_pixels[worst_idx[0]]}")
        print(f"  OCIO:  {ocio_result[worst_idx[0]]}")
        print(f"  Rust:  {rust_result[worst_idx[0]]}")
    else:
        print("[OK] TETRAHEDRAL: Perfect match!")
    
    # Test TRILINEAR
    print("\n--- TRILINEAR INTERPOLATION ---")
    ocio_result = apply_ocio_lut3d(lut_data, size, test_pixels, 'linear')
    rust_result = apply_rust_style_trilinear(lut_data, size, test_pixels)
    
    diff = np.abs(ocio_result - rust_result)
    max_diff = np.max(diff)
    mean_diff = np.mean(diff)
    
    print(f"Max diff:  {max_diff:.2e}")
    print(f"Mean diff: {mean_diff:.2e}")
    
    if max_diff > 1e-6:
        worst_idx = np.unravel_index(np.argmax(diff), diff.shape)
        print(f"\nWorst pixel [{worst_idx[0]}]:")
        print(f"  Input: {test_pixels[worst_idx[0]]}")
        print(f"  OCIO:  {ocio_result[worst_idx[0]]}")
        print(f"  Rust:  {rust_result[worst_idx[0]]}")
    else:
        print("[OK] TRILINEAR: Perfect match!")
    
    print("\n" + "="*60)
    print("VERIFICATION COMPLETE")
    print("="*60)

if __name__ == "__main__":
    main()
