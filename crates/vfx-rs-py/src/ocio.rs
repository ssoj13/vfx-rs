//! OCIO color management for Python.
//!
//! Provides OpenColorIO-compatible color management operations.

use pyo3::prelude::*;
use pyo3::exceptions::{PyValueError, PyIOError};

use std::path::PathBuf;

use vfx_io::imagebuf::ImageBuf;
use vfx_io::imagebufalgo::ocio as rust_ocio;
use vfx_io::ColorConfig as RustColorConfig;
use vfx_core::Roi3D as RustRoi3D;

use crate::Image;
use crate::core::Roi3D;

// ============================================================================
// Helper Functions
// ============================================================================

fn image_to_imagebuf(img: &Image) -> ImageBuf {
    ImageBuf::from_image_data(img.as_image_data())
}

fn imagebuf_to_image(buf: &ImageBuf) -> PyResult<Image> {
    let data = buf.to_image_data()
        .map_err(|e| PyIOError::new_err(format!("Failed to convert ImageBuf: {}", e)))?;
    Ok(Image::from_image_data(data))
}

fn py_roi_to_rust(roi: &Roi3D) -> RustRoi3D {
    RustRoi3D {
        xbegin: roi.xbegin,
        xend: roi.xend,
        ybegin: roi.ybegin,
        yend: roi.yend,
        zbegin: roi.zbegin,
        zend: roi.zend,
        chbegin: roi.chbegin,
        chend: roi.chend,
    }
}

fn convert_roi(roi: Option<&Roi3D>) -> Option<RustRoi3D> {
    roi.map(py_roi_to_rust)
}

// ============================================================================
// ColorConfig Class
// ============================================================================

/// OCIO color configuration.
///
/// Provides access to color spaces, displays, views, looks, and
/// color transform processors.
///
/// Example:
///     >>> config = ColorConfig()  # Default ACES 1.3
///     >>> config = ColorConfig.from_file("config.ocio")
///     >>> config = ColorConfig.aces_1_3()
///
///     >>> print(config.colorspace_names())
///     >>> print(config.display_names())
///     >>> print(config.is_colorspace_linear("ACEScg"))
#[pyclass]
#[derive(Clone)]
pub struct ColorConfig {
    inner: RustColorConfig,
}

#[pymethods]
impl ColorConfig {
    /// Create a new ColorConfig using the built-in ACES 1.3 configuration.
    #[new]
    pub fn new() -> Self {
        Self {
            inner: RustColorConfig::new(),
        }
    }

    /// Load configuration from file.
    ///
    /// Args:
    ///     path: Path to OCIO config file
    ///
    /// Returns:
    ///     ColorConfig object (check valid() for success)
    #[staticmethod]
    pub fn from_file(path: PathBuf) -> Self {
        Self {
            inner: RustColorConfig::from_file(path),
        }
    }

    /// Load configuration from YAML string.
    ///
    /// Args:
    ///     yaml_str: OCIO config as YAML string
    ///     working_dir: Base directory for resolving paths
    ///
    /// Returns:
    ///     ColorConfig object
    #[staticmethod]
    pub fn from_string(yaml_str: &str, working_dir: PathBuf) -> Self {
        Self {
            inner: RustColorConfig::from_string(yaml_str, working_dir),
        }
    }

    /// Create built-in ACES 1.3 configuration.
    #[staticmethod]
    pub fn aces_1_3() -> Self {
        Self {
            inner: RustColorConfig::aces_1_3(),
        }
    }

    /// Create sRGB-focused configuration (uses ACES 1.3 with sRGB spaces).
    #[staticmethod]
    pub fn srgb() -> Self {
        Self {
            inner: RustColorConfig::srgb(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ColorConfig(valid={}, colorspaces={}, displays={})",
            self.inner.valid(),
            self.inner.num_colorspaces(),
            self.inner.num_displays()
        )
    }

    // ========================================================================
    // Validity and error
    // ========================================================================

    /// Check if configuration is valid.
    pub fn valid(&self) -> bool {
        self.inner.valid()
    }

    /// Get error message if loading failed.
    pub fn error_message(&self) -> String {
        self.inner.error_message().to_string()
    }

    /// Get config file path (if loaded from file).
    pub fn config_path(&self) -> Option<String> {
        self.inner.config_path().map(|p| p.to_string_lossy().to_string())
    }

    // ========================================================================
    // Color spaces
    // ========================================================================

    /// Number of color spaces.
    pub fn num_colorspaces(&self) -> usize {
        self.inner.num_colorspaces()
    }

    /// Get color space name by index.
    pub fn colorspace_name_by_index(&self, index: usize) -> String {
        self.inner.colorspace_name_by_index(index).to_string()
    }

    /// Get all color space names.
    pub fn colorspace_names(&self) -> Vec<String> {
        self.inner.colorspace_names().iter().map(|s| s.to_string()).collect()
    }

    /// Check if color space exists.
    pub fn has_colorspace(&self, name: &str) -> bool {
        self.inner.has_colorspace(name)
    }

    /// Get color space family name.
    pub fn colorspace_family(&self, name: &str) -> String {
        self.inner.colorspace_family_by_name(name).to_string()
    }

    /// Get color space encoding.
    pub fn colorspace_encoding(&self, name: &str) -> String {
        self.inner.colorspace_encoding_by_name(name).to_string()
    }

    /// Check if color space is linear.
    pub fn is_colorspace_linear(&self, name: &str) -> bool {
        self.inner.is_colorspace_linear(name)
    }

    /// Check if color space represents non-color data.
    pub fn is_colorspace_data(&self, name: &str) -> bool {
        self.inner.is_colorspace_data(name)
    }

    /// Get recommended data type for color space.
    pub fn colorspace_data_type(&self, name: &str) -> String {
        self.inner.colorspace_data_type(name).to_string()
    }

    /// Get color space description.
    pub fn colorspace_description(&self, name: &str) -> String {
        self.inner.colorspace_description(name).to_string()
    }

    /// Determine color space from file path using config rules.
    pub fn colorspace_from_filepath(&self, filepath: &str) -> Option<String> {
        self.inner.colorspace_from_filepath(filepath).map(|s| s.to_string())
    }

    /// Parse color space name from a string (e.g., filename).
    pub fn parse_colorspace_from_string(&self, text: &str) -> Option<String> {
        self.inner.parse_colorspace_from_string(text).map(|s| s.to_string())
    }

    // ========================================================================
    // Roles
    // ========================================================================

    /// Get color space name for a role.
    pub fn role_colorspace(&self, role: &str) -> Option<String> {
        self.inner.role_colorspace(role).map(|s| s.to_string())
    }

    /// Check if role is defined.
    pub fn has_role(&self, role: &str) -> bool {
        self.inner.has_role(role)
    }

    /// Number of defined roles.
    pub fn num_roles(&self) -> usize {
        self.inner.num_roles()
    }

    /// Get scene_linear role color space.
    pub fn scene_linear(&self) -> Option<String> {
        self.inner.scene_linear().map(|s| s.to_string())
    }

    /// Get default input color space.
    pub fn default_input(&self) -> Option<String> {
        self.inner.default_input().map(|s| s.to_string())
    }

    // ========================================================================
    // Displays and views
    // ========================================================================

    /// Number of displays.
    pub fn num_displays(&self) -> usize {
        self.inner.num_displays()
    }

    /// Get display name by index.
    pub fn display_name_by_index(&self, index: usize) -> String {
        self.inner.display_name_by_index(index).to_string()
    }

    /// Get all display names.
    pub fn display_names(&self) -> Vec<String> {
        self.inner.display_names().iter().map(|s| s.to_string()).collect()
    }

    /// Get default display name.
    pub fn default_display(&self) -> Option<String> {
        self.inner.default_display().map(|s| s.to_string())
    }

    /// Number of views for a display.
    pub fn num_views(&self, display: &str) -> usize {
        self.inner.num_views(display)
    }

    /// Get view name by index.
    pub fn view_name_by_index(&self, display: &str, index: usize) -> String {
        self.inner.view_name_by_index(display, index).to_string()
    }

    /// Get default view for a display.
    pub fn default_view(&self, display: &str) -> Option<String> {
        self.inner.default_view(display).map(|s| s.to_string())
    }

    /// Get color space for a view.
    pub fn view_colorspace(&self, display: &str, view: &str) -> Option<String> {
        self.inner.view_colorspace(display, view).map(|s| s.to_string())
    }

    /// Get looks for a view.
    pub fn view_looks(&self, display: &str, view: &str) -> Option<String> {
        self.inner.view_looks(display, view).map(|s| s.to_string())
    }

    // ========================================================================
    // Looks
    // ========================================================================

    /// Number of looks.
    pub fn num_looks(&self) -> usize {
        self.inner.num_looks()
    }

    /// Get look name by index.
    pub fn look_name_by_index(&self, index: usize) -> String {
        self.inner.look_name_by_index(index).to_string()
    }

    /// Check if look exists.
    pub fn has_look(&self, name: &str) -> bool {
        self.inner.has_look(name)
    }

}

impl ColorConfig {
    pub fn inner(&self) -> &RustColorConfig {
        &self.inner
    }
}

// ============================================================================
// Color Conversion Functions
// ============================================================================

/// Convert image from one color space to another.
///
/// Args:
///     image: Input image
///     from_space: Source color space name
///     to_space: Destination color space name
///     config: Optional ColorConfig (uses default ACES if None)
///     roi: Optional region of interest
///
/// Returns:
///     Converted image
///
/// Example:
///     >>> srgb = colorconvert(linear, "ACEScg", "sRGB")
#[pyfunction]
#[pyo3(signature = (image, from_space, to_space, config=None, roi=None))]
pub fn colorconvert(
    image: &Image,
    from_space: &str,
    to_space: &str,
    config: Option<&ColorConfig>,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let rust_config = config.map(|c| c.inner());
    let result = rust_ocio::colorconvert(&buf, from_space, to_space, rust_config, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Apply display transform to an image.
///
/// Args:
///     image: Input image
///     display: Display name (e.g., "sRGB", "Rec.709")
///     view: View name (e.g., "Film", "Raw")
///     from_space: Source color space name
///     config: Optional ColorConfig
///     roi: Optional region of interest
///
/// Returns:
///     Display-transformed image
///
/// Example:
///     >>> display_img = ociodisplay(linear, "sRGB", "Film", "ACEScg")
#[pyfunction]
#[pyo3(signature = (image, display, view, from_space, config=None, roi=None))]
pub fn ociodisplay(
    image: &Image,
    display: &str,
    view: &str,
    from_space: &str,
    config: Option<&ColorConfig>,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let rust_config = config.map(|c| c.inner());
    let result = rust_ocio::ociodisplay(&buf, display, view, from_space, rust_config, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Apply a look transform to an image.
///
/// Args:
///     image: Input image
///     looks: Look specification (e.g., "+FilmGrade", "-ContrastBoost")
///     from_space: Source color space name
///     to_space: Destination color space name
///     config: Optional ColorConfig
///     roi: Optional region of interest
///
/// Look Syntax:
///     - +LookName: Apply look forward
///     - -LookName: Apply look inverse
///     - Multiple looks: "+GradeA, +GradeB"
///
/// Returns:
///     Look-transformed image
///
/// Example:
///     >>> graded = ociolook(img, "+ShowLUT", "ACEScg", "ACEScg")
#[pyfunction]
#[pyo3(signature = (image, looks, from_space, to_space, config=None, roi=None))]
pub fn ociolook(
    image: &Image,
    looks: &str,
    from_space: &str,
    to_space: &str,
    config: Option<&ColorConfig>,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let rust_config = config.map(|c| c.inner());
    let result = rust_ocio::ociolook(&buf, looks, from_space, to_space, rust_config, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Apply a file-based transform (LUT file).
///
/// Args:
///     image: Input image
///     filename: Path to LUT file (.cube, .csp, .clf, etc.)
///     inverse: Apply inverse transform (default False)
///     config: Optional ColorConfig (used for resolving paths)
///     roi: Optional region of interest
///
/// Returns:
///     Transformed image
///
/// Example:
///     >>> graded = ociofiletransform(img, "grade.cube")
///     >>> ungraded = ociofiletransform(img, "grade.cube", inverse=True)
#[pyfunction]
#[pyo3(signature = (image, filename, inverse=false, config=None, roi=None))]
pub fn ociofiletransform(
    image: &Image,
    filename: &str,
    inverse: bool,
    config: Option<&ColorConfig>,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let buf = image_to_imagebuf(image);
    let rust_config = config.map(|c| c.inner());
    let result = rust_ocio::ociofiletransform(&buf, filename, inverse, rust_config, convert_roi(roi));
    imagebuf_to_image(&result)
}

/// Check if two color space names are equivalent.
///
/// Args:
///     name1: First color space name
///     name2: Second color space name
///     config: Optional ColorConfig
///
/// Returns:
///     True if both names refer to the same color space
///
/// Example:
///     >>> equivalent_colorspace("scene_linear", "ACEScg", config)
///     True
#[pyfunction]
#[pyo3(signature = (name1, name2, config=None))]
pub fn equivalent_colorspace(
    name1: &str,
    name2: &str,
    config: Option<&ColorConfig>,
) -> bool {
    let rust_config = config.map(|c| c.inner());
    rust_ocio::equivalent_colorspace(name1, name2, rust_config)
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Convert from ACEScg to sRGB.
///
/// Args:
///     image: Input image in ACEScg
///     roi: Optional region of interest
///
/// Returns:
///     Image in sRGB
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn acescg_to_srgb(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    colorconvert(image, "ACEScg", "sRGB", None, roi)
}

/// Convert from sRGB to ACEScg.
///
/// Args:
///     image: Input image in sRGB
///     roi: Optional region of interest
///
/// Returns:
///     Image in ACEScg
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn srgb_to_acescg(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    colorconvert(image, "sRGB", "ACEScg", None, roi)
}

/// Convert from ACEScg to ACES2065-1 (ACES reference).
///
/// Args:
///     image: Input image in ACEScg
///     roi: Optional region of interest
///
/// Returns:
///     Image in ACES2065-1
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn acescg_to_aces(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    colorconvert(image, "ACEScg", "ACES2065-1", None, roi)
}

/// Convert from ACES2065-1 to ACEScg.
///
/// Args:
///     image: Input image in ACES2065-1
///     roi: Optional region of interest
///
/// Returns:
///     Image in ACEScg
#[pyfunction]
#[pyo3(signature = (image, roi=None))]
pub fn aces_to_acescg(image: &Image, roi: Option<&Roi3D>) -> PyResult<Image> {
    colorconvert(image, "ACES2065-1", "ACEScg", None, roi)
}

/// List all color spaces in default ACES config.
#[pyfunction]
pub fn list_colorspaces() -> Vec<String> {
    let config = RustColorConfig::aces_1_3();
    config.colorspace_names().iter().map(|s| s.to_string()).collect()
}

/// List all displays in default ACES config.
#[pyfunction]
pub fn list_displays() -> Vec<String> {
    let config = RustColorConfig::aces_1_3();
    config.display_names().iter().map(|s| s.to_string()).collect()
}

// ============================================================================
// Module Registration
// ============================================================================

/// Register all OCIO functions to the module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Classes
    m.add_class::<ColorConfig>()?;

    // Color conversion functions
    m.add_function(wrap_pyfunction!(colorconvert, m)?)?;
    m.add_function(wrap_pyfunction!(ociodisplay, m)?)?;
    m.add_function(wrap_pyfunction!(ociolook, m)?)?;
    m.add_function(wrap_pyfunction!(ociofiletransform, m)?)?;
    m.add_function(wrap_pyfunction!(equivalent_colorspace, m)?)?;

    // Convenience functions
    m.add_function(wrap_pyfunction!(acescg_to_srgb, m)?)?;
    m.add_function(wrap_pyfunction!(srgb_to_acescg, m)?)?;
    m.add_function(wrap_pyfunction!(acescg_to_aces, m)?)?;
    m.add_function(wrap_pyfunction!(aces_to_acescg, m)?)?;
    m.add_function(wrap_pyfunction!(list_colorspaces, m)?)?;
    m.add_function(wrap_pyfunction!(list_displays, m)?)?;

    Ok(())
}
