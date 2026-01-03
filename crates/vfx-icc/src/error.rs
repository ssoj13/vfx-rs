//! ICC error types.

use thiserror::Error;

/// Result type for ICC operations.
pub type IccResult<T> = Result<T, IccError>;

/// Errors that can occur during ICC operations.
#[derive(Debug, Error)]
pub enum IccError {
    /// Failed to load profile from file.
    #[error("failed to load profile: {0}")]
    LoadFailed(String),

    /// Failed to create profile.
    #[error("failed to create profile: {0}")]
    CreateFailed(String),

    /// Failed to create transform.
    #[error("failed to create transform: {0}")]
    TransformFailed(String),

    /// Invalid profile data.
    #[error("invalid profile data: {0}")]
    InvalidProfile(String),

    /// Profile color space mismatch.
    #[error("color space mismatch: expected {expected}, got {actual}")]
    ColorSpaceMismatch {
        /// Expected color space.
        expected: String,
        /// Actual color space.
        actual: String,
    },

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
