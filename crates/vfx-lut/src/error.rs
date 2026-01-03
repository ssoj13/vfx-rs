//! LUT error types.

use thiserror::Error;

/// Result type for LUT operations.
pub type LutResult<T> = Result<T, LutError>;

/// Errors that can occur during LUT operations.
#[derive(Debug, Error)]
pub enum LutError {
    /// Invalid LUT size.
    #[error("invalid LUT size: {0}")]
    InvalidSize(String),

    /// Invalid input range.
    #[error("invalid input range: [{min}, {max}]")]
    InvalidRange {
        /// Minimum value
        min: f32,
        /// Maximum value
        max: f32,
    },

    /// Parse error when loading LUT files.
    #[error("parse error: {0}")]
    ParseError(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
