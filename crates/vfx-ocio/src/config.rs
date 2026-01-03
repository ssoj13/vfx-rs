//! OCIO configuration file parsing and management.
//!
//! This module handles loading and parsing `.ocio` configuration files
//! in YAML format. Supports OCIO v1 and v2 config formats.
//!
//! # Example
//!
//! ```ignore
//! use vfx_ocio::Config;
//!
//! // Load from file
//! let config = Config::from_file("aces_1.2/config.ocio")?;
//!
//! // Get color spaces
//! for cs in config.colorspaces() {
//!     println!("{}: {:?}", cs.name(), cs.encoding());
//! }
//!
//! // Create processor
//! let proc = config.processor("ACEScg", "sRGB")?;
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::Deserialize;

use crate::colorspace::{ColorSpace, Encoding, Family};
use crate::context::Context;
use crate::display::{Display, DisplayManager, View, ViewTransform};
use crate::error::{OcioError, OcioResult};
use crate::look::{Look, LookManager};
use crate::processor::{OptimizationLevel, Processor};
use crate::role::Roles;
use crate::transform::*;

/// OCIO configuration.
///
/// The main entry point for color management. A config defines:
/// - Color spaces and their transforms
/// - Roles (semantic mappings)
/// - Displays and views
/// - Looks (creative grades)
#[derive(Debug, Clone)]
pub struct Config {
    /// Config name/description.
    name: String,
    /// Config version (1 or 2).
    version: ConfigVersion,
    /// Search paths for LUTs.
    search_paths: Vec<PathBuf>,
    /// Working directory (config file location).
    working_dir: PathBuf,
    /// All color spaces.
    colorspaces: Vec<ColorSpace>,
    /// Role mappings.
    roles: Roles,
    /// Display/view configuration.
    displays: DisplayManager,
    /// Looks.
    looks: LookManager,
    /// Active displays (subset to show in UI).
    active_displays: Vec<String>,
    /// Active views (subset to show in UI).
    active_views: Vec<String>,
    /// Inactive color spaces (hidden from UI).
    #[allow(dead_code)]
    inactive_colorspaces: Vec<String>,
    /// File rules for automatic color space detection.
    file_rules: Vec<FileRule>,
    /// Environment/context.
    context: Context,
    /// Strict parsing mode.
    #[allow(dead_code)]
    strict_parsing: bool,
}

/// Config format version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfigVersion {
    /// OCIO v1.x.
    #[default]
    V1,
    /// OCIO v2.x.
    V2,
}

/// File rule for automatic color space assignment.
#[derive(Debug, Clone)]
pub struct FileRule {
    /// Rule name.
    pub name: String,
    /// File pattern (glob or regex).
    pub pattern: String,
    /// Extension filter.
    pub extension: Option<String>,
    /// Assigned color space.
    pub colorspace: String,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    /// Creates a new empty configuration.
    pub fn new() -> Self {
        Self {
            name: String::new(),
            version: ConfigVersion::V1,
            search_paths: Vec::new(),
            working_dir: PathBuf::from("."),
            colorspaces: Vec::new(),
            roles: Roles::new(),
            displays: DisplayManager::new(),
            looks: LookManager::new(),
            active_displays: Vec::new(),
            active_views: Vec::new(),
            inactive_colorspaces: Vec::new(),
            file_rules: Vec::new(),
            context: Context::new(),
            strict_parsing: false,
        }
    }

    /// Loads configuration from a file.
    pub fn from_file(path: impl AsRef<Path>) -> OcioResult<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(OcioError::ConfigNotFound {
                path: path.to_path_buf(),
            });
        }

        let content = std::fs::read_to_string(path)?;
        let working_dir = path
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        Self::from_yaml_str(&content, working_dir)
    }

    /// Loads configuration from YAML string.
    pub fn from_yaml_str(yaml: &str, working_dir: PathBuf) -> OcioResult<Self> {
        let raw: RawConfig = serde_yaml::from_str(yaml)?;
        Self::from_raw(raw, working_dir)
    }

    /// Constructs config from parsed raw data.
    fn from_raw(raw: RawConfig, working_dir: PathBuf) -> OcioResult<Self> {
        let version = if raw.ocio_profile_version.starts_with('2') {
            ConfigVersion::V2
        } else if raw.ocio_profile_version.starts_with('1') {
            ConfigVersion::V1
        } else {
            return Err(OcioError::UnsupportedVersion {
                version: raw.ocio_profile_version.clone(),
            });
        };

        let mut config = Self {
            name: raw.name.unwrap_or_default(),
            version,
            working_dir: working_dir.clone(),
            search_paths: raw
                .search_path
                .map(|s| {
                    s.split(':')
                        .filter(|p| !p.is_empty())
                        .map(|p| working_dir.join(p))
                        .collect()
                })
                .unwrap_or_default(),
            colorspaces: Vec::new(),
            roles: Roles::new(),
            displays: DisplayManager::new(),
            looks: LookManager::new(),
            active_displays: raw.active_displays.unwrap_or_default(),
            active_views: raw.active_views.unwrap_or_default(),
            inactive_colorspaces: raw.inactive_colorspaces.unwrap_or_default(),
            file_rules: Vec::new(),
            context: Context::new(),
            strict_parsing: raw.strictparsing.unwrap_or(true),
        };

        // Parse roles
        if let Some(roles) = raw.roles {
            for (role, cs) in roles {
                config.roles.define(role, cs);
            }
        }

        // Parse color spaces
        if let Some(colorspaces) = raw.colorspaces {
            for raw_cs in colorspaces {
                let cs = config.parse_colorspace(raw_cs)?;
                config.colorspaces.push(cs);
            }
        }

        // Parse displays
        if let Some(displays) = raw.displays {
            for (name, views) in displays {
                let mut display = Display::new(&name);
                for raw_view in views {
                    let view = View::new(&raw_view.name, &raw_view.colorspace)
                        .with_look(raw_view.looks.unwrap_or_default());
                    display.add_view(view);
                }
                config.displays.add_display(display);
            }
        }

        // Parse looks
        if let Some(looks) = raw.looks {
            for raw_look in looks {
                let look = Look::new(&raw_look.name)
                    .process_space(raw_look.process_space.unwrap_or_default())
                    .description(raw_look.description.unwrap_or_default());
                config.looks.add(look);
            }
        }

        // Parse view transforms (v2)
        if let Some(view_transforms) = raw.view_transforms {
            for raw_vt in view_transforms {
                let vt = ViewTransform::new(&raw_vt.name)
                    .with_description(raw_vt.description.unwrap_or_default());
                config.displays.add_view_transform(vt);
            }
        }

        // Parse file rules
        if let Some(file_rules) = raw.file_rules {
            for raw_rule in file_rules {
                config.file_rules.push(FileRule {
                    name: raw_rule.name,
                    pattern: raw_rule.pattern.unwrap_or_default(),
                    extension: raw_rule.extension,
                    colorspace: raw_rule.colorspace,
                });
            }
        }

        Ok(config)
    }

    /// Parses a raw colorspace definition.
    fn parse_colorspace(&self, raw: RawColorSpace) -> OcioResult<ColorSpace> {
        let mut builder = ColorSpace::builder(&raw.name);

        if let Some(desc) = raw.description {
            builder = builder.description(desc);
        }

        if let Some(family) = raw.family {
            builder = builder.family(Family::from_str(&family));
        }

        if let Some(encoding) = raw.encoding {
            builder = builder.encoding(Encoding::from_str(&encoding));
        }

        if raw.isdata == Some(true) {
            builder = builder.is_data(true);
        }

        if let Some(aliases) = raw.aliases {
            for alias in aliases {
                builder = builder.alias(alias);
            }
        }

        // TODO: Parse transforms (to_reference, from_reference)

        Ok(builder.build())
    }

    /// Returns config name.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns config version.
    #[inline]
    pub fn version(&self) -> ConfigVersion {
        self.version
    }

    /// Returns the working directory.
    #[inline]
    pub fn working_dir(&self) -> &Path {
        &self.working_dir
    }

    /// Returns all search paths.
    #[inline]
    pub fn search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }

    /// Returns all color spaces.
    #[inline]
    pub fn colorspaces(&self) -> &[ColorSpace] {
        &self.colorspaces
    }

    /// Gets a color space by name (or alias).
    pub fn colorspace(&self, name: &str) -> Option<&ColorSpace> {
        // Check roles first
        if let Some(cs_name) = self.roles.get(name) {
            return self.colorspaces.iter().find(|cs| cs.matches_name(cs_name));
        }
        self.colorspaces.iter().find(|cs| cs.matches_name(name))
    }

    /// Returns color space names.
    pub fn colorspace_names(&self) -> impl Iterator<Item = &str> {
        self.colorspaces.iter().map(|cs| cs.name())
    }

    /// Returns the roles mapping.
    #[inline]
    pub fn roles(&self) -> &Roles {
        &self.roles
    }

    /// Returns the display manager.
    #[inline]
    pub fn displays(&self) -> &DisplayManager {
        &self.displays
    }

    /// Returns the look manager.
    #[inline]
    pub fn looks(&self) -> &LookManager {
        &self.looks
    }

    /// Returns active display names.
    #[inline]
    pub fn active_displays(&self) -> &[String] {
        &self.active_displays
    }

    /// Returns active view names.
    #[inline]
    pub fn active_views(&self) -> &[String] {
        &self.active_views
    }

    /// Returns the default display name.
    pub fn default_display(&self) -> Option<&str> {
        self.active_displays
            .first()
            .map(String::as_str)
            .or_else(|| self.displays.default_display())
    }

    /// Returns the default view for a display.
    pub fn default_view(&self, display: &str) -> Option<&str> {
        self.displays
            .display(display)
            .and_then(|d| d.default_view())
    }

    /// Gets the context.
    #[inline]
    pub fn context(&self) -> &Context {
        &self.context
    }

    /// Gets mutable context.
    #[inline]
    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    /// Creates a processor for conversion between two color spaces.
    pub fn processor(&self, src: &str, dst: &str) -> OcioResult<Processor> {
        self.processor_with_opts(src, dst, OptimizationLevel::default())
    }

    /// Creates a processor with optimization level.
    pub fn processor_with_opts(
        &self,
        src: &str,
        dst: &str,
        _optimization: OptimizationLevel,
    ) -> OcioResult<Processor> {
        let src_cs = self
            .colorspace(src)
            .ok_or_else(|| OcioError::ColorSpaceNotFound { name: src.into() })?;
        let dst_cs = self
            .colorspace(dst)
            .ok_or_else(|| OcioError::ColorSpaceNotFound { name: dst.into() })?;

        // Build transform chain: src -> reference -> dst
        let mut transforms = Vec::new();

        // Source to reference
        if let Some(t) = src_cs.to_reference() {
            transforms.push(t.clone());
        }

        // Reference to destination
        if let Some(t) = dst_cs.from_reference() {
            transforms.push(t.clone());
        }

        if transforms.is_empty() {
            return Ok(Processor::new());
        }

        let group = Transform::group(transforms);
        Processor::from_transform(&group, TransformDirection::Forward)
    }

    /// Creates a display processor.
    pub fn display_processor(
        &self,
        src: &str,
        display: &str,
        view: &str,
    ) -> OcioResult<Processor> {
        let disp = self
            .displays
            .display(display)
            .ok_or_else(|| OcioError::DisplayNotFound {
                name: display.into(),
            })?;

        let v = disp
            .view(view)
            .ok_or_else(|| OcioError::ViewNotFound {
                display: display.into(),
                view: view.into(),
            })?;

        // Get display color space
        let dst = v.colorspace();
        self.processor(src, dst)
    }

    /// Creates a processor with looks applied.
    ///
    /// Looks are applied in the look's process space between src and dst.
    /// Multiple looks can be specified as comma-separated string.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let proc = config.processor_with_looks("ACEScg", "sRGB", "ShowLUT, ShotGrade")?;
    /// ```
    pub fn processor_with_looks(
        &self,
        src: &str,
        dst: &str,
        looks: &str,
    ) -> OcioResult<Processor> {
        use crate::look::parse_looks;
        
        let look_specs = parse_looks(looks);
        if look_specs.is_empty() {
            return self.processor(src, dst);
        }
        
        let mut transforms = Vec::new();
        
        // Source to reference
        let src_cs = self
            .colorspace(src)
            .ok_or_else(|| OcioError::ColorSpaceNotFound { name: src.into() })?;
        if let Some(t) = src_cs.to_reference() {
            transforms.push(t.clone());
        }
        
        // Apply each look
        for (look_name, forward) in look_specs {
            let look = self
                .looks
                .get(look_name)
                .ok_or_else(|| OcioError::LookNotFound { name: look_name.into() })?;
            
            // Convert to process space if specified
            if let Some(ps_name) = look.get_process_space() {
                if let Some(ps) = self.colorspace(ps_name) {
                    if let Some(t) = ps.from_reference() {
                        transforms.push(t.clone());
                    }
                }
            }
            
            // Apply look transform
            let look_transform = if forward {
                look.get_transform()
            } else {
                look.get_inverse_transform().or_else(|| look.get_transform())
            };
            
            if let Some(t) = look_transform {
                if forward {
                    transforms.push(t.clone());
                } else {
                    // Wrap in group with inverse direction
                    transforms.push(Transform::Group(GroupTransform {
                        transforms: vec![t.clone()],
                        direction: TransformDirection::Inverse,
                    }));
                }
            }
            
            // Return from process space
            if let Some(ps_name) = look.get_process_space() {
                if let Some(ps) = self.colorspace(ps_name) {
                    if let Some(t) = ps.to_reference() {
                        transforms.push(t.clone());
                    }
                }
            }
        }
        
        // Reference to destination
        let dst_cs = self
            .colorspace(dst)
            .ok_or_else(|| OcioError::ColorSpaceNotFound { name: dst.into() })?;
        if let Some(t) = dst_cs.from_reference() {
            transforms.push(t.clone());
        }
        
        if transforms.is_empty() {
            return Ok(Processor::new());
        }
        
        let group = Transform::group(transforms);
        Processor::from_transform(&group, TransformDirection::Forward)
    }

    /// Resolves a file path using search paths.
    pub fn resolve_file(&self, filename: &str) -> Option<PathBuf> {
        // Try as absolute path first
        let path = PathBuf::from(filename);
        if path.is_absolute() && path.exists() {
            return Some(path);
        }

        // Try relative to working dir
        let path = self.working_dir.join(filename);
        if path.exists() {
            return Some(path);
        }

        // Try search paths
        for search_path in &self.search_paths {
            let path = search_path.join(filename);
            if path.exists() {
                return Some(path);
            }
        }

        None
    }

    /// Gets color space from file rules.
    pub fn colorspace_from_filepath(&self, filepath: &str) -> Option<&str> {
        for rule in &self.file_rules {
            if let Some(ext) = &rule.extension {
                if !filepath.ends_with(ext) {
                    continue;
                }
            }
            // Simple glob matching
            if rule.pattern.is_empty() || filepath.contains(&rule.pattern) {
                return Some(&rule.colorspace);
            }
        }
        None
    }

    /// Adds a color space to the config.
    pub fn add_colorspace(&mut self, cs: ColorSpace) {
        self.colorspaces.push(cs);
    }

    /// Adds a look to the config.
    pub fn add_look(&mut self, look: Look) {
        self.looks.add(look);
    }

    /// Sets a role mapping.
    pub fn set_role(&mut self, role: impl Into<String>, colorspace: impl Into<String>) {
        self.roles.define(role, colorspace);
    }
}

// ============================================================================
// Raw YAML structures for serde (WIP - for full OCIO config parsing)
// ============================================================================

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RawConfig {
    ocio_profile_version: String,
    name: Option<String>,
    description: Option<String>,
    search_path: Option<String>,
    strictparsing: Option<bool>,
    roles: Option<HashMap<String, String>>,
    colorspaces: Option<Vec<RawColorSpace>>,
    displays: Option<HashMap<String, Vec<RawView>>>,
    active_displays: Option<Vec<String>>,
    active_views: Option<Vec<String>>,
    inactive_colorspaces: Option<Vec<String>>,
    looks: Option<Vec<RawLook>>,
    view_transforms: Option<Vec<RawViewTransform>>,
    file_rules: Option<Vec<RawFileRule>>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RawColorSpace {
    name: String,
    description: Option<String>,
    family: Option<String>,
    encoding: Option<String>,
    bitdepth: Option<String>,
    isdata: Option<bool>,
    aliases: Option<Vec<String>>,
    to_reference: Option<RawTransform>,
    from_reference: Option<RawTransform>,
    to_scene_reference: Option<RawTransform>,
    from_scene_reference: Option<RawTransform>,
    to_display_reference: Option<RawTransform>,
    from_display_reference: Option<RawTransform>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RawView {
    name: String,
    colorspace: String,
    looks: Option<String>,
    view_transform: Option<String>,
    rule: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RawLook {
    name: String,
    process_space: Option<String>,
    description: Option<String>,
    transform: Option<RawTransform>,
    inverse_transform: Option<RawTransform>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RawViewTransform {
    name: String,
    description: Option<String>,
    family: Option<String>,
    from_scene_reference: Option<RawTransform>,
    to_scene_reference: Option<RawTransform>,
    from_display_reference: Option<RawTransform>,
    to_display_reference: Option<RawTransform>,
}

#[derive(Debug, Deserialize)]
struct RawFileRule {
    name: String,
    pattern: Option<String>,
    extension: Option<String>,
    colorspace: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawTransform {
    Single(RawTransformDef),
    Group(Vec<RawTransformDef>),
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RawTransformDef {
    #[serde(rename = "!<MatrixTransform>")]
    matrix: Option<RawMatrixTransform>,
    #[serde(rename = "!<FileTransform>")]
    file: Option<RawFileTransform>,
    #[serde(rename = "!<ExponentTransform>")]
    exponent: Option<RawExponentTransform>,
    #[serde(rename = "!<LogTransform>")]
    log: Option<RawLogTransform>,
    #[serde(rename = "!<CDLTransform>")]
    cdl: Option<RawCdlTransform>,
    #[serde(rename = "!<ColorSpaceTransform>")]
    colorspace: Option<RawColorSpaceTransform>,
    #[serde(rename = "!<BuiltinTransform>")]
    builtin: Option<RawBuiltinTransform>,
    #[serde(rename = "!<RangeTransform>")]
    range: Option<RawRangeTransform>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RawMatrixTransform {
    matrix: Option<Vec<f64>>,
    offset: Option<Vec<f64>>,
    direction: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RawFileTransform {
    src: String,
    cccid: Option<String>,
    interpolation: Option<String>,
    direction: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RawExponentTransform {
    value: Vec<f64>,
    direction: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RawLogTransform {
    base: Option<f64>,
    direction: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RawCdlTransform {
    slope: Option<Vec<f64>>,
    offset: Option<Vec<f64>>,
    power: Option<Vec<f64>>,
    saturation: Option<f64>,
    direction: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RawColorSpaceTransform {
    src: String,
    dst: String,
    direction: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RawBuiltinTransform {
    style: String,
    direction: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RawRangeTransform {
    min_in_value: Option<f64>,
    max_in_value: Option<f64>,
    min_out_value: Option<f64>,
    max_out_value: Option<f64>,
    direction: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_CONFIG: &str = r#"
ocio_profile_version: 2

roles:
  reference: Linear
  scene_linear: Linear
  default: sRGB

colorspaces:
  - name: Linear
    family: Scene
    encoding: scene-linear
    description: Linear reference

  - name: sRGB
    family: Display
    encoding: sdr-video
    description: sRGB display

displays:
  sRGB:
    - name: Raw
      colorspace: Linear
    - name: sRGB
      colorspace: sRGB
"#;

    #[test]
    fn parse_minimal_config() {
        let config = Config::from_yaml_str(MINIMAL_CONFIG, PathBuf::from(".")).unwrap();

        assert_eq!(config.version(), ConfigVersion::V2);
        assert_eq!(config.colorspaces().len(), 2);
        assert!(config.colorspace("Linear").is_some());
        assert!(config.colorspace("sRGB").is_some());
    }

    #[test]
    fn roles_lookup() {
        let config = Config::from_yaml_str(MINIMAL_CONFIG, PathBuf::from(".")).unwrap();

        // Role should resolve to color space
        let cs = config.colorspace("scene_linear").unwrap();
        assert_eq!(cs.name(), "Linear");
    }

    #[test]
    fn displays_parsed() {
        let config = Config::from_yaml_str(MINIMAL_CONFIG, PathBuf::from(".")).unwrap();

        let displays = config.displays();
        assert!(displays.display("sRGB").is_some());

        let display = displays.display("sRGB").unwrap();
        assert_eq!(display.views().len(), 2);
    }

    #[test]
    fn create_processor() {
        let config = Config::from_yaml_str(MINIMAL_CONFIG, PathBuf::from(".")).unwrap();

        // Should not fail even without transforms defined
        let result = config.processor("Linear", "sRGB");
        assert!(result.is_ok());
    }

    #[test]
    fn colorspace_not_found() {
        let config = Config::from_yaml_str(MINIMAL_CONFIG, PathBuf::from(".")).unwrap();

        let result = config.processor("NonExistent", "sRGB");
        assert!(matches!(result, Err(OcioError::ColorSpaceNotFound { .. })));
    }
}
