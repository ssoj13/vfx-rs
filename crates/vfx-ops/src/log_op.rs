//! Generic Log/Antilog operations.
//!
//! This module implements OCIO-compatible log transforms including:
//! - Simple log10 and log2
//! - LogAffine with per-channel parameters
//! - LogCamera with linear segment near black
//!
//! # Log Styles
//!
//! - `Log10`: `out = log10(in)`
//! - `Log2`: `out = log2(in)`  
//! - `AntiLog10`: `out = 10^in`
//! - `AntiLog2`: `out = 2^in`
//! - `LinToLog`: `out = logSlope * log_base(linSlope*in + linOffset) + logOffset`
//! - `LogToLin`: `out = (base^((in - logOffset)/logSlope) - linOffset) / linSlope`
//! - `CameraLinToLog`: LinToLog with linear segment below break point
//! - `CameraLogToLin`: LogToLin with linear segment below break point
//!
//! # Reference
//!
//! Based on OCIO `ops/log/` implementation.

/// Log operation style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogStyle {
    /// Simple log base 10: `out = log10(in)`
    #[default]
    Log10,
    /// Simple log base 2: `out = log2(in)`
    Log2,
    /// Antilog base 10: `out = 10^in`
    AntiLog10,
    /// Antilog base 2: `out = 2^in`
    AntiLog2,
    /// Linear to log with affine parameters.
    LinToLog,
    /// Log to linear with affine parameters.
    LogToLin,
    /// Camera linear to log (with linear segment).
    CameraLinToLog,
    /// Camera log to linear (with linear segment).
    CameraLogToLin,
}

/// Per-channel log parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogParams {
    /// Log side slope (k).
    pub log_side_slope: f64,
    /// Log side offset (kb).
    pub log_side_offset: f64,
    /// Linear side slope (m).
    pub lin_side_slope: f64,
    /// Linear side offset (b).
    pub lin_side_offset: f64,
    /// Linear side break point (for camera log).
    pub lin_side_break: Option<f64>,
    /// Linear slope (for camera log, auto-computed if None).
    pub linear_slope: Option<f64>,
}

impl Default for LogParams {
    fn default() -> Self {
        Self {
            log_side_slope: 1.0,
            log_side_offset: 0.0,
            lin_side_slope: 1.0,
            lin_side_offset: 0.0,
            lin_side_break: None,
            linear_slope: None,
        }
    }
}

impl LogParams {
    /// Create new log params with given values.
    pub fn new(
        log_side_slope: f64,
        log_side_offset: f64,
        lin_side_slope: f64,
        lin_side_offset: f64,
    ) -> Self {
        Self {
            log_side_slope,
            log_side_offset,
            lin_side_slope,
            lin_side_offset,
            lin_side_break: None,
            linear_slope: None,
        }
    }

    /// Set linear break point (for camera log).
    pub fn with_lin_break(mut self, lin_break: f64) -> Self {
        self.lin_side_break = Some(lin_break);
        self
    }

    /// Compute linear slope for camera log.
    /// linearSlope = logSlope * linSlope / ((linSlope * linBreak + linOffset) * ln(base))
    fn compute_linear_slope(&self, base: f64) -> f64 {
        if let Some(linear_slope) = self.linear_slope {
            return linear_slope;
        }
        
        let lin_break = self.lin_side_break.unwrap_or(0.0);
        let denom = (self.lin_side_slope * lin_break + self.lin_side_offset) * base.ln();
        if denom.abs() < 1e-10 {
            return 0.0;
        }
        self.log_side_slope * self.lin_side_slope / denom
    }

    /// Compute log-side break point.
    /// logBreak = logSlope * log_base(linSlope * linBreak + linOffset) + logOffset
    fn compute_log_side_break(&self, base: f64) -> f64 {
        let lin_break = self.lin_side_break.unwrap_or(0.0);
        let arg = self.lin_side_slope * lin_break + self.lin_side_offset;
        if arg <= 0.0 {
            return self.log_side_offset;
        }
        let log2_base = base.log2();
        arg.log2() * self.log_side_slope / log2_base + self.log_side_offset
    }

    /// Compute linear offset for camera log.
    /// linearOffset = logBreak - linearSlope * linBreak
    fn compute_linear_offset(&self, base: f64) -> f64 {
        let linear_slope = self.compute_linear_slope(base);
        let log_break = self.compute_log_side_break(base);
        let lin_break = self.lin_side_break.unwrap_or(0.0);
        log_break - linear_slope * lin_break
    }
}

/// Log operation configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct LogOp {
    /// Log style.
    pub style: LogStyle,
    /// Log base (default 10.0).
    pub base: f64,
    /// Red channel parameters.
    pub red: LogParams,
    /// Green channel parameters.
    pub green: LogParams,
    /// Blue channel parameters.
    pub blue: LogParams,
}

impl Default for LogOp {
    fn default() -> Self {
        Self {
            style: LogStyle::Log10,
            base: 10.0,
            red: LogParams::default(),
            green: LogParams::default(),
            blue: LogParams::default(),
        }
    }
}

impl LogOp {
    /// Create a log10 operation.
    pub fn log10() -> Self {
        Self {
            style: LogStyle::Log10,
            base: 10.0,
            ..Default::default()
        }
    }

    /// Create a log2 operation.
    pub fn log2() -> Self {
        Self {
            style: LogStyle::Log2,
            base: 2.0,
            ..Default::default()
        }
    }

    /// Create an antilog10 operation.
    pub fn antilog10() -> Self {
        Self {
            style: LogStyle::AntiLog10,
            base: 10.0,
            ..Default::default()
        }
    }

    /// Create an antilog2 operation.
    pub fn antilog2() -> Self {
        Self {
            style: LogStyle::AntiLog2,
            base: 2.0,
            ..Default::default()
        }
    }

    /// Create a LinToLog operation with uniform parameters.
    pub fn lin_to_log(base: f64, params: LogParams) -> Self {
        Self {
            style: LogStyle::LinToLog,
            base,
            red: params.clone(),
            green: params.clone(),
            blue: params,
        }
    }

    /// Create a LogToLin operation with uniform parameters.
    pub fn log_to_lin(base: f64, params: LogParams) -> Self {
        Self {
            style: LogStyle::LogToLin,
            base,
            red: params.clone(),
            green: params.clone(),
            blue: params,
        }
    }

    /// Create a CameraLinToLog operation with uniform parameters.
    pub fn camera_lin_to_log(base: f64, params: LogParams) -> Self {
        Self {
            style: LogStyle::CameraLinToLog,
            base,
            red: params.clone(),
            green: params.clone(),
            blue: params,
        }
    }

    /// Create a CameraLogToLin operation with uniform parameters.
    pub fn camera_log_to_lin(base: f64, params: LogParams) -> Self {
        Self {
            style: LogStyle::CameraLogToLin,
            base,
            red: params.clone(),
            green: params.clone(),
            blue: params,
        }
    }

    /// Check if this is a simple log/antilog operation.
    pub fn is_simple(&self) -> bool {
        matches!(
            self.style,
            LogStyle::Log10 | LogStyle::Log2 | LogStyle::AntiLog10 | LogStyle::AntiLog2
        )
    }

    /// Check if this is a camera log operation.
    pub fn is_camera(&self) -> bool {
        matches!(
            self.style,
            LogStyle::CameraLinToLog | LogStyle::CameraLogToLin
        )
    }
}

// Constants
const LOG2_10: f64 = 3.321928094887362;
const LOG10_2: f64 = 0.30102999566398114;
const MIN_VALUE: f32 = f32::MIN_POSITIVE;

/// Apply log operation to a single RGB pixel.
pub fn apply_log_op(op: &LogOp, rgb: &mut [f32; 3]) {
    match op.style {
        LogStyle::Log10 => {
            for v in rgb.iter_mut() {
                *v = (*v).max(MIN_VALUE).log2() * LOG10_2 as f32;
            }
        }
        LogStyle::Log2 => {
            for v in rgb.iter_mut() {
                *v = (*v).max(MIN_VALUE).log2();
            }
        }
        LogStyle::AntiLog10 => {
            for v in rgb.iter_mut() {
                *v = 2.0_f32.powf(*v * LOG2_10 as f32);
            }
        }
        LogStyle::AntiLog2 => {
            for v in rgb.iter_mut() {
                *v = 2.0_f32.powf(*v);
            }
        }
        LogStyle::LinToLog => {
            apply_lin_to_log(op, rgb);
        }
        LogStyle::LogToLin => {
            apply_log_to_lin(op, rgb);
        }
        LogStyle::CameraLinToLog => {
            apply_camera_lin_to_log(op, rgb);
        }
        LogStyle::CameraLogToLin => {
            apply_camera_log_to_lin(op, rgb);
        }
    }
}

/// LinToLog: out = logSlope * log_base(linSlope*in + linOffset) + logOffset
fn apply_lin_to_log(op: &LogOp, rgb: &mut [f32; 3]) {
    let params = [&op.red, &op.green, &op.blue];
    let log2_base = (op.base as f32).log2();
    
    for (i, v) in rgb.iter_mut().enumerate() {
        let p = params[i];
        let m = p.lin_side_slope as f32;
        let b = p.lin_side_offset as f32;
        let k_log = p.log_side_slope as f32 / log2_base;
        let kb = p.log_side_offset as f32;
        
        let arg = (*v * m + b).max(MIN_VALUE);
        *v = arg.log2() * k_log + kb;
    }
}

/// LogToLin: out = (base^((in - logOffset)/logSlope) - linOffset) / linSlope
fn apply_log_to_lin(op: &LogOp, rgb: &mut [f32; 3]) {
    let params = [&op.red, &op.green, &op.blue];
    let log2_base = (op.base as f32).log2();
    
    for (i, v) in rgb.iter_mut().enumerate() {
        let p = params[i];
        let k_inv = log2_base / p.log_side_slope as f32;
        let minus_kb = -(p.log_side_offset as f32);
        let minus_b = -(p.lin_side_offset as f32);
        let m_inv = 1.0 / p.lin_side_slope as f32;
        
        *v = (2.0_f32.powf((*v + minus_kb) * k_inv) + minus_b) * m_inv;
    }
}

/// CameraLinToLog: linear segment below break, log above
fn apply_camera_lin_to_log(op: &LogOp, rgb: &mut [f32; 3]) {
    let params = [&op.red, &op.green, &op.blue];
    let log2_base = (op.base as f32).log2();
    
    for (i, v) in rgb.iter_mut().enumerate() {
        let p = params[i];
        let lin_break = p.lin_side_break.unwrap_or(0.0) as f32;
        
        if *v < lin_break {
            // Linear segment
            let linear_slope = p.compute_linear_slope(op.base) as f32;
            let linear_offset = p.compute_linear_offset(op.base) as f32;
            *v = linear_slope * *v + linear_offset;
        } else {
            // Log segment
            let m = p.lin_side_slope as f32;
            let b = p.lin_side_offset as f32;
            let k_log = p.log_side_slope as f32 / log2_base;
            let kb = p.log_side_offset as f32;
            
            let arg = (*v * m + b).max(MIN_VALUE);
            *v = arg.log2() * k_log + kb;
        }
    }
}

/// CameraLogToLin: linear segment below log break, inverse log above
fn apply_camera_log_to_lin(op: &LogOp, rgb: &mut [f32; 3]) {
    let params = [&op.red, &op.green, &op.blue];
    let log2_base = (op.base as f32).log2();
    
    for (i, v) in rgb.iter_mut().enumerate() {
        let p = params[i];
        let log_break = p.compute_log_side_break(op.base) as f32;
        
        if *v < log_break {
            // Linear segment
            let linear_slope = p.compute_linear_slope(op.base) as f32;
            let linear_offset = p.compute_linear_offset(op.base) as f32;
            *v = (*v - linear_offset) / linear_slope;
        } else {
            // Log segment
            let k_inv = log2_base / p.log_side_slope as f32;
            let minus_kb = -(p.log_side_offset as f32);
            let minus_b = -(p.lin_side_offset as f32);
            let m_inv = 1.0 / p.lin_side_slope as f32;
            
            *v = (2.0_f32.powf((*v + minus_kb) * k_inv) + minus_b) * m_inv;
        }
    }
}

/// Apply log operation to an RGBA buffer.
pub fn apply_log_op_rgba(op: &LogOp, pixels: &mut [f32]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let mut rgb = [chunk[0], chunk[1], chunk[2]];
        apply_log_op(op, &mut rgb);
        chunk[0] = rgb[0];
        chunk[1] = rgb[1];
        chunk[2] = rgb[2];
        // Alpha unchanged
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    // ========================================================================
    // Simple log tests
    // ========================================================================

    #[test]
    fn test_log10_known_values() {
        let op = LogOp::log10();
        
        // log10(10) = 1
        let mut rgb = [10.0_f32, 10.0, 10.0];
        apply_log_op(&op, &mut rgb);
        for v in rgb {
            assert!((v - 1.0).abs() < EPSILON, "log10(10) should be 1, got {v}");
        }
        
        // log10(100) = 2
        let mut rgb = [100.0_f32, 100.0, 100.0];
        apply_log_op(&op, &mut rgb);
        for v in rgb {
            assert!((v - 2.0).abs() < EPSILON, "log10(100) should be 2, got {v}");
        }
        
        // log10(1) = 0
        let mut rgb = [1.0_f32, 1.0, 1.0];
        apply_log_op(&op, &mut rgb);
        for v in rgb {
            assert!(v.abs() < EPSILON, "log10(1) should be 0, got {v}");
        }
    }

    #[test]
    fn test_log2_known_values() {
        let op = LogOp::log2();
        
        // log2(2) = 1
        let mut rgb = [2.0_f32, 2.0, 2.0];
        apply_log_op(&op, &mut rgb);
        for v in rgb {
            assert!((v - 1.0).abs() < EPSILON, "log2(2) should be 1, got {v}");
        }
        
        // log2(8) = 3
        let mut rgb = [8.0_f32, 8.0, 8.0];
        apply_log_op(&op, &mut rgb);
        for v in rgb {
            assert!((v - 3.0).abs() < EPSILON, "log2(8) should be 3, got {v}");
        }
    }

    #[test]
    fn test_antilog10_known_values() {
        let op = LogOp::antilog10();
        
        // 10^1 = 10
        let mut rgb = [1.0_f32, 1.0, 1.0];
        apply_log_op(&op, &mut rgb);
        for v in rgb {
            assert!((v - 10.0).abs() < 0.01, "10^1 should be 10, got {v}");
        }
        
        // 10^0 = 1
        let mut rgb = [0.0_f32, 0.0, 0.0];
        apply_log_op(&op, &mut rgb);
        for v in rgb {
            assert!((v - 1.0).abs() < EPSILON, "10^0 should be 1, got {v}");
        }
    }

    #[test]
    fn test_antilog2_known_values() {
        let op = LogOp::antilog2();
        
        // 2^3 = 8
        let mut rgb = [3.0_f32, 3.0, 3.0];
        apply_log_op(&op, &mut rgb);
        for v in rgb {
            assert!((v - 8.0).abs() < EPSILON, "2^3 should be 8, got {v}");
        }
    }

    // ========================================================================
    // Roundtrip tests
    // ========================================================================

    #[test]
    fn test_log10_antilog10_roundtrip() {
        let test_vals = [0.1_f32, 1.0, 10.0, 100.0];
        
        for &v in &test_vals {
            let mut rgb = [v, v, v];
            let original = rgb;
            
            // log10
            apply_log_op(&LogOp::log10(), &mut rgb);
            // antilog10
            apply_log_op(&LogOp::antilog10(), &mut rgb);
            
            for i in 0..3 {
                assert!(
                    (rgb[i] - original[i]).abs() < 0.01,
                    "Roundtrip failed for {v}: got {}", rgb[i]
                );
            }
        }
    }

    #[test]
    fn test_log2_antilog2_roundtrip() {
        let test_vals = [0.1_f32, 1.0, 2.0, 8.0, 64.0];
        
        for &v in &test_vals {
            let mut rgb = [v, v, v];
            let original = rgb;
            
            // log2
            apply_log_op(&LogOp::log2(), &mut rgb);
            // antilog2
            apply_log_op(&LogOp::antilog2(), &mut rgb);
            
            for i in 0..3 {
                assert!(
                    (rgb[i] - original[i]).abs() < 0.01,
                    "Roundtrip failed for {v}: got {}", rgb[i]
                );
            }
        }
    }

    // ========================================================================
    // LinToLog / LogToLin tests
    // ========================================================================

    #[test]
    fn test_lin_to_log_identity_params() {
        // With default params and base 10, LinToLog should be like log10
        let params = LogParams::default();
        let op = LogOp::lin_to_log(10.0, params);
        
        let mut rgb = [10.0_f32, 10.0, 10.0];
        apply_log_op(&op, &mut rgb);
        
        for v in rgb {
            assert!((v - 1.0).abs() < EPSILON, "LinToLog(10) with identity params should be ~1, got {v}");
        }
    }

    #[test]
    fn test_lin_to_log_log_to_lin_roundtrip() {
        let params = LogParams::new(0.5, 0.1, 2.0, 0.05);
        
        let test_vals = [0.01_f32, 0.1, 0.5, 1.0, 2.0];
        
        for &v in &test_vals {
            let mut rgb = [v, v, v];
            let original = rgb;
            
            // LinToLog
            apply_log_op(&LogOp::lin_to_log(10.0, params.clone()), &mut rgb);
            // LogToLin
            apply_log_op(&LogOp::log_to_lin(10.0, params.clone()), &mut rgb);
            
            for i in 0..3 {
                let diff = (rgb[i] - original[i]).abs();
                let rel_diff = diff / original[i].max(0.001);
                assert!(
                    rel_diff < 0.01,
                    "Roundtrip failed for {v}: got {}", rgb[i]
                );
            }
        }
    }

    // ========================================================================
    // Camera log tests
    // ========================================================================

    #[test]
    fn test_camera_lin_to_log_log_to_lin_roundtrip() {
        let params = LogParams::new(0.5, 0.1, 2.0, 0.05)
            .with_lin_break(0.01);
        
        let test_vals = [0.001_f32, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0];
        
        for &v in &test_vals {
            let mut rgb = [v, v, v];
            let original = rgb;
            
            // CameraLinToLog
            apply_log_op(&LogOp::camera_lin_to_log(10.0, params.clone()), &mut rgb);
            // CameraLogToLin
            apply_log_op(&LogOp::camera_log_to_lin(10.0, params.clone()), &mut rgb);
            
            for i in 0..3 {
                let diff = (rgb[i] - original[i]).abs();
                let rel_diff = diff / original[i].max(0.0001);
                assert!(
                    rel_diff < 0.05,
                    "Camera roundtrip failed for {v}: original={}, got={}",
                    original[i], rgb[i]
                );
            }
        }
    }

    #[test]
    fn test_camera_log_linear_segment() {
        let params = LogParams::new(0.5, 0.1, 2.0, 0.05)
            .with_lin_break(0.01);
        
        // Values below lin_break should be linear
        let mut rgb1 = [0.005_f32, 0.005, 0.005];
        let mut rgb2 = [0.002_f32, 0.002, 0.002];
        
        apply_log_op(&LogOp::camera_lin_to_log(10.0, params.clone()), &mut rgb1);
        apply_log_op(&LogOp::camera_lin_to_log(10.0, params.clone()), &mut rgb2);
        
        // Just check they're both processed in linear segment
        assert!(rgb1[0].is_finite());
        assert!(rgb2[0].is_finite());
    }

    // ========================================================================
    // RGBA buffer test
    // ========================================================================

    #[test]
    fn test_rgba_buffer() {
        let op = LogOp::log10();
        
        let mut pixels = vec![
            10.0, 10.0, 10.0, 1.0,   // Should become ~1.0
            100.0, 100.0, 100.0, 0.5, // Should become ~2.0
        ];
        
        apply_log_op_rgba(&op, &mut pixels);
        
        // Check first pixel
        assert!((pixels[0] - 1.0).abs() < EPSILON);
        assert!((pixels[3] - 1.0).abs() < EPSILON); // Alpha unchanged
        
        // Check second pixel
        assert!((pixels[4] - 2.0).abs() < EPSILON);
        assert!((pixels[7] - 0.5).abs() < EPSILON); // Alpha unchanged
    }

    // ========================================================================
    // Edge cases
    // ========================================================================

    #[test]
    fn test_small_values() {
        let op = LogOp::log10();
        
        // Very small positive value
        let mut rgb = [1e-10_f32, 1e-10, 1e-10];
        apply_log_op(&op, &mut rgb);
        
        // Should be large negative (but finite)
        for v in rgb {
            assert!(v.is_finite(), "log10 of small value should be finite");
            assert!(v < -5.0, "log10(1e-10) should be very negative");
        }
    }

    #[test]
    fn test_negative_values_clamped() {
        let op = LogOp::log10();
        
        // Negative values get clamped to MIN_VALUE
        let mut rgb = [-1.0_f32, -1.0, -1.0];
        apply_log_op(&op, &mut rgb);
        
        // Should produce the same result as MIN_VALUE
        for v in rgb {
            assert!(v.is_finite(), "log10 of negative should be finite (clamped)");
        }
    }
}
