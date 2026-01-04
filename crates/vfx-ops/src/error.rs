//! Error types for image operations.
//!
//! Provides unified error handling for all vfx-ops operations.

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

    /// Non-color channel detected where color processing is required.
    /// 
    /// Certain operations (blur, resize, color transforms) are only valid
    /// for color/alpha/depth channels. Id, mask, and generic channels
    /// require the `--allow-non-color` flag.
    #[error("non-color channel '{channel}' ({kind:?}) not allowed for operation '{op}'")]
    NonColorChannel {
        /// Channel name that triggered the error
        channel: String,
        /// Channel kind (Id, Mask, Generic, etc.)
        kind: String,
        /// Operation that was attempted
        op: String,
    },
}

/// Result type for image operations.
pub type OpsResult<T> = Result<T, OpsError>;
