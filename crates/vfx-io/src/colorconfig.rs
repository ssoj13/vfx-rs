//! OIIO-style color management configuration.
//!
//! This module provides a ColorConfig class that wraps vfx-ocio functionality
//! with an OIIO-compatible API for color space management.
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::ColorConfig;
//!
//! // Load from file
//! let config = ColorConfig::from_file("aces/config.ocio")?;
//!
//! // List color spaces
//! for i in 0..config.num_colorspaces() {
//!     println!("{}", config.colorspace_name_by_index(i));
//! }
//!
//! // Get color space from file path
//! if let Some(cs) = config.colorspace_from_filepath("shot_0010.exr") {
//!     println!("Detected: {}", cs);
//! }
//! ```

use std::path::{Path, PathBuf};
use std::sync::Arc;

use vfx_ocio::{Config, OcioError, OcioResult, Processor};

/// OIIO-style color configuration wrapper.
///
/// Provides a simplified interface to OCIO color management,
/// compatible with OpenImageIO's ColorConfig class.
#[derive(Clone)]
pub struct ColorConfig {
    /// Underlying OCIO config.
    config: Arc<Config>,
    /// Config file path (if loaded from file).
    config_path: Option<PathBuf>,
    /// Whether config was successfully loaded.
    valid: bool,
    /// Error message if loading failed.
    error_message: String,
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl ColorConfig {
    /// Creates an empty/default color configuration.
    ///
    /// Uses the built-in ACES 1.3 config.
    pub fn new() -> Self {
        let config = vfx_ocio::builtin::aces_1_3();
        Self {
            config: Arc::new(config),
            config_path: None,
            valid: true,
            error_message: String::new(),
        }
    }

    /// Creates a ColorConfig from a file path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to an OCIO config file
    pub fn from_file(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        match Config::from_file(path) {
            Ok(config) => Self {
                config: Arc::new(config),
                config_path: Some(path.to_path_buf()),
                valid: true,
                error_message: String::new(),
            },
            Err(e) => Self {
                config: Arc::new(Config::new()),
                config_path: Some(path.to_path_buf()),
                valid: false,
                error_message: e.to_string(),
            },
        }
    }

    /// Creates a ColorConfig from a YAML string.
    ///
    /// # Arguments
    ///
    /// * `yaml_str` - OCIO config as YAML string
    /// * `working_dir` - Base directory for resolving paths
    pub fn from_string(yaml_str: &str, working_dir: impl AsRef<Path>) -> Self {
        let working_dir = working_dir.as_ref().to_path_buf();
        match Config::from_yaml_str(yaml_str, working_dir) {
            Ok(config) => Self {
                config: Arc::new(config),
                config_path: None,
                valid: true,
                error_message: String::new(),
            },
            Err(e) => Self {
                config: Arc::new(Config::new()),
                config_path: None,
                valid: false,
                error_message: e.to_string(),
            },
        }
    }

    /// Uses the built-in ACES 1.3 configuration.
    pub fn aces_1_3() -> Self {
        Self {
            config: Arc::new(vfx_ocio::builtin::aces_1_3()),
            config_path: None,
            valid: true,
            error_message: String::new(),
        }
    }

    /// Uses the built-in sRGB-focused configuration.
    ///
    /// Note: This returns an ACES 1.3 config which includes sRGB color spaces.
    /// For simple sRGB workflows, ACEScg->sRGB transforms are readily available.
    pub fn srgb() -> Self {
        // Use ACES config which includes sRGB color spaces and displays
        Self::aces_1_3()
    }

    /// Checks if the configuration is valid.
    #[inline]
    pub fn valid(&self) -> bool {
        self.valid
    }

    /// Returns the error message if loading failed.
    #[inline]
    pub fn error_message(&self) -> &str {
        &self.error_message
    }

    /// Returns the config file path (if loaded from file).
    #[inline]
    pub fn config_path(&self) -> Option<&Path> {
        self.config_path.as_deref()
    }

    /// Returns the number of color spaces in the config.
    #[inline]
    pub fn num_colorspaces(&self) -> usize {
        self.config.colorspaces().len()
    }

    /// Returns the color space name at the given index.
    ///
    /// Returns an empty string if index is out of range.
    pub fn colorspace_name_by_index(&self, index: usize) -> &str {
        self.config
            .colorspaces()
            .get(index)
            .map(|cs| cs.name())
            .unwrap_or("")
    }

    /// Returns all color space names.
    pub fn colorspace_names(&self) -> Vec<&str> {
        self.config.colorspaces().iter().map(|cs| cs.name()).collect()
    }

    /// Checks if a color space exists in the config.
    pub fn has_colorspace(&self, name: &str) -> bool {
        self.config.colorspace(name).is_some()
    }

    /// Returns the family name for a color space.
    ///
    /// Returns an empty string if not found.
    pub fn colorspace_family_by_name(&self, name: &str) -> &str {
        self.config
            .colorspace(name)
            .map(|cs| cs.family().as_str())
            .unwrap_or("")
    }

    /// Returns the encoding for a color space.
    ///
    /// Returns an empty string if not found.
    pub fn colorspace_encoding_by_name(&self, name: &str) -> &str {
        self.config
            .colorspace(name)
            .map(|cs| cs.encoding().as_str())
            .unwrap_or("")
    }

    /// Checks if a color space is linear.
    ///
    /// A color space is considered linear if its encoding is scene-linear
    /// or display-linear.
    pub fn is_colorspace_linear(&self, name: &str) -> bool {
        self.config
            .colorspace(name)
            .map(|cs| cs.encoding().is_linear())
            .unwrap_or(false)
    }

    /// Checks if a color space represents non-color data.
    pub fn is_colorspace_data(&self, name: &str) -> bool {
        self.config
            .colorspace(name)
            .map(|cs| cs.is_data())
            .unwrap_or(false)
    }

    /// Gets the recommended data type for a color space.
    ///
    /// Returns a string like "half", "float", "uint8", etc.
    pub fn colorspace_data_type(&self, name: &str) -> &str {
        self.config
            .colorspace(name)
            .map(|cs| match cs.bit_depth() {
                vfx_ocio::BitDepth::U8 => "uint8",
                vfx_ocio::BitDepth::U10 => "uint10",
                vfx_ocio::BitDepth::U12 => "uint12",
                vfx_ocio::BitDepth::U16 => "uint16",
                vfx_ocio::BitDepth::U32 => "uint32",
                vfx_ocio::BitDepth::F16 => "half",
                vfx_ocio::BitDepth::F32 => "float",
                _ => "float", // Default for Unknown and any future variants
            })
            .unwrap_or("float")
    }

    /// Gets the color space description.
    pub fn colorspace_description(&self, name: &str) -> &str {
        self.config
            .colorspace(name)
            .map(|cs| cs.description())
            .unwrap_or("")
    }

    /// Determines the appropriate color space from a file path.
    ///
    /// Uses file rules defined in the OCIO config to match the path.
    pub fn colorspace_from_filepath(&self, filepath: &str) -> Option<&str> {
        self.config.colorspace_from_filepath(filepath)
    }

    /// Parses a color space name from a string (e.g., filename).
    ///
    /// Looks for known color space names within the string.
    pub fn parse_colorspace_from_string(&self, text: &str) -> Option<&str> {
        // Check each colorspace name against the text
        let text_lower = text.to_lowercase();

        // First try exact match in the string
        for cs in self.config.colorspaces() {
            let name = cs.name();
            if text.contains(name) {
                return Some(name);
            }
            // Try case-insensitive
            if text_lower.contains(&name.to_lowercase()) {
                return Some(name);
            }
            // Try aliases
            for alias in cs.aliases() {
                if text.contains(alias) || text_lower.contains(&alias.to_lowercase()) {
                    return Some(name);
                }
            }
        }

        // Fall back to file rules
        self.config.colorspace_from_filepath(text)
    }

    // ========================================================================
    // Role access
    // ========================================================================

    /// Gets the color space name for a role.
    ///
    /// Standard roles include: "reference", "scene_linear", "data", etc.
    pub fn role_colorspace(&self, role: &str) -> Option<&str> {
        self.config.roles().get(role)
    }

    /// Checks if a role is defined.
    pub fn has_role(&self, role: &str) -> bool {
        self.config.roles().contains(role)
    }

    /// Returns the number of defined roles.
    pub fn num_roles(&self) -> usize {
        self.config.roles().len()
    }

    /// Gets the scene_linear role color space.
    pub fn scene_linear(&self) -> Option<&str> {
        self.config.roles().scene_linear()
    }

    /// Gets the default input color space.
    pub fn default_input(&self) -> Option<&str> {
        self.config.roles().default_input()
    }

    // ========================================================================
    // Display/View access
    // ========================================================================

    /// Returns the number of displays.
    pub fn num_displays(&self) -> usize {
        self.config.displays().displays().len()
    }

    /// Returns the display name at the given index.
    pub fn display_name_by_index(&self, index: usize) -> &str {
        self.config
            .displays()
            .displays()
            .get(index)
            .map(|d| d.name())
            .unwrap_or("")
    }

    /// Returns all display names.
    pub fn display_names(&self) -> Vec<&str> {
        self.config.displays().display_names().collect()
    }

    /// Returns the default display name.
    pub fn default_display(&self) -> Option<&str> {
        self.config.default_display()
    }

    /// Returns the number of views for a display.
    pub fn num_views(&self, display: &str) -> usize {
        self.config
            .displays()
            .display(display)
            .map(|d| d.views().len())
            .unwrap_or(0)
    }

    /// Returns the view name at the given index for a display.
    pub fn view_name_by_index(&self, display: &str, index: usize) -> &str {
        self.config
            .displays()
            .display(display)
            .and_then(|d| d.views().get(index))
            .map(|v| v.name())
            .unwrap_or("")
    }

    /// Returns the default view for a display.
    pub fn default_view(&self, display: &str) -> Option<&str> {
        self.config.default_view(display)
    }

    /// Returns the color space for a view.
    pub fn view_colorspace(&self, display: &str, view: &str) -> Option<&str> {
        self.config
            .displays()
            .display(display)
            .and_then(|d| d.view(view))
            .map(|v| v.colorspace())
    }

    /// Returns the looks for a view.
    pub fn view_looks(&self, display: &str, view: &str) -> Option<&str> {
        self.config
            .displays()
            .display(display)
            .and_then(|d| d.view(view))
            .and_then(|v| v.looks())
    }

    // ========================================================================
    // Look access
    // ========================================================================

    /// Returns the number of looks.
    pub fn num_looks(&self) -> usize {
        self.config.looks().len()
    }

    /// Returns the look name at the given index.
    pub fn look_name_by_index(&self, index: usize) -> &str {
        self.config
            .looks()
            .all()
            .get(index)
            .map(|l| l.name())
            .unwrap_or("")
    }

    /// Checks if a look exists.
    pub fn has_look(&self, name: &str) -> bool {
        self.config.looks().get(name).is_some()
    }

    // ========================================================================
    // Named Transforms (OCIO v2.0+)
    // ========================================================================

    /// Returns the number of named transforms.
    pub fn num_named_transforms(&self) -> usize {
        self.config.named_transforms().len()
    }

    /// Returns all named transform names.
    pub fn named_transform_names(&self) -> Vec<&str> {
        self.config.named_transforms().iter().map(|nt| nt.name.as_str()).collect()
    }

    /// Checks if a named transform exists.
    pub fn has_named_transform(&self, name: &str) -> bool {
        self.config.named_transform(name).is_some()
    }

    /// Returns the family of a named transform.
    pub fn named_transform_family(&self, name: &str) -> Option<&str> {
        self.config.named_transform(name)
            .and_then(|nt| nt.family.as_deref())
    }

    /// Returns the description of a named transform.
    pub fn named_transform_description(&self, name: &str) -> Option<&str> {
        self.config.named_transform(name)
            .and_then(|nt| nt.description.as_deref())
    }

    // ========================================================================
    // Shared Views (OCIO v2.3+)
    // ========================================================================

    /// Returns the number of shared views.
    pub fn num_shared_views(&self) -> usize {
        self.config.shared_views().len()
    }

    /// Returns all shared view names.
    pub fn shared_view_names(&self) -> Vec<&str> {
        self.config.shared_views().iter().map(|sv| sv.name.as_str()).collect()
    }

    // ========================================================================
    // Processor creation
    // ========================================================================

    /// Creates a processor for converting between color spaces.
    pub fn processor(&self, from: &str, to: &str) -> OcioResult<Processor> {
        self.config.processor(from, to)
    }

    /// Creates a processor with looks applied.
    pub fn processor_with_looks(&self, from: &str, to: &str, looks: &str) -> OcioResult<Processor> {
        self.config.processor_with_looks(from, to, looks)
    }

    /// Creates a display processor.
    pub fn display_processor(
        &self,
        from: &str,
        display: &str,
        view: &str,
    ) -> OcioResult<Processor> {
        self.config.display_processor(from, display, view)
    }

    // ========================================================================
    // Internal access
    // ========================================================================

    /// Returns a reference to the underlying OCIO config.
    #[inline]
    pub fn inner(&self) -> &Config {
        &self.config
    }

    /// Consumes this ColorConfig and returns the underlying Config.
    pub fn into_inner(self) -> Config {
        Arc::try_unwrap(self.config).unwrap_or_else(|arc| (*arc).clone())
    }
}

impl std::fmt::Debug for ColorConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ColorConfig")
            .field("valid", &self.valid)
            .field("config_path", &self.config_path)
            .field("num_colorspaces", &self.num_colorspaces())
            .field("num_displays", &self.num_displays())
            .finish()
    }
}

/// Error type for ColorConfig operations.
#[derive(Debug, Clone)]
#[allow(missing_docs)]  // Enum variant fields are self-documenting
pub enum ColorConfigError {
    /// Configuration file not found.
    ConfigNotFound { path: PathBuf },
    /// Color space not found.
    ColorSpaceNotFound { name: String },
    /// Display not found.
    DisplayNotFound { name: String },
    /// View not found.
    ViewNotFound { display: String, view: String },
    /// Look not found.
    LookNotFound { name: String },
    /// Processing error.
    ProcessingError { message: String },
    /// OCIO error wrapper.
    Ocio(String),
}

impl std::fmt::Display for ColorConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigNotFound { path } => write!(f, "config not found: {}", path.display()),
            Self::ColorSpaceNotFound { name } => write!(f, "color space not found: {}", name),
            Self::DisplayNotFound { name } => write!(f, "display not found: {}", name),
            Self::ViewNotFound { display, view } => {
                write!(f, "view '{}' not found in display '{}'", view, display)
            }
            Self::LookNotFound { name } => write!(f, "look not found: {}", name),
            Self::ProcessingError { message } => write!(f, "processing error: {}", message),
            Self::Ocio(msg) => write!(f, "OCIO error: {}", msg),
        }
    }
}

impl std::error::Error for ColorConfigError {}

impl From<OcioError> for ColorConfigError {
    fn from(e: OcioError) -> Self {
        Self::Ocio(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = ColorConfig::new();
        assert!(config.valid());
        assert!(config.num_colorspaces() > 0);
    }

    #[test]
    fn aces_config() {
        let config = ColorConfig::aces_1_3();
        assert!(config.valid());
        assert!(config.has_colorspace("ACEScg"));
        assert!(config.has_colorspace("sRGB"));
    }

    #[test]
    fn colorspace_info() {
        let config = ColorConfig::aces_1_3();

        assert!(config.is_colorspace_linear("ACEScg"));
        assert!(!config.is_colorspace_linear("sRGB"));

        let family = config.colorspace_family_by_name("ACEScg");
        assert!(!family.is_empty());
    }

    #[test]
    fn role_access() {
        let config = ColorConfig::aces_1_3();

        assert!(config.has_role("scene_linear"));
        let linear = config.scene_linear();
        assert!(linear.is_some());
    }

    #[test]
    fn display_access() {
        let config = ColorConfig::aces_1_3();

        assert!(config.num_displays() > 0);

        if let Some(display) = config.default_display() {
            assert!(config.num_views(display) > 0);
        }
    }

    #[test]
    fn create_processor() {
        let config = ColorConfig::aces_1_3();

        let proc = config.processor("ACEScg", "sRGB");
        assert!(proc.is_ok());
    }

    #[test]
    fn parse_colorspace() {
        let config = ColorConfig::aces_1_3();

        // Should find ACEScg in the filename
        let result = config.parse_colorspace_from_string("render_ACEScg_v001.exr");
        assert!(result.is_some());
    }
}
