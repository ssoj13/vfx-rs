//! Unified pixel and bit depth formats.
//!
//! This module provides the canonical definitions for pixel data formats
//! used across all vfx-rs crates.
//!
//! # Types
//!
//! - [`BitDepth`] - Bit depth specification (includes packed formats like 10/12-bit)
//! - [`DataFormat`] - Runtime pixel data type (U8, U16, U32, F16, F32)
//!
//! # Usage
//!
//! ```rust
//! use vfx_core::format::{BitDepth, DataFormat};
//!
//! // DPX uses 10-bit packed
//! let dpx_depth = BitDepth::U10;
//! let storage = dpx_depth.storage_format(); // U16 (smallest that fits)
//!
//! // EXR uses half-float
//! let exr_format = DataFormat::F16;
//! let bit_depth = BitDepth::from(exr_format); // F16
//! ```

/// Bit depth specification for image data.
///
/// Represents the precision of pixel values, including packed formats
/// commonly used in film/broadcast (10-bit, 12-bit DPX).
///
/// # Variants
///
/// Integer formats:
/// - `U8` - 8-bit unsigned [0, 255]
/// - `U10` - 10-bit unsigned [0, 1023] (DPX, broadcast)
/// - `U12` - 12-bit unsigned [0, 4095] (cinema cameras)
/// - `U16` - 16-bit unsigned [0, 65535]
/// - `U32` - 32-bit unsigned [0, 4294967295]
///
/// Floating-point formats:
/// - `F16` - 16-bit half-precision IEEE 754
/// - `F32` - 32-bit single-precision IEEE 754
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BitDepth {
    /// Unknown/auto-detect.
    Unknown,
    /// 8-bit unsigned integer.
    U8,
    /// 10-bit unsigned integer (DPX, broadcast).
    U10,
    /// 12-bit unsigned integer (cinema cameras).
    U12,
    /// 16-bit unsigned integer.
    U16,
    /// 32-bit unsigned integer.
    U32,
    /// 16-bit half-precision float.
    F16,
    /// 32-bit single-precision float (VFX standard).
    #[default]
    F32,
}

impl BitDepth {
    /// Number of bits per channel.
    /// Returns 0 for Unknown.
    #[inline]
    pub const fn bits(&self) -> u32 {
        match self {
            Self::Unknown => 0,
            Self::U8 => 8,
            Self::U10 => 10,
            Self::U12 => 12,
            Self::U16 => 16,
            Self::U32 => 32,
            Self::F16 => 16,
            Self::F32 => 32,
        }
    }

    /// Whether this is a floating-point format.
    /// Returns false for Unknown.
    #[inline]
    pub const fn is_float(&self) -> bool {
        matches!(self, Self::F16 | Self::F32)
    }

    /// Whether this is unknown/auto-detect.
    #[inline]
    pub const fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }

    /// Whether this is an integer format.
    #[inline]
    pub const fn is_integer(&self) -> bool {
        !self.is_float()
    }

    /// Whether this is a packed format (not byte-aligned).
    #[inline]
    pub const fn is_packed(&self) -> bool {
        matches!(self, Self::U10 | Self::U12)
    }

    /// Maximum representable integer value.
    /// Returns 0 for Unknown.
    #[inline]
    pub const fn max_value(&self) -> u32 {
        match self {
            Self::Unknown => 0,
            Self::U8 => 255,
            Self::U10 => 1023,
            Self::U12 => 4095,
            Self::U16 => 65535,
            Self::U32 => u32::MAX,
            Self::F16 | Self::F32 => u32::MAX, // Not really applicable
        }
    }

    /// Returns the smallest [`DataFormat`] that can store this bit depth.
    ///
    /// Packed formats (10-bit, 12-bit) require U16 storage.
    /// Unknown defaults to F32.
    #[inline]
    pub const fn storage_format(&self) -> DataFormat {
        match self {
            Self::Unknown => DataFormat::F32,
            Self::U8 => DataFormat::U8,
            Self::U10 | Self::U12 | Self::U16 => DataFormat::U16,
            Self::U32 => DataFormat::U32,
            Self::F16 => DataFormat::F16,
            Self::F32 => DataFormat::F32,
        }
    }

    /// Bytes needed per channel in storage format.
    #[inline]
    pub const fn bytes_per_channel(&self) -> usize {
        self.storage_format().bytes_per_channel()
    }

    /// Normalization factor for converting to [0, 1] float range.
    /// Returns 1.0 for Unknown.
    #[inline]
    pub fn normalize_factor(&self) -> f32 {
        match self {
            Self::Unknown => 1.0,
            Self::U8 => 255.0,
            Self::U10 => 1023.0,
            Self::U12 => 4095.0,
            Self::U16 => 65535.0,
            Self::U32 => 4294967295.0,
            Self::F16 | Self::F32 => 1.0,
        }
    }

    /// Alias for `normalize_factor()` (CLF compatibility).
    #[inline]
    pub fn scale(&self) -> f32 {
        self.normalize_factor()
    }

    /// Parse from CLF bit depth string ("8i", "10i", "12i", "16i", "16f", "32f").
    ///
    /// # Example
    /// ```rust
    /// use vfx_core::BitDepth;
    /// assert_eq!(BitDepth::from_clf_str("10i"), Some(BitDepth::U10));
    /// assert_eq!(BitDepth::from_clf_str("32f"), Some(BitDepth::F32));
    /// ```
    pub fn from_clf_str(s: &str) -> Option<Self> {
        match s {
            "8i" => Some(Self::U8),
            "10i" => Some(Self::U10),
            "12i" => Some(Self::U12),
            "16i" => Some(Self::U16),
            "16f" => Some(Self::F16),
            "32f" => Some(Self::F32),
            _ => None,
        }
    }

    /// Returns CLF bit depth string.
    ///
    /// # Example
    /// ```rust
    /// use vfx_core::BitDepth;
    /// assert_eq!(BitDepth::U10.clf_str(), "10i");
    /// assert_eq!(BitDepth::F32.clf_str(), "32f");
    /// ```
    pub fn clf_str(&self) -> &'static str {
        match self {
            Self::Unknown => "32f", // Default to float
            Self::U8 => "8i",
            Self::U10 => "10i",
            Self::U12 => "12i",
            Self::U16 => "16i",
            Self::U32 => "16i", // CLF doesn't have 32i, use 16i
            Self::F16 => "16f",
            Self::F32 => "32f",
        }
    }
}

impl std::fmt::Display for BitDepth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "unknown"),
            Self::U8 => write!(f, "8-bit"),
            Self::U10 => write!(f, "10-bit"),
            Self::U12 => write!(f, "12-bit"),
            Self::U16 => write!(f, "16-bit"),
            Self::U32 => write!(f, "32-bit"),
            Self::F16 => write!(f, "half"),
            Self::F32 => write!(f, "float"),
        }
    }
}

/// Runtime pixel data format.
///
/// Represents the actual storage type of pixel data in memory.
/// Unlike [`BitDepth`], this only includes byte-aligned types.
///
/// # Relationship with BitDepth
///
/// - `BitDepth::U10` and `BitDepth::U12` both use `DataFormat::U16` for storage
/// - Use [`BitDepth::storage_format()`] to get the appropriate storage format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DataFormat {
    /// 8-bit unsigned integer.
    U8,
    /// 16-bit unsigned integer.
    #[default]
    U16,
    /// 32-bit unsigned integer.
    U32,
    /// 16-bit half-precision float.
    F16,
    /// 32-bit single-precision float.
    F32,
}

impl DataFormat {
    /// Number of bytes per channel.
    #[inline]
    pub const fn bytes_per_channel(&self) -> usize {
        match self {
            Self::U8 => 1,
            Self::U16 => 2,
            Self::U32 => 4,
            Self::F16 => 2,
            Self::F32 => 4,
        }
    }

    /// Number of bits per channel.
    #[inline]
    pub const fn bits(&self) -> u32 {
        match self {
            Self::U8 => 8,
            Self::U16 => 16,
            Self::U32 => 32,
            Self::F16 => 16,
            Self::F32 => 32,
        }
    }

    /// Whether this is a floating-point format.
    #[inline]
    pub const fn is_float(&self) -> bool {
        matches!(self, Self::F16 | Self::F32)
    }

    /// Whether this is an integer format.
    #[inline]
    pub const fn is_integer(&self) -> bool {
        !self.is_float()
    }

    /// Short name for display.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::F16 => "f16",
            Self::F32 => "f32",
        }
    }
}

impl std::fmt::Display for DataFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

// Conversions between BitDepth and DataFormat

impl From<DataFormat> for BitDepth {
    fn from(fmt: DataFormat) -> Self {
        match fmt {
            DataFormat::U8 => BitDepth::U8,
            DataFormat::U16 => BitDepth::U16,
            DataFormat::U32 => BitDepth::U32,
            DataFormat::F16 => BitDepth::F16,
            DataFormat::F32 => BitDepth::F32,
        }
    }
}

impl From<BitDepth> for DataFormat {
    /// Converts to storage format. Packed formats become U16.
    fn from(depth: BitDepth) -> Self {
        depth.storage_format()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_depth_bits() {
        assert_eq!(BitDepth::U8.bits(), 8);
        assert_eq!(BitDepth::U10.bits(), 10);
        assert_eq!(BitDepth::U12.bits(), 12);
        assert_eq!(BitDepth::U16.bits(), 16);
        assert_eq!(BitDepth::F16.bits(), 16);
        assert_eq!(BitDepth::F32.bits(), 32);
    }

    #[test]
    fn test_storage_format() {
        assert_eq!(BitDepth::U8.storage_format(), DataFormat::U8);
        assert_eq!(BitDepth::U10.storage_format(), DataFormat::U16);
        assert_eq!(BitDepth::U12.storage_format(), DataFormat::U16);
        assert_eq!(BitDepth::U16.storage_format(), DataFormat::U16);
        assert_eq!(BitDepth::F16.storage_format(), DataFormat::F16);
        assert_eq!(BitDepth::F32.storage_format(), DataFormat::F32);
    }

    #[test]
    fn test_is_float() {
        assert!(!BitDepth::U8.is_float());
        assert!(!BitDepth::U10.is_float());
        assert!(BitDepth::F16.is_float());
        assert!(BitDepth::F32.is_float());
    }

    #[test]
    fn test_normalize_factor() {
        assert_eq!(BitDepth::U8.normalize_factor(), 255.0);
        assert_eq!(BitDepth::U10.normalize_factor(), 1023.0);
        assert_eq!(BitDepth::U12.normalize_factor(), 4095.0);
        assert_eq!(BitDepth::F32.normalize_factor(), 1.0);
    }

    #[test]
    fn test_conversions() {
        // DataFormat -> BitDepth
        assert_eq!(BitDepth::from(DataFormat::U8), BitDepth::U8);
        assert_eq!(BitDepth::from(DataFormat::F16), BitDepth::F16);

        // BitDepth -> DataFormat (storage)
        assert_eq!(DataFormat::from(BitDepth::U10), DataFormat::U16);
        assert_eq!(DataFormat::from(BitDepth::F32), DataFormat::F32);
    }
}
