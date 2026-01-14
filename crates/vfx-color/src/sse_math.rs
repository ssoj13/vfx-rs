//! SSE-optimized math functions matching OCIO's implementation.
//!
//! These functions use the same Chebyshev polynomial approximations as OCIO
//! to ensure bit-exact compatibility with OpenColorIO's CDL processing.

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[cfg(target_arch = "x86")]
use std::arch::x86::*;

// Chebyshev polynomial coefficients for log2() over [1.0, 2.0)
// ~15 bits of mantissa precision
const PNLOG5: f32 = 4.487361286440374006195e-2;
const PNLOG4: f32 = -4.165637071209677112635e-1;
const PNLOG3: f32 = 1.631148826119436277100;
const PNLOG2: f32 = -3.550793018041176193407;
const PNLOG1: f32 = 5.091710879305474367557;
const PNLOG0: f32 = -2.800364054395965731506;

// Chebyshev polynomial coefficients for exp2() over [0.0, 1.0)
const PNEXP4: f32 = 1.353416792833547468620e-2;
const PNEXP3: f32 = 5.201146058412685018921e-2;
const PNEXP2: f32 = 2.414427569091865207710e-1;
const PNEXP1: f32 = 6.930038344665415134202e-1;
const PNEXP0: f32 = 1.000002593370603213644;

const EXP_MASK: i32 = 0x7F800000;
const EXP_BIAS: i32 = 127;
const EXP_SHIFT: i32 = 23;

/// Fast log2 using OCIO's polynomial approximation (scalar version).
///
/// Matches OCIO's sseLog2 algorithm for bit-exact compatibility.
#[inline]
pub fn fast_log2(x: f32) -> f32 {
    if x <= 0.0 {
        return f32::NEG_INFINITY;
    }
    
    let bits = x.to_bits() as i32;
    
    // Extract mantissa and set exponent to 0 (value in [1, 2))
    let mantissa_bits = (bits & !EXP_MASK) | (EXP_BIAS << EXP_SHIFT);
    let mantissa = f32::from_bits(mantissa_bits as u32);
    
    // Polynomial evaluation: log2(mantissa)
    let log2_mantissa = PNLOG0 + mantissa * (
        PNLOG1 + mantissa * (
            PNLOG2 + mantissa * (
                PNLOG3 + mantissa * (
                    PNLOG4 + mantissa * PNLOG5
                )
            )
        )
    );
    
    // Extract exponent
    let exponent = ((bits & EXP_MASK) >> EXP_SHIFT) - EXP_BIAS;
    
    log2_mantissa + exponent as f32
}

/// Fast exp2 using OCIO's polynomial approximation (scalar version).
///
/// Matches OCIO's sseExp2 algorithm for bit-exact compatibility.
#[inline]
pub fn fast_exp2(x: f32) -> f32 {
    // Handle underflow
    if x < -126.0 {
        return 0.0;
    }
    // Handle overflow  
    if x >= 128.0 {
        return f32::INFINITY;
    }
    
    // Split into integer and fractional parts
    // Use proper floor to handle negative numbers correctly
    // (x as i32 truncates toward zero, not floor)
    let floor_x = x.floor() as i32;
    
    let fraction = x - floor_x as f32;
    
    // exp2(fraction) using polynomial
    let mexp = PNEXP0 + fraction * (
        PNEXP1 + fraction * (
            PNEXP2 + fraction * (
                PNEXP3 + fraction * PNEXP4
            )
        )
    );
    
    // exp2(floor_x) by directly setting exponent bits
    let zf_bits = ((floor_x + EXP_BIAS) << EXP_SHIFT) as u32;
    let zf = f32::from_bits(zf_bits);
    
    zf * mexp
}

/// Fast power function using OCIO's polynomial approximation.
///
/// Implements: pow(x, exp) = exp2(exp * log2(x))
///
/// This matches OCIO's ssePower algorithm for bit-exact CDL compatibility.
/// Handles negative bases by returning 0 (OCIO behavior).
#[inline]
pub fn fast_pow(base: f32, exp: f32) -> f32 {
    if base <= 0.0 {
        return 0.0;
    }
    
    let log2_base = fast_log2(base);
    let result = fast_exp2(exp * log2_base);
    
    result
}

// ============================================================================
// SSE SIMD versions (x86/x86_64 only)
// ============================================================================

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod simd {
    use super::*;
    
    /// SSE log2 - processes 4 floats at once.
    #[target_feature(enable = "sse2")]
    pub unsafe fn sse_log2(x: __m128) -> __m128 {
        let emask = _mm_set1_epi32(EXP_MASK);
        let ebias = _mm_set1_epi32(EXP_BIAS);
        let eone = _mm_set1_ps(1.0);
        
        // Extract mantissa in [1, 2)
        let mantissa = _mm_or_ps(
            _mm_andnot_ps(_mm_castsi128_ps(emask), x),
            eone
        );
        
        // Polynomial coefficients
        let pnlog5 = _mm_set1_ps(PNLOG5);
        let pnlog4 = _mm_set1_ps(PNLOG4);
        let pnlog3 = _mm_set1_ps(PNLOG3);
        let pnlog2 = _mm_set1_ps(PNLOG2);
        let pnlog1 = _mm_set1_ps(PNLOG1);
        let pnlog0 = _mm_set1_ps(PNLOG0);
        
        // Evaluate polynomial
        let log2 = _mm_add_ps(
            _mm_mul_ps(
                _mm_add_ps(
                    _mm_mul_ps(
                        _mm_add_ps(
                            _mm_mul_ps(
                                _mm_add_ps(
                                    _mm_mul_ps(
                                        _mm_add_ps(
                                            _mm_mul_ps(pnlog5, mantissa),
                                            pnlog4
                                        ),
                                        mantissa
                                    ),
                                    pnlog3
                                ),
                                mantissa
                            ),
                            pnlog2
                        ),
                        mantissa
                    ),
                    pnlog1
                ),
                mantissa
            ),
            pnlog0
        );
        
        // Extract exponent
        let exponent = _mm_sub_epi32(
            _mm_srli_epi32(
                _mm_and_si128(_mm_castps_si128(x), emask),
                EXP_SHIFT as i32
            ),
            ebias
        );
        
        _mm_add_ps(log2, _mm_cvtepi32_ps(exponent))
    }
    
    /// SSE exp2 - processes 4 floats at once.
    #[target_feature(enable = "sse2")]
    pub unsafe fn sse_exp2(x: __m128) -> __m128 {
        let ezero = _mm_setzero_ps();
        let eneg126 = _mm_set1_ps(-126.0);
        let epos128 = _mm_set1_ps(128.0);
        let eposinf = _mm_set1_ps(f32::INFINITY);
        let ebias = _mm_set1_epi32(EXP_BIAS);
        
        // Polynomial coefficients
        let pnexp4 = _mm_set1_ps(PNEXP4);
        let pnexp3 = _mm_set1_ps(PNEXP3);
        let pnexp2 = _mm_set1_ps(PNEXP2);
        let pnexp1 = _mm_set1_ps(PNEXP1);
        let pnexp0 = _mm_set1_ps(PNEXP0);
        
        // floor(x) with proper negative handling
        let floor_x = _mm_add_epi32(
            _mm_cvttps_epi32(x),
            _mm_castps_si128(_mm_cmpnle_ps(ezero, x))
        );
        
        // exp2(floor_x) via exponent bits
        let zf = _mm_castsi128_ps(
            _mm_slli_epi32(
                _mm_add_epi32(floor_x, ebias),
                EXP_SHIFT as i32
            )
        );
        
        let iexp = _mm_cvtepi32_ps(floor_x);
        let fraction = _mm_sub_ps(x, iexp);
        
        // Polynomial for exp2(fraction)
        let mexp = _mm_add_ps(
            _mm_mul_ps(
                _mm_add_ps(
                    _mm_mul_ps(
                        _mm_add_ps(
                            _mm_mul_ps(
                                _mm_add_ps(
                                    _mm_mul_ps(pnexp4, fraction),
                                    pnexp3
                                ),
                                fraction
                            ),
                            pnexp2
                        ),
                        fraction
                    ),
                    pnexp1
                ),
                fraction
            ),
            pnexp0
        );
        
        let mut exp2 = _mm_mul_ps(zf, mexp);
        
        // Handle underflow
        exp2 = _mm_andnot_ps(_mm_cmplt_ps(x, eneg126), exp2);
        
        // Handle overflow
        let overflow_mask = _mm_cmpge_ps(x, epos128);
        exp2 = _mm_or_ps(
            _mm_and_ps(overflow_mask, eposinf),
            _mm_andnot_ps(overflow_mask, exp2)
        );
        
        exp2
    }
    
    /// SSE power - processes 4 floats at once.
    /// 
    /// Matches OCIO's ssePower exactly.
    #[target_feature(enable = "sse2")]
    pub unsafe fn sse_power(base: __m128, exp: __m128) -> __m128 {
        let ezero = _mm_setzero_ps();
        
        // SAFETY: calling sibling unsafe SSE functions within unsafe fn
        let values = unsafe { sse_log2(base) };
        let values = _mm_mul_ps(exp, values);
        let values = unsafe { sse_exp2(values) };
        
        // Handle negative/zero bases - return 0
        _mm_and_ps(values, _mm_cmpgt_ps(base, ezero))
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub use simd::*;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fast_pow_basic() {
        // Test basic powers
        let result = fast_pow(2.0, 3.0);
        assert!((result - 8.0).abs() < 0.01, "2^3 = {}", result);
        
        let result = fast_pow(0.5, 2.0);
        assert!((result - 0.25).abs() < 0.001, "0.5^2 = {}", result);
        
        let result = fast_pow(0.757, 1.2);
        let expected = 0.757_f32.powf(1.2);
        let rel_error = (result - expected).abs() / expected;
        assert!(rel_error < 0.0001, "0.757^1.2: got {}, expected {}, error {}", 
                result, expected, rel_error);
    }
    
    #[test]
    fn test_fast_pow_cdl_range() {
        // Test typical CDL ranges
        let test_values = [0.1, 0.25, 0.5, 0.75, 0.9, 1.0];
        let test_powers = [0.8, 1.0, 1.2, 1.5, 2.0];
        
        for &base in &test_values {
            for &power in &test_powers {
                let result = fast_pow(base, power);
                let expected = base.powf(power);
                let rel_error = if expected > 0.0 {
                    (result - expected).abs() / expected
                } else {
                    (result - expected).abs()
                };
                
                assert!(rel_error < 0.001, 
                    "{}^{}: got {}, expected {}, error {}", 
                    base, power, result, expected, rel_error);
            }
        }
    }
    
    #[test]
    fn test_fast_pow_edge_cases() {
        assert_eq!(fast_pow(0.0, 1.0), 0.0);
        assert_eq!(fast_pow(-1.0, 2.0), 0.0); // OCIO returns 0 for negative bases
        assert!(fast_pow(1.0, 100.0) > 0.99 && fast_pow(1.0, 100.0) < 1.01);
    }
    
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[test]
    fn test_sse_power() {
        if !is_x86_feature_detected!("sse2") {
            return;
        }
        
        unsafe {
            let base = _mm_setr_ps(0.5, 0.75, 0.9, 1.0);
            let exp = _mm_set1_ps(1.2);
            let result = sse_power(base, exp);
            
            let mut out = [0.0f32; 4];
            _mm_storeu_ps(out.as_mut_ptr(), result);
            
            for i in 0..4 {
                let base_val: f32 = [0.5, 0.75, 0.9, 1.0][i];
                let expected = base_val.powf(1.2);
                let rel_error = (out[i] - expected).abs() / expected;
                assert!(rel_error < 0.001, 
                    "SSE power {}^1.2: got {}, expected {}", 
                    base_val, out[i], expected);
            }
        }
    }
}
