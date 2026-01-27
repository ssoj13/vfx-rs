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

use crate::processor::{Processor, ProcessorOp, TransferStyle};
use crate::transform::{CdlStyle, NegativeStyle, ExposureContrastStyle};
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
    /// LUT textures required by the shader.
    textures: Vec<GpuTexture>,
}

/// GPU-compatible operation.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Lut1D/Lut3D reserved for future LUT texture support
enum GpuOp {
    /// Matrix multiply (4x4 matrix as row-major [16], offset [4]).
    Matrix { matrix: [f32; 16], offset: [f32; 4] },
    /// CDL (slope, offset, power, saturation, style).
    Cdl {
        slope: [f32; 3],
        offset: [f32; 3],
        power: [f32; 3],
        saturation: f32,
        style: CdlStyle,
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
    /// 1D LUT (texture index + domain).
    Lut1D {
        texture_idx: usize,
        domain_min: f32,
        domain_max: f32,
    },
    /// 3D LUT (texture index + domain).
    Lut3D {
        texture_idx: usize,
        domain_min: [f32; 3],
        domain_max: [f32; 3],
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
    /// Transfer function (sRGB, PQ, HLG, LogC, etc).
    Transfer {
        style: TransferStyle,
        forward: bool,
    },
    /// Exposure/Contrast adjustment.
    ExposureContrast {
        exposure: f32,
        contrast: f32,
        gamma: f32,
        pivot: f32,
        style: ExposureContrastStyle,
    },
    /// Grading Primary (lift/gamma/gain).
    GradingPrimary {
        lift: [f32; 3],
        gamma: [f32; 3],
        gain: [f32; 3],
        offset: f32,
        exposure: f32,
        contrast: f32,
        saturation: f32,
        pivot: f32,
    },
    /// Grading Tone (shadows/midtones/highlights).
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

impl GpuProcessor {
    /// Creates a GPU processor from a CPU processor.
    pub fn from_processor(processor: &Processor) -> OcioResult<Self> {
        let mut ops = Vec::new();
        let mut textures = Vec::new();

        // Convert processor ops to GPU ops
        for op in processor.ops() {
            if let Some(gpu_op) = Self::convert_op(op, &mut textures) {
                ops.push(gpu_op);
            }
        }

        Ok(Self { ops, textures })
    }

    /// Converts a processor op to a GPU op.
    fn convert_op(op: &ProcessorOp, textures: &mut Vec<GpuTexture>) -> Option<GpuOp> {
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
                style,
            } => Some(GpuOp::Cdl {
                slope: *slope,
                offset: *offset,
                power: *power,
                saturation: *saturation,
                style: *style,
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
            // 1D LUT - create texture and return op with index
            ProcessorOp::Lut1d { lut, size, channels, domain_min, domain_max } => {
                let tex_idx = textures.len();
                let name = format!("ocio_lut1d_{}", tex_idx);
                
                // Convert to RGBA for GPU (expand if needed)
                let mut data = Vec::with_capacity(*size * 4);
                for i in 0..*size {
                    if *channels == 3 {
                        data.push(lut[i * 3]);
                        data.push(lut[i * 3 + 1]);
                        data.push(lut[i * 3 + 2]);
                        data.push(1.0);
                    } else {
                        // Single channel - replicate to RGB
                        let v = lut[i];
                        data.push(v);
                        data.push(v);
                        data.push(v);
                        data.push(1.0);
                    }
                }
                
                textures.push(GpuTexture {
                    name: name.clone(),
                    texture_type: GpuTextureType::Texture1D,
                    width: *size as u32,
                    height: 1,
                    depth: 1,
                    data,
                    interpolation: GpuInterpolation::Linear,
                });
                
                // GPU uses uniform domain; use R channel (typical for most LUTs)
                Some(GpuOp::Lut1D {
                    texture_idx: tex_idx,
                    domain_min: domain_min[0],
                    domain_max: domain_max[0],
                })
            }
            // 3D LUT - create texture and return op with index
            ProcessorOp::Lut3d { lut, size, domain_min, domain_max, .. } => {
                let tex_idx = textures.len();
                let name = format!("ocio_lut3d_{}", tex_idx);
                
                // Convert to RGBA for GPU
                let total = *size * *size * *size;
                let mut data = Vec::with_capacity(total * 4);
                for i in 0..total {
                    data.push(lut[i * 3]);
                    data.push(lut[i * 3 + 1]);
                    data.push(lut[i * 3 + 2]);
                    data.push(1.0);
                }
                
                textures.push(GpuTexture {
                    name: name.clone(),
                    texture_type: GpuTextureType::Texture3D,
                    width: *size as u32,
                    height: *size as u32,
                    depth: *size as u32,
                    data,
                    interpolation: GpuInterpolation::Linear,
                });
                
                Some(GpuOp::Lut3D {
                    texture_idx: tex_idx,
                    domain_min: *domain_min,
                    domain_max: *domain_max,
                })
            }
            // Transfer functions inlined as GLSL math
            ProcessorOp::Transfer { style, forward } => Some(GpuOp::Transfer {
                style: *style,
                forward: *forward,
            }),
            // ExposureContrast GPU support
            ProcessorOp::ExposureContrast {
                exposure,
                contrast,
                gamma,
                pivot,
                style,
            } => Some(GpuOp::ExposureContrast {
                exposure: *exposure,
                contrast: *contrast,
                gamma: *gamma,
                pivot: *pivot,
                style: *style,
            }),
            ProcessorOp::FixedFunction { .. } => None,
            ProcessorOp::Allocation { .. } => None,
            ProcessorOp::GradingPrimary {
                lift,
                gamma,
                gain,
                offset,
                exposure,
                contrast,
                saturation,
                pivot,
                ..
            } => Some(GpuOp::GradingPrimary {
                lift: *lift,
                gamma: *gamma,
                gain: *gain,
                offset: *offset,
                exposure: *exposure,
                contrast: *contrast,
                saturation: *saturation,
                pivot: *pivot,
            }),
            ProcessorOp::GradingRgbCurve { .. } => None, // Requires LUT texture
            ProcessorOp::GradingHueCurve { .. } => None, // Complex HSY curves, no GPU path yet
            ProcessorOp::Aces2OutputTransform { .. } => None, // ACES 2.0 too complex for inline GPU
            ProcessorOp::Aces2RgbJmh { .. } => None,
            ProcessorOp::Aces2TonescaleCompress { .. } => None,
            ProcessorOp::Aces2GamutCompress { .. } => None,
            ProcessorOp::GradingTone {
                shadows,
                midtones,
                highlights,
                whites,
                blacks,
                shadow_start,
                shadow_pivot,
                highlight_start,
                highlight_pivot,
            } => Some(GpuOp::GradingTone {
                shadows: *shadows,
                midtones: *midtones,
                highlights: *highlights,
                whites: *whites,
                blacks: *blacks,
                shadow_start: *shadow_start,
                shadow_pivot: *shadow_pivot,
                highlight_start: *highlight_start,
                highlight_pivot: *highlight_pivot,
            }),
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
            textures: self.textures.clone(),
            uniforms,
        }
    }

    /// Generates GLSL shader code.
    fn generate_glsl(&self, code: &mut String) {
        // Input/output
        writeln!(code, "in vec2 v_texCoord;").unwrap();
        writeln!(code, "out vec4 fragColor;").unwrap();
        writeln!(code, "uniform sampler2D u_inputTexture;").unwrap();
        
        // LUT texture uniforms
        for tex in &self.textures {
            match tex.texture_type {
                GpuTextureType::Texture1D => {
                    writeln!(code, "uniform sampler1D {};", tex.name).unwrap();
                }
                GpuTextureType::Texture2D => {
                    writeln!(code, "uniform sampler2D {};", tex.name).unwrap();
                }
                GpuTextureType::Texture3D => {
                    writeln!(code, "uniform sampler3D {};", tex.name).unwrap();
                }
            }
        }
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
                style,
            } => {
                // Apply slope, offset, power with style-dependent clamping
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
                
                match style {
                    CdlStyle::AscCdl => {
                        // ASC CDL: clamp negatives before power
                        writeln!(code, "    color.rgb = max(color.rgb, vec3(0.0));").unwrap();
                        writeln!(
                            code,
                            "    color.rgb = pow(color.rgb, vec3({:.8}, {:.8}, {:.8}));",
                            power[0], power[1], power[2]
                        )
                        .unwrap();
                    }
                    CdlStyle::NoClamp => {
                        // No clamping: use mirror style for negatives
                        writeln!(
                            code,
                            "    color.rgb = sign(color.rgb) * pow(abs(color.rgb), vec3({:.8}, {:.8}, {:.8}));",
                            power[0], power[1], power[2]
                        )
                        .unwrap();
                    }
                }

                // Saturation (Rec.709 luma - see vfx_core::pixel::REC709_LUMA_*)
                if (*saturation - 1.0).abs() > 1e-6 {
                    writeln!(
                        code,
                        "    float luma = dot(color.rgb, vec3(0.2126, 0.7152, 0.0722));"  // Rec.709
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
                NegativeStyle::Linear => {
                    // Linear extrapolation - same as Mirror for exponent
                    writeln!(
                        code,
                        "    color.rgb = sign(color.rgb) * pow(abs(color.rgb), vec3({:.8}, {:.8}, {:.8}));",
                        value[0], value[1], value[2]
                    )
                    .unwrap();
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
            GpuOp::Lut1D { texture_idx, domain_min, domain_max } => {
                writeln!(code, "    {{ // 1D LUT").unwrap();
                // Normalize input to [0,1] based on domain
                let range = domain_max - domain_min;
                if range.abs() > 1e-10 {
                    writeln!(code, "        vec3 t = (color.rgb - {:.10}) / {:.10};", domain_min, range).unwrap();
                } else {
                    writeln!(code, "        vec3 t = color.rgb;").unwrap();
                }
                writeln!(code, "        t = clamp(t, 0.0, 1.0);").unwrap();
                writeln!(code, "        color.r = texture(ocio_lut1d_{}, t.r).r;", texture_idx).unwrap();
                writeln!(code, "        color.g = texture(ocio_lut1d_{}, t.g).g;", texture_idx).unwrap();
                writeln!(code, "        color.b = texture(ocio_lut1d_{}, t.b).b;", texture_idx).unwrap();
                writeln!(code, "    }}").unwrap();
            }
            GpuOp::Lut3D { texture_idx, domain_min, domain_max } => {
                writeln!(code, "    {{ // 3D LUT").unwrap();
                // Normalize input to [0,1] based on domain
                writeln!(code, "        vec3 d_min = vec3({:.10}, {:.10}, {:.10});", domain_min[0], domain_min[1], domain_min[2]).unwrap();
                writeln!(code, "        vec3 d_max = vec3({:.10}, {:.10}, {:.10});", domain_max[0], domain_max[1], domain_max[2]).unwrap();
                writeln!(code, "        vec3 t = (color.rgb - d_min) / (d_max - d_min);").unwrap();
                writeln!(code, "        t = clamp(t, 0.0, 1.0);").unwrap();
                writeln!(code, "        color.rgb = texture(ocio_lut3d_{}, t).rgb;", texture_idx).unwrap();
                writeln!(code, "    }}").unwrap();
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
                        NegativeStyle::Linear => {
                            // Linear extrapolation for negatives
                            writeln!(code, "        vec3 ac = abs(color.rgb);").unwrap();
                            writeln!(code, "        vec3 pow_val = pow(ac + off, g);").unwrap();
                            writeln!(code, "        vec3 lin_val = lin_s * ac;").unwrap();
                            writeln!(code, "        color.rgb = sign(color.rgb) * mix(lin_val, pow_val, step(brk, ac));").unwrap();
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
                        NegativeStyle::Linear => {
                            // Linear extrapolation for negatives (inverse)
                            writeln!(code, "        vec3 ac = abs(color.rgb);").unwrap();
                            writeln!(code, "        vec3 pow_val = pow(ac, inv_g) - off;").unwrap();
                            writeln!(code, "        vec3 lin_val = ac / lin_s;").unwrap();
                            writeln!(code, "        color.rgb = sign(color.rgb) * mix(lin_val, pow_val, step(brk, ac));").unwrap();
                        }
                    }
                    writeln!(code, "    }}").unwrap();
                }
            }
            GpuOp::Transfer { style, forward } => {
                self.generate_transfer_glsl(code, *style, *forward);
            }
            GpuOp::ExposureContrast {
                exposure,
                contrast,
                gamma,
                pivot,
                style,
            } => {
                let exp_mult = 2.0_f32.powf(*exposure);
                match style {
                    ExposureContrastStyle::Linear => {
                        writeln!(code, "    {{ // ExposureContrast Linear").unwrap();
                        writeln!(code, "        color.rgb *= {:.10};", exp_mult).unwrap();
                        writeln!(code, "        color.rgb = pow(color.rgb / {:.10}, vec3({:.10})) * {:.10};", pivot, contrast, pivot).unwrap();
                        if (*gamma - 1.0).abs() > 1e-6 {
                            writeln!(code, "        color.rgb = pow(max(color.rgb, vec3(0.0)), vec3({:.10}));", gamma).unwrap();
                        }
                        writeln!(code, "    }}").unwrap();
                    }
                    ExposureContrastStyle::Video => {
                        writeln!(code, "    {{ // ExposureContrast Video").unwrap();
                        writeln!(code, "        color.rgb *= {:.10};", exp_mult).unwrap();
                        writeln!(code, "        color.rgb = {:.10} + (color.rgb - {:.10}) * {:.10};", pivot, pivot, contrast).unwrap();
                        if (*gamma - 1.0).abs() > 1e-6 {
                            writeln!(code, "        color.rgb = pow(max(color.rgb, vec3(0.0)), vec3({:.10}));", gamma).unwrap();
                        }
                        writeln!(code, "    }}").unwrap();
                    }
                    ExposureContrastStyle::Logarithmic => {
                        writeln!(code, "    {{ // ExposureContrast Logarithmic").unwrap();
                        writeln!(code, "        color.rgb *= {:.10};", exp_mult).unwrap();
                        writeln!(code, "        vec3 log_v = log(max(color.rgb, vec3(1e-10))) / log(10.0);").unwrap();
                        writeln!(code, "        float log_pivot = log({:.10}) / log(10.0);", pivot.max(1e-10)).unwrap();
                        writeln!(code, "        vec3 adjusted = log_pivot + (log_v - log_pivot) * {:.10};", contrast).unwrap();
                        writeln!(code, "        color.rgb = pow(vec3(10.0), adjusted);").unwrap();
                        if (*gamma - 1.0).abs() > 1e-6 {
                            writeln!(code, "        color.rgb = pow(max(color.rgb, vec3(0.0)), vec3({:.10}));", gamma).unwrap();
                        }
                        writeln!(code, "    }}").unwrap();
                    }
                }
            }
            GpuOp::GradingPrimary {
                lift,
                gamma,
                gain,
                offset,
                exposure,
                contrast,
                saturation,
                pivot,
            } => {
                let exp_mult = 2.0_f32.powf(*exposure);
                writeln!(code, "    {{ // GradingPrimary").unwrap();
                
                // Exposure
                writeln!(code, "        color.rgb *= {:.10};", exp_mult).unwrap();
                
                // Lift/Gamma/Gain: out = (gain * (in + lift * (1 - in)))^(1/gamma)
                writeln!(code, "        vec3 lft = vec3({:.10}, {:.10}, {:.10});", lift[0], lift[1], lift[2]).unwrap();
                writeln!(code, "        vec3 gn = vec3({:.10}, {:.10}, {:.10});", gain[0], gain[1], gain[2]).unwrap();
                writeln!(code, "        vec3 gam = vec3({:.10}, {:.10}, {:.10});", 1.0/gamma[0], 1.0/gamma[1], 1.0/gamma[2]).unwrap();
                writeln!(code, "        vec3 lifted = color.rgb + lft * (1.0 - color.rgb);").unwrap();
                writeln!(code, "        vec3 gained = lifted * gn;").unwrap();
                writeln!(code, "        color.rgb = pow(max(gained, vec3(0.0)), gam);").unwrap();
                
                // Offset
                if offset.abs() > 1e-6 {
                    writeln!(code, "        color.rgb += {:.10};", offset).unwrap();
                }
                
                // Contrast around pivot
                if (*contrast - 1.0).abs() > 1e-6 {
                    writeln!(code, "        color.rgb = {:.10} + (color.rgb - {:.10}) * {:.10};", pivot, pivot, contrast).unwrap();
                }
                
                // Saturation (Rec.709 luma - see vfx_core::pixel::REC709_LUMA_*)
                if (*saturation - 1.0).abs() > 1e-6 {
                    writeln!(code, "        float luma = dot(color.rgb, vec3(0.2126, 0.7152, 0.0722));").unwrap();  // Rec.709
                    writeln!(code, "        color.rgb = luma + (color.rgb - luma) * {:.10};", saturation).unwrap();
                }
                
                writeln!(code, "    }}").unwrap();
            }
            GpuOp::GradingTone {
                shadows,
                midtones,
                highlights,
                whites,
                blacks,
                shadow_start,
                shadow_pivot,
                highlight_start,
                highlight_pivot,
            } => {
                writeln!(code, "    {{ // GradingTone").unwrap();
                // Rec.709 luma - see vfx_core::pixel::REC709_LUMA_*
                writeln!(code, "        float luma = dot(color.rgb, vec3(0.2126, 0.7152, 0.0722));").unwrap();
                
                // Shadow weight
                writeln!(code, "        float shadow_w = luma < {:.10} ? 1.0 : (luma < {:.10} ? 1.0 - (luma - {:.10}) / {:.10} : 0.0);",
                    shadow_start, shadow_pivot, shadow_start, shadow_pivot - shadow_start).unwrap();
                
                // Highlight weight  
                writeln!(code, "        float highlight_w = luma > {:.10} ? 1.0 : (luma > {:.10} ? (luma - {:.10}) / {:.10} : 0.0);",
                    highlight_pivot, highlight_start, highlight_start, highlight_pivot - highlight_start).unwrap();
                
                // Midtone weight
                writeln!(code, "        float midtone_w = max(1.0 - shadow_w - highlight_w, 0.0);").unwrap();
                
                // Blacks offset
                writeln!(code, "        color.rgb += vec3({:.10}, {:.10}, {:.10}) + {:.10};",
                    blacks[0], blacks[1], blacks[2], blacks[3]).unwrap();
                
                // Tonal adjustments
                writeln!(code, "        vec3 shadow_adj = (vec3({:.10}, {:.10}, {:.10}) * {:.10} - 1.0) * shadow_w;",
                    shadows[0], shadows[1], shadows[2], shadows[3]).unwrap();
                writeln!(code, "        vec3 midtone_adj = (vec3({:.10}, {:.10}, {:.10}) * {:.10} - 1.0) * midtone_w;",
                    midtones[0], midtones[1], midtones[2], midtones[3]).unwrap();
                writeln!(code, "        vec3 highlight_adj = (vec3({:.10}, {:.10}, {:.10}) * {:.10} - 1.0) * highlight_w;",
                    highlights[0], highlights[1], highlights[2], highlights[3]).unwrap();
                writeln!(code, "        color.rgb *= 1.0 + shadow_adj + midtone_adj + highlight_adj;").unwrap();
                
                // Whites scale
                writeln!(code, "        color.rgb *= vec3({:.10}, {:.10}, {:.10}) * {:.10};",
                    whites[0], whites[1], whites[2], whites[3]).unwrap();
                
                writeln!(code, "    }}").unwrap();
            }
        }
    }

    /// Generates GLSL for transfer functions.
    fn generate_transfer_glsl(&self, code: &mut String, style: TransferStyle, forward: bool) {
        match style {
            TransferStyle::Linear => {
                // No-op
            }
            TransferStyle::Srgb => {
                if forward {
                    // Linear to sRGB (OETF)
                    writeln!(code, "    {{ // sRGB OETF").unwrap();
                    writeln!(code, "        vec3 lo = color.rgb * 12.92;").unwrap();
                    writeln!(code, "        vec3 hi = 1.055 * pow(max(color.rgb, vec3(0.0)), vec3(1.0/2.4)) - 0.055;").unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3(0.0031308), color.rgb));").unwrap();
                    writeln!(code, "    }}").unwrap();
                } else {
                    // sRGB to linear (EOTF)
                    writeln!(code, "    {{ // sRGB EOTF").unwrap();
                    writeln!(code, "        vec3 lo = color.rgb / 12.92;").unwrap();
                    writeln!(code, "        vec3 hi = pow((color.rgb + 0.055) / 1.055, vec3(2.4));").unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3(0.04045), color.rgb));").unwrap();
                    writeln!(code, "    }}").unwrap();
                }
            }
            TransferStyle::Rec709 => {
                if forward {
                    writeln!(code, "    {{ // Rec.709 OETF").unwrap();
                    writeln!(code, "        vec3 lo = color.rgb * 4.5;").unwrap();
                    writeln!(code, "        vec3 hi = 1.099 * pow(max(color.rgb, vec3(0.0)), vec3(0.45)) - 0.099;").unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3(0.018), color.rgb));").unwrap();
                    writeln!(code, "    }}").unwrap();
                } else {
                    writeln!(code, "    {{ // Rec.709 EOTF").unwrap();
                    writeln!(code, "        vec3 lo = color.rgb / 4.5;").unwrap();
                    writeln!(code, "        vec3 hi = pow((color.rgb + 0.099) / 1.099, vec3(1.0/0.45));").unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3(0.081), color.rgb));").unwrap();
                    writeln!(code, "    }}").unwrap();
                }
            }
            TransferStyle::Rec2020 => {
                const ALPHA: f32 = 1.09929682680944;
                const BETA: f32 = 0.018053968510807;
                if forward {
                    writeln!(code, "    {{ // Rec.2020 OETF").unwrap();
                    writeln!(code, "        vec3 lo = color.rgb * 4.5;").unwrap();
                    writeln!(code, "        vec3 hi = {:.10} * pow(max(color.rgb, vec3(0.0)), vec3(0.45)) - {:.10};", ALPHA, ALPHA - 1.0).unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3({:.10}), color.rgb));", BETA).unwrap();
                    writeln!(code, "    }}").unwrap();
                } else {
                    writeln!(code, "    {{ // Rec.2020 EOTF").unwrap();
                    writeln!(code, "        vec3 lo = color.rgb / 4.5;").unwrap();
                    writeln!(code, "        vec3 hi = pow((color.rgb + {:.10}) / {:.10}, vec3(1.0/0.45));", ALPHA - 1.0, ALPHA).unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3({:.10}), color.rgb));", BETA * 4.5).unwrap();
                    writeln!(code, "    }}").unwrap();
                }
            }
            TransferStyle::Gamma22 => {
                if forward {
                    writeln!(code, "    color.rgb = pow(max(color.rgb, vec3(0.0)), vec3(1.0/2.2));").unwrap();
                } else {
                    writeln!(code, "    color.rgb = pow(max(color.rgb, vec3(0.0)), vec3(2.2));").unwrap();
                }
            }
            TransferStyle::Gamma24 => {
                if forward {
                    writeln!(code, "    color.rgb = pow(max(color.rgb, vec3(0.0)), vec3(1.0/2.4));").unwrap();
                } else {
                    writeln!(code, "    color.rgb = pow(max(color.rgb, vec3(0.0)), vec3(2.4));").unwrap();
                }
            }
            TransferStyle::Gamma26 => {
                if forward {
                    writeln!(code, "    color.rgb = pow(max(color.rgb, vec3(0.0)), vec3(1.0/2.6));").unwrap();
                } else {
                    writeln!(code, "    color.rgb = pow(max(color.rgb, vec3(0.0)), vec3(2.6));").unwrap();
                }
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
                    writeln!(code, "    {{ // PQ OETF").unwrap();
                    writeln!(code, "        vec3 y = pow(max(color.rgb / 10000.0, vec3(0.0)), vec3({:.10}));", M1).unwrap();
                    writeln!(code, "        color.rgb = pow(({:.10} + {:.10} * y) / (1.0 + {:.10} * y), vec3({:.10}));", C1, C2, C3, M2).unwrap();
                    writeln!(code, "    }}").unwrap();
                } else {
                    // PQ to linear
                    writeln!(code, "    {{ // PQ EOTF").unwrap();
                    writeln!(code, "        vec3 vp = pow(max(color.rgb, vec3(0.0)), vec3(1.0/{:.10}));", M2).unwrap();
                    writeln!(code, "        vec3 n = max(vp - {:.10}, vec3(0.0));", C1).unwrap();
                    writeln!(code, "        vec3 d = {:.10} - {:.10} * vp;", C2, C3).unwrap();
                    writeln!(code, "        color.rgb = 10000.0 * pow(n / max(d, vec3(1e-10)), vec3(1.0/{:.10}));", M1).unwrap();
                    writeln!(code, "    }}").unwrap();
                }
            }
            TransferStyle::Hlg => {
                const A: f32 = 0.17883277;
                const B: f32 = 0.28466892;
                const C: f32 = 0.55991073;
                if forward {
                    // Linear to HLG
                    writeln!(code, "    {{ // HLG OETF").unwrap();
                    writeln!(code, "        vec3 lo = sqrt(3.0 * max(color.rgb, vec3(0.0)));").unwrap();
                    writeln!(code, "        vec3 hi = {:.10} * log(12.0 * color.rgb - {:.10}) + {:.10};", A, B, C).unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3(1.0/12.0), color.rgb));").unwrap();
                    writeln!(code, "    }}").unwrap();
                } else {
                    // HLG to linear
                    writeln!(code, "    {{ // HLG EOTF").unwrap();
                    writeln!(code, "        vec3 lo = color.rgb * color.rgb / 3.0;").unwrap();
                    writeln!(code, "        vec3 hi = (exp((color.rgb - {:.10}) / {:.10}) + {:.10}) / 12.0;", C, A, B).unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3(0.5), color.rgb));").unwrap();
                    writeln!(code, "    }}").unwrap();
                }
            }
            TransferStyle::AcesCct => {
                const CUT: f32 = 0.0078125;
                const A: f32 = 10.5402377416545;
                const B: f32 = 0.0729055341958355;
                const LOG_CUT: f32 = 0.155251141552511;
                if forward {
                    writeln!(code, "    {{ // ACEScct encode").unwrap();
                    writeln!(code, "        vec3 lo = {:.10} * color.rgb + {:.10};", A, B).unwrap();
                    writeln!(code, "        vec3 hi = (log2(max(color.rgb, vec3(1e-10))) + 9.72) / 17.52;").unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3({:.10}), color.rgb));", CUT).unwrap();
                    writeln!(code, "    }}").unwrap();
                } else {
                    writeln!(code, "    {{ // ACEScct decode").unwrap();
                    writeln!(code, "        vec3 lo = (color.rgb - {:.10}) / {:.10};", B, A).unwrap();
                    writeln!(code, "        vec3 hi = pow(vec3(2.0), color.rgb * 17.52 - 9.72);").unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3({:.10}), color.rgb));", LOG_CUT).unwrap();
                    writeln!(code, "    }}").unwrap();
                }
            }
            TransferStyle::AcesCc => {
                if forward {
                    writeln!(code, "    {{ // ACEScc encode").unwrap();
                    writeln!(code, "        vec3 t = max(color.rgb, vec3(0.0));").unwrap();
                    writeln!(code, "        vec3 lo = (log2(pow(vec3(2.0), vec3(-16.0)) + t * 0.5) + 9.72) / 17.52;").unwrap();
                    writeln!(code, "        vec3 hi = (log2(t) + 9.72) / 17.52;").unwrap();
                    writeln!(code, "        color.rgb = mix(vec3(-0.3584474886), mix(lo, hi, step(vec3(pow(2.0, -15.0)), t)), step(vec3(0.0), color.rgb));").unwrap();
                    writeln!(code, "    }}").unwrap();
                } else {
                    writeln!(code, "    {{ // ACEScc decode").unwrap();
                    writeln!(code, "        vec3 lo = (pow(vec3(2.0), color.rgb * 17.52 - 9.72) - pow(vec3(2.0), vec3(-16.0))) * 2.0;").unwrap();
                    writeln!(code, "        vec3 hi = pow(vec3(2.0), color.rgb * 17.52 - 9.72);").unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3(-0.3013698630), color.rgb));").unwrap();
                    writeln!(code, "    }}").unwrap();
                }
            }
            TransferStyle::LogC3 => {
                // ARRI LogC3 EI 800
                const CUT: f32 = 0.010591;
                const A: f32 = 5.555556;
                const B: f32 = 0.052272;
                const C: f32 = 0.247190;
                const D: f32 = 0.385537;
                const E: f32 = 5.367655;
                const F: f32 = 0.092809;
                if forward {
                    writeln!(code, "    {{ // LogC3 encode").unwrap();
                    writeln!(code, "        vec3 lo = {:.10} * color.rgb + {:.10};", E, F).unwrap();
                    writeln!(code, "        vec3 hi = {:.10} * log(max({:.10} * color.rgb + {:.10}, vec3(1e-10))) / log(10.0) + {:.10};", C, A, B, D).unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3({:.10}), color.rgb));", CUT).unwrap();
                    writeln!(code, "    }}").unwrap();
                } else {
                    let log_cut = E * CUT + F;
                    writeln!(code, "    {{ // LogC3 decode").unwrap();
                    writeln!(code, "        vec3 lo = (color.rgb - {:.10}) / {:.10};", F, E).unwrap();
                    writeln!(code, "        vec3 hi = (pow(vec3(10.0), (color.rgb - {:.10}) / {:.10}) - {:.10}) / {:.10};", D, C, B, A).unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3({:.10}), color.rgb));", log_cut).unwrap();
                    writeln!(code, "    }}").unwrap();
                }
            }
            TransferStyle::LogC4 => {
                const A: f32 = 2231.82;
                const B: f32 = 0.9071;
                const C: f32 = 0.0929;
                const S: f32 = 8.735;
                if forward {
                    writeln!(code, "    {{ // LogC4 encode").unwrap();
                    writeln!(code, "        vec3 t = max(color.rgb * {:.10}, vec3(0.0)) + 1.0;", A).unwrap();
                    writeln!(code, "        color.rgb = {:.10} * log(t) / {:.10} + {:.10};", B, S, C).unwrap();
                    writeln!(code, "    }}").unwrap();
                } else {
                    writeln!(code, "    {{ // LogC4 decode").unwrap();
                    writeln!(code, "        vec3 t = exp((color.rgb - {:.10}) * {:.10} / {:.10});", C, S, B).unwrap();
                    writeln!(code, "        color.rgb = (t - 1.0) / {:.10};", A).unwrap();
                    writeln!(code, "    }}").unwrap();
                }
            }
            TransferStyle::SLog3 => {
                if forward {
                    writeln!(code, "    {{ // S-Log3 encode").unwrap();
                    writeln!(code, "        vec3 lo = (color.rgb * 76.2102946929 + 95.0) / 1023.0;").unwrap();
                    writeln!(code, "        vec3 hi = (420.0 + log(max(color.rgb * 261.5, vec3(1e-10))) / log(10.0) * 261.5) / 1023.0;").unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3(0.01125), color.rgb));").unwrap();
                    writeln!(code, "    }}").unwrap();
                } else {
                    writeln!(code, "    {{ // S-Log3 decode").unwrap();
                    writeln!(code, "        vec3 x = color.rgb * 1023.0;").unwrap();
                    writeln!(code, "        vec3 lo = (x - 95.0) / 76.2102946929;").unwrap();
                    writeln!(code, "        vec3 hi = pow(vec3(10.0), (x - 420.0) / 261.5) / 261.5 * 0.18;").unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3(171.2102946929), x));").unwrap();
                    writeln!(code, "    }}").unwrap();
                }
            }
            TransferStyle::VLog => {
                const CUT_F: f32 = 0.01;
                const B: f32 = 0.00873;
                const C: f32 = 0.241514;
                const D: f32 = 0.598206;
                if forward {
                    writeln!(code, "    {{ // V-Log encode").unwrap();
                    writeln!(code, "        vec3 lo = 5.6 * color.rgb + 0.125;").unwrap();
                    writeln!(code, "        vec3 hi = {:.10} * log(max(color.rgb + {:.10}, vec3(1e-10))) / log(10.0) + {:.10};", C, B, D).unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3({:.10}), color.rgb));", CUT_F).unwrap();
                    writeln!(code, "    }}").unwrap();
                } else {
                    writeln!(code, "    {{ // V-Log decode").unwrap();
                    writeln!(code, "        vec3 lo = (color.rgb - 0.125) / 5.6;").unwrap();
                    writeln!(code, "        vec3 hi = pow(vec3(10.0), (color.rgb - {:.10}) / {:.10}) - {:.10};", D, C, B).unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3(0.181), color.rgb));").unwrap();
                    writeln!(code, "    }}").unwrap();
                }
            }
            TransferStyle::Log3G10 => {
                const A: f32 = 0.224282;
                const B: f32 = 155.975327;
                const C: f32 = 0.01;
                if forward {
                    writeln!(code, "    {{ // Log3G10 encode").unwrap();
                    writeln!(code, "        vec3 t = abs(color.rgb) * {:.10} + 1.0;", B).unwrap();
                    writeln!(code, "        color.rgb = sign(color.rgb) * {:.10} * log(t) / log(10.0) + {:.10};", A, C).unwrap();
                    writeln!(code, "    }}").unwrap();
                } else {
                    writeln!(code, "    {{ // Log3G10 decode").unwrap();
                    writeln!(code, "        vec3 t = pow(vec3(10.0), (abs(color.rgb) - {:.10}) / {:.10});", C, A).unwrap();
                    writeln!(code, "        color.rgb = sign(color.rgb) * (t - 1.0) / {:.10};", B).unwrap();
                    writeln!(code, "    }}").unwrap();
                }
            }
            TransferStyle::BmdFilmGen5 => {
                const A: f32 = 0.09246575342;
                const B: f32 = 0.5300133392;
                const C: f32 = 0.1496994601;
                if forward {
                    writeln!(code, "    {{ // BMD Film Gen5 encode").unwrap();
                    writeln!(code, "        vec3 lo = color.rgb * {:.10} + {:.10};", A, A).unwrap();
                    writeln!(code, "        vec3 hi = {:.10} * log(max(color.rgb + {:.10}, vec3(1e-10))) + 0.5;", B, C).unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3(0.005), color.rgb));").unwrap();
                    writeln!(code, "    }}").unwrap();
                } else {
                    writeln!(code, "    {{ // BMD Film Gen5 decode").unwrap();
                    writeln!(code, "        vec3 lo = (color.rgb - {:.10}) / {:.10};", A, A).unwrap();
                    writeln!(code, "        vec3 hi = exp((color.rgb - 0.5) / {:.10}) - {:.10};", B, C).unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3(0.09292915127), color.rgb));").unwrap();
                    writeln!(code, "    }}").unwrap();
                }
            }
            TransferStyle::Rec1886 => {
                // Rec.1886: pure gamma 2.4 for broadcast displays
                if forward {
                    writeln!(code, "    color.rgb = pow(max(color.rgb, vec3(0.0)), vec3(1.0/2.4));").unwrap();
                } else {
                    writeln!(code, "    color.rgb = pow(max(color.rgb, vec3(0.0)), vec3(2.4));").unwrap();
                }
            }
            TransferStyle::AppleLog => {
                // Apple Log constants
                const R0: f32 = -0.05641088;
                const RT: f32 = 0.01;
                const C0: f32 = 0.089286;
                const C1: f32 = 0.080886;
                const G: f32 = 0.2568629;
                const D: f32 = 0.486665;
                if forward {
                    writeln!(code, "    {{ // Apple Log encode").unwrap();
                    writeln!(code, "        vec3 t = color.rgb - {:.10};", R0).unwrap();
                    writeln!(code, "        vec3 lo = {:.10} * t + {:.10};", C1 / (RT - R0), C0 - C1 * R0 / (RT - R0)).unwrap();
                    writeln!(code, "        vec3 hi = {:.10} * log(max(t + 1.0, vec3(1e-10))) + {:.10};", G, D).unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3({:.10}), color.rgb));", RT).unwrap();
                    writeln!(code, "    }}").unwrap();
                } else {
                    let pt: f32 = C1 / (RT - R0) * RT + C0 - C1 * R0 / (RT - R0);
                    writeln!(code, "    {{ // Apple Log decode").unwrap();
                    writeln!(code, "        vec3 lo = (color.rgb - {:.10}) / {:.10} + {:.10};", C0 - C1 * R0 / (RT - R0), C1 / (RT - R0), R0).unwrap();
                    writeln!(code, "        vec3 hi = exp((color.rgb - {:.10}) / {:.10}) - 1.0 + {:.10};", D, G, R0).unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3({:.10}), color.rgb));", pt).unwrap();
                    writeln!(code, "    }}").unwrap();
                }
            }
            TransferStyle::CanonCLog2 => {
                // Canon C-Log2 constants
                const A: f32 = 0.092864125;
                const B: f32 = 0.24136077;
                const C: f32 = 87.09937;
                if forward {
                    writeln!(code, "    {{ // Canon C-Log2 encode").unwrap();
                    writeln!(code, "        vec3 t = {:.10} * color.rgb + 1.0;", C).unwrap();
                    writeln!(code, "        color.rgb = {:.10} * log(max(t, vec3(1e-10))) + {:.10};", A, B).unwrap();
                    writeln!(code, "    }}").unwrap();
                } else {
                    writeln!(code, "    {{ // Canon C-Log2 decode").unwrap();
                    writeln!(code, "        vec3 t = exp((color.rgb - {:.10}) / {:.10});", B, A).unwrap();
                    writeln!(code, "        color.rgb = (t - 1.0) / {:.10};", C).unwrap();
                    writeln!(code, "    }}").unwrap();
                }
            }
            TransferStyle::CanonCLog3 => {
                // Canon C-Log3 constants
                const A: f32 = 0.07623209;
                const B: f32 = 0.11602634;
                const C: f32 = 0.3118549;
                const D: f32 = 14.98325;
                if forward {
                    writeln!(code, "    {{ // Canon C-Log3 encode").unwrap();
                    writeln!(code, "        vec3 lo = {:.10} * color.rgb + {:.10};", D * A, C - B - D * A * B / D).unwrap();
                    writeln!(code, "        vec3 hi = {:.10} * log(max(color.rgb + {:.10}, vec3(1e-10))) + {:.10};", A, B, C).unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3(-{:.10}), color.rgb));", B).unwrap();
                    writeln!(code, "    }}").unwrap();
                } else {
                    let pt: f32 = C - B - D * A * B / D + D * A * (-B);
                    writeln!(code, "    {{ // Canon C-Log3 decode").unwrap();
                    writeln!(code, "        vec3 lo = (color.rgb - {:.10} + {:.10} * {:.10} / {:.10}) / ({:.10} * {:.10});", C - B, D, A, D, D, A).unwrap();
                    writeln!(code, "        vec3 hi = exp((color.rgb - {:.10}) / {:.10}) - {:.10};", C, A, B).unwrap();
                    writeln!(code, "        color.rgb = mix(lo, hi, step(vec3({:.10}), color.rgb));", pt).unwrap();
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
