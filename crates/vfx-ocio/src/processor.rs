//! Transform processor for applying color conversions.
//!
//! The processor compiles a chain of transforms into an optimized
//! pipeline for efficient pixel processing.
//!
//! # Example
//!
//! ```ignore
//! use vfx_ocio::{Config, Processor};
//!
//! let config = Config::from_file("config.ocio")?;
//! let processor = config.processor("ACEScg", "sRGB")?;
//!
//! // Process pixels
//! let mut pixels = [[0.18_f32, 0.18, 0.18]; 100];
//! processor.apply(&mut pixels);
//! ```

use crate::error::OcioResult;
use crate::transform::*;

/// Applies a transfer function to a single value.
#[allow(clippy::excessive_precision)] // Standard color science constants
fn apply_transfer(v: f32, style: TransferStyle, forward: bool) -> f32 {
    match style {
        TransferStyle::Linear => v,
        
        TransferStyle::Srgb => {
            if forward {
                // Linear to sRGB (OETF)
                if v <= 0.0031308 {
                    v * 12.92
                } else {
                    1.055 * v.powf(1.0 / 2.4) - 0.055
                }
            } else {
                // sRGB to linear (EOTF)
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            }
        }
        
        TransferStyle::Rec709 => {
            if forward {
                // Linear to Rec.709
                if v < 0.018 {
                    v * 4.5
                } else {
                    1.099 * v.powf(0.45) - 0.099
                }
            } else {
                // Rec.709 to linear
                if v < 0.081 {
                    v / 4.5
                } else {
                    ((v + 0.099) / 1.099).powf(1.0 / 0.45)
                }
            }
        }
        
        TransferStyle::Rec2020 => {
            // Rec.2020 uses same formula as Rec.709 but with different constants
            // For 12-bit: alpha = 1.0993, beta = 0.0181
            const ALPHA: f32 = 1.09929682680944;
            const BETA: f32 = 0.018053968510807;
            
            if forward {
                // Linear to Rec.2020
                if v < BETA {
                    v * 4.5
                } else {
                    ALPHA * v.powf(0.45) - (ALPHA - 1.0)
                }
            } else {
                // Rec.2020 to linear
                if v < BETA * 4.5 {
                    v / 4.5
                } else {
                    ((v + (ALPHA - 1.0)) / ALPHA).powf(1.0 / 0.45)
                }
            }
        }
        
        TransferStyle::Gamma22 => {
            if forward { v.max(0.0).powf(1.0 / 2.2) } else { v.max(0.0).powf(2.2) }
        }
        
        TransferStyle::Gamma24 => {
            if forward { v.max(0.0).powf(1.0 / 2.4) } else { v.max(0.0).powf(2.4) }
        }
        
        TransferStyle::Gamma26 => {
            if forward { v.max(0.0).powf(1.0 / 2.6) } else { v.max(0.0).powf(2.6) }
        }
        
        TransferStyle::Pq => {
            // PQ (ST.2084) constants
            const M1: f32 = 0.1593017578125;
            const M2: f32 = 78.84375;
            const C1: f32 = 0.8359375;
            const C2: f32 = 18.8515625;
            const C3: f32 = 18.6875;
            
            if forward {
                // Linear to PQ
                let y = (v / 10000.0).max(0.0).powf(M1);
                ((C1 + C2 * y) / (1.0 + C3 * y)).powf(M2)
            } else {
                // PQ to linear
                let vp = v.max(0.0).powf(1.0 / M2);
                let n = (vp - C1).max(0.0);
                let d = C2 - C3 * vp;
                10000.0 * (n / d.max(1e-10)).powf(1.0 / M1)
            }
        }
        
        TransferStyle::Hlg => {
            const A: f32 = 0.17883277;
            const B: f32 = 0.28466892;
            const C: f32 = 0.55991073;
            
            if forward {
                // Linear to HLG
                if v <= 1.0 / 12.0 {
                    (3.0 * v).sqrt()
                } else {
                    A * (12.0 * v - B).ln() + C
                }
            } else {
                // HLG to linear
                if v <= 0.5 {
                    v * v / 3.0
                } else {
                    (((v - C) / A).exp() + B) / 12.0
                }
            }
        }
        
        TransferStyle::AcesCct => {
            const CUT: f32 = 0.0078125;
            const A: f32 = 10.5402377416545;
            const B: f32 = 0.0729055341958355;
            
            if forward {
                // Linear to ACEScct
                if v <= CUT {
                    A * v + B
                } else {
                    (v.log2() + 9.72) / 17.52
                }
            } else {
                // ACEScct to linear
                if v <= 0.155251141552511 {
                    (v - B) / A
                } else {
                    2.0_f32.powf(v * 17.52 - 9.72)
                }
            }
        }
        
        TransferStyle::AcesCc => {
            if forward {
                // Linear to ACEScc
                if v <= 0.0 {
                    -0.3584474886
                } else if v < 2.0_f32.powf(-15.0) {
                    (2.0_f32.powf(-16.0) + v * 0.5).log2() / 17.52 + 9.72 / 17.52
                } else {
                    v.log2() / 17.52 + 9.72 / 17.52
                }
            } else {
                // ACEScc to linear
                if v <= -0.3013698630 {
                    (2.0_f32.powf(v * 17.52 - 9.72) - 2.0_f32.powf(-16.0)) * 2.0
                } else {
                    2.0_f32.powf(v * 17.52 - 9.72)
                }
            }
        }
        
        TransferStyle::LogC3 => {
            // ARRI LogC3 (EI 800)
            const CUT: f32 = 0.010591;
            const A: f32 = 5.555556;
            const B: f32 = 0.052272;
            const C: f32 = 0.247190;
            const D: f32 = 0.385537;
            const E: f32 = 5.367655;
            const F: f32 = 0.092809;
            
            if forward {
                if v > CUT {
                    C * (A * v + B).log10() + D
                } else {
                    E * v + F
                }
            } else if v > E * CUT + F {
                (10.0_f32.powf((v - D) / C) - B) / A
            } else {
                (v - F) / E
            }
        }
        
        TransferStyle::LogC4 => {
            // ARRI LogC4 - precomputed constants
            // A = (2^18 - 16) / 117.45 = 2231.82
            // B = (1023 - 95) / 1023 = 0.9071
            // C = 95 / 1023 = 0.0929
            // S = (7 * ln(2^18)) / 10 = 8.735
            const A: f32 = 2231.82;
            const B: f32 = 0.9071;
            const C: f32 = 0.0929;
            const S: f32 = 8.735;
            
            if forward {
                let t = (v * A).max(0.0) + 1.0;
                B * t.ln() / S + C
            } else {
                let t = ((v - C) * S / B).exp();
                (t - 1.0) / A
            }
        }
        
        TransferStyle::SLog3 => {
            if forward {
                if v >= 0.01125 {
                    (420.0 + (v * 261.5).log10() * 261.5) / 1023.0
                } else {
                    (v * 76.2102946929 + 95.0) / 1023.0
                }
            } else {
                let x = v * 1023.0;
                if x >= 171.2102946929 {
                    10.0_f32.powf((x - 420.0) / 261.5) / 261.5 * 0.18
                } else {
                    (x - 95.0) / 76.2102946929
                }
            }
        }
        
        TransferStyle::VLog => {
            const CUT_F: f32 = 0.01;
            const B: f32 = 0.00873;
            const C: f32 = 0.241514;
            const D: f32 = 0.598206;
            
            if forward {
                if v < CUT_F {
                    5.6 * v + 0.125
                } else {
                    C * (v + B).log10() + D
                }
            } else if v < 0.181 {
                (v - 0.125) / 5.6
            } else {
                10.0_f32.powf((v - D) / C) - B
            }
        }
        
        TransferStyle::Log3G10 => {
            // RED Log3G10
            const A: f32 = 0.224282;
            const B: f32 = 155.975327;
            const C: f32 = 0.01;
            #[allow(dead_code)]
            const G: f32 = 15.1927;
            
            if forward {
                let t = v.abs() * B + 1.0;
                v.signum() * A * t.log10() + C
            } else {
                let t = 10.0_f32.powf((v.abs() - C) / A);
                v.signum() * (t - 1.0) / B
            }
        }
        
        TransferStyle::BmdFilmGen5 => {
            // Blackmagic Film Gen 5 (simplified)
            const A: f32 = 0.09246575342;
            const B: f32 = 0.5300133392;
            const C: f32 = 0.1496994601;
            
            if forward {
                if v < 0.005 {
                    v * A + 0.09246575342
                } else {
                    B * (v + C).ln() + 0.5
                }
            } else if v < 0.09292915127 {
                (v - 0.09246575342) / A
            } else {
                ((v - 0.5) / B).exp() - C
            }
        }
    }
}

/// Interpolates a value from curve control points.
/// Uses linear interpolation between points.
fn interpolate_curve(points: &[[f64; 2]], x: f64) -> f64 {
    if points.is_empty() {
        return x;
    }
    if points.len() == 1 {
        return points[0][1];
    }

    // Find bracketing points
    let mut low_idx = 0;
    let mut high_idx = points.len() - 1;

    // Handle values outside the curve range
    if x <= points[0][0] {
        return points[0][1];
    }
    if x >= points[points.len() - 1][0] {
        return points[points.len() - 1][1];
    }

    // Binary search for bracket
    for (i, pt) in points.iter().enumerate() {
        if pt[0] <= x {
            low_idx = i;
        }
        if pt[0] >= x && i < high_idx {
            high_idx = i;
            break;
        }
    }

    // Linear interpolation
    let x0 = points[low_idx][0];
    let y0 = points[low_idx][1];
    let x1 = points[high_idx][0];
    let y1 = points[high_idx][1];

    if (x1 - x0).abs() < 1e-10 {
        return y0;
    }

    let t = (x - x0) / (x1 - x0);
    y0 + t * (y1 - y0)
}

/// Inverts a 1D LUT by building a reverse lookup table.
fn invert_lut1d(lut: &[f32], size: usize, channels: usize) -> Vec<f32> {
    let mut inverted = vec![0.0f32; size * channels];
    
    for c in 0..channels {
        // Build inverse mapping for this channel
        for i in 0..size {
            let t = i as f32 / (size - 1) as f32;
            
            // Find where this output value occurs in the original LUT
            // by binary search (assumes monotonic LUT)
            let target = t;
            let mut lo = 0usize;
            let mut hi = size - 1;
            
            while lo < hi {
                let mid = (lo + hi) / 2;
                let val = lut[mid * channels + c];
                if val < target {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }
            
            // Interpolate for better accuracy
            let idx = lo;
            let val_at_idx = lut[idx * channels + c];
            
            let result = if idx == 0 || (val_at_idx - target).abs() < 1e-6 {
                idx as f32 / (size - 1) as f32
            } else {
                let val_before = lut[(idx - 1) * channels + c];
                let t_interp = (target - val_before) / (val_at_idx - val_before);
                ((idx - 1) as f32 + t_interp) / (size - 1) as f32
            };
            
            inverted[i * channels + c] = result.clamp(0.0, 1.0);
        }
    }
    
    inverted
}

/// Inverts a 3D LUT using iterative Newton-Raphson method.
/// 
/// This builds a new 3D LUT where each output maps back to the original input.
/// Uses tetrahedral interpolation for forward evaluation during inversion.
fn invert_lut3d(lut: &[f32], size: usize, domain_min: [f32; 3], domain_max: [f32; 3]) -> Vec<f32> {
    let mut inverted = vec![0.0f32; size * size * size * 3];
    let max_iters = 30;
    let tolerance = 1e-6f32;
    
    // For each point in the output space, find the input that produces it
    for iz in 0..size {
        for iy in 0..size {
            for ix in 0..size {
                // Target output value (normalized 0-1)
                let target = [
                    ix as f32 / (size - 1) as f32,
                    iy as f32 / (size - 1) as f32,
                    iz as f32 / (size - 1) as f32,
                ];
                
                // Scale target to domain
                let target_scaled = [
                    domain_min[0] + target[0] * (domain_max[0] - domain_min[0]),
                    domain_min[1] + target[1] * (domain_max[1] - domain_min[1]),
                    domain_min[2] + target[2] * (domain_max[2] - domain_min[2]),
                ];
                
                // Initial guess: identity (start with the target itself)
                let mut guess = target;
                
                // Newton-Raphson iteration
                for _ in 0..max_iters {
                    // Evaluate LUT at current guess
                    let eval = eval_lut3d_tetrahedral(lut, size, &guess, domain_min, domain_max);
                    
                    // Compute error
                    let err = [
                        eval[0] - target_scaled[0],
                        eval[1] - target_scaled[1],
                        eval[2] - target_scaled[2],
                    ];
                    
                    let err_mag = (err[0]*err[0] + err[1]*err[1] + err[2]*err[2]).sqrt();
                    if err_mag < tolerance {
                        break;
                    }
                    
                    // Compute Jacobian numerically
                    let delta = 1e-4f32;
                    let mut jacobian = [[0.0f32; 3]; 3];
                    
                    for j in 0..3 {
                        let mut g_plus = guess;
                        g_plus[j] = (g_plus[j] + delta).min(1.0);
                        let eval_plus = eval_lut3d_tetrahedral(lut, size, &g_plus, domain_min, domain_max);
                        
                        for i in 0..3 {
                            jacobian[i][j] = (eval_plus[i] - eval[i]) / delta;
                        }
                    }
                    
                    // Solve J * dx = -err using Cramer's rule (3x3)
                    let dx = solve_3x3(&jacobian, &[-err[0], -err[1], -err[2]]);
                    
                    // Update guess with damping
                    let damping = 0.8f32;
                    guess[0] = (guess[0] + damping * dx[0]).clamp(0.0, 1.0);
                    guess[1] = (guess[1] + damping * dx[1]).clamp(0.0, 1.0);
                    guess[2] = (guess[2] + damping * dx[2]).clamp(0.0, 1.0);
                }
                
                // Store result (scale back to domain)
                let idx = (iz * size * size + iy * size + ix) * 3;
                inverted[idx] = domain_min[0] + guess[0] * (domain_max[0] - domain_min[0]);
                inverted[idx + 1] = domain_min[1] + guess[1] * (domain_max[1] - domain_min[1]);
                inverted[idx + 2] = domain_min[2] + guess[2] * (domain_max[2] - domain_min[2]);
            }
        }
    }
    
    inverted
}

/// Evaluate 3D LUT with tetrahedral interpolation.
fn eval_lut3d_tetrahedral(lut: &[f32], size: usize, rgb: &[f32; 3], 
                          domain_min: [f32; 3], domain_max: [f32; 3]) -> [f32; 3] {
    // Normalize to 0-1 range
    let r = ((rgb[0] - domain_min[0]) / (domain_max[0] - domain_min[0])).clamp(0.0, 1.0);
    let g = ((rgb[1] - domain_min[1]) / (domain_max[1] - domain_min[1])).clamp(0.0, 1.0);
    let b = ((rgb[2] - domain_min[2]) / (domain_max[2] - domain_min[2])).clamp(0.0, 1.0);
    
    // Scale to LUT indices
    let max_idx = (size - 1) as f32;
    let ri = r * max_idx;
    let gi = g * max_idx;
    let bi = b * max_idx;
    
    let r0 = (ri.floor() as usize).min(size - 2);
    let g0 = (gi.floor() as usize).min(size - 2);
    let b0 = (bi.floor() as usize).min(size - 2);
    
    let fr = ri - r0 as f32;
    let fg = gi - g0 as f32;
    let fb = bi - b0 as f32;
    
    // Get 8 corners
    let idx = |r: usize, g: usize, b: usize| -> [f32; 3] {
        let i = (b * size * size + g * size + r) * 3;
        [lut[i], lut[i + 1], lut[i + 2]]
    };
    
    let c000 = idx(r0, g0, b0);
    let c100 = idx(r0 + 1, g0, b0);
    let c010 = idx(r0, g0 + 1, b0);
    let c110 = idx(r0 + 1, g0 + 1, b0);
    let c001 = idx(r0, g0, b0 + 1);
    let c101 = idx(r0 + 1, g0, b0 + 1);
    let c011 = idx(r0, g0 + 1, b0 + 1);
    let c111 = idx(r0 + 1, g0 + 1, b0 + 1);
    
    // Tetrahedral interpolation
    let mut result = [0.0f32; 3];
    
    for i in 0..3 {
        result[i] = if fr > fg {
            if fg > fb {
                // fr > fg > fb
                (1.0 - fr) * c000[i] + (fr - fg) * c100[i] + (fg - fb) * c110[i] + fb * c111[i]
            } else if fr > fb {
                // fr > fb > fg
                (1.0 - fr) * c000[i] + (fr - fb) * c100[i] + (fb - fg) * c101[i] + fg * c111[i]
            } else {
                // fb > fr > fg
                (1.0 - fb) * c000[i] + (fb - fr) * c001[i] + (fr - fg) * c101[i] + fg * c111[i]
            }
        } else if fr > fb {
            // fg > fr > fb
            (1.0 - fg) * c000[i] + (fg - fr) * c010[i] + (fr - fb) * c110[i] + fb * c111[i]
        } else if fg > fb {
            // fg > fb > fr
            (1.0 - fg) * c000[i] + (fg - fb) * c010[i] + (fb - fr) * c011[i] + fr * c111[i]
        } else {
            // fb > fg > fr
            (1.0 - fb) * c000[i] + (fb - fg) * c001[i] + (fg - fr) * c011[i] + fr * c111[i]
        };
    }
    
    result
}

/// Solve 3x3 linear system using Cramer's rule.
fn solve_3x3(a: &[[f32; 3]; 3], b: &[f32; 3]) -> [f32; 3] {
    let det = a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
            - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
            + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]);
    
    if det.abs() < 1e-10 {
        return [0.0, 0.0, 0.0]; // Singular matrix, return zero
    }
    
    let inv_det = 1.0 / det;
    
    // Replace columns with b and compute determinants
    let det_x = b[0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
              - a[0][1] * (b[1] * a[2][2] - a[1][2] * b[2])
              + a[0][2] * (b[1] * a[2][1] - a[1][1] * b[2]);
    
    let det_y = a[0][0] * (b[1] * a[2][2] - a[1][2] * b[2])
              - b[0] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
              + a[0][2] * (a[1][0] * b[2] - b[1] * a[2][0]);
    
    let det_z = a[0][0] * (a[1][1] * b[2] - b[1] * a[2][1])
              - a[0][1] * (a[1][0] * b[2] - b[1] * a[2][0])
              + b[0] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]);
    
    [det_x * inv_det, det_y * inv_det, det_z * inv_det]
}

/// Optimization level for processors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptimizationLevel {
    /// No optimization.
    None,
    /// Lossless optimization only (matrix combination, identity removal).
    #[default]
    Lossless,
    /// Good quality (may combine LUTs).
    Good,
    /// Best quality.
    Best,
    /// Draft quality (faster, less accurate).
    Draft,
}

/// Compiled transform processor.
///
/// The processor holds an optimized transform chain ready for pixel application.
#[derive(Debug)]
pub struct Processor {
    /// Compiled operation list.
    ops: Vec<ProcessorOp>,
    /// Input bit depth hint.
    input_bit_depth: BitDepth,
    /// Output bit depth hint.
    output_bit_depth: BitDepth,
    /// Has dynamic properties.
    #[allow(dead_code)]
    has_dynamic: bool,
}

/// Re-export BitDepth from vfx-core.
pub use vfx_core::BitDepth;

/// Internal operation type.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum ProcessorOp {
    /// 4x4 matrix + offset.
    Matrix {
        matrix: [f32; 16],
        offset: [f32; 4],
    },
    /// 1D LUT.
    Lut1d {
        lut: Vec<f32>,
        size: usize,
        channels: usize,
        domain: [f32; 2],
    },
    /// 3D LUT.
    Lut3d {
        lut: Vec<f32>,
        size: usize,
        interp: Interpolation,
        domain_min: [f32; 3],
        domain_max: [f32; 3],
    },
    /// Exponent.
    Exponent {
        value: [f32; 4],
        negative_style: NegativeStyle,
    },
    /// Log base conversion.
    Log {
        base: f32,
        forward: bool,
    },
    /// CDL.
    Cdl {
        slope: [f32; 3],
        offset: [f32; 3],
        power: [f32; 3],
        saturation: f32,
    },
    /// Range clamp/scale.
    Range {
        scale: f32,
        offset: f32,
        clamp_min: Option<f32>,
        clamp_max: Option<f32>,
    },
    /// Built-in transfer function.
    Transfer {
        style: TransferStyle,
        forward: bool,
    },
    /// Exposure/contrast adjustment.
    ExposureContrast {
        exposure: f32,
        contrast: f32,
        gamma: f32,
        pivot: f32,
        style: ExposureContrastStyle,
    },
    /// Fixed function (ACES-specific).
    FixedFunction {
        style: FixedFunctionStyle,
        params: Vec<f32>,
        forward: bool,
    },
    /// Allocation (log/linear).
    Allocation {
        allocation: AllocationType,
        vars: Vec<f32>,
        forward: bool,
    },
    /// Grading primary (lift/gamma/gain).
    GradingPrimary {
        lift: [f32; 3],
        gamma: [f32; 3],
        gain: [f32; 3],
        offset: f32,
        exposure: f32,
        contrast: f32,
        saturation: f32,
        pivot: f32,
        clamp_black: Option<f32>,
        clamp_white: Option<f32>,
    },
    /// Grading RGB curves.
    GradingRgbCurve {
        red_lut: Vec<f32>,
        green_lut: Vec<f32>,
        blue_lut: Vec<f32>,
        master_lut: Vec<f32>,
    },
    /// Grading tone (shadows/midtones/highlights).
    GradingTone {
        shadows: [f32; 4],
        midtones: [f32; 4],
        highlights: [f32; 4],
        whites: [f32; 4],
        blacks: [f32; 4],
        shadow_start: f32,
        shadow_pivot: f32,
        highlight_start: f32,
        highlight_pivot: f32,
    },
    /// LogAffine transform (OCIO v2).
    LogAffine {
        base: f32,
        log_side_slope: [f32; 3],
        log_side_offset: [f32; 3],
        lin_side_slope: [f32; 3],
        lin_side_offset: [f32; 3],
        forward: bool,
    },
    /// LogCamera transform (ARRI LogC, Sony S-Log3, etc).
    LogCamera {
        base: f32,
        log_side_slope: [f32; 3],
        log_side_offset: [f32; 3],
        lin_side_slope: [f32; 3],
        lin_side_offset: [f32; 3],
        lin_side_break: [f32; 3],
        linear_slope: [f32; 3],
        forward: bool,
    },
    /// Exponent with linear segment (sRGB, Rec.709 style).
    ExponentWithLinear {
        gamma: [f32; 4],
        offset: [f32; 4],
        negative_style: NegativeStyle,
        forward: bool,
    },
}

impl ProcessorOp {
    /// Returns true if this operation is an identity (no-op).
    pub fn is_identity(&self) -> bool {
        match self {
            ProcessorOp::Matrix { matrix, offset } => {
                // Identity matrix check
                let identity = [
                    1.0, 0.0, 0.0, 0.0,
                    0.0, 1.0, 0.0, 0.0,
                    0.0, 0.0, 1.0, 0.0,
                    0.0, 0.0, 0.0, 1.0,
                ];
                let is_identity_matrix = matrix.iter().zip(identity.iter())
                    .all(|(a, b)| (a - b).abs() < 1e-6);
                let is_zero_offset = offset.iter().all(|v| v.abs() < 1e-6);
                is_identity_matrix && is_zero_offset
            }
            ProcessorOp::Exponent { value, .. } => {
                value.iter().all(|v| (*v - 1.0).abs() < 1e-6)
            }
            ProcessorOp::Cdl { slope, offset, power, saturation } => {
                slope.iter().all(|v| (*v - 1.0).abs() < 1e-6)
                    && offset.iter().all(|v| v.abs() < 1e-6)
                    && power.iter().all(|v| (*v - 1.0).abs() < 1e-6)
                    && (*saturation - 1.0).abs() < 1e-6
            }
            ProcessorOp::Range { scale, offset, clamp_min, clamp_max } => {
                (*scale - 1.0).abs() < 1e-6
                    && offset.abs() < 1e-6
                    && clamp_min.is_none()
                    && clamp_max.is_none()
            }
            ProcessorOp::ExposureContrast { exposure, contrast, gamma, .. } => {
                exposure.abs() < 1e-6
                    && (*contrast - 1.0).abs() < 1e-6
                    && (*gamma - 1.0).abs() < 1e-6
            }
            ProcessorOp::GradingPrimary { lift, gamma, gain, offset, exposure, contrast, saturation, .. } => {
                lift.iter().all(|v| v.abs() < 1e-6)
                    && gamma.iter().all(|v| (*v - 1.0).abs() < 1e-6)
                    && gain.iter().all(|v| (*v - 1.0).abs() < 1e-6)
                    && offset.abs() < 1e-6
                    && exposure.abs() < 1e-6
                    && (*contrast - 1.0).abs() < 1e-6
                    && (*saturation - 1.0).abs() < 1e-6
            }
            // LUTs, Transfer, Log, etc. are generally not identity
            _ => false,
        }
    }
}

/// Built-in transfer function styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferStyle {
    /// sRGB OETF.
    Srgb,
    /// Rec.709 OETF.
    Rec709,
    /// Rec.2020 OETF.
    Rec2020,
    /// Gamma 2.2.
    Gamma22,
    /// Gamma 2.4.
    Gamma24,
    /// Gamma 2.6 (DCI).
    Gamma26,
    /// Linear (passthrough).
    Linear,
    /// PQ (SMPTE ST 2084).
    Pq,
    /// HLG (ARIB STD-B67).
    Hlg,
    /// ACEScct.
    AcesCct,
    /// ACEScc.
    AcesCc,
    /// Log3G10.
    Log3G10,
    /// LogC3 (ARRI).
    LogC3,
    /// LogC4 (ARRI).
    LogC4,
    /// S-Log3 (Sony).
    SLog3,
    /// V-Log (Panasonic).
    VLog,
    /// Log-C (Blackmagic).
    BmdFilmGen5,
}

impl Processor {
    /// Creates a new empty processor.
    pub(crate) fn new() -> Self {
        Self {
            ops: Vec::new(),
            input_bit_depth: BitDepth::Unknown,
            output_bit_depth: BitDepth::Unknown,
            has_dynamic: false,
        }
    }

    /// Creates a processor from a transform.
    pub fn from_transform(transform: &Transform, direction: TransformDirection) -> OcioResult<Self> {
        Self::from_transform_with_opts(transform, direction, OptimizationLevel::default())
    }

    /// Creates a processor from a transform with optimization level.
    pub fn from_transform_with_opts(
        transform: &Transform,
        direction: TransformDirection,
        optimization: OptimizationLevel,
    ) -> OcioResult<Self> {
        let mut processor = Self::new();
        processor.compile_transform(transform, direction)?;
        processor.optimize(optimization);
        Ok(processor)
    }

    /// Applies optimization to the operation chain.
    pub fn optimize(&mut self, level: OptimizationLevel) {
        if level == OptimizationLevel::None {
            return;
        }

        // Remove identity operations (lossless)
        self.ops.retain(|op| !op.is_identity());

        // Combine adjacent matrix operations (lossless)
        if matches!(level, OptimizationLevel::Lossless | OptimizationLevel::Good | OptimizationLevel::Best) {
            self.combine_matrices();
        }
    }

    /// Combines adjacent matrix operations into single matrix.
    fn combine_matrices(&mut self) {
        if self.ops.len() < 2 {
            return;
        }

        let mut result = Vec::with_capacity(self.ops.len());
        let mut pending_matrix: Option<([f32; 16], [f32; 4])> = None;

        for op in self.ops.drain(..) {
            if let ProcessorOp::Matrix { matrix, offset } = &op {
                if let Some((prev_m, prev_o)) = pending_matrix.take() {
                    // Combine: new_m * prev_m, new_m * prev_o + new_o
                    let combined_m = Self::mat4_mul(matrix, &prev_m);
                    let combined_o = Self::mat4_apply(matrix, &prev_o);
                    let combined_o = [
                        combined_o[0] + offset[0],
                        combined_o[1] + offset[1],
                        combined_o[2] + offset[2],
                        combined_o[3] + offset[3],
                    ];
                    pending_matrix = Some((combined_m, combined_o));
                } else {
                    pending_matrix = Some((*matrix, *offset));
                }
            } else {
                // Flush any pending matrix first
                if let Some((m, o)) = pending_matrix.take() {
                    result.push(ProcessorOp::Matrix { matrix: m, offset: o });
                }
                result.push(op);
            }
        }

        // Flush final pending matrix
        if let Some((m, o)) = pending_matrix {
            result.push(ProcessorOp::Matrix { matrix: m, offset: o });
        }

        self.ops = result;
    }

    /// 4x4 matrix multiply (row-major [f32; 16] layout)
    fn mat4_mul(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
        let mut r = [0.0; 16];
        for i in 0..4 {
            for j in 0..4 {
                r[i * 4 + j] = a[i * 4] * b[j]
                    + a[i * 4 + 1] * b[4 + j]
                    + a[i * 4 + 2] * b[8 + j]
                    + a[i * 4 + 3] * b[12 + j];
            }
        }
        r
    }

    /// Apply 4x4 matrix to 4-vector (row-major [f32; 16] layout)
    fn mat4_apply(m: &[f32; 16], v: &[f32; 4]) -> [f32; 4] {
        [
            m[0] * v[0] + m[1] * v[1] + m[2] * v[2] + m[3] * v[3],
            m[4] * v[0] + m[5] * v[1] + m[6] * v[2] + m[7] * v[3],
            m[8] * v[0] + m[9] * v[1] + m[10] * v[2] + m[11] * v[3],
            m[12] * v[0] + m[13] * v[1] + m[14] * v[2] + m[15] * v[3],
        ]
    }

    /// Compiles a transform into operations.
    fn compile_transform(&mut self, transform: &Transform, direction: TransformDirection) -> OcioResult<()> {
        match transform {
            Transform::Matrix(m) => {
                let dir = if direction == TransformDirection::Inverse {
                    m.direction.inverse()
                } else {
                    m.direction
                };
                
                let (matrix, offset) = if dir == TransformDirection::Inverse {
                    // Compute inverse using glam
                    let mat4 = glam::Mat4::from_cols_array(&[
                        m.matrix[0] as f32, m.matrix[4] as f32, m.matrix[8] as f32, m.matrix[12] as f32,
                        m.matrix[1] as f32, m.matrix[5] as f32, m.matrix[9] as f32, m.matrix[13] as f32,
                        m.matrix[2] as f32, m.matrix[6] as f32, m.matrix[10] as f32, m.matrix[14] as f32,
                        m.matrix[3] as f32, m.matrix[7] as f32, m.matrix[11] as f32, m.matrix[15] as f32,
                    ]);
                    let inv = mat4.inverse();
                    let inv_arr = inv.to_cols_array();
                    // Convert back to row-major
                    let inv_matrix = [
                        inv_arr[0], inv_arr[4], inv_arr[8], inv_arr[12],
                        inv_arr[1], inv_arr[5], inv_arr[9], inv_arr[13],
                        inv_arr[2], inv_arr[6], inv_arr[10], inv_arr[14],
                        inv_arr[3], inv_arr[7], inv_arr[11], inv_arr[15],
                    ];
                    // Invert offset: -inv(M) * offset
                    let inv_offset = [
                        -(inv_matrix[0] * m.offset[0] as f32 + inv_matrix[1] * m.offset[1] as f32 + inv_matrix[2] * m.offset[2] as f32 + inv_matrix[3] * m.offset[3] as f32),
                        -(inv_matrix[4] * m.offset[0] as f32 + inv_matrix[5] * m.offset[1] as f32 + inv_matrix[6] * m.offset[2] as f32 + inv_matrix[7] * m.offset[3] as f32),
                        -(inv_matrix[8] * m.offset[0] as f32 + inv_matrix[9] * m.offset[1] as f32 + inv_matrix[10] * m.offset[2] as f32 + inv_matrix[11] * m.offset[3] as f32),
                        -(inv_matrix[12] * m.offset[0] as f32 + inv_matrix[13] * m.offset[1] as f32 + inv_matrix[14] * m.offset[2] as f32 + inv_matrix[15] * m.offset[3] as f32),
                    ];
                    (inv_matrix, inv_offset)
                } else {
                    (m.matrix.map(|v| v as f32), m.offset.map(|v| v as f32))
                };
                
                self.ops.push(ProcessorOp::Matrix { matrix, offset });
            }

            Transform::Cdl(cdl) => {
                let dir = if direction == TransformDirection::Inverse {
                    cdl.direction.inverse()
                } else {
                    cdl.direction
                };

                if dir == TransformDirection::Forward {
                    self.ops.push(ProcessorOp::Cdl {
                        slope: cdl.slope.map(|v| v as f32),
                        offset: cdl.offset.map(|v| v as f32),
                        power: cdl.power.map(|v| v as f32),
                        saturation: cdl.saturation as f32,
                    });
                } else {
                    // Inverse CDL: reverse SOP order, invert values
                    let inv_slope = [
                        1.0 / cdl.slope[0] as f32,
                        1.0 / cdl.slope[1] as f32,
                        1.0 / cdl.slope[2] as f32,
                    ];
                    let inv_power = [
                        1.0 / cdl.power[0] as f32,
                        1.0 / cdl.power[1] as f32,
                        1.0 / cdl.power[2] as f32,
                    ];
                    let inv_offset = [
                        -cdl.offset[0] as f32 * inv_slope[0],
                        -cdl.offset[1] as f32 * inv_slope[1],
                        -cdl.offset[2] as f32 * inv_slope[2],
                    ];
                    let inv_sat = 1.0 / cdl.saturation as f32;
                    
                    self.ops.push(ProcessorOp::Cdl {
                        slope: inv_slope,
                        offset: inv_offset,
                        power: inv_power,
                        saturation: inv_sat,
                    });
                }
            }

            Transform::Exponent(exp) => {
                let dir = if direction == TransformDirection::Inverse {
                    exp.direction.inverse()
                } else {
                    exp.direction
                };

                let value = if dir == TransformDirection::Inverse {
                    [
                        1.0 / exp.value[0] as f32,
                        1.0 / exp.value[1] as f32,
                        1.0 / exp.value[2] as f32,
                        1.0 / exp.value[3] as f32,
                    ]
                } else {
                    exp.value.map(|v| v as f32)
                };

                self.ops.push(ProcessorOp::Exponent {
                    value,
                    negative_style: exp.negative_style,
                });
            }

            Transform::Log(log) => {
                let dir = if direction == TransformDirection::Inverse {
                    log.direction.inverse()
                } else {
                    log.direction
                };

                self.ops.push(ProcessorOp::Log {
                    base: log.base as f32,
                    forward: dir == TransformDirection::Forward,
                });
            }

            Transform::Range(r) => {
                let dir = if direction == TransformDirection::Inverse {
                    r.direction.inverse()
                } else {
                    r.direction
                };

                let (scale, offset, clamp_min, clamp_max) = if let (Some(min_in), Some(max_in), Some(min_out), Some(max_out)) =
                    (r.min_in, r.max_in, r.min_out, r.max_out)
                {
                    let (scale, offset, clamp_min, clamp_max) = if dir == TransformDirection::Forward {
                        let scale = (max_out - min_out) / (max_in - min_in);
                        let offset = min_out - min_in * scale;
                        (scale as f32, offset as f32, r.min_out.map(|v| v as f32), r.max_out.map(|v| v as f32))
                    } else {
                        let scale = (max_in - min_in) / (max_out - min_out);
                        let offset = min_in - min_out * scale;
                        (scale as f32, offset as f32, r.min_in.map(|v| v as f32), r.max_in.map(|v| v as f32))
                    };

                    if r.style == RangeStyle::NoClamp {
                        (scale, offset, None, None)
                    } else {
                        (scale, offset, clamp_min, clamp_max)
                    }
                } else {
                    (1.0, 0.0, None, None)
                };

                self.ops.push(ProcessorOp::Range {
                    scale,
                    offset,
                    clamp_min,
                    clamp_max,
                });
            }

            Transform::Group(g) => {
                let dir = if direction == TransformDirection::Inverse {
                    g.direction.inverse()
                } else {
                    g.direction
                };

                if dir == TransformDirection::Forward {
                    for t in &g.transforms {
                        self.compile_transform(t, TransformDirection::Forward)?;
                    }
                } else {
                    for t in g.transforms.iter().rev() {
                        self.compile_transform(t, TransformDirection::Inverse)?;
                    }
                }
            }

            Transform::FileTransform(ft) => {
                let dir = if direction == TransformDirection::Inverse {
                    ft.direction.inverse()
                } else {
                    ft.direction
                };
                let forward = dir == TransformDirection::Forward;
                
                // Load LUT based on file extension
                let path = &ft.src;
                let ext = path.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase())
                    .unwrap_or_default();
                
                match ext.as_str() {
                    "cube" => {
                        if let Ok(lut) = vfx_lut::cube::read_3d(path) {
                            self.compile_lut3d(&lut, ft.interpolation, forward);
                        } else {
                            let lut = vfx_lut::cube::read_1d(path)?;
                            self.compile_lut1d(&lut, forward);
                        }
                    }
                    "spi1d" => {
                        let lut = vfx_lut::read_spi1d(path)?;
                        self.compile_lut1d(&lut, forward);
                    }
                    "spi3d" => {
                        let lut = vfx_lut::read_spi3d(path)?;
                        self.compile_lut3d(&lut, ft.interpolation, forward);
                    }
                    "clf" | "ctf" => {
                        let pl = vfx_lut::read_clf(path)?;
                        self.compile_clf(&pl, forward)?;
                    }
                    _ => {
                        // Unsupported format - skip with warning
                        // In production, this could try other loaders
                    }
                }
            }

            Transform::BuiltinTransfer(bt) => {
                let dir = if direction == TransformDirection::Inverse {
                    bt.direction.inverse()
                } else {
                    bt.direction
                };

                let style = match bt.style.to_lowercase().as_str() {
                    "srgb" | "srgb_texture" => TransferStyle::Srgb,
                    "rec709" | "bt709" | "rec.709" => TransferStyle::Rec709,
                    "rec2020" | "bt2020" | "rec.2020" => TransferStyle::Rec2020,
                    "gamma22" | "gamma_2.2" => TransferStyle::Gamma22,
                    "gamma24" | "gamma_2.4" => TransferStyle::Gamma24,
                    "gamma26" | "gamma_2.6" | "dci" => TransferStyle::Gamma26,
                    "linear" => TransferStyle::Linear,
                    "pq" | "st2084" | "smpte2084" => TransferStyle::Pq,
                    "hlg" | "arib_std_b67" => TransferStyle::Hlg,
                    "acescct" => TransferStyle::AcesCct,
                    "acescc" => TransferStyle::AcesCc,
                    "log3g10" | "redlog3g10" => TransferStyle::Log3G10,
                    "logc" | "logc3" | "arri_logc3" => TransferStyle::LogC3,
                    "logc4" | "arri_logc4" => TransferStyle::LogC4,
                    "slog3" | "sony_slog3" => TransferStyle::SLog3,
                    "vlog" | "panasonic_vlog" => TransferStyle::VLog,
                    "bmdfilmgen5" | "blackmagic" => TransferStyle::BmdFilmGen5,
                    _ => TransferStyle::Linear,
                };

                self.ops.push(ProcessorOp::Transfer {
                    style,
                    forward: dir == TransformDirection::Forward,
                });
            }

            Transform::ExposureContrast(ec) => {
                let dir = if direction == TransformDirection::Inverse {
                    ec.direction.inverse()
                } else {
                    ec.direction
                };

                // For inverse, we need to invert the values
                let (exposure, contrast, gamma) = if dir == TransformDirection::Inverse {
                    (-ec.exposure as f32, 1.0 / ec.contrast as f32, 1.0 / ec.gamma as f32)
                } else {
                    (ec.exposure as f32, ec.contrast as f32, ec.gamma as f32)
                };

                self.ops.push(ProcessorOp::ExposureContrast {
                    exposure,
                    contrast,
                    gamma,
                    pivot: ec.pivot as f32,
                    style: ec.style,
                });
            }

            Transform::FixedFunction(ff) => {
                let dir = if direction == TransformDirection::Inverse {
                    ff.direction.inverse()
                } else {
                    ff.direction
                };

                self.ops.push(ProcessorOp::FixedFunction {
                    style: ff.style,
                    params: ff.params.iter().map(|&v| v as f32).collect(),
                    forward: dir == TransformDirection::Forward,
                });
            }

            Transform::Allocation(alloc) => {
                let dir = if direction == TransformDirection::Inverse {
                    alloc.direction.inverse()
                } else {
                    alloc.direction
                };

                self.ops.push(ProcessorOp::Allocation {
                    allocation: alloc.allocation,
                    vars: alloc.vars.iter().map(|&v| v as f32).collect(),
                    forward: dir == TransformDirection::Forward,
                });
            }

            Transform::GradingPrimary(gp) => {
                let dir = if direction == TransformDirection::Inverse {
                    gp.direction.inverse()
                } else {
                    gp.direction
                };

                // For inverse, we compute inverse values
                let (lift, gamma, gain, offset, exposure, contrast, saturation) = 
                    if dir == TransformDirection::Inverse {
                        (
                            [-gp.lift[0] as f32, -gp.lift[1] as f32, -gp.lift[2] as f32],
                            [1.0 / gp.gamma[0] as f32, 1.0 / gp.gamma[1] as f32, 1.0 / gp.gamma[2] as f32],
                            [1.0 / gp.gain[0] as f32, 1.0 / gp.gain[1] as f32, 1.0 / gp.gain[2] as f32],
                            -gp.offset as f32,
                            -gp.exposure as f32,
                            1.0 / gp.contrast as f32,
                            1.0 / gp.saturation as f32,
                        )
                    } else {
                        (
                            gp.lift.map(|v| v as f32),
                            gp.gamma.map(|v| v as f32),
                            gp.gain.map(|v| v as f32),
                            gp.offset as f32,
                            gp.exposure as f32,
                            gp.contrast as f32,
                            gp.saturation as f32,
                        )
                    };

                self.ops.push(ProcessorOp::GradingPrimary {
                    lift,
                    gamma,
                    gain,
                    offset,
                    exposure,
                    contrast,
                    saturation,
                    pivot: gp.pivot as f32,
                    clamp_black: gp.clamp_black.map(|v| v as f32),
                    clamp_white: gp.clamp_white.map(|v| v as f32),
                });
            }

            Transform::GradingRgbCurve(gc) => {
                // Bake curves into 1D LUTs (1024 samples)
                let lut_size = 1024;
                let bake_curve = |pts: &[[f64; 2]]| -> Vec<f32> {
                    let mut lut = Vec::with_capacity(lut_size);
                    for i in 0..lut_size {
                        let x = i as f64 / (lut_size - 1) as f64;
                        let y = interpolate_curve(pts, x);
                        lut.push(y as f32);
                    }
                    lut
                };

                self.ops.push(ProcessorOp::GradingRgbCurve {
                    red_lut: bake_curve(&gc.red),
                    green_lut: bake_curve(&gc.green),
                    blue_lut: bake_curve(&gc.blue),
                    master_lut: bake_curve(&gc.master),
                });
            }

            Transform::GradingTone(gt) => {
                let dir = if direction == TransformDirection::Inverse {
                    gt.direction.inverse()
                } else {
                    gt.direction
                };

                // For inverse, invert multipliers
                let (shadows, midtones, highlights, whites, blacks) = 
                    if dir == TransformDirection::Inverse {
                        (
                            [1.0/gt.shadows[0] as f32, 1.0/gt.shadows[1] as f32, 1.0/gt.shadows[2] as f32, 1.0/gt.shadows[3] as f32],
                            [1.0/gt.midtones[0] as f32, 1.0/gt.midtones[1] as f32, 1.0/gt.midtones[2] as f32, 1.0/gt.midtones[3] as f32],
                            [1.0/gt.highlights[0] as f32, 1.0/gt.highlights[1] as f32, 1.0/gt.highlights[2] as f32, 1.0/gt.highlights[3] as f32],
                            [1.0/gt.whites[0] as f32, 1.0/gt.whites[1] as f32, 1.0/gt.whites[2] as f32, 1.0/gt.whites[3] as f32],
                            [-gt.blacks[0] as f32, -gt.blacks[1] as f32, -gt.blacks[2] as f32, -gt.blacks[3] as f32],
                        )
                    } else {
                        (
                            gt.shadows.map(|v| v as f32),
                            gt.midtones.map(|v| v as f32),
                            gt.highlights.map(|v| v as f32),
                            gt.whites.map(|v| v as f32),
                            gt.blacks.map(|v| v as f32),
                        )
                    };

                self.ops.push(ProcessorOp::GradingTone {
                    shadows,
                    midtones,
                    highlights,
                    whites,
                    blacks,
                    shadow_start: gt.shadow_start as f32,
                    shadow_pivot: gt.shadow_pivot as f32,
                    highlight_start: gt.highlight_start as f32,
                    highlight_pivot: gt.highlight_pivot as f32,
                });
            }

            Transform::LogAffine(la) => {
                let dir = if direction == TransformDirection::Inverse {
                    la.direction.inverse()
                } else {
                    la.direction
                };

                self.ops.push(ProcessorOp::LogAffine {
                    base: la.base as f32,
                    log_side_slope: la.log_side_slope.map(|v| v as f32),
                    log_side_offset: la.log_side_offset.map(|v| v as f32),
                    lin_side_slope: la.lin_side_slope.map(|v| v as f32),
                    lin_side_offset: la.lin_side_offset.map(|v| v as f32),
                    forward: dir == TransformDirection::Forward,
                });
            }

            Transform::LogCamera(lc) => {
                let dir = if direction == TransformDirection::Inverse {
                    lc.direction.inverse()
                } else {
                    lc.direction
                };

                // Calculate linear slope for continuity at break point
                // linear_slope = log_side_slope * lin_side_slope / (ln(base) * (lin_side_break * lin_side_slope + lin_side_offset))
                let linear_slope: [f32; 3] = lc.linear_slope.map(|arr| arr.map(|v| v as f32)).unwrap_or_else(|| {
                    let ln_base = (lc.base as f64).ln();
                    [
                        ((lc.log_side_slope[0] * lc.lin_side_slope[0]) / 
                            (ln_base * (lc.lin_side_break[0] * lc.lin_side_slope[0] + lc.lin_side_offset[0]))) as f32,
                        ((lc.log_side_slope[1] * lc.lin_side_slope[1]) / 
                            (ln_base * (lc.lin_side_break[1] * lc.lin_side_slope[1] + lc.lin_side_offset[1]))) as f32,
                        ((lc.log_side_slope[2] * lc.lin_side_slope[2]) / 
                            (ln_base * (lc.lin_side_break[2] * lc.lin_side_slope[2] + lc.lin_side_offset[2]))) as f32,
                    ]
                });

                self.ops.push(ProcessorOp::LogCamera {
                    base: lc.base as f32,
                    log_side_slope: lc.log_side_slope.map(|v| v as f32),
                    log_side_offset: lc.log_side_offset.map(|v| v as f32),
                    lin_side_slope: lc.lin_side_slope.map(|v| v as f32),
                    lin_side_offset: lc.lin_side_offset.map(|v| v as f32),
                    lin_side_break: lc.lin_side_break.map(|v| v as f32),
                    linear_slope,
                    forward: dir == TransformDirection::Forward,
                });
            }

            Transform::ExponentWithLinear(ewl) => {
                let dir = if direction == TransformDirection::Inverse {
                    ewl.direction.inverse()
                } else {
                    ewl.direction
                };

                self.ops.push(ProcessorOp::ExponentWithLinear {
                    gamma: ewl.gamma.map(|v| v as f32),
                    offset: ewl.offset.map(|v| v as f32),
                    negative_style: ewl.negative_style,
                    forward: dir == TransformDirection::Forward,
                });
            }

            Transform::Lut1D(lut) => {
                let dir = if direction == TransformDirection::Inverse {
                    lut.direction.inverse()
                } else {
                    lut.direction
                };
                let forward = dir == TransformDirection::Forward;
                
                // Convert inline LUT to vfx_lut format
                let vfx_lut = vfx_lut::Lut1D {
                    r: lut.red.clone(),
                    g: lut.green.clone(),
                    b: lut.blue.clone(),
                    domain_min: lut.input_min,
                    domain_max: lut.input_max,
                };
                self.compile_lut1d(&vfx_lut, forward);
            }

            Transform::Lut3D(lut) => {
                let dir = if direction == TransformDirection::Inverse {
                    lut.direction.inverse()
                } else {
                    lut.direction
                };
                let forward = dir == TransformDirection::Forward;
                
                // Convert inline LUT to vfx_lut format
                let vfx_lut = vfx_lut::Lut3D {
                    size: lut.size,
                    data: lut.data.clone(),
                    domain_min: lut.domain_min,
                    domain_max: lut.domain_max,
                    interpolation: match lut.interpolation {
                        Interpolation::Nearest => vfx_lut::Interpolation::Nearest,
                        Interpolation::Linear => vfx_lut::Interpolation::Linear,
                        Interpolation::Tetrahedral | Interpolation::Best => vfx_lut::Interpolation::Tetrahedral,
                    },
                };
                self.compile_lut3d(&vfx_lut, lut.interpolation, forward);
            }

            _ => {
                // Other transforms (ColorSpace, Look, DisplayView) handled at config level
            }
        }

        Ok(())
    }

    /// Compiles a 1D LUT into ProcessorOp::Lut1d.
    fn compile_lut1d(&mut self, lut: &vfx_lut::Lut1D, forward: bool) {
        let size = lut.r.len();
        let channels = if lut.g.is_some() { 3 } else { 1 };
        
        // Flatten LUT data
        let mut data = Vec::with_capacity(size * channels);
        if channels == 3 {
            let g = lut.g.as_ref().unwrap();
            let b = lut.b.as_ref().unwrap();
            for i in 0..size {
                data.push(lut.r[i]);
                data.push(g[i]);
                data.push(b[i]);
            }
        } else {
            data.extend_from_slice(&lut.r);
        }
        
        // For inverse, we need to invert the LUT
        // This is approximate - proper inversion requires interpolation
        let lut_data = if forward {
            data
        } else {
            invert_lut1d(&data, size, channels)
        };
        
        self.ops.push(ProcessorOp::Lut1d {
            lut: lut_data,
            size,
            channels,
            domain: [lut.domain_min, lut.domain_max],
        });
    }

    /// Compiles a 3D LUT into ProcessorOp::Lut3d.
    fn compile_lut3d(&mut self, lut: &vfx_lut::Lut3D, interp: Interpolation, forward: bool) {
        // Flatten Vec<[f32; 3]> to Vec<f32>
        let flat_data: Vec<f32> = lut.data.iter()
            .flat_map(|rgb| rgb.iter().copied())
            .collect();
        
        // Invert if needed using Newton-Raphson
        let lut_data = if forward {
            flat_data
        } else {
            invert_lut3d(&flat_data, lut.size, lut.domain_min, lut.domain_max)
        };
        
        self.ops.push(ProcessorOp::Lut3d {
            lut: lut_data,
            size: lut.size,
            interp,
            domain_min: lut.domain_min,
            domain_max: lut.domain_max,
        });
    }

    /// Compiles a CLF ProcessList into ops.
    fn compile_clf(&mut self, pl: &vfx_lut::ProcessList, forward: bool) -> OcioResult<()> {
        let nodes: Vec<_> = if forward {
            pl.nodes.iter().collect()
        } else {
            pl.nodes.iter().rev().collect()
        };
        
        for node in nodes {
            match node {
                vfx_lut::ProcessNode::Matrix { values, .. } => {
                    let mut m16 = [0.0f32; 16];
                    // CLF uses 3x3 or 3x4 matrix
                    for (i, &v) in values.iter().take(9).enumerate() {
                        let row = i / 3;
                        let col = i % 3;
                        m16[row * 4 + col] = v;
                    }
                    m16[15] = 1.0;
                    
                    // Offset is in columns 9-11 if present
                    let mut off4 = [0.0f32; 4];
                    if values.len() >= 12 {
                        off4[0] = values[9];
                        off4[1] = values[10];
                        off4[2] = values[11];
                    }
                    
                    self.ops.push(ProcessorOp::Matrix {
                        matrix: m16,
                        offset: off4,
                    });
                }
                vfx_lut::ProcessNode::Lut1D { lut, .. } => {
                    self.compile_lut1d(lut, forward);
                }
                vfx_lut::ProcessNode::Lut3D { lut, .. } => {
                    // Convert vfx_lut::Interpolation to transform::Interpolation
                    let interp = match lut.interpolation {
                        vfx_lut::Interpolation::Nearest => Interpolation::Nearest,
                        vfx_lut::Interpolation::Linear => Interpolation::Linear,
                        vfx_lut::Interpolation::Tetrahedral => Interpolation::Tetrahedral,
                    };
                    self.compile_lut3d(lut, interp, forward);
                }
                vfx_lut::ProcessNode::Range(rp) => {
                    // Use first channel for scalar ops (simplified)
                    let scale = (rp.max_out[0] - rp.min_out[0]) / (rp.max_in[0] - rp.min_in[0]);
                    let offset = rp.min_out[0] - rp.min_in[0] * scale;
                    self.ops.push(ProcessorOp::Range {
                        scale,
                        offset,
                        clamp_min: if rp.clamp { Some(rp.min_out[0]) } else { None },
                        clamp_max: if rp.clamp { Some(rp.max_out[0]) } else { None },
                    });
                }
                vfx_lut::ProcessNode::Cdl(cdl) => {
                    self.ops.push(ProcessorOp::Cdl {
                        slope: cdl.slope,
                        offset: cdl.offset,
                        power: cdl.power,
                        saturation: cdl.saturation,
                    });
                }
                vfx_lut::ProcessNode::Exponent(exp) => {
                    self.ops.push(ProcessorOp::Exponent {
                        value: [exp.exponent[0], exp.exponent[1], exp.exponent[2], 1.0],
                        negative_style: NegativeStyle::Clamp,
                    });
                }
                vfx_lut::ProcessNode::Log(log) => {
                    self.ops.push(ProcessorOp::Log {
                        base: log.base,
                        forward,
                    });
                }
            }
        }
        Ok(())
    }

    /// Applies the transform to RGB pixels in-place.
    pub fn apply_rgb(&self, pixels: &mut [[f32; 3]]) {
        for pixel in pixels.iter_mut() {
            self.apply_one_rgb(pixel);
        }
    }

    /// Applies the transform to RGBA pixels in-place.
    pub fn apply_rgba(&self, pixels: &mut [[f32; 4]]) {
        for pixel in pixels.iter_mut() {
            self.apply_one_rgba(pixel);
        }
    }

    /// Applies the transform to a single RGB pixel.
    #[inline]
    fn apply_one_rgb(&self, pixel: &mut [f32; 3]) {
        for op in &self.ops {
            match op {
                ProcessorOp::Matrix { matrix, offset } => {
                    let [r, g, b] = *pixel;
                    pixel[0] = r * matrix[0] + g * matrix[1] + b * matrix[2] + offset[0];
                    pixel[1] = r * matrix[4] + g * matrix[5] + b * matrix[6] + offset[1];
                    pixel[2] = r * matrix[8] + g * matrix[9] + b * matrix[10] + offset[2];
                }

                ProcessorOp::Cdl { slope, offset, power, saturation } => {
                    // Apply SOP
                    pixel[0] = (pixel[0] * slope[0] + offset[0]).max(0.0).powf(power[0]);
                    pixel[1] = (pixel[1] * slope[1] + offset[1]).max(0.0).powf(power[1]);
                    pixel[2] = (pixel[2] * slope[2] + offset[2]).max(0.0).powf(power[2]);
                    
                    // Apply saturation
                    if *saturation != 1.0 {
                        let luma = pixel[0] * 0.2126 + pixel[1] * 0.7152 + pixel[2] * 0.0722;
                        pixel[0] = luma + (pixel[0] - luma) * saturation;
                        pixel[1] = luma + (pixel[1] - luma) * saturation;
                        pixel[2] = luma + (pixel[2] - luma) * saturation;
                    }
                }

                ProcessorOp::Exponent { value, negative_style } => {
                    for (i, v) in pixel.iter_mut().enumerate() {
                        match negative_style {
                            NegativeStyle::Clamp => {
                                *v = v.max(0.0).powf(value[i]);
                            }
                            NegativeStyle::Mirror => {
                                *v = v.signum() * v.abs().powf(value[i]);
                            }
                            NegativeStyle::PassThru => {
                                if *v >= 0.0 {
                                    *v = v.powf(value[i]);
                                }
                            }
                        }
                    }
                }

                ProcessorOp::Log { base, forward } => {
                    if *forward {
                        // Linear to log
                        for v in pixel.iter_mut() {
                            *v = v.max(1e-10).log(*base);
                        }
                    } else {
                        // Log to linear
                        for v in pixel.iter_mut() {
                            *v = base.powf(*v);
                        }
                    }
                }

                ProcessorOp::Range { scale, offset, clamp_min, clamp_max } => {
                    for v in pixel.iter_mut() {
                        *v = *v * scale + offset;
                        if let Some(min) = clamp_min {
                            *v = v.max(*min);
                        }
                        if let Some(max) = clamp_max {
                            *v = v.min(*max);
                        }
                    }
                }

                ProcessorOp::Lut1d { lut, size, channels, domain } => {
                    let scale = (*size - 1) as f32 / (domain[1] - domain[0]);
                    for (i, v) in pixel.iter_mut().enumerate() {
                        let idx = ((*v - domain[0]) * scale).clamp(0.0, (*size - 1) as f32);
                        let idx_floor = idx.floor() as usize;
                        let idx_ceil = (idx_floor + 1).min(*size - 1);
                        let frac = idx - idx_floor as f32;
                        
                        let ch = if *channels == 1 { 0 } else { i };
                        let v0 = lut[idx_floor * channels + ch];
                        let v1 = lut[idx_ceil * channels + ch];
                        *v = v0 + (v1 - v0) * frac;
                    }
                }

                ProcessorOp::ExposureContrast { exposure, contrast, gamma, pivot, style } => {
                    // Apply exposure (in stops)
                    let exp_mult = 2.0_f32.powf(*exposure);
                    
                    match style {
                        ExposureContrastStyle::Linear => {
                            // Linear domain: exposure, then contrast around pivot
                            for v in pixel.iter_mut() {
                                *v *= exp_mult;
                                // Contrast around pivot
                                *v = (*v / pivot).powf(*contrast) * pivot;
                                // Gamma
                                if *gamma != 1.0 {
                                    *v = v.max(0.0).powf(*gamma);
                                }
                            }
                        }
                        ExposureContrastStyle::Video => {
                            // Video domain: apply in gamma-encoded space
                            for v in pixel.iter_mut() {
                                *v *= exp_mult;
                                *v = pivot + (*v - pivot) * contrast;
                                if *gamma != 1.0 {
                                    *v = v.max(0.0).powf(*gamma);
                                }
                            }
                        }
                        ExposureContrastStyle::Logarithmic => {
                            // Log domain: linear exposure, log contrast
                            for v in pixel.iter_mut() {
                                *v *= exp_mult;
                                let log_v = v.max(1e-10).log10();
                                let log_pivot = pivot.max(1e-10).log10();
                                let adjusted = log_pivot + (log_v - log_pivot) * contrast;
                                *v = 10.0_f32.powf(adjusted);
                                if *gamma != 1.0 {
                                    *v = v.max(0.0).powf(*gamma);
                                }
                            }
                        }
                    }
                }

                ProcessorOp::FixedFunction { style, params, forward } => {
                    match style {
                        FixedFunctionStyle::RgbToHsv => {
                            if *forward {
                                let [r, g, b] = *pixel;
                                let max = r.max(g).max(b);
                                let min = r.min(g).min(b);
                                let delta = max - min;
                                
                                let h = if delta.abs() < 1e-10 {
                                    0.0
                                } else if (max - r).abs() < 1e-10 {
                                    60.0 * (((g - b) / delta) % 6.0)
                                } else if (max - g).abs() < 1e-10 {
                                    60.0 * (((b - r) / delta) + 2.0)
                                } else {
                                    60.0 * (((r - g) / delta) + 4.0)
                                };
                                let h = if h < 0.0 { h + 360.0 } else { h } / 360.0;
                                let s = if max.abs() < 1e-10 { 0.0 } else { delta / max };
                                let v = max;
                                
                                pixel[0] = h;
                                pixel[1] = s;
                                pixel[2] = v;
                            } else {
                                // HSV to RGB
                                let [h, s, v] = *pixel;
                                let h = h * 360.0;
                                let c = v * s;
                                let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
                                let m = v - c;
                                
                                let (r, g, b) = if h < 60.0 {
                                    (c, x, 0.0)
                                } else if h < 120.0 {
                                    (x, c, 0.0)
                                } else if h < 180.0 {
                                    (0.0, c, x)
                                } else if h < 240.0 {
                                    (0.0, x, c)
                                } else if h < 300.0 {
                                    (x, 0.0, c)
                                } else {
                                    (c, 0.0, x)
                                };
                                
                                pixel[0] = r + m;
                                pixel[1] = g + m;
                                pixel[2] = b + m;
                            }
                        }
                        FixedFunctionStyle::HsvToRgb => {
                            // Same as RgbToHsv inverse
                            let [h, s, v] = *pixel;
                            let h = h * 360.0;
                            let c = v * s;
                            let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
                            let m = v - c;
                            
                            let (r, g, b) = if h < 60.0 {
                                (c, x, 0.0)
                            } else if h < 120.0 {
                                (x, c, 0.0)
                            } else if h < 180.0 {
                                (0.0, c, x)
                            } else if h < 240.0 {
                                (0.0, x, c)
                            } else if h < 300.0 {
                                (x, 0.0, c)
                            } else {
                                (c, 0.0, x)
                            };
                            
                            pixel[0] = r + m;
                            pixel[1] = g + m;
                            pixel[2] = b + m;
                        }
                        FixedFunctionStyle::AcesGamutComp13 => {
                            // ACES 1.3 gamut compression
                            // Simplified implementation - full version uses LMT
                            let threshold = params.first().copied().unwrap_or(0.815);
                            let limit = params.get(1).copied().unwrap_or(1.2);
                            
                            if *forward {
                                for v in pixel.iter_mut() {
                                    if *v > threshold {
                                        let t = (*v - threshold) / (limit - threshold);
                                        *v = threshold + (limit - threshold) * (t / (1.0 + t));
                                    }
                                }
                            } else {
                                for v in pixel.iter_mut() {
                                    if *v > threshold {
                                        let compressed = *v;
                                        let range = limit - threshold;
                                        let t = (compressed - threshold) / range;
                                        *v = threshold + range * t / (1.0 - t).max(1e-10);
                                    }
                                }
                            }
                        }
                        FixedFunctionStyle::AcesRedMod03 | FixedFunctionStyle::AcesRedMod10 => {
                            // ACES Red Modifier - reduce saturation in red region
                            let [r, g, b] = *pixel;
                            let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                            
                            // Hue detection (simplified)
                            let max = r.max(g).max(b);
                            let min = r.min(g).min(b);
                            let chroma = max - min;
                            
                            if chroma > 1e-6 {
                                // Rough hue angle
                                let hue = if (r - max).abs() < 1e-6 {
                                    (g - b) / chroma
                                } else if (g - max).abs() < 1e-6 {
                                    2.0 + (b - r) / chroma
                                } else {
                                    4.0 + (r - g) / chroma
                                };
                                
                                // Red region weight (hue near 0 or 6)
                                let hue_norm = if hue < 0.0 { hue + 6.0 } else { hue };
                                let red_weight = if hue_norm < 1.0 || hue_norm > 5.0 {
                                    let dist = if hue_norm < 1.0 { hue_norm } else { 6.0 - hue_norm };
                                    1.0 - dist
                                } else {
                                    0.0
                                };
                                
                                // Saturation reduction factor
                                let sat = if max > 1e-6 { chroma / max } else { 0.0 };
                                let mod_factor = 1.0 - 0.2 * red_weight * sat;
                                
                                if *forward {
                                    pixel[0] = lum + (r - lum) * mod_factor;
                                    pixel[1] = lum + (g - lum) * mod_factor;
                                    pixel[2] = lum + (b - lum) * mod_factor;
                                } else {
                                    let inv = 1.0 / mod_factor.max(1e-6);
                                    pixel[0] = lum + (r - lum) * inv;
                                    pixel[1] = lum + (g - lum) * inv;
                                    pixel[2] = lum + (b - lum) * inv;
                                }
                            }
                        }
                        FixedFunctionStyle::AcesGlow03 | FixedFunctionStyle::AcesGlow10 => {
                            // ACES Glow - add glow to bright saturated regions
                            let [r, g, b] = *pixel;
                            let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                            
                            // Glow parameters
                            let glow_gain = 0.05;
                            let glow_mid = 0.08;
                            
                            // Sigmoid for glow amount
                            let x = (y - glow_mid) * 50.0;
                            let sigmoid = 1.0 / (1.0 + (-x).exp());
                            
                            // Saturation estimate
                            let max = r.max(g).max(b);
                            let min = r.min(g).min(b);
                            let sat = if max > 1e-6 { (max - min) / max } else { 0.0 };
                            
                            let glow = glow_gain * sigmoid * sat;
                            
                            if *forward {
                                pixel[0] = r + glow;
                                pixel[1] = g + glow;
                                pixel[2] = b + glow;
                            } else {
                                pixel[0] = r - glow;
                                pixel[1] = g - glow;
                                pixel[2] = b - glow;
                            }
                        }
                        _ => {
                            // Other fixed functions - XYZ/xyY, XYZ/Luv etc.
                        }
                    }
                }

                ProcessorOp::Allocation { allocation, vars, forward } => {
                    let min_val = vars.first().copied().unwrap_or(0.0);
                    let max_val = vars.get(1).copied().unwrap_or(1.0);
                    
                    match allocation {
                        AllocationType::Uniform => {
                            if *forward {
                                // Normalize to [0, 1]
                                for v in pixel.iter_mut() {
                                    *v = (*v - min_val) / (max_val - min_val);
                                }
                            } else {
                                // Expand from [0, 1]
                                for v in pixel.iter_mut() {
                                    *v = *v * (max_val - min_val) + min_val;
                                }
                            }
                        }
                        AllocationType::Log2 => {
                            if *forward {
                                // Log2 allocation
                                for v in pixel.iter_mut() {
                                    let log_v = v.max(1e-10).log2();
                                    *v = (log_v - min_val) / (max_val - min_val);
                                }
                            } else {
                                // Inverse log2
                                for v in pixel.iter_mut() {
                                    let log_v = *v * (max_val - min_val) + min_val;
                                    *v = 2.0_f32.powf(log_v);
                                }
                            }
                        }
                    }
                }

                ProcessorOp::GradingPrimary { lift, gamma, gain, offset, exposure, contrast, saturation, pivot, clamp_black, clamp_white } => {
                    // Apply exposure
                    let exp_mult = 2.0_f32.powf(*exposure);
                    
                    for (i, v) in pixel.iter_mut().enumerate() {
                        // Exposure
                        *v *= exp_mult;
                        
                        // Lift/Gamma/Gain formula:
                        // out = (gain * (in + lift * (1 - in)))^(1/gamma)
                        let lifted = *v + lift[i] * (1.0 - *v);
                        let gained = lifted * gain[i];
                        *v = gained.max(0.0).powf(1.0 / gamma[i]);
                        
                        // Offset
                        *v += offset;
                    }
                    
                    // Contrast around pivot
                    if *contrast != 1.0 {
                        for v in pixel.iter_mut() {
                            *v = pivot + (*v - pivot) * contrast;
                        }
                    }
                    
                    // Saturation
                    if *saturation != 1.0 {
                        let luma = pixel[0] * 0.2126 + pixel[1] * 0.7152 + pixel[2] * 0.0722;
                        for v in pixel.iter_mut() {
                            *v = luma + (*v - luma) * saturation;
                        }
                    }
                    
                    // Clamping
                    if let Some(black) = clamp_black {
                        for v in pixel.iter_mut() {
                            *v = v.max(*black);
                        }
                    }
                    if let Some(white) = clamp_white {
                        for v in pixel.iter_mut() {
                            *v = v.min(*white);
                        }
                    }
                }

                ProcessorOp::GradingRgbCurve { red_lut, green_lut, blue_lut, master_lut } => {
                    let lut_size = red_lut.len();
                    let scale = (lut_size - 1) as f32;
                    
                    // Apply per-channel curves
                    let luts = [red_lut, green_lut, blue_lut];
                    for (i, v) in pixel.iter_mut().enumerate() {
                        let idx = (*v * scale).clamp(0.0, scale);
                        let idx_floor = idx.floor() as usize;
                        let idx_ceil = (idx_floor + 1).min(lut_size - 1);
                        let frac = idx - idx_floor as f32;
                        
                        let v0 = luts[i][idx_floor];
                        let v1 = luts[i][idx_ceil];
                        *v = v0 + (v1 - v0) * frac;
                    }
                    
                    // Apply master curve
                    for v in pixel.iter_mut() {
                        let idx = (*v * scale).clamp(0.0, scale);
                        let idx_floor = idx.floor() as usize;
                        let idx_ceil = (idx_floor + 1).min(lut_size - 1);
                        let frac = idx - idx_floor as f32;
                        
                        let v0 = master_lut[idx_floor];
                        let v1 = master_lut[idx_ceil];
                        *v = v0 + (v1 - v0) * frac;
                    }
                }

                ProcessorOp::GradingTone { shadows, midtones, highlights, whites, blacks, shadow_start, shadow_pivot, highlight_start, highlight_pivot } => {
                    // Compute tonal weights based on luminance
                    let luma = pixel[0] * 0.2126 + pixel[1] * 0.7152 + pixel[2] * 0.0722;
                    
                    // Shadow weight (high in shadows, fades to zero)
                    let shadow_w = if luma < *shadow_start {
                        1.0
                    } else if luma < *shadow_pivot {
                        1.0 - (luma - shadow_start) / (shadow_pivot - shadow_start)
                    } else {
                        0.0
                    };
                    
                    // Highlight weight (high in highlights, fades to zero)
                    let highlight_w = if luma > *highlight_pivot {
                        1.0
                    } else if luma > *highlight_start {
                        (luma - highlight_start) / (highlight_pivot - highlight_start)
                    } else {
                        0.0
                    };
                    
                    // Midtone weight (high in middle, fades at extremes)
                    let midtone_w = 1.0 - shadow_w - highlight_w;
                    let midtone_w = midtone_w.max(0.0);
                    
                    for (i, v) in pixel.iter_mut().enumerate() {
                        // Apply blacks (offset)
                        *v += blacks[i] + blacks[3];
                        
                        // Apply tonal adjustments
                        let shadow_adj = (shadows[i] * shadows[3] - 1.0) * shadow_w;
                        let midtone_adj = (midtones[i] * midtones[3] - 1.0) * midtone_w;
                        let highlight_adj = (highlights[i] * highlights[3] - 1.0) * highlight_w;
                        
                        *v *= 1.0 + shadow_adj + midtone_adj + highlight_adj;
                        
                        // Apply whites (scale)
                        *v *= whites[i] * whites[3];
                    }
                }

                ProcessorOp::Transfer { style, forward } => {
                    for v in pixel.iter_mut() {
                        *v = apply_transfer(*v, *style, *forward);
                    }
                }

                ProcessorOp::Lut3d { lut, size, interp, domain_min, domain_max } => {
                    let size_f = (*size - 1) as f32;
                    let to_unit = |v: f32, min: f32, max: f32| {
                        let range = max - min;
                        if range.abs() < 1e-10 {
                            0.0
                        } else {
                            ((v - min) / range).clamp(0.0, 1.0)
                        }
                    };
                    let r = to_unit(pixel[0], domain_min[0], domain_max[0]);
                    let g = to_unit(pixel[1], domain_min[1], domain_max[1]);
                    let b = to_unit(pixel[2], domain_min[2], domain_max[2]);
                    
                    // Clamp and scale to LUT indices
                    let ri = (r * size_f).clamp(0.0, size_f);
                    let gi = (g * size_f).clamp(0.0, size_f);
                    let bi = (b * size_f).clamp(0.0, size_f);
                    
                    let r0 = ri.floor() as usize;
                    let g0 = gi.floor() as usize;
                    let b0 = bi.floor() as usize;
                    
                    let idx = |r: usize, g: usize, b: usize| (b * *size * *size + g * *size + r) * 3;
                    
                    match interp {
                        Interpolation::Nearest => {
                            // Nearest neighbor - use closest cell
                            let ri = ri.round() as usize;
                            let gi = gi.round() as usize;
                            let bi = bi.round() as usize;
                            pixel[0] = lut[idx(ri.min(*size-1), gi.min(*size-1), bi.min(*size-1))];
                            pixel[1] = lut[idx(ri.min(*size-1), gi.min(*size-1), bi.min(*size-1)) + 1];
                            pixel[2] = lut[idx(ri.min(*size-1), gi.min(*size-1), bi.min(*size-1)) + 2];
                        }
                        _ => {
                            // Trilinear interpolation (Linear, Tetrahedral, Best)
                            let r1 = (r0 + 1).min(*size - 1);
                            let g1 = (g0 + 1).min(*size - 1);
                            let b1 = (b0 + 1).min(*size - 1);
                            
                            let fr = ri - r0 as f32;
                            let fg = gi - g0 as f32;
                            let fb = bi - b0 as f32;
                            
                            for ch in 0..3 {
                                let c000 = lut[idx(r0, g0, b0) + ch];
                                let c100 = lut[idx(r1, g0, b0) + ch];
                                let c010 = lut[idx(r0, g1, b0) + ch];
                                let c110 = lut[idx(r1, g1, b0) + ch];
                                let c001 = lut[idx(r0, g0, b1) + ch];
                                let c101 = lut[idx(r1, g0, b1) + ch];
                                let c011 = lut[idx(r0, g1, b1) + ch];
                                let c111 = lut[idx(r1, g1, b1) + ch];
                                
                                let c00 = c000 + (c100 - c000) * fr;
                                let c01 = c001 + (c101 - c001) * fr;
                                let c10 = c010 + (c110 - c010) * fr;
                                let c11 = c011 + (c111 - c011) * fr;
                                
                                let c0 = c00 + (c10 - c00) * fg;
                                let c1 = c01 + (c11 - c01) * fg;
                                
                                pixel[ch] = c0 + (c1 - c0) * fb;
                            }
                        }
                    }
                }

                ProcessorOp::LogAffine { base, log_side_slope, log_side_offset, lin_side_slope, lin_side_offset, forward } => {
                    // LogAffine formula:
                    // Forward: out = log_side_slope * log(lin_side_slope * x + lin_side_offset, base) + log_side_offset
                    // Inverse: out = (pow(base, (x - log_side_offset) / log_side_slope) - lin_side_offset) / lin_side_slope
                    let log_base = base.ln();
                    
                    for (i, v) in pixel.iter_mut().enumerate() {
                        let ch = i.min(2);
                        if *forward {
                            let lin = lin_side_slope[ch] * *v + lin_side_offset[ch];
                            if lin > 0.0 {
                                *v = log_side_slope[ch] * lin.ln() / log_base + log_side_offset[ch];
                            } else {
                                *v = log_side_offset[ch]; // Clamp to minimum
                            }
                        } else {
                            let exp_arg = (*v - log_side_offset[ch]) / log_side_slope[ch];
                            let lin = base.powf(exp_arg) - lin_side_offset[ch];
                            *v = lin / lin_side_slope[ch];
                        }
                    }
                }

                ProcessorOp::LogCamera { base, log_side_slope, log_side_offset, lin_side_slope, lin_side_offset, lin_side_break, linear_slope, forward } => {
                    // LogCamera formula (piecewise):
                    // Forward: if x >= lin_side_break:
                    //            out = log_side_slope * log(lin_side_slope * x + lin_side_offset, base) + log_side_offset
                    //          else:
                    //            out = linear_slope * x + linear_offset (calculated from continuity)
                    let log_base = base.ln();
                    
                    for (i, v) in pixel.iter_mut().enumerate() {
                        let ch = i.min(2);
                        if *forward {
                            if *v >= lin_side_break[ch] {
                                // Log region
                                let lin = lin_side_slope[ch] * *v + lin_side_offset[ch];
                                if lin > 0.0 {
                                    *v = log_side_slope[ch] * lin.ln() / log_base + log_side_offset[ch];
                                }
                            } else {
                                // Linear region
                                // Calculate linear offset from continuity at break point
                                let break_lin = lin_side_slope[ch] * lin_side_break[ch] + lin_side_offset[ch];
                                let break_log = if break_lin > 0.0 {
                                    log_side_slope[ch] * break_lin.ln() / log_base + log_side_offset[ch]
                                } else {
                                    log_side_offset[ch]
                                };
                                let linear_offset = break_log - linear_slope[ch] * lin_side_break[ch];
                                *v = linear_slope[ch] * *v + linear_offset;
                            }
                        } else {
                            // Inverse: determine which region based on output
                            let break_lin = lin_side_slope[ch] * lin_side_break[ch] + lin_side_offset[ch];
                            let break_log = if break_lin > 0.0 {
                                log_side_slope[ch] * break_lin.ln() / log_base + log_side_offset[ch]
                            } else {
                                log_side_offset[ch]
                            };
                            
                            if *v >= break_log {
                                // Inverse log region
                                let exp_arg = (*v - log_side_offset[ch]) / log_side_slope[ch];
                                let lin = base.powf(exp_arg) - lin_side_offset[ch];
                                *v = lin / lin_side_slope[ch];
                            } else {
                                // Inverse linear region
                                let linear_offset = break_log - linear_slope[ch] * lin_side_break[ch];
                                *v = (*v - linear_offset) / linear_slope[ch];
                            }
                        }
                    }
                }

                ProcessorOp::ExponentWithLinear { gamma, offset, negative_style, forward } => {
                    // ExponentWithLinear (sRGB/Rec.709 style):
                    // Forward: if x >= break: out = (x + offset)^gamma - offset^gamma
                    //          else: out = linear_slope * x
                    // The break point and linear slope are derived from continuity
                    
                    for (i, v) in pixel.iter_mut().enumerate() {
                        let g = gamma[i];
                        let off = offset[i];
                        
                        // Handle negatives based on style
                        let (sign, abs_v) = if *v < 0.0 {
                            match negative_style {
                                NegativeStyle::Clamp => { *v = 0.0; continue; }
                                NegativeStyle::Mirror => (-1.0, -(*v)),
                                NegativeStyle::PassThru => { continue; }
                            }
                        } else {
                            (1.0, *v)
                        };
                        
                        // Calculate break point where derivative matches
                        // d/dx[(x + off)^g] = g * (x + off)^(g-1)
                        // At break: linear_slope = g * (break + off)^(g-1)
                        // For continuity: break * linear_slope = (break + off)^g - off^g
                        let break_point = off * (g - 1.0) / (1.0 - g * off.powf(g - 1.0)).max(1e-10);
                        let break_point = break_point.max(0.0);
                        let linear_slope = g * (break_point + off).powf(g - 1.0);
                        
                        let result = if *forward {
                            if abs_v >= break_point {
                                (abs_v + off).powf(g) - off.powf(g)
                            } else {
                                abs_v * linear_slope
                            }
                        } else {
                            // Inverse
                            let break_out = (break_point + off).powf(g) - off.powf(g);
                            if abs_v >= break_out {
                                (abs_v + off.powf(g)).powf(1.0 / g) - off
                            } else {
                                abs_v / linear_slope
                            }
                        };
                        
                        *v = sign * result;
                    }
                }
            }
        }
    }

    /// Applies the transform to a single RGBA pixel.
    #[inline]
    fn apply_one_rgba(&self, pixel: &mut [f32; 4]) {
        let mut rgb = [pixel[0], pixel[1], pixel[2]];
        self.apply_one_rgb(&mut rgb);
        pixel[0] = rgb[0];
        pixel[1] = rgb[1];
        pixel[2] = rgb[2];
        // Alpha is preserved
    }

    /// Returns the number of operations.
    #[inline]
    pub fn num_ops(&self) -> usize {
        self.ops.len()
    }

    /// Checks if processor has any operations.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Checks if processor is identity (no-op).
    pub fn is_identity(&self) -> bool {
        self.ops.is_empty()
    }

    /// Returns input bit depth hint.
    #[inline]
    pub fn input_bit_depth(&self) -> BitDepth {
        self.input_bit_depth
    }

    /// Returns output bit depth hint.
    #[inline]
    pub fn output_bit_depth(&self) -> BitDepth {
        self.output_bit_depth
    }

    /// Returns the internal operation list.
    ///
    /// Used by GPU processor for shader generation.
    pub fn ops(&self) -> &[ProcessorOp] {
        &self.ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposure_contrast_processor() {
        let ec = Transform::ExposureContrast(ExposureContrastTransform {
            exposure: 1.0,  // +1 stop
            contrast: 1.0,
            gamma: 1.0,
            pivot: 0.18,
            style: ExposureContrastStyle::Linear,
            direction: TransformDirection::Forward,
        });

        let processor = Processor::from_transform(&ec, TransformDirection::Forward).unwrap();
        assert_eq!(processor.num_ops(), 1);

        let mut pixels = [[0.18_f32, 0.18, 0.18]];
        processor.apply_rgb(&mut pixels);
        
        // +1 stop should double the value
        assert!((pixels[0][0] - 0.36).abs() < 0.01);
    }

    #[test]
    fn grading_primary_processor() {
        let gp = Transform::GradingPrimary(GradingPrimaryTransform {
            lift: [0.0, 0.0, 0.0],
            gamma: [1.0, 1.0, 1.0],
            gain: [1.5, 1.0, 0.8],  // Boost red, reduce blue
            offset: 0.0,
            exposure: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            pivot: 0.18,
            clamp_black: None,
            clamp_white: None,
            direction: TransformDirection::Forward,
        });

        let processor = Processor::from_transform(&gp, TransformDirection::Forward).unwrap();
        
        let mut pixels = [[0.5_f32, 0.5, 0.5]];
        processor.apply_rgb(&mut pixels);
        
        // Red boosted, green same, blue reduced
        assert!(pixels[0][0] > 0.5);
        assert!((pixels[0][1] - 0.5).abs() < 0.01);
        assert!(pixels[0][2] < 0.5);
    }

    #[test]
    fn rgb_curve_processor() {
        let gc = Transform::GradingRgbCurve(GradingRgbCurveTransform {
            // S-curve on red: lift shadows, pull highlights
            red: vec![[0.0, 0.1], [0.5, 0.5], [1.0, 0.9]],
            green: vec![[0.0, 0.0], [1.0, 1.0]],  // Identity
            blue: vec![[0.0, 0.0], [1.0, 1.0]],
            master: vec![[0.0, 0.0], [1.0, 1.0]],
            direction: TransformDirection::Forward,
        });

        let processor = Processor::from_transform(&gc, TransformDirection::Forward).unwrap();
        
        let mut pixels = [[0.0_f32, 0.5, 1.0]];
        processor.apply_rgb(&mut pixels);
        
        // Red: 0.0 -> 0.1 (lifted), green: 0.5 unchanged, blue: 1.0 unchanged
        assert!((pixels[0][0] - 0.1).abs() < 0.01);
        assert!((pixels[0][1] - 0.5).abs() < 0.01);
    }

    #[test]
    fn allocation_processor() {
        let alloc = Transform::Allocation(AllocationTransform {
            allocation: AllocationType::Log2,
            vars: vec![-8.0, 4.0],  // Log2 range: -8 to +4 stops
            direction: TransformDirection::Forward,
        });

        let processor = Processor::from_transform(&alloc, TransformDirection::Forward).unwrap();
        
        // 0.18 is roughly 18% gray, log2(0.18) ~ -2.47
        let mut pixels = [[0.18_f32, 0.18, 0.18]];
        processor.apply_rgb(&mut pixels);
        
        // Should be normalized to [0, 1] range based on -8 to +4 log2 range
        // log2(0.18) = -2.47, normalized = (-2.47 - (-8)) / (4 - (-8)) = 5.53 / 12 = 0.46
        assert!(pixels[0][0] > 0.4 && pixels[0][0] < 0.5);
    }

    #[test]
    fn fixed_function_rgb_to_hsv() {
        let ff = Transform::FixedFunction(FixedFunctionTransform {
            style: FixedFunctionStyle::RgbToHsv,
            params: vec![],
            direction: TransformDirection::Forward,
        });

        let processor = Processor::from_transform(&ff, TransformDirection::Forward).unwrap();
        
        // Pure red: H=0, S=1, V=1
        let mut pixels = [[1.0_f32, 0.0, 0.0]];
        processor.apply_rgb(&mut pixels);
        
        assert!((pixels[0][0] - 0.0).abs() < 0.01);  // Hue = 0 (red)
        assert!((pixels[0][1] - 1.0).abs() < 0.01);  // Saturation = 1
        assert!((pixels[0][2] - 1.0).abs() < 0.01);  // Value = 1
    }

    #[test]
    fn cdl_processor() {
        let cdl = Transform::Cdl(CdlTransform {
            slope: [1.1, 1.0, 0.9],
            offset: [0.01, 0.0, -0.01],
            power: [1.0, 1.0, 1.0],
            saturation: 1.0,
            style: CdlStyle::AscCdl,
            direction: TransformDirection::Forward,
        });

        let processor = Processor::from_transform(&cdl, TransformDirection::Forward).unwrap();
        assert_eq!(processor.num_ops(), 1);

        let mut pixels = [[0.18_f32, 0.18, 0.18]];
        processor.apply_rgb(&mut pixels);
        
        assert!((pixels[0][0] - 0.208).abs() < 0.001); // 0.18 * 1.1 + 0.01
        assert!((pixels[0][1] - 0.18).abs() < 0.001);
        assert!((pixels[0][2] - 0.152).abs() < 0.001); // 0.18 * 0.9 - 0.01
    }

    #[test]
    fn exponent_processor() {
        let exp = Transform::Exponent(ExponentTransform {
            value: [2.2, 2.2, 2.2, 1.0],
            negative_style: NegativeStyle::Clamp,
            direction: TransformDirection::Forward,
        });

        let processor = Processor::from_transform(&exp, TransformDirection::Forward).unwrap();
        
        let mut pixels = [[0.5_f32, 0.5, 0.5]];
        processor.apply_rgb(&mut pixels);
        
        let expected = 0.5_f32.powf(2.2);
        assert!((pixels[0][0] - expected).abs() < 0.0001);
    }

    #[test]
    fn group_processor() {
        let group = Transform::group(vec![
            Transform::Cdl(CdlTransform::default()),
            Transform::Exponent(ExponentTransform {
                value: [1.0, 1.0, 1.0, 1.0],
                negative_style: NegativeStyle::Clamp,
                direction: TransformDirection::Forward,
            }),
        ]);

        // Use OptimizationLevel::None to test compilation without removal of identities
        let processor = Processor::from_transform_with_opts(
            &group,
            TransformDirection::Forward,
            OptimizationLevel::None,
        ).unwrap();
        assert_eq!(processor.num_ops(), 2);
    }

    #[test]
    fn lut3d_inversion() {
        // Create a 3D LUT with 0.5x gain (bijective, no clamping)
        let size = 17; // Larger for better accuracy
        let mut lut_data = Vec::with_capacity(size * size * size * 3);
        
        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    let ri = r as f32 / (size - 1) as f32;
                    let gi = g as f32 / (size - 1) as f32;
                    let bi = b as f32 / (size - 1) as f32;
                    // Apply 0.5x gain (fully invertible)
                    lut_data.push(ri * 0.5);
                    lut_data.push(gi * 0.5);
                    lut_data.push(bi * 0.5);
                }
            }
        }
        
        let domain_min = [0.0, 0.0, 0.0];
        let domain_max = [1.0, 1.0, 1.0];
        
        // Test forward evaluation
        let input = [0.4, 0.6, 0.8];
        let forward = super::eval_lut3d_tetrahedral(&lut_data, size, &input, domain_min, domain_max);
        
        // Expected: input * 0.5
        assert!((forward[0] - 0.2).abs() < 0.01);
        assert!((forward[1] - 0.3).abs() < 0.01);
        assert!((forward[2] - 0.4).abs() < 0.01);
        
        // Create inverse LUT
        let inv_lut = super::invert_lut3d(&lut_data, size, domain_min, domain_max);
        
        // Evaluate inverse at forward result - should get back original
        let roundtrip = super::eval_lut3d_tetrahedral(&inv_lut, size, &forward, domain_min, domain_max);
        
        assert!((roundtrip[0] - input[0]).abs() < 0.05, "R: {} vs {}", roundtrip[0], input[0]);
        assert!((roundtrip[1] - input[1]).abs() < 0.05, "G: {} vs {}", roundtrip[1], input[1]);
        assert!((roundtrip[2] - input[2]).abs() < 0.05, "B: {} vs {}", roundtrip[2], input[2]);
    }

    #[test]
    fn tetrahedral_interpolation() {
        // Identity LUT
        let size = 5;
        let mut lut = Vec::with_capacity(size * size * size * 3);
        
        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    lut.push(r as f32 / (size - 1) as f32);
                    lut.push(g as f32 / (size - 1) as f32);
                    lut.push(b as f32 / (size - 1) as f32);
                }
            }
        }
        
        let domain_min = [0.0, 0.0, 0.0];
        let domain_max = [1.0, 1.0, 1.0];
        
        // Test identity: output should equal input
        let input = [0.33, 0.66, 0.5];
        let output = super::eval_lut3d_tetrahedral(&lut, size, &input, domain_min, domain_max);
        
        assert!((output[0] - input[0]).abs() < 0.01);
        assert!((output[1] - input[1]).abs() < 0.01);
        assert!((output[2] - input[2]).abs() < 0.01);
    }

    #[test]
    fn optimization_removes_identity() {
        // Create processor with identity CDL (should be removed by optimization)
        let identity_cdl = Transform::Cdl(CdlTransform::default());
        
        // Without optimization
        let proc_none = Processor::from_transform_with_opts(
            &identity_cdl,
            TransformDirection::Forward,
            OptimizationLevel::None,
        ).unwrap();
        assert_eq!(proc_none.num_ops(), 1, "Without optimization, identity CDL should remain");
        
        // With Lossless optimization
        let proc_opt = Processor::from_transform_with_opts(
            &identity_cdl,
            TransformDirection::Forward,
            OptimizationLevel::Lossless,
        ).unwrap();
        assert_eq!(proc_opt.num_ops(), 0, "With optimization, identity CDL should be removed");
    }

    #[test]
    fn optimization_combines_matrices() {
        // Create two matrix transforms
        let m1 = Transform::Matrix(MatrixTransform {
            matrix: [
                2.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
            offset: [0.0, 0.0, 0.0, 0.0],
            direction: TransformDirection::Forward,
        });
        let m2 = Transform::Matrix(MatrixTransform {
            matrix: [
                0.5, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
            offset: [0.0, 0.0, 0.0, 0.0],
            direction: TransformDirection::Forward,
        });
        
        let group = Transform::group(vec![m1, m2]);
        
        // Without optimization
        let proc_none = Processor::from_transform_with_opts(
            &group,
            TransformDirection::Forward,
            OptimizationLevel::None,
        ).unwrap();
        assert_eq!(proc_none.num_ops(), 2, "Without optimization should have 2 matrices");
        
        // With Lossless optimization - matrices should be combined
        let proc_opt = Processor::from_transform_with_opts(
            &group,
            TransformDirection::Forward,
            OptimizationLevel::Lossless,
        ).unwrap();
        assert_eq!(proc_opt.num_ops(), 1, "With optimization, matrices should be combined to 1");
        
        // Verify combined matrix produces correct output (2.0 * 0.5 = 1.0 = identity)
        let mut pixels = [[0.5_f32, 0.5, 0.5]];
        proc_opt.apply_rgb(&mut pixels);
        assert!((pixels[0][0] - 0.5).abs() < 0.0001, "Combined matrix should be identity");
    }
}
