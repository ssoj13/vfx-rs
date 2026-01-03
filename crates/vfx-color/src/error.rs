//! Error types for color operations.
//!
//! Provides unified error handling for all color-related operations,
//! including transfer function failures, matrix operations, and LUT errors.

use thiserror::Error;

/// Color operation error.
///
/// Covers all possible failure modes in the color pipeline:
/// - Invalid input values (NaN, Inf, out of range)
/// - Matrix singularity or numerical issues
/// - LUT errors (wrong size, invalid data)
/// - Unsupported conversions
#[derive(Debug, Error)]
pub enum ColorError {
    /// Input value is invalid (NaN, Inf, out of expected range).
    #[error("invalid input value: {0}")]
    InvalidValue(String),

    /// Matrix operation failed (singular, numerical instability).
    #[error("matrix error: {0}")]
    MatrixError(String),

    /// LUT operation failed.
    #[error("LUT error: {0}")]
    LutError(#[from] vfx_lut::LutError),

    /// Transfer function error.
    #[error("transfer function error: {0}")]
    TransferError(String),

    /// Color space conversion not supported.
    #[error("unsupported conversion: {from} -> {to}")]
    UnsupportedConversion {
        /// Source color space.
        from: String,
        /// Target color space.
        to: String,
    },

    /// Pipeline is empty or invalid.
    #[error("invalid pipeline: {0}")]
    InvalidPipeline(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Parse error (CDL, XML, etc.).
    #[error("parse error: {0}")]
    ParseError(String),
}

/// Result type for color operations.
pub type ColorResult<T> = Result<T, ColorError>;
