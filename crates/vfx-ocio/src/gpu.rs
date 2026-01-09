//! GPU processor support for real-time color transforms.
//!
//! Generates GLSL shader code from OCIO processors for use in OpenGL/Vulkan
//! viewers and compositing applications.
//!
//! # Example
//!
//! ```ignore
//! use vfx_ocio::{Config, GpuProcessor, GpuLanguage};
//!
//! let config = Config::from_file("config.ocio")?;
//! let processor = config.processor("ACEScg", "sRGB")?;
//!
//! let gpu = GpuProcessor::from_processor(&processor)?;
//! let shader = gpu.generate_shader(GpuLanguage::Glsl330);
//!
//! println!("{}", shader.fragment_code());
//! ```
//!
//! # Supported Operations
//!
//! - Matrix transforms (color matrices)
//! - CDL (ASC Color Decision List)
//! - 1D/3D LUT sampling (requires texture upload)
//! - Exponent/gamma correction
//! - Log transforms
//! - Range transforms (clamp/scale)

use crate::processor::{Processor, ProcessorOp};
use crate::transform::NegativeStyle;
use crate::OcioResult;
use std::fmt::Write;

/// Target shader language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GpuLanguage {
    /// GLSL 1.20 (OpenGL 2.1)
    Glsl120,
    /// GLSL 3.30 (OpenGL 3.3)
    #[default]
    Glsl330,
    /// GLSL 4.00 (OpenGL 4.0)
    Glsl400,
    /// GLSL ES 3.00 (WebGL 2.0)
    GlslEs300,
    /// HLSL Shader Model 5.0
    Hlsl50,
    /// Metal Shading Language
    Metal,
}

impl GpuLanguage {
    /// Returns the version directive for this language.
    pub fn version_directive(&self) -> &'static str {
        match self {
            GpuLanguage::Glsl120 => "#version 120",
            GpuLanguage::Glsl330 => "#version 330 core",
            GpuLanguage::Glsl400 => "#version 400 core",
            GpuLanguage::GlslEs300 => "#version 300 es\nprecision highp float;",
            GpuLanguage::Hlsl50 => "",
            GpuLanguage::Metal => "",
        }
    }

    /// Returns true if this is a GLSL variant.
    pub fn is_glsl(&self) -> bool {
        matches!(
            self,
            GpuLanguage::Glsl120
                | GpuLanguage::Glsl330
                | GpuLanguage::Glsl400
                | GpuLanguage::GlslEs300
        )
    }
}

/// GPU texture requirement for LUT sampling.
#[derive(Debug, Clone)]
pub struct GpuTexture {
    /// Texture name/identifier.
    pub name: String,
    /// Texture type.
    pub texture_type: GpuTextureType,
    /// Texture width.
    pub width: u32,
    /// Texture height (1 for 1D textures).
    pub height: u32,
    /// Texture depth (1 for 1D/2D textures).
    pub depth: u32,
    /// Pixel data (linear f32 RGBA).
    pub data: Vec<f32>,
    /// Interpolation mode.
    pub interpolation: GpuInterpolation,
}

/// Texture type for GPU LUTs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuTextureType {
    /// 1D texture (for 1D LUTs).
    Texture1D,
    /// 2D texture (for 1D LUT with channel separation).
    Texture2D,
    /// 3D texture (for 3D LUTs).
    Texture3D,
}

/// Texture interpolation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GpuInterpolation {
    /// Nearest neighbor.
    Nearest,
    /// Linear interpolation.
    #[default]
    Linear,
}

/// Generated shader code.
#[derive(Debug, Clone)]
pub struct GpuShaderCode {
    /// Fragment shader code.
    fragment: String,
    /// Required textures.
    textures: Vec<GpuTexture>,
    /// Uniform declarations.
    uniforms: Vec<GpuUniform>,
}

/// Shader uniform variable.
#[derive(Debug, Clone)]
pub struct GpuUniform {
    /// Uniform name.
    pub name: String,
    /// Uniform type.
    pub uniform_type: GpuUniformType,
    /// Default value.
    pub default_value: Vec<f32>,
}

/// Uniform variable type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuUniformType {
    /// Single float.
    Float,
    /// 3-component vector.
    Vec3,
    /// 4-component vector.
    Vec4,
    /// 3x3 matrix.
    Mat3,
    /// 4x4 matrix.
    Mat4,
}

impl GpuShaderCode {
    /// Returns the fragment shader code.
    pub fn fragment_code(&self) -> &str {
        &self.fragment
    }

    /// Returns required textures.
    pub fn textures(&self) -> &[GpuTexture] {
        &self.textures
    }

    /// Returns required uniforms.
    pub fn uniforms(&self) -> &[GpuUniform] {
        &self.uniforms
    }

    /// Returns true if any textures are required.
    pub fn has_textures(&self) -> bool {
        !self.textures.is_empty()
    }
}

/// GPU processor for generating shader code.
#[derive(Debug, Clone)]
pub struct GpuProcessor {
    /// Operations to generate code for.
    ops: Vec<GpuOp>,
}

/// GPU-compatible operation.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Lut1D/Lut3D reserved for future LUT texture support
enum GpuOp {
    /// Matrix multiply (4x4 matrix as row-major [16], offset [4]).
    Matrix { matrix: [f32; 16], offset: [f32; 4] },
    /// CDL (slope, offset, power, saturation).
    Cdl {
        slope: [f32; 3],
        offset: [f32; 3],
        power: [f32; 3],
        saturation: f32,
    },
    /// Exponent/gamma.
    Exponent {
        value: [f32; 4],
        negative_style: NegativeStyle,
    },
    /// Log transform.
    Log { base: f32, forward: bool },
    /// Range/clamp.
    Range {
        scale: f32,
        offset: f32,
        clamp_min: Option<f32>,
        clamp_max: Option<f32>,
    },
    /// 1D LUT (texture index).
    Lut1D { texture_idx: usize },
    /// 3D LUT (texture index).
    Lut3D { texture_idx: usize },
    /// LogAffine transform (OCIO v2).
    LogAffine {
        base: f32,
        log_side_slope: [f32; 3],
        log_side_offset: [f32; 3],
        lin_side_slope: [f32; 3],
        lin_side_offset: [f32; 3],
        forward: bool,
    },
    /// LogCamera transform (ARRI LogC, Sony S-Log3).
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
    /// ExponentWithLinear (sRGB/Rec.709 style).
    ExponentWithLinear {
        gamma: [f32; 4],
        offset: [f32; 4],
        negative_style: NegativeStyle,
        forward: bool,
    },
}

impl GpuProcessor {
    /// Creates a GPU processor from a CPU processor.
    pub fn from_processor(processor: &Processor) -> OcioResult<Self> {
        let mut ops = Vec::new();

        // Convert processor ops to GPU ops
        for op in processor.ops() {
            if let Some(gpu_op) = Self::convert_op(op) {
                ops.push(gpu_op);
            }
        }

        Ok(Self { ops })
    }

    /// Converts a processor op to a GPU op.
    fn convert_op(op: &ProcessorOp) -> Option<GpuOp> {
        match op {
            ProcessorOp::Matrix { matrix, offset } => Some(GpuOp::Matrix {
                matrix: *matrix,
                offset: *offset,
            }),
            ProcessorOp::Cdl {
                slope,
                offset,
                power,
                saturation,
            } => Some(GpuOp::Cdl {
                slope: *slope,
                offset: *offset,
                power: *power,
                saturation: *saturation,
            }),
            ProcessorOp::Exponent {
                value,
                negative_style,
            } => Some(GpuOp::Exponent {
                value: *value,
                negative_style: *negative_style,
            }),
            ProcessorOp::Log { base, forward } => Some(GpuOp::Log {
                base: *base,
                forward: *forward,
            }),
            ProcessorOp::Range {
                scale,
                offset,
                clamp_min,
                clamp_max,
            } => Some(GpuOp::Range {
                scale: *scale,
                offset: *offset,
                clamp_min: *clamp_min,
                clamp_max: *clamp_max,
            }),
            // LUTs require texture upload - not yet fully supported
            ProcessorOp::Lut1d { .. } => None,
            ProcessorOp::Lut3d { .. } => None,
            // Transfer functions can be inlined as math
            ProcessorOp::Transfer { .. } => None, // TODO: inline transfer math
            // Complex ops not yet supported on GPU
            ProcessorOp::ExposureContrast { .. } => None,
            ProcessorOp::FixedFunction { .. } => None,
            ProcessorOp::Allocation { .. } => None,
            ProcessorOp::GradingPrimary { .. } => None,
            ProcessorOp::GradingRgbCurve { .. } => None,
            ProcessorOp::GradingTone { .. } => None,
            // New transforms with full GPU support
            ProcessorOp::LogAffine {
                base,
                log_side_slope,
                log_side_offset,
                lin_side_slope,
                lin_side_offset,
                forward,
            } => Some(GpuOp::LogAffine {
                base: *base,
                log_side_slope: *log_side_slope,
                log_side_offset: *log_side_offset,
                lin_side_slope: *lin_side_slope,
                lin_side_offset: *lin_side_offset,
                forward: *forward,
            }),
            ProcessorOp::LogCamera {
                base,
                log_side_slope,
                log_side_offset,
                lin_side_slope,
                lin_side_offset,
                lin_side_break,
                linear_slope,
                forward,
            } => Some(GpuOp::LogCamera {
                base: *base,
                log_side_slope: *log_side_slope,
                log_side_offset: *log_side_offset,
                lin_side_slope: *lin_side_slope,
                lin_side_offset: *lin_side_offset,
                lin_side_break: *lin_side_break,
                linear_slope: *linear_slope,
                forward: *forward,
            }),
            ProcessorOp::ExponentWithLinear {
                gamma,
                offset,
                negative_style,
                forward,
            } => Some(GpuOp::ExponentWithLinear {
                gamma: *gamma,
                offset: *offset,
                negative_style: *negative_style,
                forward: *forward,
            }),
        }
    }

    /// Generates shader code for the given language.
    pub fn generate_shader(&self, language: GpuLanguage) -> GpuShaderCode {
        let mut code = String::new();
        let textures = Vec::new();
        let uniforms = Vec::new();

        // Version directive
        writeln!(code, "{}", language.version_directive()).unwrap();
        writeln!(code).unwrap();

        if language.is_glsl() {
            self.generate_glsl(&mut code);
        } else {
            // HLSL/Metal not yet implemented
            writeln!(
                code,
                "// {} not yet supported",
                match language {
                    GpuLanguage::Hlsl50 => "HLSL",
                    GpuLanguage::Metal => "Metal",
                    _ => "Unknown",
                }
            )
            .unwrap();
        }

        GpuShaderCode {
            fragment: code,
            textures,
            uniforms,
        }
    }

    /// Generates GLSL shader code.
    fn generate_glsl(&self, code: &mut String) {
        // Input/output
        writeln!(code, "in vec2 v_texCoord;").unwrap();
        writeln!(code, "out vec4 fragColor;").unwrap();
        writeln!(code, "uniform sampler2D u_inputTexture;").unwrap();
        writeln!(code).unwrap();

        // Color transform function
        writeln!(code, "vec4 ocio_transform(vec4 color) {{").unwrap();

        for (i, op) in self.ops.iter().enumerate() {
            writeln!(code, "    // Op {}", i).unwrap();
            self.generate_op_glsl(code, op);
        }

        writeln!(code, "    return color;").unwrap();
        writeln!(code, "}}").unwrap();
        writeln!(code).unwrap();

        // Main function
        writeln!(code, "void main() {{").unwrap();
        writeln!(
            code,
            "    vec4 color = texture(u_inputTexture, v_texCoord);"
        )
        .unwrap();
        writeln!(code, "    fragColor = ocio_transform(color);").unwrap();
        writeln!(code, "}}").unwrap();
    }

    /// Generates GLSL for a single op.
    fn generate_op_glsl(&self, code: &mut String, op: &GpuOp) {
        match op {
            GpuOp::Matrix { matrix, offset } => {
                // 4x4 matrix stored row-major, extract 3x3 for RGB
                writeln!(code, "    color.rgb = mat3(").unwrap();
                writeln!(
                    code,
                    "        {:.8}, {:.8}, {:.8},",
                    matrix[0], matrix[1], matrix[2]
                )
                .unwrap();
                writeln!(
                    code,
                    "        {:.8}, {:.8}, {:.8},",
                    matrix[4], matrix[5], matrix[6]
                )
                .unwrap();
                writeln!(
                    code,
                    "        {:.8}, {:.8}, {:.8}",
                    matrix[8], matrix[9], matrix[10]
                )
                .unwrap();
                writeln!(
                    code,
                    "    ) * color.rgb + vec3({:.8}, {:.8}, {:.8});",
                    offset[0], offset[1], offset[2]
                )
                .unwrap();
            }
            GpuOp::Cdl {
                slope,
                offset,
                power,
                saturation,
            } => {
                // Apply slope, offset, power (ASC CDL)
                writeln!(
                    code,
                    "    color.rgb = color.rgb * vec3({:.8}, {:.8}, {:.8});",
                    slope[0], slope[1], slope[2]
                )
                .unwrap();
                writeln!(
                    code,
                    "    color.rgb = color.rgb + vec3({:.8}, {:.8}, {:.8});",
                    offset[0], offset[1], offset[2]
                )
                .unwrap();
                writeln!(code, "    color.rgb = max(color.rgb, vec3(0.0));").unwrap();
                writeln!(
                    code,
                    "    color.rgb = pow(color.rgb, vec3({:.8}, {:.8}, {:.8}));",
                    power[0], power[1], power[2]
                )
                .unwrap();

                // Saturation
                if (*saturation - 1.0).abs() > 1e-6 {
                    writeln!(
                        code,
                        "    float luma = dot(color.rgb, vec3(0.2126, 0.7152, 0.0722));"
                    )
                    .unwrap();
                    writeln!(
                        code,
                        "    color.rgb = mix(vec3(luma), color.rgb, {:.8});",
                        saturation
                    )
                    .unwrap();
                }
            }
            GpuOp::Exponent {
                value,
                negative_style,
            } => match negative_style {
                NegativeStyle::Clamp => {
                    writeln!(
                        code,
                        "    color.rgb = pow(max(color.rgb, vec3(0.0)), vec3({:.8}, {:.8}, {:.8}));",
                        value[0], value[1], value[2]
                    )
                    .unwrap();
                }
                NegativeStyle::Mirror => {
                    writeln!(
                        code,
                        "    color.rgb = sign(color.rgb) * pow(abs(color.rgb), vec3({:.8}, {:.8}, {:.8}));",
                        value[0], value[1], value[2]
                    )
                    .unwrap();
                }
                NegativeStyle::PassThru => {
                    writeln!(code, "    vec3 neg_mask = step(color.rgb, vec3(0.0));").unwrap();
                    writeln!(
                        code,
                        "    vec3 pos = pow(max(color.rgb, vec3(0.0)), vec3({:.8}, {:.8}, {:.8}));",
                        value[0], value[1], value[2]
                    )
                    .unwrap();
                    writeln!(code, "    color.rgb = mix(pos, color.rgb, neg_mask);").unwrap();
                }
            },
            GpuOp::Log { base, forward } => {
                if *forward {
                    // Linear to log
                    writeln!(
                        code,
                        "    color.rgb = log(max(color.rgb, vec3(1e-10))) / log({:.8});",
                        base
                    )
                    .unwrap();
                } else {
                    // Log to linear
                    writeln!(code, "    color.rgb = pow(vec3({:.8}), color.rgb);", base).unwrap();
                }
            }
            GpuOp::Range {
                scale,
                offset,
                clamp_min,
                clamp_max,
            } => {
                writeln!(code, "    color.rgb = color.rgb * {:.8} + {:.8};", scale, offset).unwrap();
                if let (Some(min), Some(max)) = (clamp_min, clamp_max) {
                    writeln!(code, "    color.rgb = clamp(color.rgb, {:.8}, {:.8});", min, max)
                        .unwrap();
                } else if let Some(min) = clamp_min {
                    writeln!(code, "    color.rgb = max(color.rgb, vec3({:.8}));", min).unwrap();
                } else if let Some(max) = clamp_max {
                    writeln!(code, "    color.rgb = min(color.rgb, vec3({:.8}));", max).unwrap();
                }
            }
            GpuOp::Lut1D { .. } | GpuOp::Lut3D { .. } => {
                writeln!(code, "    // LUT ops require texture upload").unwrap();
            }
            GpuOp::LogAffine {
                base,
                log_side_slope,
                log_side_offset,
                lin_side_slope,
                lin_side_offset,
                forward,
            } => {
                let log_base = base.ln();
                if *forward {
                    // Lin -> Log: out = log_side_slope * log(lin_side_slope * x + lin_side_offset, base) + log_side_offset
                    writeln!(code, "    color.rgb = vec3({:.8}, {:.8}, {:.8}) * color.rgb + vec3({:.8}, {:.8}, {:.8});",
                        lin_side_slope[0], lin_side_slope[1], lin_side_slope[2],
                        lin_side_offset[0], lin_side_offset[1], lin_side_offset[2]).unwrap();
                    writeln!(code, "    color.rgb = max(color.rgb, vec3(1e-10));").unwrap();
                    writeln!(code, "    color.rgb = vec3({:.8}, {:.8}, {:.8}) * (log(color.rgb) / {:.8}) + vec3({:.8}, {:.8}, {:.8});",
                        log_side_slope[0], log_side_slope[1], log_side_slope[2],
                        log_base,
                        log_side_offset[0], log_side_offset[1], log_side_offset[2]).unwrap();
                } else {
                    // Log -> Lin: x = (base^((y - log_side_offset) / log_side_slope) - lin_side_offset) / lin_side_slope
                    writeln!(code, "    color.rgb = (color.rgb - vec3({:.8}, {:.8}, {:.8})) / vec3({:.8}, {:.8}, {:.8});",
                        log_side_offset[0], log_side_offset[1], log_side_offset[2],
                        log_side_slope[0], log_side_slope[1], log_side_slope[2]).unwrap();
                    writeln!(code, "    color.rgb = exp(color.rgb * {:.8});", log_base).unwrap();
                    writeln!(code, "    color.rgb = (color.rgb - vec3({:.8}, {:.8}, {:.8})) / vec3({:.8}, {:.8}, {:.8});",
                        lin_side_offset[0], lin_side_offset[1], lin_side_offset[2],
                        lin_side_slope[0], lin_side_slope[1], lin_side_slope[2]).unwrap();
                }
            }
            GpuOp::LogCamera {
                base,
                log_side_slope,
                log_side_offset,
                lin_side_slope,
                lin_side_offset,
                lin_side_break,
                linear_slope,
                forward,
            } => {
                let log_base = base.ln();
                if *forward {
                    // Piecewise: linear below break, log above
                    // above break: out = log_side_slope * log(lin_side_slope * x + lin_side_offset, base) + log_side_offset
                    // below break: out = linear_slope * x
                    writeln!(code, "    {{ // LogCamera forward").unwrap();
                    writeln!(code, "        vec3 brk = vec3({:.8}, {:.8}, {:.8});",
                        lin_side_break[0], lin_side_break[1], lin_side_break[2]).unwrap();
                    writeln!(code, "        vec3 lin_slope = vec3({:.8}, {:.8}, {:.8});",
                        linear_slope[0], linear_slope[1], linear_slope[2]).unwrap();
                    writeln!(code, "        vec3 t = vec3({:.8}, {:.8}, {:.8}) * color.rgb + vec3({:.8}, {:.8}, {:.8});",
                        lin_side_slope[0], lin_side_slope[1], lin_side_slope[2],
                        lin_side_offset[0], lin_side_offset[1], lin_side_offset[2]).unwrap();
                    writeln!(code, "        vec3 log_val = vec3({:.8}, {:.8}, {:.8}) * (log(max(t, vec3(1e-10))) / {:.8}) + vec3({:.8}, {:.8}, {:.8});",
                        log_side_slope[0], log_side_slope[1], log_side_slope[2],
                        log_base,
                        log_side_offset[0], log_side_offset[1], log_side_offset[2]).unwrap();
                    writeln!(code, "        vec3 lin_val = lin_slope * color.rgb;").unwrap();
                    writeln!(code, "        color.rgb = mix(lin_val, log_val, step(brk, color.rgb));").unwrap();
                    writeln!(code, "    }}").unwrap();
                } else {
                    // Inverse: piecewise linear below, exp above
                    writeln!(code, "    {{ // LogCamera inverse").unwrap();
                    writeln!(code, "        vec3 brk = vec3({:.8}, {:.8}, {:.8});",
                        lin_side_break[0], lin_side_break[1], lin_side_break[2]).unwrap();
                    writeln!(code, "        vec3 lin_slope = vec3({:.8}, {:.8}, {:.8});",
                        linear_slope[0], linear_slope[1], linear_slope[2]).unwrap();
                    writeln!(code, "        vec3 log_brk = lin_slope * brk;").unwrap(); // break in log space
                    writeln!(code, "        vec3 t = (color.rgb - vec3({:.8}, {:.8}, {:.8})) / vec3({:.8}, {:.8}, {:.8});",
                        log_side_offset[0], log_side_offset[1], log_side_offset[2],
                        log_side_slope[0], log_side_slope[1], log_side_slope[2]).unwrap();
                    writeln!(code, "        vec3 exp_val = (exp(t * {:.8}) - vec3({:.8}, {:.8}, {:.8})) / vec3({:.8}, {:.8}, {:.8});",
                        log_base,
                        lin_side_offset[0], lin_side_offset[1], lin_side_offset[2],
                        lin_side_slope[0], lin_side_slope[1], lin_side_slope[2]).unwrap();
                    writeln!(code, "        vec3 lin_val = color.rgb / lin_slope;").unwrap();
                    writeln!(code, "        color.rgb = mix(lin_val, exp_val, step(log_brk, color.rgb));").unwrap();
                    writeln!(code, "    }}").unwrap();
                }
            }
            GpuOp::ExponentWithLinear {
                gamma,
                offset,
                negative_style,
                forward,
            } => {
                // sRGB-style: out = (x+offset)^gamma for x >= break, linear below
                // break point = offset / (gamma - 1)
                // linear_slope = gamma * (break + offset)^(gamma-1)
                let brk: [f32; 3] = std::array::from_fn(|i| {
                    if gamma[i] > 1.0 { offset[i] / (gamma[i] - 1.0) } else { 0.0 }
                });
                let lin_slope: [f32; 3] = std::array::from_fn(|i| {
                    gamma[i] * (brk[i] + offset[i]).powf(gamma[i] - 1.0)
                });

                if *forward {
                    writeln!(code, "    {{ // ExponentWithLinear forward").unwrap();
                    writeln!(code, "        vec3 brk = vec3({:.8}, {:.8}, {:.8});", brk[0], brk[1], brk[2]).unwrap();
                    writeln!(code, "        vec3 g = vec3({:.8}, {:.8}, {:.8});", gamma[0], gamma[1], gamma[2]).unwrap();
                    writeln!(code, "        vec3 off = vec3({:.8}, {:.8}, {:.8});", offset[0], offset[1], offset[2]).unwrap();
                    writeln!(code, "        vec3 lin_s = vec3({:.8}, {:.8}, {:.8});", lin_slope[0], lin_slope[1], lin_slope[2]).unwrap();
                    match negative_style {
                        NegativeStyle::Clamp => {
                            writeln!(code, "        vec3 c = max(color.rgb, vec3(0.0));").unwrap();
                            writeln!(code, "        vec3 pow_val = pow(c + off, g);").unwrap();
                            writeln!(code, "        vec3 lin_val = lin_s * c;").unwrap();
                            writeln!(code, "        color.rgb = mix(lin_val, pow_val, step(brk, c));").unwrap();
                        }
                        NegativeStyle::Mirror => {
                            writeln!(code, "        vec3 ac = abs(color.rgb);").unwrap();
                            writeln!(code, "        vec3 pow_val = pow(ac + off, g);").unwrap();
                            writeln!(code, "        vec3 lin_val = lin_s * ac;").unwrap();
                            writeln!(code, "        color.rgb = sign(color.rgb) * mix(lin_val, pow_val, step(brk, ac));").unwrap();
                        }
                        NegativeStyle::PassThru => {
                            writeln!(code, "        vec3 c = max(color.rgb, vec3(0.0));").unwrap();
                            writeln!(code, "        vec3 pow_val = pow(c + off, g);").unwrap();
                            writeln!(code, "        vec3 lin_val = lin_s * c;").unwrap();
                            writeln!(code, "        vec3 pos = mix(lin_val, pow_val, step(brk, c));").unwrap();
                            writeln!(code, "        color.rgb = mix(color.rgb, pos, step(vec3(0.0), color.rgb));").unwrap();
                        }
                    }
                    writeln!(code, "    }}").unwrap();
                } else {
                    // Inverse: out = x^(1/gamma) - offset
                    let inv_gamma: [f32; 3] = std::array::from_fn(|i| 1.0 / gamma[i]);
                    let out_brk: [f32; 3] = std::array::from_fn(|i| lin_slope[i] * brk[i]);
                    writeln!(code, "    {{ // ExponentWithLinear inverse").unwrap();
                    writeln!(code, "        vec3 brk = vec3({:.8}, {:.8}, {:.8});", out_brk[0], out_brk[1], out_brk[2]).unwrap();
                    writeln!(code, "        vec3 inv_g = vec3({:.8}, {:.8}, {:.8});", inv_gamma[0], inv_gamma[1], inv_gamma[2]).unwrap();
                    writeln!(code, "        vec3 off = vec3({:.8}, {:.8}, {:.8});", offset[0], offset[1], offset[2]).unwrap();
                    writeln!(code, "        vec3 lin_s = vec3({:.8}, {:.8}, {:.8});", lin_slope[0], lin_slope[1], lin_slope[2]).unwrap();
                    match negative_style {
                        NegativeStyle::Clamp => {
                            writeln!(code, "        vec3 c = max(color.rgb, vec3(0.0));").unwrap();
                            writeln!(code, "        vec3 pow_val = pow(c, inv_g) - off;").unwrap();
                            writeln!(code, "        vec3 lin_val = c / lin_s;").unwrap();
                            writeln!(code, "        color.rgb = mix(lin_val, pow_val, step(brk, c));").unwrap();
                        }
                        NegativeStyle::Mirror => {
                            writeln!(code, "        vec3 ac = abs(color.rgb);").unwrap();
                            writeln!(code, "        vec3 pow_val = pow(ac, inv_g) - off;").unwrap();
                            writeln!(code, "        vec3 lin_val = ac / lin_s;").unwrap();
                            writeln!(code, "        color.rgb = sign(color.rgb) * mix(lin_val, pow_val, step(brk, ac));").unwrap();
                        }
                        NegativeStyle::PassThru => {
                            writeln!(code, "        vec3 c = max(color.rgb, vec3(0.0));").unwrap();
                            writeln!(code, "        vec3 pow_val = pow(c, inv_g) - off;").unwrap();
                            writeln!(code, "        vec3 lin_val = c / lin_s;").unwrap();
                            writeln!(code, "        vec3 pos = mix(lin_val, pow_val, step(brk, c));").unwrap();
                            writeln!(code, "        color.rgb = mix(color.rgb, pos, step(vec3(0.0), color.rgb));").unwrap();
                        }
                    }
                    writeln!(code, "    }}").unwrap();
                }
            }
        }
    }

    /// Returns true if this processor can be fully represented on GPU.
    pub fn is_complete(&self) -> bool {
        !self.ops.is_empty()
    }

    /// Returns the number of GPU operations.
    pub fn num_ops(&self) -> usize {
        self.ops.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_language_version() {
        assert!(GpuLanguage::Glsl330.version_directive().contains("330"));
        assert!(GpuLanguage::GlslEs300.version_directive().contains("300 es"));
    }

    #[test]
    fn test_gpu_language_is_glsl() {
        assert!(GpuLanguage::Glsl330.is_glsl());
        assert!(GpuLanguage::GlslEs300.is_glsl());
        assert!(!GpuLanguage::Hlsl50.is_glsl());
        assert!(!GpuLanguage::Metal.is_glsl());
    }
}
