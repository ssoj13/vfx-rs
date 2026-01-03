//! Attribute value types for image metadata.
//!
//! [`AttrValue`] represents any metadata value that can be stored in an image file.
//! It supports all common EXIF types plus extensions for VFX workflows.
//!
//! # Type Categories
//!
//! ## Basic Types
//! - `Bool`, `Str`, `Int`, `UInt`, `Int64`, `UInt64`, `Float`, `Double`
//!
//! ## EXIF-Specific Types
//! - `Rational(i32, i32)` - Signed rational (e.g., ExposureBiasValue: -1/3)
//! - `URational(u32, u32)` - Unsigned rational (e.g., ExposureTime: 1/125)
//! - `Bytes(Vec<u8>)` - Binary data (e.g., MakerNote, thumbnail)
//!
//! ## Collection Types
//! - `List(Vec<AttrValue>)` - Ordered list of values
//! - `Map(HashMap<String, AttrValue>)` - Key-value pairs
//! - `Group(Box<Attrs>)` - Nested attribute group (for MakerNotes sub-IFDs)
//!
//! # Example
//!
//! ```rust
//! use vfx_io::attrs::AttrValue;
//!
//! // Basic types
//! let make = AttrValue::Str("Canon".to_string());
//! let iso = AttrValue::UInt(400);
//!
//! // EXIF rational: 1/125 second exposure
//! let exposure = AttrValue::URational(1, 125);
//!
//! // Access the value
//! assert_eq!(exposure.as_f64(), Some(1.0 / 125.0));
//! ```

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Typed metadata value supporting EXIF and extended types.
///
/// This enum can represent any value found in image metadata,
/// from simple strings and integers to complex nested structures.
///
/// # Type Coercion
///
/// Use the `as_*` methods for type-safe access with automatic
/// conversion where sensible (e.g., `as_f64` converts rationals to floats).
///
/// # Display
///
/// All variants implement `Display` for human-readable output:
/// - Rationals display as "1/125"
/// - Bytes display as "<1234 bytes>"
/// - Groups display as "<group: 5 attrs>"
#[derive(Debug, Clone)]
#[must_use]
pub enum AttrValue {
    // === Basic types ===
    /// Boolean value.
    Bool(bool),

    /// UTF-8 string value.
    ///
    /// Used for: Make, Model, Software, Artist, Copyright, etc.
    Str(String),

    /// Signed 32-bit integer.
    ///
    /// Used for: Orientation, ColorSpace, etc.
    Int(i32),

    /// Unsigned 32-bit integer.
    ///
    /// Used for: ImageWidth, ImageHeight, ISO, etc.
    UInt(u32),

    /// Signed 64-bit integer.
    ///
    /// Used for: large file sizes, timestamps.
    Int64(i64),

    /// Unsigned 64-bit integer.
    ///
    /// Used for: StripOffsets in large files.
    UInt64(u64),

    /// 32-bit floating point.
    ///
    /// Used for: Gamma, custom float values.
    Float(f32),

    /// 64-bit floating point.
    ///
    /// Used for: GPS coordinates, high-precision values.
    Double(f64),

    // === EXIF-specific types ===
    /// Signed rational (numerator, denominator).
    ///
    /// Used for: ExposureBiasValue, BrightnessValue.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_io::attrs::AttrValue;
    ///
    /// // -2/3 EV exposure compensation
    /// let bias = AttrValue::Rational(-2, 3);
    /// assert_eq!(bias.as_f64(), Some(-2.0 / 3.0));
    /// ```
    Rational(i32, i32),

    /// Unsigned rational (numerator, denominator).
    ///
    /// Used for: ExposureTime (1/125), FNumber (28/10 = f/2.8),
    /// FocalLength (50/1 = 50mm), XResolution, YResolution.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_io::attrs::AttrValue;
    ///
    /// // f/2.8 aperture
    /// let fnumber = AttrValue::URational(28, 10);
    /// assert_eq!(fnumber.as_f64(), Some(2.8));
    /// ```
    URational(u32, u32),

    /// Raw binary data.
    ///
    /// Used for: MakerNote, ICC profiles, thumbnails, undefined EXIF fields.
    Bytes(Vec<u8>),

    // === Collection types ===
    /// Ordered list of values.
    ///
    /// Used for: SubjectArea, SubjectLocation, CFAPattern.
    List(Vec<AttrValue>),

    /// Key-value map.
    ///
    /// Used for: XMP structured data.
    Map(HashMap<String, AttrValue>),

    /// Nested attribute group.
    ///
    /// Used for: MakerNotes sub-IFDs (Canon:AFInfo, Nikon:ShotInfo),
    /// hierarchical XMP data.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_io::attrs::{Attrs, AttrValue};
    ///
    /// let mut canon = Attrs::new();
    /// canon.set("ModelID", AttrValue::UInt(0x80000001));
    ///
    /// let group = AttrValue::Group(Box::new(canon));
    /// ```
    Group(Box<super::Attrs>),
}

impl AttrValue {
    /// Returns the type name for error messages and debugging.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_io::attrs::AttrValue;
    ///
    /// let val = AttrValue::URational(1, 125);
    /// assert_eq!(val.type_name(), "urational");
    /// ```
    pub fn type_name(&self) -> &'static str {
        match self {
            AttrValue::Bool(_) => "bool",
            AttrValue::Str(_) => "string",
            AttrValue::Int(_) => "int32",
            AttrValue::UInt(_) => "uint32",
            AttrValue::Int64(_) => "int64",
            AttrValue::UInt64(_) => "uint64",
            AttrValue::Float(_) => "float",
            AttrValue::Double(_) => "double",
            AttrValue::Rational(_, _) => "rational",
            AttrValue::URational(_, _) => "urational",
            AttrValue::Bytes(_) => "bytes",
            AttrValue::List(_) => "list",
            AttrValue::Map(_) => "map",
            AttrValue::Group(_) => "group",
        }
    }

    // === Type-specific accessors ===

    /// Tries to get as string reference.
    ///
    /// Returns `None` if not a `Str` variant.
    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            AttrValue::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Tries to get as i32.
    ///
    /// Returns `None` if not an `Int` variant.
    #[inline]
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            AttrValue::Int(v) => Some(*v),
            _ => None,
        }
    }

    /// Tries to get as u32.
    ///
    /// Returns `None` if not a `UInt` variant.
    #[inline]
    pub fn as_u32(&self) -> Option<u32> {
        match self {
            AttrValue::UInt(v) => Some(*v),
            _ => None,
        }
    }

    /// Tries to get as f32.
    ///
    /// Returns `None` if not a `Float` variant.
    #[inline]
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            AttrValue::Float(v) => Some(*v),
            _ => None,
        }
    }

    /// Tries to get as f64, with automatic conversion from numerics.
    ///
    /// Converts from: `Float`, `Double`, `Int`, `UInt`, `Rational`, `URational`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_io::attrs::AttrValue;
    ///
    /// // Rational 1/125 -> 0.008
    /// let exposure = AttrValue::URational(1, 125);
    /// assert!((exposure.as_f64().unwrap() - 0.008).abs() < 0.0001);
    ///
    /// // Float -> Double (approximate due to f32->f64 precision)
    /// let gamma = AttrValue::Float(2.2);
    /// assert!((gamma.as_f64().unwrap() - 2.2).abs() < 0.0001);
    /// ```
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            AttrValue::Float(v) => Some(*v as f64),
            AttrValue::Double(v) => Some(*v),
            AttrValue::Int(v) => Some(*v as f64),
            AttrValue::UInt(v) => Some(*v as f64),
            AttrValue::Int64(v) => Some(*v as f64),
            AttrValue::UInt64(v) => Some(*v as f64),
            AttrValue::Rational(n, d) if *d != 0 => Some(*n as f64 / *d as f64),
            AttrValue::URational(n, d) if *d != 0 => Some(*n as f64 / *d as f64),
            _ => None,
        }
    }

    /// Tries to get as bool.
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            AttrValue::Bool(v) => Some(*v),
            _ => None,
        }
    }

    /// Tries to get as byte slice.
    #[inline]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            AttrValue::Bytes(v) => Some(v),
            _ => None,
        }
    }

    /// Tries to get as signed rational (numerator, denominator).
    #[inline]
    pub fn as_rational(&self) -> Option<(i32, i32)> {
        match self {
            AttrValue::Rational(n, d) => Some((*n, *d)),
            _ => None,
        }
    }

    /// Tries to get as unsigned rational (numerator, denominator).
    #[inline]
    pub fn as_urational(&self) -> Option<(u32, u32)> {
        match self {
            AttrValue::URational(n, d) => Some((*n, *d)),
            _ => None,
        }
    }

    /// Tries to get as list reference.
    #[inline]
    pub fn as_list(&self) -> Option<&Vec<AttrValue>> {
        match self {
            AttrValue::List(v) => Some(v),
            _ => None,
        }
    }

    /// Tries to get as map reference.
    #[inline]
    pub fn as_map(&self) -> Option<&HashMap<String, AttrValue>> {
        match self {
            AttrValue::Map(v) => Some(v),
            _ => None,
        }
    }

    /// Tries to get as nested group reference.
    #[inline]
    pub fn as_group(&self) -> Option<&super::Attrs> {
        match self {
            AttrValue::Group(v) => Some(v.as_ref()),
            _ => None,
        }
    }
}

// === Hash implementation for AttrValue ===

impl Hash for AttrValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            AttrValue::Bool(v) => v.hash(state),
            AttrValue::Str(v) => v.hash(state),
            AttrValue::Int(v) => v.hash(state),
            AttrValue::UInt(v) => v.hash(state),
            AttrValue::Int64(v) => v.hash(state),
            AttrValue::UInt64(v) => v.hash(state),
            AttrValue::Float(v) => v.to_bits().hash(state),
            AttrValue::Double(v) => v.to_bits().hash(state),
            AttrValue::Rational(n, d) => {
                n.hash(state);
                d.hash(state);
            }
            AttrValue::URational(n, d) => {
                n.hash(state);
                d.hash(state);
            }
            AttrValue::Bytes(v) => v.hash(state),
            AttrValue::List(v) => v.hash(state),
            AttrValue::Map(v) => {
                // XOR all key-value hashes for order-independence
                let mut acc: u64 = 0;
                for (k, val) in v {
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    k.hash(&mut h);
                    val.hash(&mut h);
                    acc ^= h.finish();
                }
                acc.hash(state);
            }
            AttrValue::Group(v) => v.hash_all().hash(state),
        }
    }
}

// === PartialEq implementation ===

impl PartialEq for AttrValue {
    fn eq(&self, other: &Self) -> bool {
        use AttrValue::*;
        match (self, other) {
            (Bool(a), Bool(b)) => a == b,
            (Str(a), Str(b)) => a == b,
            (Int(a), Int(b)) => a == b,
            (UInt(a), UInt(b)) => a == b,
            (Int64(a), Int64(b)) => a == b,
            (UInt64(a), UInt64(b)) => a == b,
            (Float(a), Float(b)) => a.to_bits() == b.to_bits(),
            (Double(a), Double(b)) => a.to_bits() == b.to_bits(),
            (Rational(n1, d1), Rational(n2, d2)) => n1 == n2 && d1 == d2,
            (URational(n1, d1), URational(n2, d2)) => n1 == n2 && d1 == d2,
            (Bytes(a), Bytes(b)) => a == b,
            (List(a), List(b)) => a == b,
            (Map(a), Map(b)) => {
                a.len() == b.len() && a.iter().all(|(k, v)| b.get(k).is_some_and(|ov| ov == v))
            }
            (Group(a), Group(b)) => a.hash_all() == b.hash_all(),
            _ => false,
        }
    }
}

impl Eq for AttrValue {}

// === Display implementation ===

impl std::fmt::Display for AttrValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttrValue::Bool(v) => write!(f, "{}", v),
            AttrValue::Str(v) => write!(f, "{}", v),
            AttrValue::Int(v) => write!(f, "{}", v),
            AttrValue::UInt(v) => write!(f, "{}", v),
            AttrValue::Int64(v) => write!(f, "{}", v),
            AttrValue::UInt64(v) => write!(f, "{}", v),
            AttrValue::Float(v) => write!(f, "{}", v),
            AttrValue::Double(v) => write!(f, "{}", v),
            AttrValue::Rational(n, d) => write!(f, "{}/{}", n, d),
            AttrValue::URational(n, d) => write!(f, "{}/{}", n, d),
            AttrValue::Bytes(v) => write!(f, "<{} bytes>", v.len()),
            AttrValue::List(v) => write!(f, "[{} items]", v.len()),
            AttrValue::Map(v) => write!(f, "{{{} entries}}", v.len()),
            AttrValue::Group(v) => write!(f, "<group: {} attrs>", v.len()),
        }
    }
}

// === From implementations for convenience ===

impl From<bool> for AttrValue {
    fn from(v: bool) -> Self {
        AttrValue::Bool(v)
    }
}

impl From<i32> for AttrValue {
    fn from(v: i32) -> Self {
        AttrValue::Int(v)
    }
}

impl From<u32> for AttrValue {
    fn from(v: u32) -> Self {
        AttrValue::UInt(v)
    }
}

impl From<i64> for AttrValue {
    fn from(v: i64) -> Self {
        AttrValue::Int64(v)
    }
}

impl From<u64> for AttrValue {
    fn from(v: u64) -> Self {
        AttrValue::UInt64(v)
    }
}

impl From<f32> for AttrValue {
    fn from(v: f32) -> Self {
        AttrValue::Float(v)
    }
}

impl From<f64> for AttrValue {
    fn from(v: f64) -> Self {
        AttrValue::Double(v)
    }
}

impl From<String> for AttrValue {
    fn from(v: String) -> Self {
        AttrValue::Str(v)
    }
}

impl From<&str> for AttrValue {
    fn from(v: &str) -> Self {
        AttrValue::Str(v.to_string())
    }
}

impl From<Vec<u8>> for AttrValue {
    fn from(v: Vec<u8>) -> Self {
        AttrValue::Bytes(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rational_to_f64() {
        let exposure = AttrValue::URational(1, 125);
        let f = exposure.as_f64().unwrap();
        assert!((f - 0.008).abs() < 0.0001);
    }

    #[test]
    fn test_display() {
        assert_eq!(AttrValue::URational(1, 125).to_string(), "1/125");
        assert_eq!(AttrValue::Bytes(vec![0; 100]).to_string(), "<100 bytes>");
        assert_eq!(AttrValue::UInt(400).to_string(), "400");
    }

    #[test]
    fn test_from_conversions() {
        let v: AttrValue = 42u32.into();
        assert_eq!(v.as_u32(), Some(42));

        let v: AttrValue = "hello".into();
        assert_eq!(v.as_str(), Some("hello"));
    }
}
