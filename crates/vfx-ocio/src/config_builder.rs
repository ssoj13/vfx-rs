//! Configuration builder for programmatic OCIO config creation.
//!
//! Provides a fluent API for creating OCIO configurations without parsing YAML.
//!
//! # Example
//!
//! ```
//! use vfx_ocio::{ConfigBuilder, ConfigVersion, ColorSpace, Encoding, Family};
//! use vfx_ocio::{Display, View, Look, Transform, MatrixTransform};
//!
//! let config = ConfigBuilder::new("Studio Config")
//!     .version(ConfigVersion::V2)
//!     .description("Custom studio configuration")
//!     .add_colorspace(
//!         ColorSpace::builder("ACES2065-1")
//!             .encoding(Encoding::SceneLinear)
//!             .family(Family::Aces)
//!             .description("ACES 2065-1 (AP0)")
//!             .build()
//!     )
//!     .add_colorspace(
//!         ColorSpace::builder("ACEScg")
//!             .encoding(Encoding::SceneLinear)
//!             .family(Family::Scene)
//!             .build()
//!     )
//!     .set_role("reference", "ACES2065-1")
//!     .set_role("scene_linear", "ACEScg")
//!     .add_display(
//!         Display::new("sRGB")
//!             .with_view(View::new("Film", "sRGB"))
//!             .with_view(View::new("Raw", "Raw"))
//!     )
//!     .build()
//!     .unwrap();
//! ```

use std::path::PathBuf;

use crate::colorspace::ColorSpace;
use crate::config::{Config, ConfigVersion, FileRule, NamedTransform, SharedView, ViewingRule};
use crate::context::Context;
use crate::display::{Display, DisplayManager};
use crate::error::{OcioError, OcioResult};
use crate::look::{Look, LookManager};
use crate::role::Roles;

/// Builder for creating OCIO configurations programmatically.
///
/// Provides a fluent API for constructing configs without parsing YAML files.
#[derive(Debug, Clone)]
pub struct ConfigBuilder {
    name: String,
    description: String,
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
}

impl ConfigBuilder {
    /// Creates a new config builder with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            version: ConfigVersion::V2,
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
        }
    }

    /// Sets the config description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Sets the config version.
    pub fn version(mut self, version: ConfigVersion) -> Self {
        self.version = version;
        self
    }

    /// Adds a search path for LUT files.
    pub fn search_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.search_paths.push(path.into());
        self
    }

    /// Sets the working directory.
    pub fn working_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_dir = path.into();
        self
    }

    /// Adds a color space to the config.
    pub fn add_colorspace(mut self, colorspace: ColorSpace) -> Self {
        self.colorspaces.push(colorspace);
        self
    }

    /// Adds multiple color spaces at once.
    pub fn add_colorspaces(mut self, colorspaces: impl IntoIterator<Item = ColorSpace>) -> Self {
        self.colorspaces.extend(colorspaces);
        self
    }

    /// Defines a role mapping.
    ///
    /// # Arguments
    ///
    /// * `role` - Role name (e.g., "scene_linear", "reference")
    /// * `colorspace` - Color space name this role maps to
    pub fn set_role(mut self, role: impl Into<String>, colorspace: impl Into<String>) -> Self {
        self.roles.define(role, colorspace);
        self
    }

    /// Adds a display with its views.
    pub fn add_display(mut self, display: Display) -> Self {
        self.displays.add_display(display);
        self
    }

    /// Adds multiple displays at once.
    pub fn add_displays(mut self, displays: impl IntoIterator<Item = Display>) -> Self {
        for display in displays {
            self.displays.add_display(display);
        }
        self
    }

    /// Adds a look (creative color transform).
    pub fn add_look(mut self, look: Look) -> Self {
        self.looks.add(look);
        self
    }

    /// Adds multiple looks at once.
    pub fn add_looks(mut self, looks: impl IntoIterator<Item = Look>) -> Self {
        for look in looks {
            self.looks.add(look);
        }
        self
    }

    /// Sets the active displays (subset shown in UI).
    pub fn active_displays(mut self, displays: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.active_displays = displays.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the active views (subset shown in UI).
    pub fn active_views(mut self, views: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.active_views = views.into_iter().map(Into::into).collect();
        self
    }

    /// Adds a shared view (OCIO v2.3+).
    pub fn add_shared_view(mut self, view: SharedView) -> Self {
        self.shared_views.push(view);
        self
    }

    /// Adds a viewing rule (OCIO v2.0+).
    pub fn add_viewing_rule(mut self, rule: ViewingRule) -> Self {
        self.viewing_rules.push(rule);
        self
    }

    /// Adds a named transform (OCIO v2.0+).
    pub fn add_named_transform(mut self, transform: NamedTransform) -> Self {
        self.named_transforms.push(transform);
        self
    }

    /// Marks color spaces as inactive (hidden from UI).
    pub fn inactive_colorspaces(mut self, names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.inactive_colorspaces = names.into_iter().map(Into::into).collect();
        self
    }

    /// Adds a file rule for automatic color space detection.
    pub fn add_file_rule(mut self, rule: FileRule) -> Self {
        self.file_rules.push(rule);
        self
    }

    /// Sets context variables.
    pub fn context_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.set(key, value);
        self
    }

    /// Builds the config, validating required fields.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No colorspaces are defined
    /// - No reference role is set (required)
    /// - A role references a non-existent colorspace
    pub fn build(self) -> OcioResult<Config> {
        // Validate: must have at least one colorspace
        if self.colorspaces.is_empty() {
            return Err(OcioError::Validation(
                "Config must have at least one colorspace".into(),
            ));
        }

        // Validate: reference role must be defined
        if !self.roles.has_reference() {
            return Err(OcioError::Validation(
                "Config must define a 'reference' role".into(),
            ));
        }

        // Validate: all role references must point to existing colorspaces
        for (role, cs_name) in self.roles.iter() {
            if !self.colorspaces.iter().any(|cs| cs.matches_name(cs_name)) {
                return Err(OcioError::Validation(format!(
                    "Role '{}' references non-existent colorspace '{}'",
                    role, cs_name
                )));
            }
        }

        // Build config via internal constructor
        Ok(Config::from_builder(
            self.name,
            self.description,
            self.version,
            self.search_paths,
            self.working_dir,
            self.colorspaces,
            self.roles,
            self.displays,
            self.looks,
            self.active_displays,
            self.active_views,
            self.shared_views,
            self.viewing_rules,
            self.named_transforms,
            self.inactive_colorspaces,
            self.file_rules,
            self.context,
        ))
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new("Untitled")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colorspace::{Encoding, Family};
    use crate::display::View;

    #[test]
    fn build_minimal_config() {
        let config = ConfigBuilder::new("Test")
            .add_colorspace(
                ColorSpace::builder("Linear")
                    .encoding(Encoding::SceneLinear)
                    .build(),
            )
            .set_role("reference", "Linear")
            .build()
            .unwrap();

        assert_eq!(config.colorspaces().len(), 1);
        assert!(config.colorspace("Linear").is_some());
    }

    #[test]
    fn build_with_display() {
        let config = ConfigBuilder::new("Test")
            .add_colorspace(
                ColorSpace::builder("Linear")
                    .encoding(Encoding::SceneLinear)
                    .build(),
            )
            .add_colorspace(
                ColorSpace::builder("sRGB")
                    .encoding(Encoding::Sdr)
                    .build(),
            )
            .set_role("reference", "Linear")
            .add_display(
                Display::new("sRGB Monitor")
                    .with_view(View::new("Standard", "sRGB"))
            )
            .build()
            .unwrap();

        assert_eq!(config.displays().len(), 1);
    }

    #[test]
    fn build_with_roles() {
        let config = ConfigBuilder::new("Test")
            .add_colorspace(
                ColorSpace::builder("ACES2065-1")
                    .encoding(Encoding::SceneLinear)
                    .family(Family::Aces)
                    .build(),
            )
            .add_colorspace(
                ColorSpace::builder("ACEScg")
                    .encoding(Encoding::SceneLinear)
                    .family(Family::Scene)
                    .build(),
            )
            .set_role("reference", "ACES2065-1")
            .set_role("scene_linear", "ACEScg")
            .build()
            .unwrap();

        // scene_linear should resolve to ACEScg
        assert!(config.colorspace("scene_linear").is_some());
        assert_eq!(config.colorspace("scene_linear").unwrap().name(), "ACEScg");
    }

    #[test]
    fn build_fails_without_colorspaces() {
        let result = ConfigBuilder::new("Test")
            .set_role("reference", "Linear")
            .build();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("colorspace"));
    }

    #[test]
    fn build_fails_without_reference() {
        let result = ConfigBuilder::new("Test")
            .add_colorspace(ColorSpace::builder("Linear").build())
            .build();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("reference"));
    }

    #[test]
    fn build_fails_with_invalid_role() {
        let result = ConfigBuilder::new("Test")
            .add_colorspace(ColorSpace::builder("Linear").build())
            .set_role("reference", "NonExistent")
            .build();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("NonExistent"));
    }
}
