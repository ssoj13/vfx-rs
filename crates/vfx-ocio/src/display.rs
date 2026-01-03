//! Display and View management.
//!
//! OCIO organizes output transforms through displays and views:
//! - **Display**: A physical or virtual display device (monitor, projector)
//! - **View**: A viewing condition for that display (SDR, HDR, Raw, etc.)
//!
//! # Example
//!
//! ```
//! use vfx_ocio::{Display, View};
//!
//! let mut display = Display::new("sRGB Monitor");
//! display.add_view(View::new("Film", "sRGB")
//!     .with_look("Show LUT"));
//! display.add_view(View::new("Raw", "Raw"));
//!
//! assert_eq!(display.views().len(), 2);
//! ```

use crate::transform::Transform;

/// A view within a display.
///
/// Views define how to transform from the working space to the display.
#[derive(Debug, Clone)]
pub struct View {
    /// View name (e.g., "Film", "Raw", "Log").
    name: String,
    /// Target color space name.
    colorspace: String,
    /// Optional look to apply.
    looks: Option<String>,
    /// Optional view transform (OCIO v2).
    view_transform: Option<String>,
    /// Rule for matching input files.
    rule: Option<String>,
    /// Description.
    description: String,
}

impl View {
    /// Creates a new view.
    ///
    /// # Arguments
    ///
    /// * `name` - View name
    /// * `colorspace` - Target color space name
    pub fn new(name: impl Into<String>, colorspace: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            colorspace: colorspace.into(),
            looks: None,
            view_transform: None,
            rule: None,
            description: String::new(),
        }
    }

    /// Adds a look to apply.
    pub fn with_look(mut self, looks: impl Into<String>) -> Self {
        self.looks = Some(looks.into());
        self
    }

    /// Sets the view transform (OCIO v2).
    pub fn with_view_transform(mut self, vt: impl Into<String>) -> Self {
        self.view_transform = Some(vt.into());
        self
    }

    /// Sets a rule.
    pub fn with_rule(mut self, rule: impl Into<String>) -> Self {
        self.rule = Some(rule.into());
        self
    }

    /// Sets description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Returns the view name.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the target color space.
    #[inline]
    pub fn colorspace(&self) -> &str {
        &self.colorspace
    }

    /// Returns the look(s) to apply.
    #[inline]
    pub fn looks(&self) -> Option<&str> {
        self.looks.as_deref()
    }

    /// Returns the view transform name.
    #[inline]
    pub fn view_transform(&self) -> Option<&str> {
        self.view_transform.as_deref()
    }

    /// Returns the rule.
    #[inline]
    pub fn rule(&self) -> Option<&str> {
        self.rule.as_deref()
    }

    /// Returns the description.
    #[inline]
    pub fn description(&self) -> &str {
        &self.description
    }
}

/// A display device configuration.
///
/// Contains multiple views for different viewing conditions.
#[derive(Debug, Clone)]
pub struct Display {
    /// Display name (e.g., "sRGB", "Rec.709", "DCI-P3").
    name: String,
    /// Available views for this display.
    views: Vec<View>,
    /// Default view name.
    default_view: Option<String>,
}

impl Display {
    /// Creates a new display.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            views: Vec::new(),
            default_view: None,
        }
    }

    /// Returns the display name.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Adds a view to this display.
    pub fn add_view(&mut self, view: View) {
        if self.default_view.is_none() {
            self.default_view = Some(view.name.clone());
        }
        self.views.push(view);
    }

    /// Sets the default view.
    pub fn set_default_view(&mut self, name: impl Into<String>) {
        self.default_view = Some(name.into());
    }

    /// Returns all views.
    #[inline]
    pub fn views(&self) -> &[View] {
        &self.views
    }

    /// Gets a view by name.
    pub fn view(&self, name: &str) -> Option<&View> {
        self.views.iter().find(|v| v.name.eq_ignore_ascii_case(name))
    }

    /// Returns the default view name.
    #[inline]
    pub fn default_view(&self) -> Option<&str> {
        self.default_view.as_deref()
    }

    /// Returns view names.
    pub fn view_names(&self) -> impl Iterator<Item = &str> {
        self.views.iter().map(|v| v.name.as_str())
    }
}

/// View transform definition (OCIO v2).
///
/// View transforms are shared transforms that can be reused across views.
#[derive(Debug, Clone)]
pub struct ViewTransform {
    /// Name.
    name: String,
    /// Family (for categorization).
    family: String,
    /// Description.
    description: String,
    /// Transform from scene reference.
    from_scene_reference: Option<Transform>,
    /// Transform to scene reference.
    to_scene_reference: Option<Transform>,
    /// Transform from display reference.
    from_display_reference: Option<Transform>,
    /// Transform to display reference.
    to_display_reference: Option<Transform>,
}

impl ViewTransform {
    /// Creates a new view transform.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            family: String::new(),
            description: String::new(),
            from_scene_reference: None,
            to_scene_reference: None,
            from_display_reference: None,
            to_display_reference: None,
        }
    }

    /// Returns the name.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the family.
    #[inline]
    pub fn family(&self) -> &str {
        &self.family
    }

    /// Sets the family.
    pub fn with_family(mut self, family: impl Into<String>) -> Self {
        self.family = family.into();
        self
    }

    /// Returns the description.
    #[inline]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Sets the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Sets transform from scene reference.
    pub fn with_from_scene_reference(mut self, t: Transform) -> Self {
        self.from_scene_reference = Some(t);
        self
    }

    /// Sets transform to scene reference.
    pub fn with_to_scene_reference(mut self, t: Transform) -> Self {
        self.to_scene_reference = Some(t);
        self
    }

    /// Sets transform from display reference.
    pub fn with_from_display_reference(mut self, t: Transform) -> Self {
        self.from_display_reference = Some(t);
        self
    }

    /// Sets transform to display reference.
    pub fn with_to_display_reference(mut self, t: Transform) -> Self {
        self.to_display_reference = Some(t);
        self
    }

    /// Gets transform from scene reference.
    #[inline]
    pub fn from_scene_reference(&self) -> Option<&Transform> {
        self.from_scene_reference.as_ref()
    }

    /// Gets transform to scene reference.
    #[inline]
    pub fn to_scene_reference(&self) -> Option<&Transform> {
        self.to_scene_reference.as_ref()
    }

    /// Gets transform from display reference.
    #[inline]
    pub fn from_display_reference(&self) -> Option<&Transform> {
        self.from_display_reference.as_ref()
    }

    /// Gets transform to display reference.
    #[inline]
    pub fn to_display_reference(&self) -> Option<&Transform> {
        self.to_display_reference.as_ref()
    }
}

/// Collection of displays.
#[derive(Debug, Clone, Default)]
pub struct DisplayManager {
    /// All displays.
    displays: Vec<Display>,
    /// Default display name.
    default_display: Option<String>,
    /// View transforms (OCIO v2).
    view_transforms: Vec<ViewTransform>,
}

impl DisplayManager {
    /// Creates an empty display manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a display.
    pub fn add_display(&mut self, display: Display) {
        if self.default_display.is_none() {
            self.default_display = Some(display.name.clone());
        }
        self.displays.push(display);
    }

    /// Sets the default display.
    pub fn set_default_display(&mut self, name: impl Into<String>) {
        self.default_display = Some(name.into());
    }

    /// Returns all displays.
    #[inline]
    pub fn displays(&self) -> &[Display] {
        &self.displays
    }

    /// Gets a display by name.
    pub fn display(&self, name: &str) -> Option<&Display> {
        self.displays
            .iter()
            .find(|d| d.name.eq_ignore_ascii_case(name))
    }

    /// Gets mutable display by name.
    pub fn display_mut(&mut self, name: &str) -> Option<&mut Display> {
        self.displays
            .iter_mut()
            .find(|d| d.name.eq_ignore_ascii_case(name))
    }

    /// Returns the default display name.
    #[inline]
    pub fn default_display(&self) -> Option<&str> {
        self.default_display.as_deref()
    }

    /// Returns display names.
    pub fn display_names(&self) -> impl Iterator<Item = &str> {
        self.displays.iter().map(|d| d.name.as_str())
    }

    /// Adds a view transform.
    pub fn add_view_transform(&mut self, vt: ViewTransform) {
        self.view_transforms.push(vt);
    }

    /// Gets a view transform by name.
    pub fn view_transform(&self, name: &str) -> Option<&ViewTransform> {
        self.view_transforms
            .iter()
            .find(|vt| vt.name.eq_ignore_ascii_case(name))
    }

    /// Returns all view transforms.
    #[inline]
    pub fn view_transforms(&self) -> &[ViewTransform] {
        &self.view_transforms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_display() {
        let mut display = Display::new("sRGB Monitor");
        display.add_view(View::new("Film", "sRGB"));
        display.add_view(View::new("Raw", "Raw"));

        assert_eq!(display.name(), "sRGB Monitor");
        assert_eq!(display.views().len(), 2);
        assert_eq!(display.default_view(), Some("Film"));
    }

    #[test]
    fn view_with_look() {
        let view = View::new("Graded", "sRGB").with_look("Show LUT");

        assert_eq!(view.looks(), Some("Show LUT"));
    }

    #[test]
    fn display_manager() {
        let mut mgr = DisplayManager::new();

        let mut srgb = Display::new("sRGB");
        srgb.add_view(View::new("Film", "sRGB"));
        mgr.add_display(srgb);

        let mut rec709 = Display::new("Rec.709");
        rec709.add_view(View::new("Video", "Rec709"));
        mgr.add_display(rec709);

        assert_eq!(mgr.displays().len(), 2);
        assert_eq!(mgr.default_display(), Some("sRGB"));
        assert!(mgr.display("rec.709").is_some());
    }
}
