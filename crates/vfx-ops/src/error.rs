//! Error types for image operations.

use thiserror::Error;

/// Error type for image operations.
#[derive(Error, Debug)]
pub enum OpsError {
    /// Invalid dimensions specified.
    #[error("invalid dimensions: {0}")]
    InvalidDimensions(String),

    /// Images have incompatible sizes.
    #[error("size mismatch: {0}")]
    SizeMismatch(String),

    /// Invalid parameter value.
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),

    /// Operation not supported for this format.
    #[error("unsupported operation: {0}")]
    Unsupported(String),
}

/// Result type for image operations.
pub type OpsResult<T> = Result<T, OpsError>;
