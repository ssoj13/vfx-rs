//! Lightweight metadata storage inspired by exiftool-attrs.
//!
//! Provides typed attribute storage for image metadata.

use std::collections::HashMap;

/// Typed metadata value.
#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    /// Boolean value.
    Bool(bool),
    /// UTF-8 string value.
    Str(String),
    /// Signed 32-bit integer.
    Int(i32),
    /// Unsigned 32-bit integer.
    UInt(u32),
    /// Signed 64-bit integer.
    Int64(i64),
    /// Unsigned 64-bit integer.
    UInt64(u64),
    /// 32-bit float.
    Float(f32),
    /// 64-bit float.
    Double(f64),
    /// Raw byte blob.
    Bytes(Vec<u8>),
    /// Ordered list of values.
    List(Vec<AttrValue>),
    /// Map of string keys to values.
    Map(HashMap<String, AttrValue>),
}

impl AttrValue {
    /// Returns string slice if this is a string value.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            AttrValue::Str(v) => Some(v),
            _ => None,
        }
    }

    /// Returns u32 if this is a UInt value.
    pub fn as_u32(&self) -> Option<u32> {
        match self {
            AttrValue::UInt(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns f32 if this is a Float value.
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            AttrValue::Float(v) => Some(*v),
            _ => None,
        }
    }
}

/// Attribute container: key -> typed value.
#[derive(Debug, Clone, Default)]
pub struct Attrs {
    map: HashMap<String, AttrValue>,
}

impl Attrs {
    /// Creates an empty attribute map.
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Inserts or replaces an attribute.
    pub fn set(&mut self, key: impl Into<String>, value: AttrValue) {
        self.map.insert(key.into(), value);
    }

    /// Returns a reference to a value by key.
    pub fn get(&self, key: &str) -> Option<&AttrValue> {
        self.map.get(key)
    }

    /// Returns true if the key exists.
    pub fn contains(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    /// Iterates over key/value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &AttrValue)> {
        self.map.iter()
    }

    /// Returns true if empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}
