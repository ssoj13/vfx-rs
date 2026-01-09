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

use std::path::{Path, PathBuf};
use glob::Pattern;
use regex::Regex;
use saphyr::{Yaml, LoadableYamlNode, Scalar};

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
    /// Shared views (OCIO v2.3+).
    shared_views: Vec<SharedView>,
    /// Viewing rules (OCIO v2.0+) - filter views by colorspace encoding.
    viewing_rules: Vec<ViewingRule>,
    /// Named transforms (OCIO v2.0+).
    named_transforms: Vec<NamedTransform>,
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
    /// Assigned color space.
    pub colorspace: String,
    /// Rule matching kind.
    pub kind: FileRuleKind,
}

/// Named transform definition (OCIO v2.0+).
/// Standalone transforms that can be referenced by name.
#[derive(Debug, Clone)]
pub struct NamedTransform {
    /// Transform name.
    pub name: String,
    /// Family grouping.
    pub family: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// Forward transform.
    pub forward: Option<Transform>,
    /// Inverse transform.
    pub inverse: Option<Transform>,
}

/// Shared view definition (OCIO v2.3+).
/// Shared views can be referenced by multiple displays.
#[derive(Debug, Clone)]
pub struct SharedView {
    /// View name.
    pub name: String,
    /// View transform (optional).
    pub view_transform: Option<String>,
    /// Display color space.
    pub display_colorspace: String,
    /// Look (optional).
    pub looks: Option<String>,
    /// Rule (optional) - references a viewing_rule by name.
    pub rule: Option<String>,
    /// Description (optional).
    pub description: Option<String>,
}

/// Viewing rule definition (OCIO v2.0+).
/// Rules filter which views are applicable based on colorspace encoding.
#[derive(Debug, Clone)]
pub struct ViewingRule {
    /// Rule name (referenced by shared_views).
    pub name: String,
    /// List of colorspace names this rule applies to.
    /// Mutually exclusive with encodings.
    pub colorspaces: Vec<String>,
    /// List of encoding types this rule applies to (e.g., "log", "scene-linear").
    /// Mutually exclusive with colorspaces.
    pub encodings: Vec<String>,
    /// Custom key-value pairs.
    pub custom_keys: Vec<(String, String)>,
}

/// File rule matching behavior.
#[derive(Debug, Clone)]
pub enum FileRuleKind {
    /// Basic rule: glob pattern + optional extension.
    Basic {
        pattern: String,
        extension: Option<String>,
    },
    /// Regex rule: regex pattern.
    Regex {
        regex: Regex,
    },
    /// Default rule (fallback).
    Default,
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
            shared_views: Vec::new(),
            viewing_rules: Vec::new(),
            named_transforms: Vec::new(),
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
    pub fn from_yaml_str(yaml_str: &str, working_dir: PathBuf) -> OcioResult<Self> {
        let docs = Yaml::load_from_str(yaml_str)
            .map_err(|e| OcioError::Yaml(format!("{}", e)))?;
        
        if docs.is_empty() {
            return Err(OcioError::Yaml("empty YAML document".into()));
        }
        
        let root = &docs[0];
        Self::from_yaml(root, working_dir)
    }

    /// Constructs config from parsed YAML.
    fn from_yaml<'a>(root: &'a Yaml<'a>, working_dir: PathBuf) -> OcioResult<Self> {
        let version_str = yaml_get(root, "ocio_profile_version")
            .and_then(yaml_to_string)
            .ok_or_else(|| OcioError::Yaml("missing ocio_profile_version".into()))?;
        
        let version = if version_str.starts_with('2') {
            ConfigVersion::V2
        } else if version_str.starts_with('1') {
            ConfigVersion::V1
        } else {
            return Err(OcioError::UnsupportedVersion {
                version: version_str.to_string(),
            });
        };

        let strict_parsing = yaml_bool(root, "strictparsing").unwrap_or(true);

        let mut config = Self {
            name: yaml_str(root, "name").unwrap_or("").to_string(),
            version,
            working_dir: working_dir.clone(),
            search_paths: yaml_str(root, "search_path")
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
            active_displays: yaml_str_list(root, "active_displays"),
            active_views: yaml_str_list(root, "active_views"),
            shared_views: parse_shared_views(root),
            viewing_rules: parse_viewing_rules(root),
            named_transforms: Vec::new(),  // Parsed below after config is created
            inactive_colorspaces: yaml_str_list(root, "inactive_colorspaces"),
            file_rules: Vec::new(),
            context: Context::new(),
            strict_parsing,
        };

        // Parse roles
        if let Some(roles) = yaml_get(root, "roles") {
            if let Yaml::Mapping(map) = unwrap_tagged(roles) {
                for (k, v) in map.iter() {
                    if let (Some(role), Some(cs)) = (yaml_as_str(k), yaml_as_str(v)) {
                        config.roles.define(role, cs);
                    }
                }
            }
        }

        // Parse color spaces
        if let Some(colorspaces) = yaml_get(root, "colorspaces") {
            if let Yaml::Sequence(seq) = unwrap_tagged(colorspaces) {
                for cs_yaml in seq {
                    match config.parse_colorspace(cs_yaml) {
                        Ok(cs) => config.colorspaces.push(cs),
                        Err(e) => {
                            if strict_parsing {
                                return Err(e);
                            }
                        }
                    }
                }
            }
        }

        // Parse named transforms (OCIO v2.0+)
        if let Some(named) = yaml_get(root, "named_transforms") {
            if let Yaml::Sequence(seq) = unwrap_tagged(named) {
                for nt_yaml in seq {
                    if let Ok(nt) = config.parse_named_transform(nt_yaml) {
                        config.named_transforms.push(nt);
                    }
                }
            }
        }

        // Parse displays
        if let Some(displays) = yaml_get(root, "displays") {
            if let Yaml::Mapping(map) = unwrap_tagged(displays) {
                for (name_yaml, views_yaml) in map.iter() {
                    if let Some(name) = yaml_as_str(name_yaml) {
                        let mut display = Display::new(name);
                        if let Yaml::Sequence(views) = unwrap_tagged(views_yaml) {
                            for view_yaml in views {
                                let view_yaml = unwrap_tagged(view_yaml);
                                if let (Some(vname), Some(vcs)) = (
                                    yaml_str(view_yaml, "name"),
                                    yaml_str(view_yaml, "colorspace"),
                                ) {
                                    let view = View::new(vname, vcs)
                                        .with_look(yaml_str(view_yaml, "looks").unwrap_or("").to_string());
                                    display.add_view(view);
                                }
                            }
                        }
                        config.displays.add_display(display);
                    }
                }
            }
        }

        // Parse looks
        if let Some(looks) = yaml_get(root, "looks") {
            if let Yaml::Sequence(seq) = unwrap_tagged(looks) {
                for look_yaml in seq {
                    let look_yaml = unwrap_tagged(look_yaml);
                    if let Some(name) = yaml_str(look_yaml, "name") {
                        let mut look = Look::new(name)
                            .process_space(yaml_str(look_yaml, "process_space").unwrap_or("").to_string())
                            .description(yaml_str(look_yaml, "description").unwrap_or("").to_string());

                        if let Some(t) = yaml_get(look_yaml, "transform") {
                            if let Ok(parsed) = config.parse_transform(t) {
                                look = look.transform(parsed);
                            }
                        }

                        if let Some(t) = yaml_get(look_yaml, "inverse_transform") {
                            if let Ok(parsed) = config.parse_transform(t) {
                                look = look.inverse_transform(parsed);
                            }
                        }

                        config.looks.add(look);
                    }
                }
            }
        }

        // Parse view transforms (v2)
        if let Some(view_transforms) = yaml_get(root, "view_transforms") {
            if let Yaml::Sequence(seq) = unwrap_tagged(view_transforms) {
                for vt_yaml in seq {
                    let vt_yaml = unwrap_tagged(vt_yaml);
                    if let Some(name) = yaml_str(vt_yaml, "name") {
                        let mut vt = ViewTransform::new(name)
                            .with_description(yaml_str(vt_yaml, "description").unwrap_or("").to_string());

                        if let Some(family) = yaml_str(vt_yaml, "family") {
                            vt = vt.with_family(family.to_string());
                        }

                        if let Some(t) = yaml_get(vt_yaml, "from_scene_reference") {
                            if let Ok(parsed) = config.parse_transform(t) {
                                vt = vt.with_from_scene_reference(parsed);
                            }
                        }
                        if let Some(t) = yaml_get(vt_yaml, "to_scene_reference") {
                            if let Ok(parsed) = config.parse_transform(t) {
                                vt = vt.with_to_scene_reference(parsed);
                            }
                        }
                        if let Some(t) = yaml_get(vt_yaml, "from_display_reference") {
                            if let Ok(parsed) = config.parse_transform(t) {
                                vt = vt.with_from_display_reference(parsed);
                            }
                        }
                        if let Some(t) = yaml_get(vt_yaml, "to_display_reference") {
                            if let Ok(parsed) = config.parse_transform(t) {
                                vt = vt.with_to_display_reference(parsed);
                            }
                        }

                        config.displays.add_view_transform(vt);
                    }
                }
            }
        }

        // Parse file rules (OCIO v2)
        if let Some(file_rules) = yaml_get(root, "file_rules") {
            if let Yaml::Sequence(seq) = unwrap_tagged(file_rules) {
                for rule_yaml in seq {
                    let rule_yaml = unwrap_tagged(rule_yaml);
                    if let (Some(name), Some(colorspace)) = (
                        yaml_str(rule_yaml, "name"),
                        yaml_str(rule_yaml, "colorspace"),
                    ) {
                        let kind = if name.eq_ignore_ascii_case("Default") {
                            FileRuleKind::Default
                        } else if let Some(regex_str) = yaml_str(rule_yaml, "regex") {
                            let regex = Regex::new(regex_str).map_err(|e| {
                                OcioError::Validation(format!("invalid regex rule '{}': {}", name, e))
                            })?;
                            FileRuleKind::Regex { regex }
                        } else {
                            let pattern = yaml_str(rule_yaml, "pattern").unwrap_or("").to_string();
                            if !pattern.is_empty() {
                                Pattern::new(&pattern).map_err(|e| {
                                    OcioError::Validation(format!("invalid glob pattern '{}': {}", name, e))
                                })?;
                            }
                            let extension = yaml_str(rule_yaml, "extension").map(|s| s.to_string());
                            if let Some(ext) = extension.as_deref() {
                                if has_glob_chars(ext) {
                                    Pattern::new(ext).map_err(|e| {
                                        OcioError::Validation(format!("invalid extension glob '{}': {}", name, e))
                                    })?;
                                }
                            }
                            FileRuleKind::Basic { pattern, extension }
                        };

                        config.file_rules.push(FileRule {
                            name: name.to_string(),
                            colorspace: colorspace.to_string(),
                            kind,
                        });
                    }
                }
            }

            // Validate file rules
            if strict_parsing {
                let default_idx = config
                    .file_rules
                    .iter()
                    .position(|r| matches!(r.kind, FileRuleKind::Default));
                if default_idx.is_none() {
                    return Err(OcioError::Validation(
                        "file_rules must include a Default rule".into(),
                    ));
                }
                if let Some(idx) = default_idx {
                    if idx + 1 != config.file_rules.len() {
                        return Err(OcioError::Validation(
                            "Default rule must be the last file rule".into(),
                        ));
                    }
                }
            }
        }

        Ok(config)
    }

    /// Parses a colorspace from YAML.
    fn parse_colorspace(&self, yaml: &Yaml) -> OcioResult<ColorSpace> {
        let yaml = unwrap_tagged(yaml);
        let name = yaml_str(yaml, "name")
            .ok_or_else(|| OcioError::Yaml("colorspace missing name".into()))?;
        
        let mut builder = ColorSpace::builder(name);

        if let Some(desc) = yaml_str(yaml, "description") {
            builder = builder.description(desc);
        }

        if let Some(family) = yaml_str(yaml, "family") {
            builder = builder.family(Family::parse(family));
        }

        if let Some(encoding) = yaml_str(yaml, "encoding") {
            builder = builder.encoding(Encoding::parse(encoding));
        }

        if yaml_bool(yaml, "isdata") == Some(true) {
            builder = builder.is_data(true);
        }

        if let Some(aliases) = yaml_get(yaml, "aliases") {
            if let Yaml::Sequence(seq) = unwrap_tagged(aliases) {
                for alias in seq {
                    if let Some(s) = yaml_as_str(alias) {
                        builder = builder.alias(s);
                    }
                }
            }
        }

        // Parse transforms - OCIO v1: to_reference/from_reference
        // OCIO v2: to_scene_reference/from_scene_reference + display variants
        let transform_fields = [
            ("to_reference", "to_ref"),
            ("to_scene_reference", "to_ref"),
            ("from_reference", "from_ref"),
            ("from_scene_reference", "from_ref"),
            ("to_display_reference", "to_disp"),
            ("from_display_reference", "from_disp"),
        ];

        for (field, kind) in transform_fields {
            if let Some(t) = yaml_get(yaml, field) {
                if let Ok(parsed) = self.parse_transform(t) {
                    builder = match kind {
                        "to_ref" => builder.to_reference(parsed),
                        "from_ref" => builder.from_reference(parsed),
                        "to_disp" => builder.to_display_reference(parsed),
                        "from_disp" => builder.from_display_reference(parsed),
                        _ => builder,
                    };
                }
            }
        }

        Ok(builder.build())
    }

    /// Parses a named transform from YAML (OCIO v2.0+).
    fn parse_named_transform(&self, yaml: &Yaml) -> OcioResult<NamedTransform> {
        let yaml = unwrap_tagged(yaml);
        let name = yaml_str(yaml, "name")
            .ok_or_else(|| OcioError::Yaml("named_transform missing name".into()))?;
        
        let forward = if let Some(t) = yaml_get(yaml, "transform") {
            self.parse_transform(t).ok()
        } else if let Some(t) = yaml_get(yaml, "forward_transform") {
            self.parse_transform(t).ok()
        } else {
            None
        };
        
        let inverse = if let Some(t) = yaml_get(yaml, "inverse_transform") {
            self.parse_transform(t).ok()
        } else {
            None
        };
        
        Ok(NamedTransform {
            name: name.to_string(),
            family: yaml_str(yaml, "family").map(|s| s.to_string()),
            description: yaml_str(yaml, "description").map(|s| s.to_string()),
            forward,
            inverse,
        })
    }

    /// Parses a transform from YAML (handles tags like !<MatrixTransform>).
    fn parse_transform(&self, yaml: &Yaml) -> OcioResult<Transform> {
        // Check if it's a sequence (group of transforms)
        if let Yaml::Sequence(seq) = yaml {
            let mut transforms = Vec::new();
            for item in seq {
                transforms.push(self.parse_transform(item)?);
            }
            return Ok(Transform::group(transforms));
        }

        // Check for tagged value
        if let Yaml::Tagged(tag, inner) = yaml {
            let tag_name = &tag.suffix;
            return self.parse_tagged_transform(tag_name, inner.as_ref());
        }

        // Check for GroupTransform with children
        if let Some(children) = yaml_get(yaml, "children") {
            if let Yaml::Sequence(seq) = unwrap_tagged(children) {
                let mut transforms = Vec::new();
                for item in seq {
                    transforms.push(self.parse_transform(item)?);
                }
                return Ok(Transform::group(transforms));
            }
        }

        Err(OcioError::Yaml("unknown transform format".into()))
    }

    /// Parses a tagged transform (e.g., !<MatrixTransform>).
    fn parse_tagged_transform(&self, tag: &str, yaml: &Yaml) -> OcioResult<Transform> {
        let yaml = unwrap_tagged(yaml);
        
        match tag {
            "MatrixTransform" => {
                Ok(Transform::Matrix(MatrixTransform {
                    matrix: parse_matrix_16(yaml_f64_list(yaml, "matrix")),
                    offset: parse_offset_4(yaml_f64_list(yaml, "offset")),
                    direction: parse_direction(yaml_str(yaml, "direction")),
                }))
            }

            "FileTransform" => {
                let src = yaml_str(yaml, "src")
                    .ok_or_else(|| OcioError::Yaml("FileTransform missing src".into()))?;
                let resolved = self.context.resolve(src);
                let resolved_path = self
                    .resolve_file(&resolved)
                    .unwrap_or_else(|| self.working_dir.join(&resolved));
                Ok(Transform::FileTransform(FileTransform {
                    src: resolved_path,
                    ccc_id: yaml_str(yaml, "cccid").map(|s| s.to_string()),
                    interpolation: parse_interpolation(yaml_str(yaml, "interpolation")),
                    direction: parse_direction(yaml_str(yaml, "direction")),
                }))
            }

            "ExponentTransform" => {
                let values = yaml_f64_list(yaml, "value");
                let value = match values.len() {
                    4 => [values[0], values[1], values[2], values[3]],
                    3 => [values[0], values[1], values[2], 1.0],
                    1 => [values[0], values[0], values[0], 1.0],
                    _ => [1.0, 1.0, 1.0, 1.0],
                };
                Ok(Transform::Exponent(ExponentTransform {
                    value,
                    negative_style: NegativeStyle::Clamp,
                    direction: parse_direction(yaml_str(yaml, "direction")),
                }))
            }

            "LogTransform" => {
                Ok(Transform::Log(LogTransform {
                    base: yaml_f64(yaml, "base").unwrap_or(2.0),
                    direction: parse_direction(yaml_str(yaml, "direction")),
                }))
            }

            "CDLTransform" => {
                Ok(Transform::Cdl(CdlTransform {
                    slope: parse_rgb(yaml_f64_list(yaml, "slope"), 1.0),
                    offset: parse_rgb(yaml_f64_list(yaml, "offset"), 0.0),
                    power: parse_rgb(yaml_f64_list(yaml, "power"), 1.0),
                    saturation: yaml_f64(yaml, "saturation").unwrap_or(1.0),
                    style: CdlStyle::AscCdl,
                    direction: parse_direction(yaml_str(yaml, "direction")),
                }))
            }

            "ColorSpaceTransform" => {
                let src = yaml_str(yaml, "src")
                    .ok_or_else(|| OcioError::Yaml("ColorSpaceTransform missing src".into()))?;
                let dst = yaml_str(yaml, "dst")
                    .ok_or_else(|| OcioError::Yaml("ColorSpaceTransform missing dst".into()))?;
                Ok(Transform::ColorSpace(ColorSpaceTransform {
                    src: src.to_string(),
                    dst: dst.to_string(),
                    direction: parse_direction(yaml_str(yaml, "direction")),
                }))
            }

            "BuiltinTransform" => {
                let style = yaml_str(yaml, "style")
                    .ok_or_else(|| OcioError::Yaml("BuiltinTransform missing style".into()))?;
                Ok(Transform::Builtin(BuiltinTransform {
                    style: style.to_string(),
                    direction: parse_direction(yaml_str(yaml, "direction")),
                }))
            }

            "RangeTransform" => {
                let style = match yaml_str(yaml, "style") {
                    Some("noClamp") | Some("noclamp") | Some("NoClamp") | Some("NOCLAMP") => {
                        RangeStyle::NoClamp
                    }
                    _ => RangeStyle::Clamp,
                };
                Ok(Transform::Range(RangeTransform {
                    min_in: yaml_f64(yaml, "min_in_value"),
                    max_in: yaml_f64(yaml, "max_in_value"),
                    min_out: yaml_f64(yaml, "min_out_value"),
                    max_out: yaml_f64(yaml, "max_out_value"),
                    style,
                    direction: parse_direction(yaml_str(yaml, "direction")),
                }))
            }

            "FixedFunctionTransform" => {
                let style_str = yaml_str(yaml, "style")
                    .ok_or_else(|| OcioError::Yaml("FixedFunctionTransform missing style".into()))?;
                let style = parse_fixed_function_style(style_str);
                Ok(Transform::FixedFunction(FixedFunctionTransform {
                    style,
                    params: yaml_f64_list(yaml, "params"),
                    direction: parse_direction(yaml_str(yaml, "direction")),
                }))
            }

            "ExposureContrastTransform" => {
                let style = match yaml_str(yaml, "style") {
                    Some("video") | Some("Video") | Some("VIDEO") => ExposureContrastStyle::Video,
                    Some("log") | Some("Log") | Some("LOG") => ExposureContrastStyle::Logarithmic,
                    _ => ExposureContrastStyle::Linear,
                };
                Ok(Transform::ExposureContrast(ExposureContrastTransform {
                    exposure: yaml_f64(yaml, "exposure").unwrap_or(0.0),
                    contrast: yaml_f64(yaml, "contrast").unwrap_or(1.0),
                    gamma: yaml_f64(yaml, "gamma").unwrap_or(1.0),
                    pivot: yaml_f64(yaml, "pivot").unwrap_or(0.18),
                    style,
                    direction: parse_direction(yaml_str(yaml, "direction")),
                }))
            }

            "LookTransform" => {
                let src = yaml_str(yaml, "src")
                    .ok_or_else(|| OcioError::Yaml("LookTransform missing src".into()))?;
                let dst = yaml_str(yaml, "dst")
                    .ok_or_else(|| OcioError::Yaml("LookTransform missing dst".into()))?;
                Ok(Transform::Look(LookTransform {
                    src: src.to_string(),
                    dst: dst.to_string(),
                    looks: yaml_str(yaml, "looks").unwrap_or("").to_string(),
                    direction: parse_direction(yaml_str(yaml, "direction")),
                }))
            }

            "GroupTransform" => {
                if let Some(children) = yaml_get(yaml, "children") {
                    if let Yaml::Sequence(seq) = unwrap_tagged(children) {
                        let mut transforms = Vec::new();
                        for item in seq {
                            transforms.push(self.parse_transform(item)?);
                        }
                        return Ok(Transform::group(transforms));
                    }
                }
                Err(OcioError::Yaml("GroupTransform missing children".into()))
            }

            "GradingPrimaryTransform" => {
                Ok(Transform::GradingPrimary(GradingPrimaryTransform {
                    lift: parse_rgb(yaml_f64_list(yaml, "lift"), 0.0),
                    gamma: parse_rgb(yaml_f64_list(yaml, "gamma"), 1.0),
                    gain: parse_rgb(yaml_f64_list(yaml, "gain"), 1.0),
                    offset: yaml_f64(yaml, "offset").unwrap_or(0.0),
                    exposure: yaml_f64(yaml, "exposure").unwrap_or(0.0),
                    contrast: yaml_f64(yaml, "contrast").unwrap_or(1.0),
                    saturation: yaml_f64(yaml, "saturation").unwrap_or(1.0),
                    pivot: yaml_f64(yaml, "pivot").unwrap_or(0.18),
                    clamp_black: yaml_f64(yaml, "clamp_black"),
                    clamp_white: yaml_f64(yaml, "clamp_white"),
                    direction: parse_direction(yaml_str(yaml, "direction")),
                }))
            }

            "GradingRGBCurveTransform" => {
                let identity = vec![[0.0, 0.0], [1.0, 1.0]];
                Ok(Transform::GradingRgbCurve(GradingRgbCurveTransform {
                    red: parse_curve_points(yaml_get(yaml, "red")).unwrap_or_else(|| identity.clone()),
                    green: parse_curve_points(yaml_get(yaml, "green")).unwrap_or_else(|| identity.clone()),
                    blue: parse_curve_points(yaml_get(yaml, "blue")).unwrap_or_else(|| identity.clone()),
                    master: parse_curve_points(yaml_get(yaml, "master")).unwrap_or(identity),
                    direction: parse_direction(yaml_str(yaml, "direction")),
                }))
            }

            "GradingToneTransform" => {
                let neutral = [1.0, 1.0, 1.0, 1.0];
                let black = [0.0, 0.0, 0.0, 0.0];
                Ok(Transform::GradingTone(GradingToneTransform {
                    shadows: parse_rgbm(yaml_f64_list(yaml, "shadows"), neutral),
                    midtones: parse_rgbm(yaml_f64_list(yaml, "midtones"), neutral),
                    highlights: parse_rgbm(yaml_f64_list(yaml, "highlights"), neutral),
                    whites: parse_rgbm(yaml_f64_list(yaml, "whites"), neutral),
                    blacks: parse_rgbm(yaml_f64_list(yaml, "blacks"), black),
                    shadow_start: yaml_f64(yaml, "shadow_start").unwrap_or(0.0),
                    shadow_pivot: yaml_f64(yaml, "shadow_pivot").unwrap_or(0.09),
                    highlight_start: yaml_f64(yaml, "highlight_start").unwrap_or(0.5),
                    highlight_pivot: yaml_f64(yaml, "highlight_pivot").unwrap_or(0.89),
                    direction: parse_direction(yaml_str(yaml, "direction")),
                }))
            }

            "LogAffineTransform" => {
                Ok(Transform::LogAffine(LogAffineTransform {
                    base: yaml_f64(yaml, "base").unwrap_or(2.0),
                    log_side_slope: parse_rgb(yaml_f64_list(yaml, "logSideSlope"), 1.0),
                    log_side_offset: parse_rgb(yaml_f64_list(yaml, "logSideOffset"), 0.0),
                    lin_side_slope: parse_rgb(yaml_f64_list(yaml, "linSideSlope"), 1.0),
                    lin_side_offset: parse_rgb(yaml_f64_list(yaml, "linSideOffset"), 0.0),
                    direction: parse_direction(yaml_str(yaml, "direction")),
                }))
            }

            "LogCameraTransform" => {
                Ok(Transform::LogCamera(LogCameraTransform {
                    base: yaml_f64(yaml, "base").unwrap_or(2.0),
                    log_side_slope: parse_rgb(yaml_f64_list(yaml, "logSideSlope"), 1.0),
                    log_side_offset: parse_rgb(yaml_f64_list(yaml, "logSideOffset"), 0.0),
                    lin_side_slope: parse_rgb(yaml_f64_list(yaml, "linSideSlope"), 1.0),
                    lin_side_offset: parse_rgb(yaml_f64_list(yaml, "linSideOffset"), 0.0),
                    lin_side_break: parse_rgb(yaml_f64_list(yaml, "linSideBreak"), 0.0),
                    linear_slope: yaml_f64_list(yaml, "linearSlope")
                        .get(0..3)
                        .map(|v| [v[0], v[1], v[2]]),
                    direction: parse_direction(yaml_str(yaml, "direction")),
                }))
            }

            "AllocationTransform" => {
                let allocation = match yaml_str(yaml, "allocation") {
                    Some("lg2") | Some("log2") | Some("LG2") | Some("LOG2") => AllocationType::Log2,
                    _ => AllocationType::Uniform,
                };
                Ok(Transform::Allocation(AllocationTransform {
                    allocation,
                    vars: yaml_f64_list(yaml, "vars"),
                    direction: parse_direction(yaml_str(yaml, "direction")),
                }))
            }

            _ => Err(OcioError::Yaml(format!("unknown transform type: {}", tag))),
        }
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
        optimization: OptimizationLevel,
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
        } else if let Some(t) = dst_cs.to_reference() {
            // Auto-invert: use inverse of to_reference
            transforms.push(t.clone().inverse());
        }

        if transforms.is_empty() {
            return Ok(Processor::new());
        }

        let group = Transform::group(transforms);
        Processor::from_transform_with_opts(&group, TransformDirection::Forward, optimization)
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

        let mut transforms = Vec::new();

        // Source to reference
        let src_cs = self
            .colorspace(src)
            .ok_or_else(|| OcioError::ColorSpaceNotFound { name: src.into() })?;
        if let Some(t) = src_cs.to_reference() {
            transforms.push(t.clone());
        }

        // Apply view looks (if any) in reference space
        if let Some(looks) = v.looks() {
            self.append_look_transforms(&mut transforms, looks)?;
        }

        // Apply view transform (OCIO v2) if defined
        if let Some(vt_name) = v.view_transform() {
            let vt = self
                .displays
                .view_transform(vt_name)
                .ok_or_else(|| OcioError::Validation(format!("view transform not found: {}", vt_name)))?;

            if let Some(t) = vt.from_scene_reference() {
                transforms.push(t.clone());
            } else if let Some(t) = vt.to_scene_reference() {
                transforms.push(t.clone().inverse());
            } else if let Some(t) = vt.to_display_reference() {
                transforms.push(t.clone());
            } else if let Some(t) = vt.from_display_reference() {
                transforms.push(t.clone().inverse());
            }
        }

        // Display/view color space
        let dst_cs = self
            .colorspace(v.colorspace())
            .ok_or_else(|| OcioError::ColorSpaceNotFound { name: v.colorspace().into() })?;

        if v.view_transform().is_some() {
            if let Some(t) = dst_cs.from_display_reference() {
                transforms.push(t.clone());
            } else if let Some(t) = dst_cs.from_reference() {
                transforms.push(t.clone());
            } else if let Some(t) = dst_cs.to_reference() {
                transforms.push(t.clone().inverse());
            }
        } else if let Some(t) = dst_cs.from_reference() {
            transforms.push(t.clone());
        } else if let Some(t) = dst_cs.to_reference() {
            transforms.push(t.clone().inverse());
        }

        if transforms.is_empty() {
            return Ok(Processor::new());
        }

        let group = Transform::group(transforms);
        Processor::from_transform(&group, TransformDirection::Forward)
    }

    /// Creates a processor with looks applied.
    pub fn processor_with_looks(
        &self,
        src: &str,
        dst: &str,
        looks: &str,
    ) -> OcioResult<Processor> {
        if looks.trim().is_empty() {
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
        self.append_look_transforms(&mut transforms, looks)?;

        // Reference to destination
        let dst_cs = self
            .colorspace(dst)
            .ok_or_else(|| OcioError::ColorSpaceNotFound { name: dst.into() })?;
        if let Some(t) = dst_cs.from_reference() {
            transforms.push(t.clone());
        } else if let Some(t) = dst_cs.to_reference() {
            transforms.push(t.clone().inverse());
        }
        
        if transforms.is_empty() {
            return Ok(Processor::new());
        }
        
        let group = Transform::group(transforms);
        Processor::from_transform(&group, TransformDirection::Forward)
    }

    fn append_look_transforms(&self, transforms: &mut Vec<Transform>, looks: &str) -> OcioResult<()> {
        use crate::look::parse_looks;

        let look_specs = parse_looks(looks);
        if look_specs.is_empty() {
            return Ok(());
        }

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

        Ok(())
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
        if self.file_rules.is_empty() {
            return None;
        }

        let normalized = normalize_path(filepath);
        let ext = Path::new(&normalized)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        for rule in &self.file_rules {
            match &rule.kind {
                FileRuleKind::Basic { pattern, extension } => {
                    if !pattern.is_empty() {
                        let glob = Pattern::new(pattern).ok()?;
                        if !glob.matches(&normalized) {
                            continue;
                        }
                    }

                    if let Some(ext_rule) = extension.as_deref() {
                        let ext_rule = ext_rule.trim_start_matches('.').to_lowercase();
                        let file_ext = ext.as_deref().unwrap_or("");
                        if has_glob_chars(&ext_rule) {
                            let glob = Pattern::new(&ext_rule).ok()?;
                            if !glob.matches(file_ext) {
                                continue;
                            }
                        } else if file_ext != ext_rule {
                            continue;
                        }
                    }

                    return Some(&rule.colorspace);
                }
                FileRuleKind::Regex { regex } => {
                    if regex.is_match(&normalized) {
                        return Some(&rule.colorspace);
                    }
                }
                FileRuleKind::Default => {
                    return Some(&rule.colorspace);
                }
            }
        }

        None
    }

    /// Adds a color space to the config.
    pub fn add_colorspace(&mut self, cs: ColorSpace) {
        self.colorspaces.push(cs);
    }

    /// Adds a display to the config.
    pub fn add_display(&mut self, display: Display) {
        self.displays.add_display(display);
    }

    /// Adds a look to the config.
    pub fn add_look(&mut self, look: Look) {
        self.looks.add(look);
    }

    /// Sets a role mapping.
    pub fn set_role(&mut self, role: impl Into<String>, colorspace: impl Into<String>) {
        self.roles.define(role, colorspace);
    }

    // ========================================================================
    // Config creation methods
    // ========================================================================

    /// Creates a minimal "raw" configuration.
    ///
    /// This config only contains a "Raw" data color space with no transforms.
    /// Useful for applications that need to bypass color management.
    ///
    /// # Example
    ///
    /// ```
    /// use vfx_ocio::Config;
    ///
    /// let config = Config::create_raw();
    /// assert!(config.colorspace("Raw").is_some());
    /// ```
    pub fn create_raw() -> Self {
        use crate::colorspace::{ColorSpace, Encoding, Family};

        let mut config = Self::new();
        config.name = "Raw Config".to_string();
        config.version = ConfigVersion::V2;

        // Add a Raw/data color space
        let raw = ColorSpace::builder("Raw")
            .encoding(Encoding::Data)
            .family(Family::Utility)
            .description("Raw data bypass - no color processing")
            .is_data(true)
            .build();

        config.colorspaces.push(raw);
        config.roles.define(crate::role::names::REFERENCE, "Raw");
        config.roles.define(crate::role::names::DATA, "Raw");
        config.roles.define(crate::role::names::DEFAULT, "Raw");

        config
    }

    // ========================================================================
    // Version access
    // ========================================================================

    /// Gets the major version number.
    ///
    /// Returns 1 for OCIO v1.x configs, 2 for OCIO v2.x configs.
    pub fn major_version(&self) -> u32 {
        match self.version {
            ConfigVersion::V1 => 1,
            ConfigVersion::V2 => 2,
        }
    }

    /// Gets the minor version number.
    ///
    /// Currently returns 0 as we don't track minor versions.
    pub fn minor_version(&self) -> u32 {
        0
    }

    /// Sets the config version.
    pub fn set_version(&mut self, major: u32, _minor: u32) {
        self.version = if major >= 2 {
            ConfigVersion::V2
        } else {
            ConfigVersion::V1
        };
    }

    // ========================================================================
    // Config metadata
    // ========================================================================

    /// Sets the config name.
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Sets the config description.
    ///
    /// Note: This is stored in the name field as OCIO uses name for description.
    pub fn set_description(&mut self, description: impl Into<String>) {
        self.name = description.into();
    }

    /// Gets the config description.
    pub fn description(&self) -> &str {
        &self.name
    }

    // ========================================================================
    // Search path management
    // ========================================================================

    /// Adds a search path for LUT files.
    pub fn add_search_path(&mut self, path: impl Into<PathBuf>) {
        self.search_paths.push(path.into());
    }

    /// Sets all search paths.
    pub fn set_search_paths(&mut self, paths: Vec<PathBuf>) {
        self.search_paths = paths;
    }

    /// Clears all search paths.
    pub fn clear_search_paths(&mut self) {
        self.search_paths.clear();
    }

    /// Sets the working directory.
    pub fn set_working_dir(&mut self, dir: impl Into<PathBuf>) {
        self.working_dir = dir.into();
    }

    // ========================================================================
    // File rules management
    // ========================================================================

    /// Adds a file rule.
    pub fn add_file_rule(&mut self, rule: FileRule) {
        self.file_rules.push(rule);
    }

    /// Gets all file rules.
    pub fn file_rules(&self) -> &[FileRule] {
        &self.file_rules
    }

    /// Clears all file rules.
    pub fn clear_file_rules(&mut self) {
        self.file_rules.clear();
    }

    // ========================================================================
    // Validation
    // ========================================================================

    /// Validates the configuration.
    ///
    /// Checks for:
    /// - At least one color space defined
    /// - Reference role is defined
    /// - All role color spaces exist
    /// - All display view color spaces exist
    ///
    /// Returns a list of validation errors/warnings.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // Check for color spaces
        if self.colorspaces.is_empty() {
            errors.push("Config has no color spaces defined".to_string());
        }

        // Check reference role
        if !self.roles.has_reference() {
            errors.push("Config is missing the 'reference' role".to_string());
        }

        // Check that all roles point to existing color spaces
        for (role, cs_name) in self.roles.iter() {
            if self.colorspace(cs_name).is_none() {
                errors.push(format!(
                    "Role '{}' references non-existent color space '{}'",
                    role, cs_name
                ));
            }
        }

        // Check display/view color spaces
        for display in self.displays.displays() {
            for view in display.views() {
                let cs_name = view.colorspace();
                if self.colorspace(cs_name).is_none() {
                    errors.push(format!(
                        "View '{}' in display '{}' references non-existent color space '{}'",
                        view.name(),
                        display.name(),
                        cs_name
                    ));
                }
            }
        }

        // Check look process spaces
        for look in self.looks.all() {
            if let Some(ps) = look.get_process_space() {
                if !ps.is_empty() && self.colorspace(ps).is_none() {
                    errors.push(format!(
                        "Look '{}' references non-existent process space '{}'",
                        look.name(),
                        ps
                    ));
                }
            }
        }

        // Check file rules (v2)
        if self.version == ConfigVersion::V2 && !self.file_rules.is_empty() {
            // Count Default rules
            let default_count = self
                .file_rules
                .iter()
                .filter(|r| matches!(r.kind, FileRuleKind::Default))
                .count();

            if default_count == 0 {
                errors.push(
                    "File rules must include a Default rule (OCIO v2 requirement)".to_string(),
                );
            } else if default_count > 1 {
                errors.push(format!(
                    "File rules must have exactly one Default rule, found {}",
                    default_count
                ));
            }

            // Check that Default rule is last
            let last_is_default = self
                .file_rules
                .last()
                .map(|r| matches!(r.kind, FileRuleKind::Default))
                .unwrap_or(false);
            if !last_is_default && default_count > 0 {
                errors.push(
                    "Default rule must be the last file rule".to_string(),
                );
            }

            // Check that all file rules have valid color spaces (including Default)
            for rule in &self.file_rules {
                // Default rule colorspace can be a role name or direct colorspace
                let cs_name = &rule.colorspace;
                let is_valid = self.colorspace(cs_name).is_some()
                    || self.roles.get(cs_name).is_some();
                
                if !is_valid {
                    errors.push(format!(
                        "File rule '{}' references non-existent color space or role '{}'",
                        rule.name, rule.colorspace
                    ));
                }
            }
        }

        errors
    }

    /// Checks if the config is valid.
    ///
    /// Returns true if `validate()` returns no errors.
    pub fn is_valid(&self) -> bool {
        self.validate().is_empty()
    }

    // ========================================================================
    // Serialization
    // ========================================================================

    /// Serializes the config to a YAML string.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use vfx_ocio::Config;
    ///
    /// let config = Config::create_raw();
    /// let yaml = config.serialize()?;
    /// println!("{}", yaml);
    /// ```
    pub fn serialize(&self) -> OcioResult<String> {
        let mut output = String::new();

        // Header
        let version_str = match self.version {
            ConfigVersion::V1 => "1",
            ConfigVersion::V2 => "2.0",
        };
        output.push_str(&format!("ocio_profile_version: {}\n\n", version_str));

        // Name/description
        if !self.name.is_empty() {
            output.push_str(&format!("name: {}\n", self.name));
        }

        // Search paths
        if !self.search_paths.is_empty() {
            let paths: Vec<&str> = self
                .search_paths
                .iter()
                .filter_map(|p| p.to_str())
                .collect();
            if !paths.is_empty() {
                output.push_str(&format!("search_path: {}\n", paths.join(":")));
            }
        }

        output.push('\n');

        // Roles
        if !self.roles.is_empty() {
            output.push_str("roles:\n");
            for (role, cs) in self.roles.iter() {
                output.push_str(&format!("  {}: {}\n", role, cs));
            }
            output.push('\n');
        }

        // File rules (v2)
        if self.version == ConfigVersion::V2 && !self.file_rules.is_empty() {
            output.push_str("file_rules:\n");
            for rule in &self.file_rules {
                output.push_str(&format!("  - !<Rule> {{name: {}", rule.name));
                match &rule.kind {
                    FileRuleKind::Default => {}
                    FileRuleKind::Basic { pattern, extension } => {
                        if !pattern.is_empty() {
                            output.push_str(&format!(", pattern: \"{}\"", pattern));
                        }
                        if let Some(ext) = extension {
                            output.push_str(&format!(", extension: {}", ext));
                        }
                    }
                    FileRuleKind::Regex { regex } => {
                        output.push_str(&format!(", regex: \"{}\"", regex.as_str()));
                    }
                }
                output.push_str(&format!(", colorspace: {}}}\n", rule.colorspace));
            }
            output.push('\n');
        }

        // Displays
        if !self.displays.displays().is_empty() {
            output.push_str("displays:\n");
            for display in self.displays.displays() {
                output.push_str(&format!("  {}:\n", display.name()));
                for view in display.views() {
                    output.push_str(&format!(
                        "    - !<View> {{name: {}, colorspace: {}",
                        view.name(),
                        view.colorspace()
                    ));
                    if let Some(looks) = view.looks() {
                        if !looks.is_empty() {
                            output.push_str(&format!(", looks: {}", looks));
                        }
                    }
                    output.push_str("}\n");
                }
            }
            output.push('\n');
        }

        // Active displays/views
        if !self.active_displays.is_empty() {
            output.push_str(&format!(
                "active_displays: [{}]\n",
                self.active_displays.join(", ")
            ));
        }
        if !self.active_views.is_empty() {
            output.push_str(&format!(
                "active_views: [{}]\n",
                self.active_views.join(", ")
            ));
        }

        output.push('\n');

        // Looks
        if !self.looks.all().is_empty() {
            output.push_str("looks:\n");
            for look in self.looks.all() {
                output.push_str(&format!("  - !<Look>\n"));
                output.push_str(&format!("    name: {}\n", look.name()));
                if let Some(ps) = look.get_process_space() {
                    if !ps.is_empty() {
                        output.push_str(&format!("    process_space: {}\n", ps));
                    }
                }
                if !look.get_description().is_empty() {
                    output.push_str(&format!("    description: {}\n", look.get_description()));
                }
                // Note: Transform serialization would go here
            }
            output.push('\n');
        }

        // Color spaces
        output.push_str("colorspaces:\n");
        for cs in &self.colorspaces {
            output.push_str("  - !<ColorSpace>\n");
            output.push_str(&format!("    name: {}\n", cs.name()));

            if !cs.description().is_empty() {
                output.push_str(&format!("    description: {}\n", cs.description()));
            }

            let family_str = cs.family().as_str();
            if !family_str.is_empty() {
                output.push_str(&format!("    family: {}\n", family_str));
            }

            let encoding_str = cs.encoding().as_str();
            if !encoding_str.is_empty() {
                output.push_str(&format!("    encoding: {}\n", encoding_str));
            }

            if cs.is_data() {
                output.push_str("    isdata: true\n");
            }

            if !cs.aliases().is_empty() {
                output.push_str(&format!(
                    "    aliases: [{}]\n",
                    cs.aliases().join(", ")
                ));
            }

            // Note: Transform serialization would require more work
        }

        Ok(output)
    }

    /// Writes the config to a file.
    pub fn write_to_file(&self, path: impl AsRef<Path>) -> OcioResult<()> {
        let content = self.serialize()?;
        std::fs::write(path, content)?;
        Ok(())
    }

    // ========================================================================
    // Environment variable management
    // ========================================================================

    /// Sets an environment variable in the context.
    pub fn set_environment_var(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.context.set_var(name, value);
    }

    /// Gets an environment variable from the context.
    pub fn environment_var(&self, name: &str) -> Option<&str> {
        self.context.get_var(name)
    }

    /// Returns the number of environment variables.
    pub fn num_environment_vars(&self) -> usize {
        self.context.vars().count()
    }

    /// Gets environment variable name at index.
    pub fn environment_var_name_at(&self, index: usize) -> Option<&str> {
        self.context.vars().nth(index).map(|(k, _)| k)
    }

    /// Gets environment variable value at index.
    pub fn environment_var_value_at(&self, index: usize) -> Option<&str> {
        self.context.vars().nth(index).map(|(_, v)| v)
    }

    // ========================================================================
    // Color space management extensions
    // ========================================================================

    /// Returns the number of color spaces.
    #[inline]
    pub fn num_colorspaces(&self) -> usize {
        self.colorspaces.len()
    }

    /// Returns color spaces filtered by family.
    pub fn colorspaces_by_family(&self, family: crate::colorspace::Family) -> Vec<&ColorSpace> {
        self.colorspaces
            .iter()
            .filter(|cs| cs.family() == family)
            .collect()
    }

    /// Returns color spaces filtered by encoding.
    pub fn colorspaces_by_encoding(&self, encoding: crate::colorspace::Encoding) -> Vec<&ColorSpace> {
        self.colorspaces
            .iter()
            .filter(|cs| cs.encoding() == encoding)
            .collect()
    }

    /// Returns all linear color spaces.
    pub fn linear_colorspaces(&self) -> Vec<&ColorSpace> {
        self.colorspaces
            .iter()
            .filter(|cs| cs.encoding().is_linear())
            .collect()
    }

    /// Returns all data/non-color color spaces.
    pub fn data_colorspaces(&self) -> Vec<&ColorSpace> {
        self.colorspaces
            .iter()
            .filter(|cs| cs.is_data())
            .collect()
    }

    /// Checks if a color space is linear.
    pub fn is_colorspace_linear(&self, name: &str) -> bool {
        self.colorspace(name)
            .map(|cs| cs.encoding().is_linear())
            .unwrap_or(false)
    }

    /// Removes a color space by name.
    ///
    /// Returns true if the color space was found and removed.
    pub fn remove_colorspace(&mut self, name: &str) -> bool {
        if let Some(idx) = self.colorspaces.iter().position(|cs| cs.matches_name(name)) {
            self.colorspaces.remove(idx);
            true
        } else {
            false
        }
    }

    /// Clears all color spaces.
    pub fn clear_colorspaces(&mut self) {
        self.colorspaces.clear();
    }

    /// Sets the list of inactive (hidden) color spaces.
    pub fn set_inactive_colorspaces(&mut self, names: Vec<String>) {
        self.inactive_colorspaces = names;
    }

    /// Adds a color space to the inactive list.
    pub fn add_inactive_colorspace(&mut self, name: impl Into<String>) {
        self.inactive_colorspaces.push(name.into());
    }

    /// Checks if a color space is inactive (hidden).
    pub fn is_colorspace_inactive(&self, name: &str) -> bool {
        self.inactive_colorspaces
            .iter()
            .any(|n| n.eq_ignore_ascii_case(name))
    }

    /// Returns only active (non-hidden) color spaces.
    pub fn active_colorspaces(&self) -> Vec<&ColorSpace> {
        self.colorspaces
            .iter()
            .filter(|cs| !self.is_colorspace_inactive(cs.name()))
            .collect()
    }

    // ========================================================================
    // Display management extensions
    // ========================================================================

    /// Returns the number of displays.
    #[inline]
    pub fn num_displays(&self) -> usize {
        self.displays.displays().len()
    }

    /// Returns display name at index.
    pub fn display_name_at(&self, index: usize) -> Option<&str> {
        self.displays.displays().get(index).map(|d| d.name())
    }

    /// Adds a view to a display.
    pub fn add_view(&mut self, display: &str, view: View) {
        if let Some(d) = self.displays.display_mut(display) {
            d.add_view(view);
        }
    }

    /// Removes a display by name.
    pub fn remove_display(&mut self, name: &str) {
        self.displays.remove_display(name);
    }

    /// Sets active displays list.
    pub fn set_active_displays(&mut self, displays: Vec<String>) {
        self.active_displays = displays;
    }

    /// Sets active views list.
    pub fn set_active_views(&mut self, views: Vec<String>) {
        self.active_views = views;
    }

    // ========================================================================
    // Role management extensions
    // ========================================================================

    /// Returns the number of defined roles.
    #[inline]
    pub fn num_roles(&self) -> usize {
        self.roles.len()
    }

    /// Returns role name at index.
    pub fn role_name_at(&self, index: usize) -> Option<&str> {
        self.roles.iter().nth(index).map(|(name, _)| name)
    }

    /// Returns color space name for role at index.
    pub fn role_colorspace_at(&self, index: usize) -> Option<&str> {
        self.roles.iter().nth(index).map(|(_, cs)| cs)
    }

    /// Checks if a role is defined.
    #[inline]
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(role)
    }

    /// Removes a role.
    pub fn remove_role(&mut self, role: &str) -> Option<String> {
        // Note: This would need Roles to have a remove method
        // For now, we can't implement this without modifying Roles
        let _ = role;
        None
    }

    // ========================================================================
    // Look management extensions
    // ========================================================================

    /// Returns the number of looks.
    #[inline]
    pub fn num_looks(&self) -> usize {
        self.looks.len()
    }

    /// Returns look name at index.
    pub fn look_name_at(&self, index: usize) -> Option<&str> {
        self.looks.all().get(index).map(|l| l.name())
    }

    /// Checks if a look is defined.
    pub fn has_look(&self, name: &str) -> bool {
        self.looks.get(name).is_some()
    }

    /// Returns the look by name.
    pub fn look(&self, name: &str) -> Option<&Look> {
        self.looks.get(name)
    }

    // ========================================================================
    // View transform management (OCIO v2)
    // ========================================================================

    /// Returns the number of view transforms.
    #[inline]
    pub fn num_view_transforms(&self) -> usize {
        self.displays.num_view_transforms()
    }

    /// Returns all view transforms.
    #[inline]
    pub fn view_transforms(&self) -> &[ViewTransform] {
        self.displays.view_transforms()
    }

    /// Returns a view transform by name.
    pub fn view_transform(&self, name: &str) -> Option<&ViewTransform> {
        self.displays.view_transform(name)
    }

    /// Adds a view transform.
    pub fn add_view_transform(&mut self, vt: ViewTransform) {
        self.displays.add_view_transform(vt);
    }

    /// Returns view transform name at index.
    pub fn view_transform_name_at(&self, index: usize) -> Option<&str> {
        self.displays.view_transforms().get(index).map(|vt| vt.name())
    }

    // ========================================================================
    // Named transform support (OCIO v2)
    // ========================================================================

    // Note: Named transforms would require additional storage and parsing
    // from OCIO v2 configs. For now, we provide the infrastructure.

    /// Placeholder for named transform count.
    /// Named transforms are a v2 feature for reusable transform definitions.
    pub fn num_named_transforms(&self) -> usize {
        self.named_transforms.len()
    }

    /// Returns all named transforms.
    pub fn named_transforms(&self) -> &[NamedTransform] {
        &self.named_transforms
    }

    /// Gets a named transform by name.
    pub fn named_transform(&self, name: &str) -> Option<&NamedTransform> {
        self.named_transforms.iter().find(|nt| nt.name == name)
    }

    // ========================================================================
    // Shared view support (OCIO v2)
    // ========================================================================

    /// Gets the number of shared views.
    /// Shared views are display-independent views defined at the config level.
    pub fn num_shared_views(&self) -> usize {
        self.shared_views.len()
    }

    /// Returns shared views (OCIO v2.3+).
    pub fn shared_views(&self) -> &[SharedView] {
        &self.shared_views
    }

    /// Gets the number of viewing rules.
    pub fn num_viewing_rules(&self) -> usize {
        self.viewing_rules.len()
    }

    /// Returns viewing rules (OCIO v2.0+).
    pub fn viewing_rules(&self) -> &[ViewingRule] {
        &self.viewing_rules
    }

    /// Gets a viewing rule by name.
    pub fn viewing_rule(&self, name: &str) -> Option<&ViewingRule> {
        self.viewing_rules.iter().find(|r| r.name == name)
    }

    /// Checks if a view is applicable for a given colorspace based on viewing rules.
    /// 
    /// If the view has no rule, it's always applicable.
    /// If the view has a rule, checks if the colorspace matches the rule's criteria.
    pub fn is_view_applicable(&self, view: &SharedView, colorspace_name: &str) -> bool {
        // No rule means always applicable
        let rule_name = match &view.rule {
            Some(r) => r,
            None => return true,
        };
        
        // Find the rule
        let rule = match self.viewing_rule(rule_name) {
            Some(r) => r,
            None => return true, // Unknown rule, assume applicable
        };
        
        // Find the colorspace
        let cs = match self.colorspace(colorspace_name) {
            Some(cs) => cs,
            None => return true, // Unknown colorspace, assume applicable
        };
        
        // Check colorspace list match
        if !rule.colorspaces.is_empty() {
            return rule.colorspaces.iter().any(|n| n == colorspace_name);
        }
        
        // Check encoding match
        if !rule.encodings.is_empty() {
            let encoding = cs.encoding();
            if encoding != Encoding::Unknown {
                let enc_str = encoding.as_str();
                return rule.encodings.iter().any(|e| e == enc_str);
            }
            return false; // Unknown encoding, rule requires specific encoding
        }
        
        // Empty rule matches everything
        true
    }

    /// Returns filtered shared views applicable for a given colorspace.
    pub fn applicable_views(&self, colorspace_name: &str) -> Vec<&SharedView> {
        self.shared_views
            .iter()
            .filter(|v| self.is_view_applicable(v, colorspace_name))
            .collect()
    }

    // ========================================================================
    // Additional OCIO compatibility methods
    // ========================================================================

    /// Gets the reference color space name.
    ///
    /// Returns the color space assigned to the "reference" role.
    pub fn reference_space(&self) -> Option<&str> {
        self.roles.reference()
    }

    /// Gets the scene-linear color space name.
    ///
    /// Returns the color space assigned to the "scene_linear" role.
    pub fn scene_linear_space(&self) -> Option<&str> {
        self.roles.scene_linear()
    }

    /// Gets the data color space name.
    ///
    /// Returns the color space assigned to the "data" role.
    pub fn data_space(&self) -> Option<&str> {
        self.roles.data()
    }

    /// Resolves a color space name, including role resolution.
    ///
    /// If the name is a role, returns the color space assigned to that role.
    /// Otherwise returns the name unchanged if the color space exists.
    pub fn resolve_colorspace_name(&self, name: &str) -> Option<&str> {
        // First check if it's a role
        if let Some(cs_name) = self.roles.get(name) {
            if self.colorspaces.iter().any(|cs| cs.matches_name(cs_name)) {
                return Some(cs_name);
            }
        }
        // Then check if it's a direct color space name
        self.colorspaces
            .iter()
            .find(|cs| cs.matches_name(name))
            .map(|cs| cs.name())
    }

    /// Creates a display/view processor using display and view names.
    ///
    /// This is a convenience method that combines source color space,
    /// display, and view into a single processor.
    pub fn create_display_view_processor(
        &self,
        src: &str,
        display: &str,
        view: &str,
    ) -> OcioResult<Processor> {
        self.display_processor(src, display, view)
    }

    /// Returns the inverse direction processor for a color conversion.
    pub fn inverse_processor(&self, src: &str, dst: &str) -> OcioResult<Processor> {
        // Simply swap src and dst
        self.processor(dst, src)
    }

    /// Checks if this is an OCIO v2 config.
    #[inline]
    pub fn is_v2(&self) -> bool {
        self.version == ConfigVersion::V2
    }

    /// Checks if this is an OCIO v1 config.
    #[inline]
    pub fn is_v1(&self) -> bool {
        self.version == ConfigVersion::V1
    }

    // ========================================================================
    // Builder support
    // ========================================================================

    /// Creates a Config from builder components (internal use).
    #[doc(hidden)]
    pub fn from_builder(
        name: String,
        _description: String,
        version: ConfigVersion,
        search_paths: Vec<PathBuf>,
        working_dir: PathBuf,
        colorspaces: Vec<ColorSpace>,
        roles: Roles,
        displays: DisplayManager,
        looks: LookManager,
        active_displays: Vec<String>,
        active_views: Vec<String>,
        shared_views: Vec<SharedView>,
        viewing_rules: Vec<ViewingRule>,
        named_transforms: Vec<NamedTransform>,
        inactive_colorspaces: Vec<String>,
        file_rules: Vec<FileRule>,
        context: Context,
    ) -> Self {
        Self {
            name,
            version,
            search_paths,
            working_dir,
            colorspaces,
            roles,
            displays,
            looks,
            active_displays,
            active_views,
            shared_views,
            viewing_rules,
            named_transforms,
            inactive_colorspaces,
            file_rules,
            context,
            strict_parsing: false,
        }
    }
}

// ============================================================================
// YAML helper functions for saphyr
// ============================================================================

/// Unwraps a tagged value, returning the inner value.
fn unwrap_tagged<'a>(yaml: &'a Yaml<'a>) -> &'a Yaml<'a> {
    match yaml {
        Yaml::Tagged(_, inner) => inner.as_ref(),
        _ => yaml,
    }
}

/// Gets a string value from a YAML mapping.
fn yaml_str<'a>(yaml: &'a Yaml<'a>, key: &str) -> Option<&'a str> {
    yaml_get(yaml, key).and_then(|v| yaml_as_str(v))
}

/// Gets a boolean value from a YAML mapping.
fn yaml_bool<'a>(yaml: &'a Yaml<'a>, key: &str) -> Option<bool> {
    yaml_get(yaml, key).and_then(|v| {
        match unwrap_tagged(v) {
            Yaml::Value(Scalar::Boolean(b)) => Some(*b),
            _ => None,
        }
    })
}

/// Gets a f64 value from a YAML mapping.
fn yaml_f64<'a>(yaml: &'a Yaml<'a>, key: &str) -> Option<f64> {
    yaml_get(yaml, key).and_then(|v| yaml_as_f64(v))
}

/// Gets a list of f64 values from a YAML mapping.
fn yaml_f64_list<'a>(yaml: &'a Yaml<'a>, key: &str) -> Vec<f64> {
    yaml_get(yaml, key)
        .map(|v| match unwrap_tagged(v) {
            Yaml::Sequence(seq) => seq.iter().filter_map(|x| yaml_as_f64(x)).collect(),
            _ => Vec::new(),
        })
        .unwrap_or_default()
}

/// Gets a list of strings from a YAML mapping.
fn yaml_str_list<'a>(yaml: &'a Yaml<'a>, key: &str) -> Vec<String> {
    yaml_get(yaml, key)
        .map(|v| match unwrap_tagged(v) {
            Yaml::Sequence(seq) => seq.iter().filter_map(|x| yaml_as_str(x).map(|s: &str| s.to_string())).collect(),
            _ => Vec::new(),
        })
        .unwrap_or_default()
}

/// Gets a value from a YAML mapping by key.
fn yaml_get<'a>(yaml: &'a Yaml<'a>, key: &str) -> Option<&'a Yaml<'a>> {
    match unwrap_tagged(yaml) {
        Yaml::Mapping(map) => {
            for (k, v) in map.iter() {
                if yaml_as_str(k) == Some(key) {
                    return Some(v);
                }
            }
            None
        }
        _ => None,
    }
}

/// Converts a YAML value to a string.
fn yaml_as_str<'a>(yaml: &'a Yaml<'a>) -> Option<&'a str> {
    match unwrap_tagged(yaml) {
        Yaml::Value(Scalar::String(s)) => Some(s.as_ref()),
        // Handle raw representation (unresolved scalars)
        Yaml::Representation(s, _, _) => Some(s.as_ref()),
        _ => None,
    }
}

/// Converts a YAML value to a String (owned), handling numbers too.
fn yaml_to_string<'a>(yaml: &'a Yaml<'a>) -> Option<String> {
    match unwrap_tagged(yaml) {
        Yaml::Value(Scalar::String(s)) => Some(s.to_string()),
        Yaml::Value(Scalar::Integer(i)) => Some(i.to_string()),
        Yaml::Value(Scalar::FloatingPoint(f)) => Some(f.to_string()),
        Yaml::Representation(s, _, _) => Some(s.to_string()),
        _ => None,
    }
}

/// Converts a YAML value to f64.
fn yaml_as_f64<'a>(yaml: &'a Yaml<'a>) -> Option<f64> {
    match unwrap_tagged(yaml) {
        Yaml::Value(Scalar::Integer(i)) => Some(*i as f64),
        Yaml::Value(Scalar::FloatingPoint(f)) => Some(f.into_inner()),
        _ => None,
    }
}

// ============================================================================
// Helper functions for parsing transform data
// ============================================================================

/// Parses direction string to TransformDirection.
fn parse_direction(dir: Option<&str>) -> TransformDirection {
    match dir {
        Some("inverse") | Some("Inverse") | Some("INVERSE") => TransformDirection::Inverse,
        _ => TransformDirection::Forward,
    }
}

/// Parses interpolation string.
fn parse_interpolation(interp: Option<&str>) -> Interpolation {
    match interp {
        Some("nearest") | Some("Nearest") | Some("NEAREST") => Interpolation::Nearest,
        Some("tetrahedral") | Some("Tetrahedral") | Some("TETRAHEDRAL") => Interpolation::Tetrahedral,
        Some("best") | Some("Best") | Some("BEST") => Interpolation::Best,
        _ => Interpolation::Linear,
    }
}

/// Parses FixedFunction style string.
fn parse_fixed_function_style(style: &str) -> FixedFunctionStyle {
    match style.to_uppercase().as_str() {
        "ACES_REDMOD03" | "ACES_RED_MOD_03" => FixedFunctionStyle::AcesRedMod03,
        "ACES_REDMOD10" | "ACES_RED_MOD_10" => FixedFunctionStyle::AcesRedMod10,
        "ACES_GLOW03" | "ACES_GLOW_03" => FixedFunctionStyle::AcesGlow03,
        "ACES_GLOW10" | "ACES_GLOW_10" => FixedFunctionStyle::AcesGlow10,
        "ACES_GAMUTCOMP13" | "ACES_GAMUT_COMP_13" => FixedFunctionStyle::AcesGamutComp13,
        "RGB_TO_HSV" | "RGBTOHSV" => FixedFunctionStyle::RgbToHsv,
        "HSV_TO_RGB" | "HSVTORGB" => FixedFunctionStyle::HsvToRgb,
        "XYZ_TO_XYY" | "XYZTOYXY" => FixedFunctionStyle::XyzToXyy,
        "XYY_TO_XYZ" | "XYYTOXYZ" => FixedFunctionStyle::XyyToXyz,
        "XYZ_TO_UVY" | "XYZTOUVY" => FixedFunctionStyle::XyzToUvy,
        "UVY_TO_XYZ" | "UVYTOXYZ" => FixedFunctionStyle::UvyToXyz,
        "XYZ_TO_LUV" | "XYZTOLUV" => FixedFunctionStyle::XyzToLuv,
        "LUV_TO_XYZ" | "LUVTOXYZ" => FixedFunctionStyle::LuvToXyz,
        _ => FixedFunctionStyle::AcesRedMod03, // default fallback
    }
}

/// Parses 16-element matrix, pads with identity if needed.
fn parse_matrix_16(v: Vec<f64>) -> [f64; 16] {
    let identity = MatrixTransform::IDENTITY;
    match v.len() {
        n if n >= 16 => [
            v[0], v[1], v[2], v[3],
            v[4], v[5], v[6], v[7],
            v[8], v[9], v[10], v[11],
            v[12], v[13], v[14], v[15],
        ],
        n if n >= 12 => [
            // 3x4 matrix (common in OCIO)
            v[0], v[1], v[2], 0.0,
            v[3], v[4], v[5], 0.0,
            v[6], v[7], v[8], 0.0,
            v[9], v[10], v[11], 1.0,
        ],
        n if n >= 9 => [
            // 3x3 matrix
            v[0], v[1], v[2], 0.0,
            v[3], v[4], v[5], 0.0,
            v[6], v[7], v[8], 0.0,
            0.0, 0.0, 0.0, 1.0,
        ],
        _ => identity,
    }
}

/// Parses 4-element offset.
fn parse_offset_4(v: Vec<f64>) -> [f64; 4] {
    match v.len() {
        n if n >= 4 => [v[0], v[1], v[2], v[3]],
        n if n >= 3 => [v[0], v[1], v[2], 0.0],
        _ => [0.0, 0.0, 0.0, 0.0],
    }
}

/// Parses RGB values with default.
fn parse_rgb(v: Vec<f64>, default: f64) -> [f64; 3] {
    match v.len() {
        n if n >= 3 => [v[0], v[1], v[2]],
        1 => [v[0], v[0], v[0]],
        _ => [default, default, default],
    }
}

/// Parses shared_views from YAML (OCIO v2.3+).
fn parse_shared_views(root: &Yaml) -> Vec<SharedView> {
    let mut views = Vec::new();
    if let Some(Yaml::Sequence(seq)) = yaml_get(root, "shared_views") {
        for item in seq {
            let item = unwrap_tagged(item);
            if let Some(name) = yaml_str(item, "name") {
                let display_colorspace = yaml_str(item, "colorspace")
                    .or_else(|| yaml_str(item, "display_colorspace"))
                    .unwrap_or("")
                    .to_string();
                views.push(SharedView {
                    name: name.to_string(),
                    view_transform: yaml_str(item, "view_transform").map(|s| s.to_string()),
                    display_colorspace,
                    looks: yaml_str(item, "looks").map(|s| s.to_string()),
                    rule: yaml_str(item, "rule").map(|s| s.to_string()),
                    description: yaml_str(item, "description").map(|s| s.to_string()),
                });
            }
        }
    }
    views
}

/// Parses viewing_rules from YAML (OCIO v2.0+).
/// 
/// Viewing rules define which views are applicable based on colorspace encoding.
/// Example YAML:
/// ```yaml
/// viewing_rules:
///   - !<Rule> {name: Any Scene-linear or Log, encodings: [log, scene-linear]}
///   - !<Rule> {name: Any Video, encodings: [sdr-video, hdr-video]}
///   - !<Rule> {name: ACEScg Only, colorspaces: [ACEScg]}
/// ```
fn parse_viewing_rules(root: &Yaml) -> Vec<ViewingRule> {
    let mut rules = Vec::new();
    if let Some(Yaml::Sequence(seq)) = yaml_get(root, "viewing_rules") {
        for item in seq {
            let item = unwrap_tagged(item);
            if let Some(name) = yaml_str(item, "name") {
                // Parse colorspaces list
                let colorspaces = yaml_str_list(item, "colorspaces");
                
                // Parse encodings list
                let encodings = yaml_str_list(item, "encodings");
                
                // Parse custom keys (any key that isn't name, colorspaces, or encodings)
                let mut custom_keys: Vec<(String, String)> = Vec::new();
                if let Yaml::Mapping(map) = item {
                    for (k, v) in map.iter() {
                        if let Some(key) = yaml_as_str(k) {
                            if key != "name" && key != "colorspaces" && key != "encodings" {
                                if let Some(val) = yaml_as_str(v) {
                                    custom_keys.push((key.to_string(), val.to_string()));
                                }
                            }
                        }
                    }
                }
                
                rules.push(ViewingRule {
                    name: name.to_string(),
                    colorspaces,
                    encodings,
                    custom_keys,
                });
            }
        }
    }
    rules
}

/// Parses RGBM (RGB + Master) array for grading transforms.
fn parse_rgbm(v: Vec<f64>, default: [f64; 4]) -> [f64; 4] {
    match v.len() {
        n if n >= 4 => [v[0], v[1], v[2], v[3]],
        3 => [v[0], v[1], v[2], default[3]],
        1 => [v[0], v[0], v[0], v[0]],
        _ => default,
    }
}

/// Parses curve control points from YAML sequence.
fn parse_curve_points(yaml: Option<&Yaml>) -> Option<Vec<[f64; 2]>> {
    let yaml = yaml?;
    if let Yaml::Sequence(seq) = unwrap_tagged(yaml) {
        let mut points = Vec::new();
        for item in seq {
            if let Yaml::Sequence(pt) = unwrap_tagged(item) {
                if pt.len() >= 2 {
                    let x = yaml_as_f64(&pt[0]).unwrap_or(0.0);
                    let y = yaml_as_f64(&pt[1]).unwrap_or(0.0);
                    points.push([x, y]);
                }
            }
        }
        if !points.is_empty() {
            return Some(points);
        }
    }
    None
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn has_glob_chars(s: &str) -> bool {
    s.chars().any(|c| matches!(c, '*' | '?' | '[' | ']'))
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

    #[test]
    fn parse_matrix_transform() {
        let yaml = r#"
ocio_profile_version: 2.1
roles:
  default: raw
colorspaces:
  - name: raw
    family: raw
  - name: WithMatrix
    from_scene_reference: !<MatrixTransform> {matrix: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]}
displays:
  sRGB:
    - name: Raw
      colorspace: raw
"#;
        let config = Config::from_yaml_str(yaml, PathBuf::from(".")).unwrap();
        let cs = config.colorspace("WithMatrix").unwrap();
        assert!(cs.from_reference().is_some());
    }

    #[test]
    fn parse_fixed_function_transform() {
        let yaml = r#"
ocio_profile_version: 2.1
roles:
  default: raw
colorspaces:
  - name: raw
    family: raw
  - name: ACES_with_RedMod
    from_scene_reference: !<FixedFunctionTransform> {style: ACES_RedMod10}
displays:
  sRGB:
    - name: Raw
      colorspace: raw
"#;
        let config = Config::from_yaml_str(yaml, PathBuf::from(".")).unwrap();
        let cs = config.colorspace("ACES_with_RedMod").unwrap();
        assert!(cs.from_reference().is_some());
    }

    #[test]
    fn parse_exposure_contrast_transform() {
        let yaml = r#"
ocio_profile_version: 2.1
roles:
  default: raw
colorspaces:
  - name: raw
    family: raw
  - name: Graded
    from_scene_reference: !<ExposureContrastTransform> {exposure: 1.0, contrast: 1.2, gamma: 0.9, pivot: 0.18, style: linear}
displays:
  sRGB:
    - name: Raw
      colorspace: raw
"#;
        let config = Config::from_yaml_str(yaml, PathBuf::from(".")).unwrap();
        let cs = config.colorspace("Graded").unwrap();
        assert!(cs.from_reference().is_some());
    }

    #[test]
    fn parse_look_transform() {
        let yaml = r#"
ocio_profile_version: 2.1
roles:
  default: raw
colorspaces:
  - name: raw
    family: raw
  - name: WithLook
    from_scene_reference: !<LookTransform> {src: raw, dst: raw, looks: +FilmGrade}
displays:
  sRGB:
    - name: Raw
      colorspace: raw
"#;
        let config = Config::from_yaml_str(yaml, PathBuf::from(".")).unwrap();
        let cs = config.colorspace("WithLook").unwrap();
        assert!(cs.from_reference().is_some());
    }

    #[test]
    fn parse_grading_primary_transform() {
        let yaml = r#"
ocio_profile_version: 2.1
roles:
  default: raw
colorspaces:
  - name: raw
    family: raw
  - name: Graded
    from_scene_reference: !<GradingPrimaryTransform>
      lift: [0.01, 0.0, -0.01]
      gamma: [1.0, 1.05, 1.0]
      gain: [1.2, 1.0, 0.9]
      exposure: 0.5
      contrast: 1.1
      saturation: 1.2
      pivot: 0.18
displays:
  sRGB:
    - name: Raw
      colorspace: raw
"#;
        let config = Config::from_yaml_str(yaml, PathBuf::from(".")).unwrap();
        let cs = config.colorspace("Graded").unwrap();
        let transform = cs.from_reference().unwrap();
        if let Transform::GradingPrimary(gp) = transform {
            assert!((gp.exposure - 0.5).abs() < 0.001);
            assert!((gp.saturation - 1.2).abs() < 0.001);
            assert!((gp.gain[0] - 1.2).abs() < 0.001);
        } else {
            panic!("Expected GradingPrimaryTransform");
        }
    }

    #[test]
    fn parse_grading_rgb_curve_transform() {
        let yaml = r#"
ocio_profile_version: 2.1
roles:
  default: raw
colorspaces:
  - name: raw
    family: raw
  - name: Curved
    from_scene_reference: !<GradingRGBCurveTransform>
      red: [[0.0, 0.0], [0.5, 0.6], [1.0, 1.0]]
      green: [[0.0, 0.0], [1.0, 1.0]]
      blue: [[0.0, 0.1], [1.0, 0.9]]
displays:
  sRGB:
    - name: Raw
      colorspace: raw
"#;
        let config = Config::from_yaml_str(yaml, PathBuf::from(".")).unwrap();
        let cs = config.colorspace("Curved").unwrap();
        let transform = cs.from_reference().unwrap();
        if let Transform::GradingRgbCurve(gc) = transform {
            assert_eq!(gc.red.len(), 3);
            assert_eq!(gc.green.len(), 2);
            assert!((gc.red[1][1] - 0.6).abs() < 0.001);
        } else {
            panic!("Expected GradingRgbCurveTransform");
        }
    }

    #[test]
    fn parse_grading_tone_transform() {
        let yaml = r#"
ocio_profile_version: 2.1
roles:
  default: raw
colorspaces:
  - name: raw
    family: raw
  - name: Toned
    from_scene_reference: !<GradingToneTransform>
      shadows: [1.1, 1.0, 0.9, 1.0]
      midtones: [1.0, 1.0, 1.0, 1.05]
      highlights: [0.95, 1.0, 1.1, 1.0]
displays:
  sRGB:
    - name: Raw
      colorspace: raw
"#;
        let config = Config::from_yaml_str(yaml, PathBuf::from(".")).unwrap();
        let cs = config.colorspace("Toned").unwrap();
        let transform = cs.from_reference().unwrap();
        if let Transform::GradingTone(gt) = transform {
            assert!((gt.shadows[0] - 1.1).abs() < 0.001);
            assert!((gt.midtones[3] - 1.05).abs() < 0.001);
        } else {
            panic!("Expected GradingToneTransform");
        }
    }

    #[test]
    fn parse_shared_views() {
        let yaml = r#"
ocio_profile_version: 2.3
roles:
  default: raw
colorspaces:
  - name: raw
    family: raw
  - name: sRGB
    family: display
shared_views:
  - name: Raw
    colorspace: raw
  - name: sRGB Display
    view_transform: ACES_1.0_SDR
    colorspace: sRGB
    looks: +FilmGrade
    description: Standard sRGB output
displays:
  Monitor:
    - name: Raw
      colorspace: raw
"#;
        let config = Config::from_yaml_str(yaml, PathBuf::from(".")).unwrap();
        assert_eq!(config.num_shared_views(), 2);
        let views = config.shared_views();
        assert_eq!(views[0].name, "Raw");
        assert_eq!(views[1].name, "sRGB Display");
        assert_eq!(views[1].view_transform, Some("ACES_1.0_SDR".to_string()));
        assert_eq!(views[1].looks, Some("+FilmGrade".to_string()));
    }

    #[test]
    fn parse_named_transforms() {
        let yaml = r#"
ocio_profile_version: 2.1
roles:
  default: raw
colorspaces:
  - name: raw
    family: raw
named_transforms:
  - name: Utility - Exposure +1
    family: Utility
    description: Add one stop of exposure
    transform: !<ExposureContrastTransform> {exposure: 1.0, style: linear}
  - name: Utility - Rec709 to XYZ
    family: Utility
    forward_transform: !<MatrixTransform> {matrix: [0.4124, 0.3576, 0.1805, 0, 0.2126, 0.7152, 0.0722, 0, 0.0193, 0.1192, 0.9505, 0, 0, 0, 0, 1]}
    inverse_transform: !<MatrixTransform> {matrix: [3.2406, -1.5372, -0.4986, 0, -0.9689, 1.8758, 0.0415, 0, 0.0557, -0.2040, 1.0570, 0, 0, 0, 0, 1]}
displays:
  sRGB:
    - name: Raw
      colorspace: raw
"#;
        let config = Config::from_yaml_str(yaml, PathBuf::from(".")).unwrap();
        assert_eq!(config.num_named_transforms(), 2);
        
        let nt1 = config.named_transform("Utility - Exposure +1").unwrap();
        assert_eq!(nt1.family, Some("Utility".to_string()));
        assert!(nt1.forward.is_some());
        assert!(nt1.inverse.is_none());
        
        let nt2 = config.named_transform("Utility - Rec709 to XYZ").unwrap();
        assert!(nt2.forward.is_some());
        assert!(nt2.inverse.is_some());
    }

    #[test]
    fn parse_viewing_rules() {
        let yaml = r#"
ocio_profile_version: 2.1

environment: {}
roles:
  default: scene_linear
file_rules:
  - !<Rule> {name: Default, colorspace: default}

viewing_rules:
  - !<Rule> {name: log-encoded, encodings: [log]}
  - !<Rule> {name: scene-linear, encodings: [scene-linear]}
  - !<Rule> {name: aces-primaries, colorspaces: [ACEScg, ACES2065-1]}

colorspaces:
  - !<ColorSpace>
    name: scene_linear
    encoding: scene-linear
  - !<ColorSpace>
    name: ACEScg
    encoding: scene-linear
  - !<ColorSpace>
    name: log_cs
    encoding: log
  - !<ColorSpace>
    name: raw
    isdata: true

displays:
  sRGB:
    - name: View
      colorspace: scene_linear
"#;
        let config = Config::from_yaml_str(yaml, PathBuf::from(".")).unwrap();
        
        assert_eq!(config.num_viewing_rules(), 3);
        
        // Check log rule
        let log_rule = config.viewing_rule("log-encoded").unwrap();
        assert_eq!(log_rule.encodings, vec!["log"]);
        assert!(log_rule.colorspaces.is_empty());
        
        // Check scene-linear rule
        let linear_rule = config.viewing_rule("scene-linear").unwrap();
        assert_eq!(linear_rule.encodings, vec!["scene-linear"]);
        
        // Check colorspace rule
        let aces_rule = config.viewing_rule("aces-primaries").unwrap();
        assert_eq!(aces_rule.colorspaces, vec!["ACEScg", "ACES2065-1"]);
        assert!(aces_rule.encodings.is_empty());
    }

    #[test]
    fn file_rule_validation() {
        // Valid config with Default rule using role reference
        let yaml = r#"
ocio_profile_version: 2
roles:
  default: sRGB
  reference: scene_linear
file_rules:
  - !<Rule> {name: exr, pattern: "*.exr", colorspace: scene_linear}
  - !<Rule> {name: Default, colorspace: default}
colorspaces:
  - !<ColorSpace>
    name: scene_linear
  - !<ColorSpace>
    name: sRGB
displays:
  sRGB:
    - name: View
      colorspace: sRGB
"#;
        let config = Config::from_yaml_str(yaml, PathBuf::from(".")).unwrap();
        let errors = config.validate();
        assert!(errors.is_empty(), "Valid config should have no errors: {:?}", errors);

        // Invalid: Default rule references non-existent colorspace/role
        let yaml_bad_cs = r#"
ocio_profile_version: 2
roles:
  default: sRGB
  reference: sRGB
file_rules:
  - !<Rule> {name: Default, colorspace: nonexistent}
colorspaces:
  - !<ColorSpace>
    name: sRGB
displays:
  sRGB:
    - name: View
      colorspace: sRGB
"#;
        let config_bad = Config::from_yaml_str(yaml_bad_cs, PathBuf::from(".")).unwrap();
        let errors = config_bad.validate();
        assert!(
            errors.iter().any(|e| e.contains("nonexistent")),
            "Should detect invalid colorspace in Default rule: {:?}",
            errors
        );
    }
}
