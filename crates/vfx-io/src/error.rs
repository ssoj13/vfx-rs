//! Error types for I/O operations.
//!
//! Provides unified error handling for all image format operations.

use std::io;
use thiserror::Error;

/// I/O operation error.
#[derive(Debug, Error)]
pub enum IoError {
    /// File I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Unsupported format.
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    /// Invalid or corrupted file.
    #[error("invalid file: {0}")]
    InvalidFile(String),

    /// Decoding error.
    #[error("decode error: {0}")]
    DecodeError(String),

    /// Encoding error.
    #[error("encode error: {0}")]
    EncodeError(String),

    /// Dimension mismatch.
    #[error("dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch {
        /// Expected size.
        expected: String,
        /// Actual size.
        actual: String,
    },

    /// Unsupported bit depth.
    #[error("unsupported bit depth: {0}")]
    UnsupportedBitDepth(String),

    /// Unsupported operation.
    #[error("unsupported operation: {0}")]
    UnsupportedOperation(String),

    /// Missing required data.
    #[error("missing data: {0}")]
    MissingData(String),

    /// Parse error (filename patterns, metadata, etc.).
    #[error("parse error: {0}")]
    Parse(String),

    /// Format-specific error.
    #[error("format error: {0}")]
    Format(String),

    /// Feature requires external SDK or dependency.
    #[error("feature unavailable: {0}")]
    UnsupportedFeature(String),
}

/// Result type for I/O operations.
pub type IoResult<T> = Result<T, IoError>;
