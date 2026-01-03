//! Typed attribute storage for image metadata.
//!
//! This module provides a rich attribute system for storing and manipulating
//! image metadata extracted from various file formats. It supports EXIF-style
//! typed values including rationals, byte arrays, and nested groups.
//!
//! # Overview
//!
//! The attribute system consists of:
//! - [`AttrValue`] - Typed metadata value (string, int, rational, bytes, etc.)
//! - [`Attrs`] - Container mapping string keys to typed values
//!
//! # Example
//!
//! ```rust
//! use vfx_io::attrs::{Attrs, AttrValue};
//!
//! let mut attrs = Attrs::new();
//! attrs.set("Make", AttrValue::Str("Canon".to_string()));
//! attrs.set("ISO", AttrValue::UInt(400));
//! attrs.set("ExposureTime", AttrValue::URational(1, 125));
//!
//! assert_eq!(attrs.get_str("Make"), Some("Canon"));
//! assert_eq!(attrs.get_u32("ISO"), Some(400));
//! ```
//!
//! # Nested Groups
//!
//! Attributes can contain nested groups for hierarchical metadata like MakerNotes:
//!
//! ```rust
//! use vfx_io::attrs::{Attrs, AttrValue};
//!
//! let mut attrs = Attrs::new();
//! attrs.set_path("Canon:AFInfo:NumPoints", 45u32);
//! attrs.set_path("Canon:AFInfo:ValidPoints", 9u32);
//!
//! // Access via path
//! assert_eq!(attrs.get_path("Canon:AFInfo:NumPoints").and_then(|v| v.as_u32()), Some(45));
//! ```
//!
//! # Design
//!
//! Based on exiftool-attrs from exiftool-rs project, adapted for vfx-io:
//! - Removed serde dependency for core functionality
//! - Simplified for image I/O use case
//! - Full EXIF type support (rationals, bytes, datetime)

mod value;

pub use value::AttrValue;

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Attribute container: string key -> typed value.
///
/// Provides storage for image metadata with:
/// - Type-safe value storage via [`AttrValue`]
/// - Nested groups for hierarchical data (MakerNotes, XMP)
/// - Path-based access (e.g., "Canon:AFInfo:Mode")
/// - Iteration and lookup utilities
///
/// # Usage in vfx-io
///
/// Every [`ImageData`](crate::ImageData) contains an `Attrs` in its metadata.
/// Format readers populate it during parsing, and writers can preserve or
/// modify values during encoding.
///
/// # Example
///
/// ```rust
/// use vfx_io::attrs::{Attrs, AttrValue};
///
/// let mut attrs = Attrs::new();
///
/// // Basic usage
/// attrs.set("ImageWidth", AttrValue::UInt(1920));
/// attrs.set("ImageHeight", AttrValue::UInt(1080));
/// attrs.set("Software", AttrValue::Str("vfx-io".to_string()));
///
/// // Type-specific getters
/// assert_eq!(attrs.get_u32("ImageWidth"), Some(1920));
/// assert_eq!(attrs.get_str("Software"), Some("vfx-io"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct Attrs {
    /// Internal storage map.
    map: HashMap<String, AttrValue>,
}

impl Attrs {
    /// Creates an empty attribute container.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_io::attrs::Attrs;
    ///
    /// let attrs = Attrs::new();
    /// assert!(attrs.is_empty());
    /// ```
    #[inline]
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Creates an attribute container with pre-allocated capacity.
    ///
    /// Use when you know approximately how many attributes will be stored.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Number of entries to pre-allocate
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
        }
    }

    /// Sets an attribute value.
    ///
    /// If the key already exists, the previous value is replaced.
    ///
    /// # Arguments
    ///
    /// * `key` - Attribute name (e.g., "Make", "ISO", "ExposureTime")
    /// * `value` - Typed attribute value
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_io::attrs::{Attrs, AttrValue};
    ///
    /// let mut attrs = Attrs::new();
    /// attrs.set("ISO", AttrValue::UInt(400));
    /// attrs.set("ISO", AttrValue::UInt(800)); // Replaces previous
    ///
    /// assert_eq!(attrs.get_u32("ISO"), Some(800));
    /// ```
    #[inline]
    pub fn set(&mut self, key: impl Into<String>, value: AttrValue) {
        self.map.insert(key.into(), value);
    }

    /// Gets an attribute value by key.
    ///
    /// Returns `None` if the key doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `key` - Attribute name to look up
    #[inline]
    pub fn get(&self, key: &str) -> Option<&AttrValue> {
        self.map.get(key)
    }

    /// Gets a mutable reference to an attribute value.
    ///
    /// Useful for modifying values in place.
    #[inline]
    pub fn get_mut(&mut self, key: &str) -> Option<&mut AttrValue> {
        self.map.get_mut(key)
    }

    /// Removes an attribute by key.
    ///
    /// Returns the removed value if it existed.
    #[inline]
    pub fn remove(&mut self, key: &str) -> Option<AttrValue> {
        self.map.remove(key)
    }

    /// Checks if an attribute exists.
    #[inline]
    pub fn contains(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    /// Returns the number of attributes.
    #[inline]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns true if no attributes are stored.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Iterates over all (key, value) pairs.
    ///
    /// Order is not guaranteed.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&String, &AttrValue)> {
        self.map.iter()
    }

    /// Iterates mutably over all (key, value) pairs.
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&String, &mut AttrValue)> {
        self.map.iter_mut()
    }

    /// Clears all attributes.
    #[inline]
    pub fn clear(&mut self) {
        self.map.clear();
    }

    // === Type-specific getters ===

    /// Gets a string value.
    ///
    /// Returns `None` if the key doesn't exist or is not a string.
    #[inline]
    pub fn get_str(&self, key: &str) -> Option<&str> {
        match self.map.get(key) {
            Some(AttrValue::Str(s)) => Some(s),
            _ => None,
        }
    }

    /// Gets an i32 value.
    #[inline]
    pub fn get_i32(&self, key: &str) -> Option<i32> {
        match self.map.get(key) {
            Some(AttrValue::Int(v)) => Some(*v),
            _ => None,
        }
    }

    /// Gets a u32 value.
    #[inline]
    pub fn get_u32(&self, key: &str) -> Option<u32> {
        match self.map.get(key) {
            Some(AttrValue::UInt(v)) => Some(*v),
            _ => None,
        }
    }

    /// Gets an f32 value.
    #[inline]
    pub fn get_f32(&self, key: &str) -> Option<f32> {
        match self.map.get(key) {
            Some(AttrValue::Float(v)) => Some(*v),
            _ => None,
        }
    }

    /// Gets an f64 value.
    #[inline]
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        match self.map.get(key) {
            Some(AttrValue::Double(v)) => Some(*v),
            _ => None,
        }
    }

    /// Gets a bool value.
    #[inline]
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.map.get(key) {
            Some(AttrValue::Bool(v)) => Some(*v),
            _ => None,
        }
    }

    /// Gets a byte slice value.
    #[inline]
    pub fn get_bytes(&self, key: &str) -> Option<&[u8]> {
        match self.map.get(key) {
            Some(AttrValue::Bytes(v)) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// Gets a signed rational value (numerator, denominator).
    ///
    /// Used for EXIF values like ExposureBiasValue.
    #[inline]
    pub fn get_rational(&self, key: &str) -> Option<(i32, i32)> {
        match self.map.get(key) {
            Some(AttrValue::Rational(n, d)) => Some((*n, *d)),
            _ => None,
        }
    }

    /// Gets an unsigned rational value (numerator, denominator).
    ///
    /// Used for EXIF values like ExposureTime (1/125), FNumber (f/2.8).
    #[inline]
    pub fn get_urational(&self, key: &str) -> Option<(u32, u32)> {
        match self.map.get(key) {
            Some(AttrValue::URational(n, d)) => Some((*n, *d)),
            _ => None,
        }
    }

    // === Nested Group Support ===

    /// Gets or creates a nested group by key.
    ///
    /// Creates the group if it doesn't exist.
    /// Used for hierarchical metadata like MakerNotes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_io::attrs::{Attrs, AttrValue};
    ///
    /// let mut attrs = Attrs::new();
    /// let canon = attrs.group_mut("Canon");
    /// canon.set("ModelID", AttrValue::UInt(0x80000001));
    /// ```
    pub fn group_mut(&mut self, key: &str) -> &mut Attrs {
        if !matches!(self.map.get(key), Some(AttrValue::Group(_))) {
            self.map
                .insert(key.to_string(), AttrValue::Group(Box::new(Attrs::new())));
        }

        match self.map.get_mut(key) {
            Some(AttrValue::Group(attrs)) => attrs.as_mut(),
            _ => unreachable!(),
        }
    }

    /// Gets a nested group by key (read-only).
    ///
    /// Returns `None` if the key doesn't exist or is not a group.
    pub fn group(&self, key: &str) -> Option<&Attrs> {
        match self.map.get(key) {
            Some(AttrValue::Group(attrs)) => Some(attrs.as_ref()),
            _ => None,
        }
    }

    /// Sets a value by colon-separated path.
    ///
    /// Creates intermediate groups as needed.
    ///
    /// # Arguments
    ///
    /// * `path` - Colon-separated path (e.g., "Canon:AFInfo:Mode")
    /// * `value` - Value to set
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_io::attrs::Attrs;
    ///
    /// let mut attrs = Attrs::new();
    /// attrs.set_path("Canon:AFInfo:NumPoints", 45u32);
    /// attrs.set_path("Canon:AFInfo:ValidPoints", 9u32);
    /// ```
    pub fn set_path(&mut self, path: &str, value: impl Into<AttrValue>) {
        let parts: Vec<&str> = path.split(':').collect();
        if parts.is_empty() {
            return;
        }

        if parts.len() == 1 {
            self.set(parts[0], value.into());
            return;
        }

        // Navigate/create intermediate groups
        let mut current = self.group_mut(parts[0]);
        for part in &parts[1..parts.len() - 1] {
            current = current.group_mut(part);
        }
        current.set(parts[parts.len() - 1], value.into());
    }

    /// Gets a value by colon-separated path.
    ///
    /// # Arguments
    ///
    /// * `path` - Colon-separated path (e.g., "Canon:AFInfo:Mode")
    ///
    /// # Returns
    ///
    /// The value if found, or `None` if any part of the path doesn't exist.
    pub fn get_path(&self, path: &str) -> Option<&AttrValue> {
        let parts: Vec<&str> = path.split(':').collect();
        if parts.is_empty() {
            return None;
        }

        if parts.len() == 1 {
            return self.get(parts[0]);
        }

        // Navigate through groups
        let mut current = self.group(parts[0])?;
        for part in &parts[1..parts.len() - 1] {
            current = current.group(part)?;
        }
        current.get(parts[parts.len() - 1])
    }

    /// Iterates over all values recursively, yielding (path, value) pairs.
    ///
    /// Paths are colon-separated (e.g., "Canon:AFInfo:Mode").
    /// Groups themselves are not yielded, only their leaf values.
    pub fn iter_flat(&self) -> FlatIter<'_> {
        FlatIter::new(self)
    }

    /// Counts all values recursively (including nested groups).
    pub fn count_recursive(&self) -> usize {
        let mut count = 0;
        for value in self.map.values() {
            match value {
                AttrValue::Group(nested) => count += nested.count_recursive(),
                _ => count += 1,
            }
        }
        count
    }

    /// Computes a hash of all attributes in sorted key order.
    ///
    /// Useful for comparing metadata or detecting changes.
    pub fn hash_all(&self) -> u64 {
        let mut keys: Vec<&String> = self.map.keys().collect();
        keys.sort_unstable();

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for key in keys {
            key.hash(&mut hasher);
            if let Some(val) = self.map.get(key) {
                val.hash(&mut hasher);
            }
        }
        hasher.finish()
    }
}

impl std::fmt::Display for Attrs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut keys: Vec<&String> = self.map.keys().collect();
        keys.sort();

        for key in keys {
            if let Some(value) = self.map.get(key) {
                writeln!(f, "{}: {}", key, value)?;
            }
        }
        Ok(())
    }
}

/// Iterator that flattens nested Attrs into (path, value) pairs.
///
/// Created by [`Attrs::iter_flat`].
pub struct FlatIter<'a> {
    stack: Vec<(String, std::collections::hash_map::Iter<'a, String, AttrValue>)>,
}

impl<'a> FlatIter<'a> {
    fn new(attrs: &'a Attrs) -> Self {
        Self {
            stack: vec![(String::new(), attrs.map.iter())],
        }
    }
}

impl<'a> Iterator for FlatIter<'a> {
    type Item = (String, &'a AttrValue);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (prefix, iter) = self.stack.last_mut()?;

            if let Some((key, value)) = iter.next() {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}:{}", prefix, key)
                };

                match value {
                    AttrValue::Group(nested) => {
                        self.stack.push((path, nested.map.iter()));
                        continue;
                    }
                    _ => return Some((path, value)),
                }
            } else {
                self.stack.pop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut attrs = Attrs::new();
        attrs.set("Make", AttrValue::Str("Canon".to_string()));
        attrs.set("ISO", AttrValue::UInt(400));

        assert_eq!(attrs.get_str("Make"), Some("Canon"));
        assert_eq!(attrs.get_u32("ISO"), Some(400));
        assert_eq!(attrs.len(), 2);
    }

    #[test]
    fn test_nested_groups() {
        let mut attrs = Attrs::new();
        attrs.set_path("Canon:AFInfo:NumPoints", 45u32);
        attrs.set_path("Canon:AFInfo:ValidPoints", 9u32);

        assert_eq!(
            attrs
                .get_path("Canon:AFInfo:NumPoints")
                .and_then(|v| v.as_u32()),
            Some(45)
        );
        assert_eq!(
            attrs
                .get_path("Canon:AFInfo:ValidPoints")
                .and_then(|v| v.as_u32()),
            Some(9)
        );
    }

    #[test]
    fn test_flat_iter() {
        let mut attrs = Attrs::new();
        attrs.set("TopLevel", AttrValue::UInt(1));
        attrs.set_path("Group:Nested", 2u32);

        let flat: Vec<_> = attrs.iter_flat().collect();
        assert_eq!(flat.len(), 2);
    }
}
