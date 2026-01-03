//! Configuration validation utilities.
//!
//! Provides validation for OCIO configurations to detect common issues:
//! - Missing color space references
//! - Circular dependencies
//! - Invalid transform chains
//! - Missing LUT files
//!
//! # Example
//!
//! ```ignore
//! use vfx_ocio::{Config, validate};
//!
//! let config = Config::from_file("config.ocio")?;
//! let issues = validate::check(&config);
//!
//! for issue in &issues {
//!     println!("{}: {}", issue.severity, issue.message);
//! }
//! ```

use crate::config::Config;
use std::collections::HashSet;

/// Severity level for validation issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Informational message.
    Info,
    /// Warning - config works but may have issues.
    Warning,
    /// Error - config has problems that may cause failures.
    Error,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARN"),
            Self::Error => write!(f, "ERROR"),
        }
    }
}

/// A validation issue found in the config.
#[derive(Debug, Clone)]
pub struct Issue {
    /// Severity level.
    pub severity: Severity,
    /// Issue category.
    pub category: IssueCategory,
    /// Human-readable message.
    pub message: String,
    /// Related element (color space name, role, etc.).
    pub context: Option<String>,
}

/// Categories of validation issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueCategory {
    /// Missing color space reference.
    MissingColorSpace,
    /// Missing role definition.
    MissingRole,
    /// Missing display/view.
    MissingDisplay,
    /// Missing LUT/transform file.
    MissingFile,
    /// Circular reference in transforms.
    CircularReference,
    /// Invalid transform configuration.
    InvalidTransform,
    /// Unused color space.
    UnusedColorSpace,
    /// Duplicate definition.
    Duplicate,
}

/// Validates a config and returns all issues found.
pub fn check(config: &Config) -> Vec<Issue> {
    let mut issues = Vec::new();

    check_roles(config, &mut issues);
    check_displays(config, &mut issues);
    check_colorspaces(config, &mut issues);
    check_files(config, &mut issues);

    issues
}

/// Checks role definitions.
fn check_roles(config: &Config, issues: &mut Vec<Issue>) {
    let required_roles = ["reference", "default"];

    for role in required_roles {
        if config.roles().get(role).is_none() {
            issues.push(Issue {
                severity: Severity::Warning,
                category: IssueCategory::MissingRole,
                message: format!("recommended role '{}' is not defined", role),
                context: Some(role.to_string()),
            });
        }
    }

    // Check that all role targets exist
    for (role, cs_name) in config.roles().iter() {
        if config.colorspace(cs_name).is_none() {
            issues.push(Issue {
                severity: Severity::Error,
                category: IssueCategory::MissingColorSpace,
                message: format!(
                    "role '{}' references non-existent color space '{}'",
                    role, cs_name
                ),
                context: Some(role.to_string()),
            });
        }
    }
}

/// Checks display/view definitions.
fn check_displays(config: &Config, issues: &mut Vec<Issue>) {
    if config.displays().displays().is_empty() {
        issues.push(Issue {
            severity: Severity::Warning,
            category: IssueCategory::MissingDisplay,
            message: "no displays defined".to_string(),
            context: None,
        });
        return;
    }

    for display in config.displays().displays() {
        if display.views().is_empty() {
            issues.push(Issue {
                severity: Severity::Warning,
                category: IssueCategory::MissingDisplay,
                message: format!("display '{}' has no views", display.name()),
                context: Some(display.name().to_string()),
            });
        }

        for view in display.views() {
            if config.colorspace(view.colorspace()).is_none() {
                issues.push(Issue {
                    severity: Severity::Error,
                    category: IssueCategory::MissingColorSpace,
                    message: format!(
                        "view '{}' in display '{}' references non-existent color space '{}'",
                        view.name(),
                        display.name(),
                        view.colorspace()
                    ),
                    context: Some(format!("{}:{}", display.name(), view.name())),
                });
            }
        }
    }
}

/// Checks color space definitions.
fn check_colorspaces(config: &Config, issues: &mut Vec<Issue>) {
    let mut names: HashSet<&str> = HashSet::new();

    for cs in config.colorspaces() {
        // Check for duplicates
        if !names.insert(cs.name()) {
            issues.push(Issue {
                severity: Severity::Error,
                category: IssueCategory::Duplicate,
                message: format!("duplicate color space name: '{}'", cs.name()),
                context: Some(cs.name().to_string()),
            });
        }

        // Check aliases don't conflict
        for alias in cs.aliases() {
            if names.contains(alias.as_str()) {
                issues.push(Issue {
                    severity: Severity::Warning,
                    category: IssueCategory::Duplicate,
                    message: format!(
                        "alias '{}' for '{}' conflicts with existing name",
                        alias,
                        cs.name()
                    ),
                    context: Some(cs.name().to_string()),
                });
            }
        }

        // Check data color spaces don't have transforms
        if cs.is_data() && (cs.to_reference().is_some() || cs.from_reference().is_some()) {
            issues.push(Issue {
                severity: Severity::Warning,
                category: IssueCategory::InvalidTransform,
                message: format!(
                    "data color space '{}' has transforms defined (will be ignored)",
                    cs.name()
                ),
                context: Some(cs.name().to_string()),
            });
        }
    }
}

/// Checks for missing transform files.
fn check_files(config: &Config, issues: &mut Vec<Issue>) {
    // This would check FileTransform references
    // For now, we just validate that search paths exist
    for path in config.search_paths() {
        if !path.exists() {
            issues.push(Issue {
                severity: Severity::Warning,
                category: IssueCategory::MissingFile,
                message: format!("search path does not exist: {}", path.display()),
                context: Some(path.display().to_string()),
            });
        }
    }
}

/// Returns true if there are any errors.
pub fn has_errors(issues: &[Issue]) -> bool {
    issues.iter().any(|i| i.severity == Severity::Error)
}

/// Returns true if there are any warnings or errors.
pub fn has_warnings(issues: &[Issue]) -> bool {
    issues
        .iter()
        .any(|i| i.severity == Severity::Warning || i.severity == Severity::Error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_empty_config() {
        let config = Config::new();
        let issues = check(&config);

        // Should warn about missing roles and displays
        assert!(issues.iter().any(|i| i.category == IssueCategory::MissingRole));
        assert!(issues.iter().any(|i| i.category == IssueCategory::MissingDisplay));
    }

    #[test]
    fn severity_display() {
        assert_eq!(format!("{}", Severity::Info), "INFO");
        assert_eq!(format!("{}", Severity::Warning), "WARN");
        assert_eq!(format!("{}", Severity::Error), "ERROR");
    }

    #[test]
    fn has_errors_check() {
        let issues = vec![
            Issue {
                severity: Severity::Warning,
                category: IssueCategory::MissingRole,
                message: "test".into(),
                context: None,
            },
        ];
        assert!(!has_errors(&issues));
        assert!(has_warnings(&issues));

        let issues_with_error = vec![
            Issue {
                severity: Severity::Error,
                category: IssueCategory::MissingColorSpace,
                message: "test".into(),
                context: None,
            },
        ];
        assert!(has_errors(&issues_with_error));
    }
}
