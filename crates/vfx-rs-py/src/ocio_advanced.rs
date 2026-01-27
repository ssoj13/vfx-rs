//! Advanced OCIO bindings - ConfigBuilder, Baker, DynamicProcessor, etc.

use pyo3::prelude::*;
use pyo3::exceptions::{PyValueError, PyIOError, PyRuntimeError};
use numpy::{PyArray1, PyArrayMethods, PyReadwriteArray1};
use std::path::PathBuf;

// ============================================================================
// ConfigBuilder
// ============================================================================

/// Builder for creating OCIO configs programmatically.
///
/// Example:
///     >>> builder = ConfigBuilder("My Config")
///     >>> builder.add_colorspace("linear", "scene_linear", is_data=False)
///     >>> builder.add_colorspace("sRGB", "sdr_video", is_data=False)
///     >>> builder.set_role("scene_linear", "linear")
///     >>> config = builder.build()
#[pyclass]
pub struct ConfigBuilder {
    name: String,
    description: String,
    colorspaces: Vec<ColorSpaceSpec>,
    roles: Vec<(String, String)>,
    displays: Vec<DisplaySpec>,
}

#[derive(Clone)]
struct ColorSpaceSpec {
    name: String,
    family: String,
    encoding: String,
    is_data: bool,
    description: String,
}

#[derive(Clone)]
struct DisplaySpec {
    name: String,
    views: Vec<(String, String)>, // (view_name, colorspace)
}

#[pymethods]
impl ConfigBuilder {
    #[new]
    #[pyo3(signature = (name, description=None))]
    fn new(name: &str, description: Option<&str>) -> Self {
        Self {
            name: name.to_string(),
            description: description.unwrap_or("").to_string(),
            colorspaces: Vec::new(),
            roles: Vec::new(),
            displays: Vec::new(),
        }
    }

    /// Add a color space to the config.
    #[pyo3(signature = (name, family="", encoding="", is_data=false, description=""))]
    fn add_colorspace(
        &mut self,
        name: &str,
        family: &str,
        encoding: &str,
        is_data: bool,
        description: &str,
    ) -> PyResult<()> {
        self.colorspaces.push(ColorSpaceSpec {
            name: name.to_string(),
            family: family.to_string(),
            encoding: encoding.to_string(),
            is_data,
            description: description.to_string(),
        });
        Ok(())
    }

    /// Set a role mapping.
    fn set_role(&mut self, role: &str, colorspace: &str) -> PyResult<()> {
        self.roles.push((role.to_string(), colorspace.to_string()));
        Ok(())
    }

    /// Add a display with views.
    #[pyo3(signature = (display_name, views))]
    fn add_display(&mut self, display_name: &str, views: Vec<(String, String)>) -> PyResult<()> {
        self.displays.push(DisplaySpec {
            name: display_name.to_string(),
            views,
        });
        Ok(())
    }

    /// Build the config.
    fn build(&self) -> PyResult<super::ocio::ColorConfig> {
        use vfx_ocio::{ConfigBuilder as RustBuilder, ColorSpace, Display, View, Encoding};

        let mut builder = RustBuilder::new(&self.name)
            .description(&self.description);

        // Add colorspaces
        for cs in &self.colorspaces {
            let encoding = match cs.encoding.as_str() {
                "scene_linear" => Encoding::SceneLinear,
                "sdr_video" | "sdr" => Encoding::Sdr,
                "hdr_video" | "hdr" => Encoding::Hdr,
                "display_linear" => Encoding::DisplayLinear,
                "log" => Encoding::Log,
                "data" => Encoding::Data,
                _ => Encoding::Unknown,
            };

            let family = match cs.family.as_str() {
                "scene" => vfx_ocio::Family::Scene,
                "display" => vfx_ocio::Family::Display,
                "input" => vfx_ocio::Family::Input,
                "output" => vfx_ocio::Family::Output,
                "utility" => vfx_ocio::Family::Utility,
                _ => vfx_ocio::Family::Scene,
            };

            let colorspace = ColorSpace::builder(&cs.name)
                .family(family)
                .encoding(encoding)
                .is_data(cs.is_data)
                .description(&cs.description)
                .build();

            builder = builder.add_colorspace(colorspace);
        }

        // Add roles
        for (role, cs) in &self.roles {
            builder = builder.set_role(role, cs);
        }

        // Add displays
        for disp in &self.displays {
            let mut display = Display::new(&disp.name);
            for (view_name, colorspace) in &disp.views {
                display.add_view(View::new(view_name, colorspace));
            }
            builder = builder.add_display(display);
        }

        // Build
        let config = builder.build()
            .map_err(|e| PyValueError::new_err(format!("Config build failed: {}", e)))?;

        // Wrap in ColorConfig
        Ok(super::ocio::ColorConfig::from_rust_config(config))
    }

    fn __repr__(&self) -> String {
        format!(
            "ConfigBuilder(name='{}', colorspaces={}, roles={}, displays={})",
            self.name,
            self.colorspaces.len(),
            self.roles.len(),
            self.displays.len()
        )
    }
}

// ============================================================================
// Baker - LUT Baking
// ============================================================================

/// LUT baker for exporting color transforms to .cube files.
///
/// Example:
///     >>> config = ColorConfig.aces_1_3()
///     >>> baker = Baker(config, "ACEScg", "sRGB")
///     >>> baker.bake_cube_1d("output_1d.cube", 4096)
///     >>> baker.bake_cube_3d("output_3d.cube", 65)
#[pyclass]
pub struct Baker {
    config: vfx_ocio::Config,
    src: String,
    dst: String,
}

#[pymethods]
impl Baker {
    #[new]
    fn new(config: &super::ocio::ColorConfig, src: &str, dst: &str) -> PyResult<Self> {
        Ok(Self {
            config: config.inner().config().clone(),
            src: src.to_string(),
            dst: dst.to_string(),
        })
    }

    /// Bake to 1D LUT and write .cube file.
    #[pyo3(signature = (path, size=4096))]
    fn bake_cube_1d(&self, path: PathBuf, size: usize) -> PyResult<()> {
        let processor = self.config.processor(&self.src, &self.dst)
            .map_err(|e| PyRuntimeError::new_err(format!("Processor error: {}", e)))?;

        let baker = vfx_ocio::Baker::new(&processor);
        let lut = baker.bake_lut_1d(size)
            .map_err(|e| PyRuntimeError::new_err(format!("Bake error: {}", e)))?;

        baker.write_cube_1d(&path, &lut)
            .map_err(|e| PyIOError::new_err(format!("Write error: {}", e)))?;

        Ok(())
    }

    /// Bake to 3D LUT and write .cube file.
    #[pyo3(signature = (path, size=65))]
    fn bake_cube_3d(&self, path: PathBuf, size: usize) -> PyResult<()> {
        let processor = self.config.processor(&self.src, &self.dst)
            .map_err(|e| PyRuntimeError::new_err(format!("Processor error: {}", e)))?;

        let baker = vfx_ocio::Baker::new(&processor);
        let lut = baker.bake_lut_3d(size)
            .map_err(|e| PyRuntimeError::new_err(format!("Bake error: {}", e)))?;

        baker.write_cube_3d(&path, &lut)
            .map_err(|e| PyIOError::new_err(format!("Write error: {}", e)))?;

        Ok(())
    }

    /// Bake to 1D LUT with custom domain.
    #[pyo3(signature = (path, size, domain_min, domain_max))]
    fn bake_cube_1d_hdr(&self, path: PathBuf, size: usize, domain_min: f32, domain_max: f32) -> PyResult<()> {
        let processor = self.config.processor(&self.src, &self.dst)
            .map_err(|e| PyRuntimeError::new_err(format!("Processor error: {}", e)))?;

        let baker = vfx_ocio::Baker::new(&processor);
        let lut = baker.bake_lut_1d_with_domain(size, domain_min, domain_max)
            .map_err(|e| PyRuntimeError::new_err(format!("Bake error: {}", e)))?;

        baker.write_cube_1d(&path, &lut)
            .map_err(|e| PyIOError::new_err(format!("Write error: {}", e)))?;

        Ok(())
    }

    /// Bake to 3D LUT with custom domain (for HDR/log).
    #[pyo3(signature = (path, size, domain_min, domain_max))]
    fn bake_cube_3d_hdr(
        &self,
        path: PathBuf,
        size: usize,
        domain_min: [f32; 3],
        domain_max: [f32; 3],
    ) -> PyResult<()> {
        let processor = self.config.processor(&self.src, &self.dst)
            .map_err(|e| PyRuntimeError::new_err(format!("Processor error: {}", e)))?;

        let baker = vfx_ocio::Baker::new(&processor);
        let lut = baker.bake_lut_3d_with_domain(size, domain_min, domain_max)
            .map_err(|e| PyRuntimeError::new_err(format!("Bake error: {}", e)))?;

        baker.write_cube_3d(&path, &lut)
            .map_err(|e| PyIOError::new_err(format!("Write error: {}", e)))?;

        Ok(())
    }

    fn __repr__(&self) -> String {
        format!("Baker(src='{}', dst='{}')", self.src, self.dst)
    }
}

// ============================================================================
// DynamicProcessor - Runtime adjustments
// ============================================================================

/// Processor with runtime exposure/contrast/gamma adjustments.
///
/// Example:
///     >>> config = ColorConfig.aces_1_3()
///     >>> proc = DynamicProcessor(config, "ACEScg", "sRGB")
///     >>> proc.exposure = 1.5  # +1.5 stops
///     >>> proc.contrast = 1.2
///     >>> proc.apply(pixels)
#[pyclass]
pub struct DynamicProcessor {
    inner: vfx_ocio::DynamicProcessor,
}

#[pymethods]
impl DynamicProcessor {
    #[new]
    #[pyo3(signature = (config, src, dst, apply_before=true))]
    fn new(config: &super::ocio::ColorConfig, src: &str, dst: &str, apply_before: bool) -> PyResult<Self> {
        let processor = config.inner().config().processor(src, dst)
            .map_err(|e| PyRuntimeError::new_err(format!("Processor error: {}", e)))?;

        let inner = vfx_ocio::DynamicProcessorBuilder::new()
            .apply_before(apply_before)
            .build(processor);

        Ok(Self { inner })
    }

    /// Set exposure adjustment in stops.
    #[setter]
    fn set_exposure(&mut self, value: f32) {
        self.inner.set_exposure(value);
    }

    #[getter]
    fn exposure(&self) -> f32 {
        self.inner.exposure()
    }

    /// Set contrast multiplier (1.0 = no change).
    #[setter]
    fn set_contrast(&mut self, value: f32) {
        self.inner.set_contrast(value);
    }

    #[getter]
    fn contrast(&self) -> f32 {
        self.inner.contrast()
    }

    /// Set gamma (1.0 = no change).
    #[setter]
    fn set_gamma(&mut self, value: f32) {
        self.inner.set_gamma(value);
    }

    #[getter]
    fn gamma(&self) -> f32 {
        self.inner.gamma()
    }

    /// Set saturation (1.0 = no change, 0.0 = grayscale).
    #[setter]
    fn set_saturation(&mut self, value: f32) {
        self.inner.set_saturation(value);
    }

    #[getter]
    fn saturation(&self) -> f32 {
        self.inner.saturation()
    }

    /// Reset all adjustments to defaults.
    fn reset(&mut self) {
        self.inner.reset();
    }

    /// Apply to RGB pixels (in-place). Expects flat array [r,g,b,r,g,b,...].
    fn apply_rgb<'py>(&self, pixels: &Bound<'py, PyArray1<f32>>) -> PyResult<()> {
        let mut pixels_rw: PyReadwriteArray1<'_, f32> = pixels.readwrite();
        let slice = pixels_rw.as_slice_mut()
            .map_err(|e| PyValueError::new_err(format!("Array error: {}", e)))?;

        if slice.len() % 3 != 0 {
            return Err(PyValueError::new_err("Pixel array length must be multiple of 3"));
        }

        let pixel_count = slice.len() / 3;
        let mut rgb_pixels: Vec<[f32; 3]> = Vec::with_capacity(pixel_count);
        for i in 0..pixel_count {
            rgb_pixels.push([slice[i * 3], slice[i * 3 + 1], slice[i * 3 + 2]]);
        }

        self.inner.apply_rgb(&mut rgb_pixels);

        for (i, p) in rgb_pixels.iter().enumerate() {
            slice[i * 3] = p[0];
            slice[i * 3 + 1] = p[1];
            slice[i * 3 + 2] = p[2];
        }

        Ok(())
    }

    fn __repr__(&self) -> String {
        format!(
            "DynamicProcessor(exposure={}, contrast={}, gamma={}, saturation={})",
            self.inner.exposure(),
            self.inner.contrast(),
            self.inner.gamma(),
            self.inner.saturation()
        )
    }
}

// ============================================================================
// ProcessorCache - Thread-safe caching
// ============================================================================

/// Thread-safe cache for compiled processors.
///
/// Example:
///     >>> cache = ProcessorCache()
///     >>> proc1 = cache.get(config, "ACEScg", "sRGB")  # Compiles
///     >>> proc2 = cache.get(config, "ACEScg", "sRGB")  # Returns cached
///     >>> print(cache.len())  # 1
#[pyclass]
pub struct ProcessorCache {
    inner: vfx_ocio::ProcessorCache,
}

#[pymethods]
impl ProcessorCache {
    #[new]
    fn new() -> Self {
        Self {
            inner: vfx_ocio::ProcessorCache::new(),
        }
    }

    /// Get or create a processor (cached).
    fn get(&self, config: &super::ocio::ColorConfig, src: &str, dst: &str) -> PyResult<OcioProcessor> {
        let processor = self.inner.get_or_create(config.inner().config(), src, dst)
            .map_err(|e| PyRuntimeError::new_err(format!("Processor error: {}", e)))?;
        Ok(OcioProcessor { inner: processor })
    }

    /// Get or create a processor with looks.
    fn get_with_looks(
        &self,
        config: &super::ocio::ColorConfig,
        src: &str,
        dst: &str,
        looks: &str,
    ) -> PyResult<OcioProcessor> {
        let processor = self.inner.get_or_create_with_looks(config.inner().config(), src, dst, looks)
            .map_err(|e| PyRuntimeError::new_err(format!("Processor error: {}", e)))?;
        Ok(OcioProcessor { inner: processor })
    }

    /// Clear all cached processors.
    fn clear(&self) {
        self.inner.clear();
    }

    /// Number of cached processors.
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if cache is empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn __repr__(&self) -> String {
        format!("ProcessorCache(len={})", self.inner.len())
    }
}

/// OCIO Processor wrapper for Python.
#[pyclass]
pub struct OcioProcessor {
    inner: vfx_ocio::Processor,
}

impl OcioProcessor {
    /// Create from a Rust Processor.
    pub fn from_processor(proc: vfx_ocio::Processor) -> Self {
        Self { inner: proc }
    }
}

#[pymethods]
impl OcioProcessor {
    /// Apply to RGB pixels. Expects flat array [r,g,b,r,g,b,...].
    fn apply_rgb<'py>(&self, pixels: &Bound<'py, PyArray1<f32>>) -> PyResult<()> {
        let mut pixels_rw: PyReadwriteArray1<'_, f32> = pixels.readwrite();
        let slice = pixels_rw.as_slice_mut()
            .map_err(|e| PyValueError::new_err(format!("Array error: {}", e)))?;

        if slice.len() % 3 != 0 {
            return Err(PyValueError::new_err("Pixel array length must be multiple of 3"));
        }

        let pixel_count = slice.len() / 3;
        let mut rgb_pixels: Vec<[f32; 3]> = Vec::with_capacity(pixel_count);
        for i in 0..pixel_count {
            rgb_pixels.push([slice[i * 3], slice[i * 3 + 1], slice[i * 3 + 2]]);
        }

        self.inner.apply_rgb(&mut rgb_pixels);

        for (i, p) in rgb_pixels.iter().enumerate() {
            slice[i * 3] = p[0];
            slice[i * 3 + 1] = p[1];
            slice[i * 3 + 2] = p[2];
        }

        Ok(())
    }

    /// Apply to RGBA pixels. Expects flat array [r,g,b,a,r,g,b,a,...].
    fn apply_rgba<'py>(&self, pixels: &Bound<'py, PyArray1<f32>>) -> PyResult<()> {
        let mut pixels_rw: PyReadwriteArray1<'_, f32> = pixels.readwrite();
        let slice = pixels_rw.as_slice_mut()
            .map_err(|e| PyValueError::new_err(format!("Array error: {}", e)))?;

        if slice.len() % 4 != 0 {
            return Err(PyValueError::new_err("Pixel array length must be multiple of 4"));
        }

        let pixel_count = slice.len() / 4;
        let mut rgba_pixels: Vec<[f32; 4]> = Vec::with_capacity(pixel_count);
        for i in 0..pixel_count {
            rgba_pixels.push([slice[i*4], slice[i*4+1], slice[i*4+2], slice[i*4+3]]);
        }

        self.inner.apply_rgba(&mut rgba_pixels);

        for (i, p) in rgba_pixels.iter().enumerate() {
            slice[i*4] = p[0];
            slice[i*4+1] = p[1];
            slice[i*4+2] = p[2];
            slice[i*4+3] = p[3];
        }

        Ok(())
    }

    /// Apply to flat pixel array (auto-detects RGB or RGBA by num_channels).
    ///
    /// Args:
    ///     pixels: Flat numpy array of float32 pixel data
    ///     num_channels: 3 for RGB, 4 for RGBA (default: 3)
    #[pyo3(signature = (pixels, num_channels=3))]
    fn apply<'py>(&self, pixels: &Bound<'py, PyArray1<f32>>, num_channels: usize) -> PyResult<()> {
        match num_channels {
            3 => self.apply_rgb(pixels),
            4 => self.apply_rgba(pixels),
            _ => Err(PyValueError::new_err("num_channels must be 3 or 4")),
        }
    }

    fn __repr__(&self) -> String {
        "OcioProcessor()".to_string()
    }
}

// ============================================================================
// OptimizationLevel Enum
// ============================================================================

/// Optimization level for processors.
///
/// Example:
///     >>> proc = config.processor_optimized("ACEScg", "sRGB", OptimizationLevel.Best)
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    /// No optimization.
    None,
    /// Lossless optimization (matrix combination, identity removal).
    Lossless,
    /// Good quality (may combine LUTs).
    Good,
    /// Best quality.
    Best,
    /// Draft quality (faster, less accurate).
    Draft,
}

impl From<OptimizationLevel> for vfx_ocio::OptimizationLevel {
    fn from(level: OptimizationLevel) -> Self {
        match level {
            OptimizationLevel::None => vfx_ocio::OptimizationLevel::None,
            OptimizationLevel::Lossless => vfx_ocio::OptimizationLevel::Lossless,
            OptimizationLevel::Good => vfx_ocio::OptimizationLevel::Good,
            OptimizationLevel::Best => vfx_ocio::OptimizationLevel::Best,
            OptimizationLevel::Draft => vfx_ocio::OptimizationLevel::Draft,
        }
    }
}

// ============================================================================
// GpuLanguage Enum
// ============================================================================

/// GPU shader language for code generation.
///
/// Example:
///     >>> shader = gpu_proc.generate_shader(GpuLanguage.Glsl330)
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GpuLanguage {
    /// GLSL 1.20 (OpenGL 2.1)
    Glsl120,
    /// GLSL 3.30 (OpenGL 3.3)
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

impl From<GpuLanguage> for vfx_ocio::GpuLanguage {
    fn from(lang: GpuLanguage) -> Self {
        match lang {
            GpuLanguage::Glsl120 => vfx_ocio::GpuLanguage::Glsl120,
            GpuLanguage::Glsl330 => vfx_ocio::GpuLanguage::Glsl330,
            GpuLanguage::Glsl400 => vfx_ocio::GpuLanguage::Glsl400,
            GpuLanguage::GlslEs300 => vfx_ocio::GpuLanguage::GlslEs300,
            GpuLanguage::Hlsl50 => vfx_ocio::GpuLanguage::Hlsl50,
            GpuLanguage::Metal => vfx_ocio::GpuLanguage::Metal,
        }
    }
}

// ============================================================================
// GpuProcessor
// ============================================================================

/// GPU processor for generating shader code.
///
/// Example:
///     >>> config = ColorConfig.aces_1_3()
///     >>> proc = config.processor("ACEScg", "sRGB")
///     >>> gpu_proc = GpuProcessor.from_config(config, "ACEScg", "sRGB")
///     >>> shader = gpu_proc.generate_shader(GpuLanguage.Glsl330)
///     >>> print(shader.fragment_code)
#[pyclass]
pub struct GpuProcessor {
    inner: vfx_ocio::GpuProcessor,
}

#[pymethods]
impl GpuProcessor {
    /// Create GPU processor from config and color spaces.
    #[staticmethod]
    fn from_config(config: &super::ocio::ColorConfig, src: &str, dst: &str) -> PyResult<Self> {
        let processor = config.inner().config().processor(src, dst)
            .map_err(|e| PyRuntimeError::new_err(format!("Processor error: {}", e)))?;
        
        let inner = vfx_ocio::GpuProcessor::from_processor(&processor)
            .map_err(|e| PyRuntimeError::new_err(format!("GPU processor error: {}", e)))?;
        
        Ok(Self { inner })
    }

    /// Generate shader code for the specified language.
    fn generate_shader(&self, language: GpuLanguage) -> GpuShaderCode {
        let code = self.inner.generate_shader(language.into());
        GpuShaderCode { inner: code }
    }

    /// Check if all ops are GPU-compatible.
    fn is_complete(&self) -> bool {
        self.inner.is_complete()
    }

    /// Number of GPU operations.
    fn num_ops(&self) -> usize {
        self.inner.num_ops()
    }

    fn __repr__(&self) -> String {
        format!("GpuProcessor(ops={}, complete={})", self.inner.num_ops(), self.inner.is_complete())
    }
}

// ============================================================================
// GpuShaderCode
// ============================================================================

/// Generated GPU shader code.
#[pyclass]
pub struct GpuShaderCode {
    inner: vfx_ocio::GpuShaderCode,
}

#[pymethods]
impl GpuShaderCode {
    /// Get the fragment shader code.
    #[getter]
    fn fragment_code(&self) -> &str {
        self.inner.fragment_code()
    }

    /// Check if shader requires textures.
    fn has_textures(&self) -> bool {
        self.inner.has_textures()
    }

    fn __repr__(&self) -> String {
        format!("GpuShaderCode(len={}, textures={})", 
            self.inner.fragment_code().len(),
            self.inner.has_textures()
        )
    }
}

// ============================================================================
// Validation
// ============================================================================

/// Config validation issue.
#[pyclass]
#[derive(Clone)]
pub struct ValidationIssue {
    #[pyo3(get)]
    pub severity: String,
    #[pyo3(get)]
    pub category: String,
    #[pyo3(get)]
    pub message: String,
}

#[pymethods]
impl ValidationIssue {
    fn __repr__(&self) -> String {
        format!("[{}] {}: {}", self.severity, self.category, self.message)
    }
}

/// Validate an OCIO config.
///
/// Returns list of issues found.
///
/// Example:
///     >>> issues = validate_config(config)
///     >>> for issue in issues:
///     ...     print(issue)
#[pyfunction]
pub fn validate_config(config: &super::ocio::ColorConfig) -> Vec<ValidationIssue> {
    let issues = vfx_ocio::validate_config(config.inner().config());

    issues
        .iter()
        .map(|issue| ValidationIssue {
            severity: format!("{:?}", issue.severity),
            category: format!("{:?}", issue.category),
            message: issue.message.clone(),
        })
        .collect()
}

/// Check if config has errors.
#[pyfunction]
pub fn config_has_errors(config: &super::ocio::ColorConfig) -> bool {
    let issues = vfx_ocio::validate_config(config.inner().config());
    vfx_ocio::has_errors(&issues)
}

/// Check if config has warnings.
#[pyfunction]
pub fn config_has_warnings(config: &super::ocio::ColorConfig) -> bool {
    let issues = vfx_ocio::validate_config(config.inner().config());
    vfx_ocio::has_warnings(&issues)
}

// ============================================================================
// Module Registration
// ============================================================================

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ConfigBuilder>()?;
    m.add_class::<Baker>()?;
    m.add_class::<DynamicProcessor>()?;
    m.add_class::<ProcessorCache>()?;
    m.add_class::<OcioProcessor>()?;
    m.add_class::<ValidationIssue>()?;
    m.add_class::<OptimizationLevel>()?;
    m.add_class::<GpuLanguage>()?;
    m.add_class::<GpuProcessor>()?;
    m.add_class::<GpuShaderCode>()?;

    m.add_function(wrap_pyfunction!(validate_config, m)?)?;
    m.add_function(wrap_pyfunction!(config_has_errors, m)?)?;
    m.add_function(wrap_pyfunction!(config_has_warnings, m)?)?;

    Ok(())
}
