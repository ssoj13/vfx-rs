//! Error types for OCIO configuration parsing and processing.
//!
//! This module provides error handling for:
//! - Config file parsing (YAML/XML)
//! - Color space lookup and validation
//! - Transform chain building
//! - Display/View configuration

use std::path::PathBuf;
use thiserror::Error;

/// Result type for OCIO operations.
pub type OcioResult<T> = Result<T, OcioError>;

/// Errors that can occur during OCIO operations.
#[derive(Debug, Error)]
pub enum OcioError {
    /// I/O error reading config files.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// YAML parsing error.
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// Config file not found.
    #[error("config file not found: {path}")]
    ConfigNotFound {
        /// Path that was searched.
        path: PathBuf,
    },

    /// Invalid config version.
    #[error("unsupported config version: {version} (supported: 1.x, 2.x)")]
    UnsupportedVersion {
        /// Version string from config.
        version: String,
    },

    /// Color space not found in config.
    #[error("color space not found: {name}")]
    ColorSpaceNotFound {
        /// Name of the missing color space.
        name: String,
    },

    /// Role not defined in config.
    #[error("role not defined: {role}")]
    RoleNotDefined {
        /// Name of the undefined role.
        role: String,
    },

    /// Display not found in config.
    #[error("display not found: {name}")]
    DisplayNotFound {
        /// Name of the missing display.
        name: String,
    },

    /// View not found for display.
    #[error("view '{view}' not found for display '{display}'")]
    ViewNotFound {
        /// Display name.
        display: String,
        /// View name.
        view: String,
    },

    /// Look not found in config.
    #[error("look not found: {name}")]
    LookNotFound {
        /// Name of the missing look.
        name: String,
    },

    /// Invalid transform definition.
    #[error("invalid transform: {reason}")]
    InvalidTransform {
        /// Description of what's wrong.
        reason: String,
    },

    /// File reference in transform not found.
    #[error("transform file not found: {path}")]
    TransformFileNotFound {
        /// Path to the missing file.
        path: PathBuf,
    },

    /// Circular reference detected in transforms.
    #[error("circular reference detected: {chain}")]
    CircularReference {
        /// Description of the circular chain.
        chain: String,
    },

    /// Context variable not set.
    #[error("context variable not set: {name}")]
    ContextVariableNotSet {
        /// Name of the missing variable.
        name: String,
    },

    /// Invalid environment variable reference.
    #[error("invalid environment reference: {expr}")]
    InvalidEnvReference {
        /// The invalid expression.
        expr: String,
    },

    /// Transform processing error.
    #[error("transform error: {0}")]
    Transform(String),

    /// LUT loading error.
    #[error("LUT error: {0}")]
    Lut(#[from] vfx_lut::LutError),

    /// General validation error.
    #[error("validation error: {0}")]
    Validation(String),
}
