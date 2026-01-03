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
use glob::Pattern;
use regex::Regex;

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
    /// Assigned color space.
    pub colorspace: String,
    /// Rule matching kind.
    pub kind: FileRuleKind,
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
                let mut look = Look::new(&raw_look.name)
                    .process_space(raw_look.process_space.unwrap_or_default())
                    .description(raw_look.description.unwrap_or_default());

                if let Some(t) = raw_look.transform {
                    match self.parse_raw_transform(&t) {
                        Ok(parsed) => {
                            look = look.transform(parsed);
                        }
                        Err(e) => {
                            if config.strict_parsing {
                                return Err(e);
                            }
                        }
                    }
                }

                if let Some(t) = raw_look.inverse_transform {
                    match self.parse_raw_transform(&t) {
                        Ok(parsed) => {
                            look = look.inverse_transform(parsed);
                        }
                        Err(e) => {
                            if config.strict_parsing {
                                return Err(e);
                            }
                        }
                    }
                }
                config.looks.add(look);
            }
        }

        // Parse view transforms (v2)
        if let Some(view_transforms) = raw.view_transforms {
            for raw_vt in view_transforms {
                let mut vt = ViewTransform::new(&raw_vt.name)
                    .with_description(raw_vt.description.unwrap_or_default());

                if let Some(family) = raw_vt.family {
                    vt = vt.with_family(family);
                }

                if let Some(t) = raw_vt.from_scene_reference {
                    match self.parse_raw_transform(&t) {
                        Ok(parsed) => {
                            vt = vt.with_from_scene_reference(parsed);
                        }
                        Err(e) => {
                            if config.strict_parsing {
                                return Err(e);
                            }
                        }
                    }
                }

                if let Some(t) = raw_vt.to_scene_reference {
                    match self.parse_raw_transform(&t) {
                        Ok(parsed) => {
                            vt = vt.with_to_scene_reference(parsed);
                        }
                        Err(e) => {
                            if config.strict_parsing {
                                return Err(e);
                            }
                        }
                    }
                }

                if let Some(t) = raw_vt.from_display_reference {
                    match self.parse_raw_transform(&t) {
                        Ok(parsed) => {
                            vt = vt.with_from_display_reference(parsed);
                        }
                        Err(e) => {
                            if config.strict_parsing {
                                return Err(e);
                            }
                        }
                    }
                }

                if let Some(t) = raw_vt.to_display_reference {
                    match self.parse_raw_transform(&t) {
                        Ok(parsed) => {
                            vt = vt.with_to_display_reference(parsed);
                        }
                        Err(e) => {
                            if config.strict_parsing {
                                return Err(e);
                            }
                        }
                    }
                }
                config.displays.add_view_transform(vt);
            }
        }

        // Parse file rules (OCIO v2)
        if let Some(file_rules) = raw.file_rules {
            for raw_rule in file_rules {
                let name = raw_rule.name;
                let colorspace = raw_rule.colorspace;

                let kind = if name.eq_ignore_ascii_case("Default") {
                    FileRuleKind::Default
                } else if let Some(regex_str) = raw_rule.regex {
                    let regex = Regex::new(&regex_str).map_err(|e| {
                        OcioError::Validation(format!("invalid regex rule '{}': {}", name, e),)
                    })?;
                    FileRuleKind::Regex { regex }
                } else {
                    let pattern = raw_rule.pattern.unwrap_or_default();
                    if !pattern.is_empty() {
                        Pattern::new(&pattern).map_err(|e| {
                            OcioError::Validation(format!("invalid glob pattern '{}': {}", name, e),)
                        })?;
                    }
                    if let Some(ext) = raw_rule.extension.as_deref() {
                        if has_glob_chars(ext) {
                            Pattern::new(ext).map_err(|e| {
                                OcioError::Validation(format!("invalid extension glob '{}': {}", name, e),)
                            })?;
                        }
                    }
                    FileRuleKind::Basic {
                        pattern,
                        extension: raw_rule.extension,
                    }
                };

                config.file_rules.push(FileRule {
                    name,
                    colorspace,
                    kind,
                });
            }

            if config.strict_parsing {
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

    /// Parses a raw colorspace definition.
    fn parse_colorspace(&self, raw: RawColorSpace) -> OcioResult<ColorSpace> {
        let mut builder = ColorSpace::builder(&raw.name);
        let strict = self.strict_parsing;

        if let Some(desc) = raw.description {
            builder = builder.description(desc);
        }

        if let Some(family) = raw.family {
            builder = builder.family(Family::parse(&family));
        }

        if let Some(encoding) = raw.encoding {
            builder = builder.encoding(Encoding::parse(&encoding));
        }

        if raw.isdata == Some(true) {
            builder = builder.is_data(true);
        }

        if let Some(aliases) = raw.aliases {
            for alias in aliases {
                builder = builder.alias(alias);
            }
        }

        // Parse transforms (to_reference, from_reference)
        // OCIO v1 uses to_reference/from_reference
        // OCIO v2 adds to_scene_reference/from_scene_reference and display variants
        if let Some(raw_t) = raw.to_reference {
            match self.parse_raw_transform(&raw_t) {
                Ok(t) => builder = builder.to_reference(t),
                Err(e) => {
                    if strict {
                        return Err(e);
                    }
                }
            }
        } else if let Some(raw_t) = raw.to_scene_reference {
            match self.parse_raw_transform(&raw_t) {
                Ok(t) => builder = builder.to_reference(t),
                Err(e) => {
                    if strict {
                        return Err(e);
                    }
                }
            }
        }

        if let Some(raw_t) = raw.from_reference {
            match self.parse_raw_transform(&raw_t) {
                Ok(t) => builder = builder.from_reference(t),
                Err(e) => {
                    if strict {
                        return Err(e);
                    }
                }
            }
        } else if let Some(raw_t) = raw.from_scene_reference {
            match self.parse_raw_transform(&raw_t) {
                Ok(t) => builder = builder.from_reference(t),
                Err(e) => {
                    if strict {
                        return Err(e);
                    }
                }
            }
        }

        if let Some(raw_t) = raw.to_display_reference {
            match self.parse_raw_transform(&raw_t) {
                Ok(t) => builder = builder.to_display_reference(t),
                Err(e) => {
                    if strict {
                        return Err(e);
                    }
                }
            }
        }

        if let Some(raw_t) = raw.from_display_reference {
            match self.parse_raw_transform(&raw_t) {
                Ok(t) => builder = builder.from_display_reference(t),
                Err(e) => {
                    if strict {
                        return Err(e);
                    }
                }
            }
        }

        Ok(builder.build())
    }

    /// Parses a RawTransform into a Transform.
    fn parse_raw_transform(&self, raw: &RawTransform) -> OcioResult<Transform> {
        match raw {
            RawTransform::Single(def) => self.parse_raw_transform_def(def.as_ref()),
            RawTransform::Group(defs) => {
                let mut transforms = Vec::new();
                for def in defs {
                    match self.parse_raw_transform_def(def) {
                        Ok(t) => transforms.push(t),
                        Err(e) => {
                            if self.strict_parsing {
                                return Err(e);
                            }
                        }
                    }
                }
                if transforms.is_empty() {
                    return Err(OcioError::Validation(
                        "empty transform group".into(),
                    ));
                }
                Ok(Transform::group(transforms))
            }
        }
    }

    /// Parses a single transform definition.
    fn parse_raw_transform_def(&self, def: &RawTransformDef) -> OcioResult<Transform> {
        // MatrixTransform
        if let Some(m) = &def.matrix {
            return Ok(Transform::Matrix(MatrixTransform {
                matrix: parse_matrix_16(&m.matrix),
                offset: parse_offset_4(&m.offset),
                direction: parse_direction(&m.direction),
            }));
        }

        // FileTransform (LUT files)
        if let Some(f) = &def.file {
            let resolved = self.context.resolve(&f.src);
            let resolved_path = self
                .resolve_file(&resolved)
                .unwrap_or_else(|| self.working_dir.join(&resolved));
            return Ok(Transform::FileTransform(FileTransform {
                src: resolved_path,
                ccc_id: f.cccid.clone(),
                interpolation: parse_interpolation(&f.interpolation),
                direction: parse_direction(&f.direction),
            }));
        }

        // ExponentTransform
        if let Some(e) = &def.exponent {
            let val = &e.value;
            let value = match val.len() {
                4 => [val[0], val[1], val[2], val[3]],
                3 => [val[0], val[1], val[2], 1.0],
                1 => [val[0], val[0], val[0], 1.0],
                _ => [1.0, 1.0, 1.0, 1.0],
            };
            return Ok(Transform::Exponent(ExponentTransform {
                value,
                negative_style: NegativeStyle::Clamp,
                direction: parse_direction(&e.direction),
            }));
        }

        // LogTransform
        if let Some(l) = &def.log {
            return Ok(Transform::Log(LogTransform {
                base: l.base.unwrap_or(2.0),
                direction: parse_direction(&l.direction),
            }));
        }

        // CDLTransform
        if let Some(c) = &def.cdl {
            return Ok(Transform::Cdl(CdlTransform {
                slope: parse_rgb(&c.slope, 1.0),
                offset: parse_rgb(&c.offset, 0.0),
                power: parse_rgb(&c.power, 1.0),
                saturation: c.saturation.unwrap_or(1.0),
                style: CdlStyle::AscCdl,
                direction: parse_direction(&c.direction),
            }));
        }

        // ColorSpaceTransform
        if let Some(cs) = &def.colorspace {
            return Ok(Transform::ColorSpace(ColorSpaceTransform {
                src: cs.src.clone(),
                dst: cs.dst.clone(),
                direction: parse_direction(&cs.direction),
            }));
        }

        // BuiltinTransform
        if let Some(b) = &def.builtin {
            return Ok(Transform::Builtin(BuiltinTransform {
                style: b.style.clone(),
                direction: parse_direction(&b.direction),
            }));
        }

        // RangeTransform
        if let Some(r) = &def.range {
            let style = match r.style.as_deref() {
                Some("noClamp") | Some("noclamp") | Some("NoClamp") | Some("NOCLAMP") => {
                    RangeStyle::NoClamp
                }
                _ => RangeStyle::Clamp,
            };
            return Ok(Transform::Range(RangeTransform {
                min_in: r.min_in_value,
                max_in: r.max_in_value,
                min_out: r.min_out_value,
                max_out: r.max_out_value,
                style,
                direction: parse_direction(&r.direction),
            }));
        }

        Err(OcioError::Validation(
            "unknown transform type".into(),
        ))
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
            }
        } else if let Some(t) = dst_cs.from_reference() {
            transforms.push(t.clone());
        }

        if transforms.is_empty() {
            return Ok(Processor::new());
        }

        let group = Transform::group(transforms);
        Processor::from_transform(&group, TransformDirection::Forward)
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
    regex: Option<String>,
    colorspace: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawTransform {
    Single(Box<RawTransformDef>),
    Group(Vec<RawTransformDef>),
}

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

#[derive(Debug, Deserialize)]
struct RawMatrixTransform {
    matrix: Option<Vec<f64>>,
    offset: Option<Vec<f64>>,
    direction: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawFileTransform {
    src: String,
    cccid: Option<String>,
    interpolation: Option<String>,
    direction: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawExponentTransform {
    value: Vec<f64>,
    direction: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawLogTransform {
    base: Option<f64>,
    direction: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawCdlTransform {
    slope: Option<Vec<f64>>,
    offset: Option<Vec<f64>>,
    power: Option<Vec<f64>>,
    saturation: Option<f64>,
    direction: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawColorSpaceTransform {
    src: String,
    dst: String,
    direction: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawBuiltinTransform {
    style: String,
    direction: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawRangeTransform {
    min_in_value: Option<f64>,
    max_in_value: Option<f64>,
    min_out_value: Option<f64>,
    max_out_value: Option<f64>,
    style: Option<String>,
    direction: Option<String>,
}

// ============================================================================
// Helper functions for parsing raw transform data
// ============================================================================

/// Parses direction string to TransformDirection.
fn parse_direction(dir: &Option<String>) -> TransformDirection {
    match dir.as_deref() {
        Some("inverse") | Some("Inverse") | Some("INVERSE") => TransformDirection::Inverse,
        _ => TransformDirection::Forward,
    }
}

/// Parses interpolation string.
fn parse_interpolation(interp: &Option<String>) -> Interpolation {
    match interp.as_deref() {
        Some("nearest") | Some("Nearest") | Some("NEAREST") => Interpolation::Nearest,
        Some("tetrahedral") | Some("Tetrahedral") | Some("TETRAHEDRAL") => Interpolation::Tetrahedral,
        Some("best") | Some("Best") | Some("BEST") => Interpolation::Best,
        _ => Interpolation::Linear,
    }
}

/// Parses 16-element matrix, pads with identity if needed.
fn parse_matrix_16(m: &Option<Vec<f64>>) -> [f64; 16] {
    let identity = MatrixTransform::IDENTITY;
    match m {
        Some(v) if v.len() >= 16 => [
            v[0], v[1], v[2], v[3],
            v[4], v[5], v[6], v[7],
            v[8], v[9], v[10], v[11],
            v[12], v[13], v[14], v[15],
        ],
        Some(v) if v.len() >= 12 => [
            // 3x4 matrix (common in OCIO)
            v[0], v[1], v[2], 0.0,
            v[3], v[4], v[5], 0.0,
            v[6], v[7], v[8], 0.0,
            v[9], v[10], v[11], 1.0,
        ],
        Some(v) if v.len() >= 9 => [
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
fn parse_offset_4(o: &Option<Vec<f64>>) -> [f64; 4] {
    match o {
        Some(v) if v.len() >= 4 => [v[0], v[1], v[2], v[3]],
        Some(v) if v.len() >= 3 => [v[0], v[1], v[2], 0.0],
        _ => [0.0, 0.0, 0.0, 0.0],
    }
}

/// Parses RGB values with default.
fn parse_rgb(v: &Option<Vec<f64>>, default: f64) -> [f64; 3] {
    match v {
        Some(vec) if vec.len() >= 3 => [vec[0], vec[1], vec[2]],
        Some(vec) if vec.len() == 1 => [vec[0], vec[0], vec[0]],
        _ => [default, default, default],
    }
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
}
