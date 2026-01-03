//! Look definitions for creative color transforms.
//!
//! Looks are named creative transforms that can be applied on top of the
//! display pipeline. They're typically used for:
//! - Show/shot-specific color grades
//! - Creative LUTs
//! - Per-sequence adjustments
//!
//! # Example
//!
//! ```
//! use vfx_ocio::Look;
//!
//! let look = Look::new("Show LUT")
//!     .process_space("ACEScct")
//!     .description("Main show look");
//!
//! assert_eq!(look.name(), "Show LUT");
//! assert_eq!(look.get_process_space(), Some("ACEScct"));
//! ```

use crate::transform::Transform;

/// A named creative look/grade.
#[derive(Debug, Clone)]
pub struct Look {
    /// Look name.
    name: String,
    /// Process space (color space where transform is applied).
    process_space: Option<String>,
    /// Description.
    description: String,
    /// Forward transform.
    transform: Option<Transform>,
    /// Inverse transform (optional, for reversibility).
    inverse_transform: Option<Transform>,
}

impl Look {
    /// Creates a new look with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            process_space: None,
            description: String::new(),
            transform: None,
            inverse_transform: None,
        }
    }

    /// Sets the process space.
    pub fn process_space(mut self, space: impl Into<String>) -> Self {
        self.process_space = Some(space.into());
        self
    }

    /// Sets the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Sets the forward transform.
    pub fn transform(mut self, t: Transform) -> Self {
        self.transform = Some(t);
        self
    }

    /// Sets the inverse transform.
    pub fn inverse_transform(mut self, t: Transform) -> Self {
        self.inverse_transform = Some(t);
        self
    }

    /// Returns the look name.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the process space name.
    #[inline]
    pub fn get_process_space(&self) -> Option<&str> {
        self.process_space.as_deref()
    }

    /// Returns the description.
    #[inline]
    pub fn get_description(&self) -> &str {
        &self.description
    }

    /// Returns the forward transform.
    #[inline]
    pub fn get_transform(&self) -> Option<&Transform> {
        self.transform.as_ref()
    }

    /// Returns the inverse transform.
    #[inline]
    pub fn get_inverse_transform(&self) -> Option<&Transform> {
        self.inverse_transform.as_ref()
    }

    /// Checks if this look has an explicit inverse.
    #[inline]
    pub fn has_inverse(&self) -> bool {
        self.inverse_transform.is_some()
    }
}

/// Collection of looks.
#[derive(Debug, Clone, Default)]
pub struct LookManager {
    /// All looks.
    looks: Vec<Look>,
}

impl LookManager {
    /// Creates an empty look manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a look.
    pub fn add(&mut self, look: Look) {
        self.looks.push(look);
    }

    /// Gets a look by name.
    pub fn get(&self, name: &str) -> Option<&Look> {
        self.looks
            .iter()
            .find(|l| l.name.eq_ignore_ascii_case(name))
    }

    /// Returns all looks.
    #[inline]
    pub fn all(&self) -> &[Look] {
        &self.looks
    }

    /// Returns look names.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.looks.iter().map(|l| l.name.as_str())
    }

    /// Number of looks.
    #[inline]
    pub fn len(&self) -> usize {
        self.looks.len()
    }

    /// Checks if empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.looks.is_empty()
    }
}

/// Parses a comma-separated look string into individual look names.
///
/// Supports:
/// - Single look: `"ShowLUT"`
/// - Multiple looks: `"ShowLUT, ShotGrade"`
/// - Look with direction: `"+ShowLUT"` (forward), `"-ShowLUT"` (inverse)
///
/// Returns tuples of (name, is_forward).
pub fn parse_looks(looks_str: &str) -> Vec<(&str, bool)> {
    looks_str
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                return None;
            }
            if let Some(name) = s.strip_prefix('-') {
                Some((name.trim(), false))
            } else if let Some(name) = s.strip_prefix('+') {
                Some((name.trim(), true))
            } else {
                Some((s, true))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_look() {
        let look = Look::new("Film Grade")
            .process_space("ACEScct")
            .description("Main film look");

        assert_eq!(look.name(), "Film Grade");
        assert_eq!(look.get_process_space(), Some("ACEScct"));
    }

    #[test]
    fn look_manager() {
        let mut mgr = LookManager::new();
        mgr.add(Look::new("Look A"));
        mgr.add(Look::new("Look B"));

        assert_eq!(mgr.len(), 2);
        assert!(mgr.get("look a").is_some());
    }

    #[test]
    fn parse_looks_single() {
        let looks = parse_looks("ShowLUT");
        assert_eq!(looks, vec![("ShowLUT", true)]);
    }

    #[test]
    fn parse_looks_multiple() {
        let looks = parse_looks("ShowLUT, ShotGrade, Burnin");
        assert_eq!(looks.len(), 3);
        assert_eq!(looks[0], ("ShowLUT", true));
        assert_eq!(looks[1], ("ShotGrade", true));
    }

    #[test]
    fn parse_looks_with_direction() {
        let looks = parse_looks("+Forward, -Inverse");
        assert_eq!(looks[0], ("Forward", true));
        assert_eq!(looks[1], ("Inverse", false));
    }
}
