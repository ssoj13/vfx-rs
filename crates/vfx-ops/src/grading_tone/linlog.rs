//! Linear-to-Log and Log-to-Linear conversion for LINEAR grading style.
//!
//! Reference: OCIO ops/gradingtone/GradingToneOpCPU.cpp (LogLinConstants, LinLog, LogLin)
//!
//! When using LINEAR grading style, OCIO internally converts scene-linear
//! values to a log-like space, applies grading, then converts back.

/// Constants for Lin-Log conversion.
pub mod constants {
    /// Breakpoint in linear space.
    pub const XBRK: f32 = 0.0041318374739483946;
    /// Shift parameter.
    pub const SHIFT: f32 = -0.000157849851665374;
    /// Multiplier: 1 / (0.18 + shift)
    pub const M: f32 = 5.560976; // 1.0 / (0.18 + SHIFT)
    /// Gain for linear segment.
    pub const GAIN: f32 = 363.034608563;
    /// Offset for linear segment.
    pub const OFFS: f32 = -7.0;
    /// Breakpoint in log space.
    pub const YBRK: f32 = -5.5;
    /// Log base 2 constant: 1 / ln(2)
    pub const LOG2_E: f32 = 1.4426950408889634;
}

/// Convert linear RGB to log-like space.
///
/// Used at the start of LINEAR style grading.
#[inline]
pub fn lin_to_log(rgb: &mut [f32; 3]) {
    use constants::*;

    for c in rgb.iter_mut() {
        *c = if *c < XBRK {
            *c * GAIN + OFFS
        } else {
            LOG2_E * ((*c + SHIFT) * M).ln()
        };
    }
}

/// Convert log-like space back to linear RGB.
///
/// Used at the end of LINEAR style grading.
#[inline]
pub fn log_to_lin(rgb: &mut [f32; 3]) {
    use constants::*;

    for c in rgb.iter_mut() {
        *c = if *c < YBRK {
            (*c - OFFS) / GAIN
        } else {
            2.0_f32.powf(*c) * (0.18 + SHIFT) - SHIFT
        };
    }
}

/// Convert linear RGBA to log-like space (alpha unchanged).
#[inline]
#[allow(dead_code)]
pub fn lin_to_log_rgba(rgba: &mut [f32; 4]) {
    let mut rgb = [rgba[0], rgba[1], rgba[2]];
    lin_to_log(&mut rgb);
    rgba[0] = rgb[0];
    rgba[1] = rgb[1];
    rgba[2] = rgb[2];
}

/// Convert log-like space back to linear RGBA (alpha unchanged).
#[inline]
#[allow(dead_code)]
pub fn log_to_lin_rgba(rgba: &mut [f32; 4]) {
    let mut rgb = [rgba[0], rgba[1], rgba[2]];
    log_to_lin(&mut rgb);
    rgba[0] = rgb[0];
    rgba[1] = rgb[1];
    rgba[2] = rgb[2];
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-3; // Higher tolerance for larger values

    #[test]
    fn test_roundtrip() {
        let test_values: [f32; 6] = [0.001, 0.01, 0.18, 0.5, 1.0, 2.0];

        for &val in &test_values {
            let mut rgb = [val, val, val];
            let orig = rgb;

            lin_to_log(&mut rgb);
            log_to_lin(&mut rgb);

            for i in 0..3 {
                // Use relative tolerance for larger values
                let rel_diff = (orig[i] - rgb[i]).abs() / orig[i].max(0.001);
                assert!(
                    rel_diff < EPSILON,
                    "Roundtrip failed for {}: expected {}, got {} (rel_diff={})",
                    val,
                    orig[i],
                    rgb[i],
                    rel_diff
                );
            }
        }
    }

    #[test]
    fn test_below_breakpoint() {
        // Values below XBRK use linear formula
        let mut rgb = [0.001, 0.002, 0.003];
        lin_to_log(&mut rgb);

        // Should be: val * GAIN + OFFS
        let expected = 0.001 * constants::GAIN + constants::OFFS;
        assert!((rgb[0] - expected).abs() < EPSILON);
    }

    #[test]
    fn test_above_breakpoint() {
        // Values above XBRK use log formula
        let mut rgb = [0.18, 0.18, 0.18];
        lin_to_log(&mut rgb);

        // 18% grey should map to approximately 0.0 in log space
        // log2((0.18 + SHIFT) * M) ≈ log2(1) = 0
        assert!(rgb[0].abs() < 0.1, "18% grey should be near 0 in log space, got {}", rgb[0]);
    }

    #[test]
    fn test_monotonic() {
        let vals = [0.001, 0.01, 0.05, 0.1, 0.18, 0.5, 1.0, 2.0];
        let mut prev = f32::NEG_INFINITY;

        for &val in &vals {
            let mut rgb = [val, val, val];
            lin_to_log(&mut rgb);
            assert!(
                rgb[0] > prev,
                "lin_to_log not monotonic at {}: {} > {}",
                val,
                prev,
                rgb[0]
            );
            prev = rgb[0];
        }
    }
}
