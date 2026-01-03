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

use crate::error::{OcioError, OcioResult};
use crate::transform::*;

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
    ops: Vec<Op>,
    /// Input bit depth hint.
    input_bit_depth: BitDepth,
    /// Output bit depth hint.
    output_bit_depth: BitDepth,
    /// Has dynamic properties.
    has_dynamic: bool,
}

/// Bit depth for processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BitDepth {
    /// Unknown/auto.
    #[default]
    Unknown,
    /// 8-bit unsigned.
    Uint8,
    /// 10-bit unsigned.
    Uint10,
    /// 12-bit unsigned.
    Uint12,
    /// 16-bit unsigned.
    Uint16,
    /// 16-bit float.
    F16,
    /// 32-bit float.
    F32,
}

/// Internal operation type.
#[derive(Debug, Clone)]
pub(crate) enum Op {
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
    /// Gamma/contrast curve.
    GammaContrast {
        gamma: f32,
        contrast: f32,
        pivot: f32,
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
}

/// Built-in transfer function styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferStyle {
    /// sRGB OETF.
    Srgb,
    /// Rec.709 OETF.
    Rec709,
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
        let mut processor = Self::new();
        processor.compile_transform(transform, direction)?;
        Ok(processor)
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
                    // TODO: Compute inverse matrix
                    (m.matrix.map(|v| v as f32), m.offset.map(|v| v as f32))
                } else {
                    (m.matrix.map(|v| v as f32), m.offset.map(|v| v as f32))
                };
                
                self.ops.push(Op::Matrix { matrix, offset });
            }

            Transform::Cdl(cdl) => {
                let dir = if direction == TransformDirection::Inverse {
                    cdl.direction.inverse()
                } else {
                    cdl.direction
                };

                if dir == TransformDirection::Forward {
                    self.ops.push(Op::Cdl {
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
                    
                    self.ops.push(Op::Cdl {
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

                self.ops.push(Op::Exponent {
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

                self.ops.push(Op::Log {
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
                    if dir == TransformDirection::Forward {
                        let scale = (max_out - min_out) / (max_in - min_in);
                        let offset = min_out - min_in * scale;
                        (scale as f32, offset as f32, r.min_out.map(|v| v as f32), r.max_out.map(|v| v as f32))
                    } else {
                        let scale = (max_in - min_in) / (max_out - min_out);
                        let offset = min_in - min_out * scale;
                        (scale as f32, offset as f32, r.min_in.map(|v| v as f32), r.max_in.map(|v| v as f32))
                    }
                } else {
                    (1.0, 0.0, None, None)
                };

                self.ops.push(Op::Range {
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

            Transform::FileTransform(_) => {
                // File transforms are loaded at config parse time
                return Err(OcioError::InvalidTransform {
                    reason: "FileTransform must be resolved before processing".into(),
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

                self.ops.push(Op::ExposureContrast {
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

                self.ops.push(Op::FixedFunction {
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

                self.ops.push(Op::Allocation {
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

                self.ops.push(Op::GradingPrimary {
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

                self.ops.push(Op::GradingRgbCurve {
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

                self.ops.push(Op::GradingTone {
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

            _ => {
                // Other transforms (ColorSpace, Look, DisplayView) handled at config level
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
                Op::Matrix { matrix, offset } => {
                    let [r, g, b] = *pixel;
                    pixel[0] = r * matrix[0] + g * matrix[1] + b * matrix[2] + offset[0];
                    pixel[1] = r * matrix[4] + g * matrix[5] + b * matrix[6] + offset[1];
                    pixel[2] = r * matrix[8] + g * matrix[9] + b * matrix[10] + offset[2];
                }

                Op::Cdl { slope, offset, power, saturation } => {
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

                Op::Exponent { value, negative_style } => {
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

                Op::Log { base, forward } => {
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

                Op::Range { scale, offset, clamp_min, clamp_max } => {
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

                Op::Lut1d { lut, size, channels, domain } => {
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

                Op::ExposureContrast { exposure, contrast, gamma, pivot, style } => {
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

                Op::FixedFunction { style, params, forward } => {
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
                            let threshold = params.get(0).copied().unwrap_or(0.815);
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
                        _ => {
                            // Other fixed functions - pass through for now
                        }
                    }
                }

                Op::Allocation { allocation, vars, forward } => {
                    let min_val = vars.get(0).copied().unwrap_or(0.0);
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

                Op::GradingPrimary { lift, gamma, gain, offset, exposure, contrast, saturation, pivot, clamp_black, clamp_white } => {
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

                Op::GradingRgbCurve { red_lut, green_lut, blue_lut, master_lut } => {
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

                Op::GradingTone { shadows, midtones, highlights, whites, blacks, shadow_start, shadow_pivot, highlight_start, highlight_pivot } => {
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

                _ => {}
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

        let processor = Processor::from_transform(&group, TransformDirection::Forward).unwrap();
        assert_eq!(processor.num_ops(), 2);
    }
}
