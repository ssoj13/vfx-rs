//! Context variables for dynamic path resolution.
//!
//! OCIO configs can use context variables like `$SHOT`, `$SEQ` in file paths.
//! This module handles variable substitution from environment or explicit values.
//!
//! # Example
//!
//! ```
//! use vfx_ocio::Context;
//!
//! let mut ctx = Context::new();
//! ctx.set("SHOT", "sh010");
//! ctx.set("SEQ", "sq01");
//!
//! let resolved = ctx.resolve("/shows/$SEQ/shots/$SHOT/luts/grade.csp");
//! assert_eq!(resolved, "/shows/sq01/shots/sh010/luts/grade.csp");
//! ```

use std::collections::HashMap;
use std::env;

/// Context for variable substitution in file paths.
#[derive(Debug, Clone, Default)]
pub struct Context {
    /// User-defined variables (take precedence over environment).
    vars: HashMap<String, String>,
    /// Whether to check environment variables as fallback.
    use_env: bool,
}

impl Context {
    /// Creates a new empty context.
    ///
    /// By default, environment variables are used as fallback.
    #[inline]
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            use_env: true,
        }
    }

    /// Creates a context that ignores environment variables.
    #[inline]
    pub fn without_env() -> Self {
        Self {
            vars: HashMap::new(),
            use_env: false,
        }
    }

    /// Sets a context variable.
    ///
    /// This takes precedence over environment variables with the same name.
    #[inline]
    pub fn set(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.vars.insert(name.into(), value.into());
    }

    /// Gets a context variable value.
    ///
    /// Checks user-defined variables first, then environment if enabled.
    pub fn get(&self, name: &str) -> Option<String> {
        // Check user-defined first
        if let Some(v) = self.vars.get(name) {
            return Some(v.clone());
        }
        // Fallback to environment
        if self.use_env {
            env::var(name).ok()
        } else {
            None
        }
    }

    /// Checks if a variable is defined.
    #[inline]
    pub fn contains(&self, name: &str) -> bool {
        self.vars.contains_key(name) || (self.use_env && env::var(name).is_ok())
    }

    /// Resolves all `$VAR` and `${VAR}` references in a string.
    ///
    /// Unknown variables are left as-is (not substituted).
    pub fn resolve(&self, input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' {
                // Check for ${VAR} or $VAR syntax
                if chars.peek() == Some(&'{') {
                    chars.next(); // consume '{'
                    let var_name: String = chars.by_ref().take_while(|&c| c != '}').collect();
                    if let Some(value) = self.get(&var_name) {
                        result.push_str(&value);
                    } else {
                        // Leave unresolved as-is
                        result.push_str("${");
                        result.push_str(&var_name);
                        result.push('}');
                    }
                } else {
                    // $VAR syntax - read while alphanumeric or underscore
                    let mut var_name = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch.is_alphanumeric() || ch == '_' {
                            var_name.push(ch);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if var_name.is_empty() {
                        result.push('$');
                    } else if let Some(value) = self.get(&var_name) {
                        result.push_str(&value);
                    } else {
                        result.push('$');
                        result.push_str(&var_name);
                    }
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Returns all user-defined variables.
    #[inline]
    pub fn vars(&self) -> &HashMap<String, String> {
        &self.vars
    }

    /// Clears all user-defined variables.
    #[inline]
    pub fn clear(&mut self) {
        self.vars.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_simple_var() {
        let mut ctx = Context::without_env();
        ctx.set("SHOT", "sh010");

        assert_eq!(ctx.resolve("/path/$SHOT/file"), "/path/sh010/file");
    }

    #[test]
    fn resolve_braced_var() {
        let mut ctx = Context::without_env();
        ctx.set("SEQ", "sq01");

        assert_eq!(ctx.resolve("/path/${SEQ}_data"), "/path/sq01_data");
    }

    #[test]
    fn resolve_multiple_vars() {
        let mut ctx = Context::without_env();
        ctx.set("A", "alpha");
        ctx.set("B", "beta");

        assert_eq!(ctx.resolve("$A-${B}-$A"), "alpha-beta-alpha");
    }

    #[test]
    fn unresolved_left_as_is() {
        let ctx = Context::without_env();
        assert_eq!(ctx.resolve("$UNKNOWN"), "$UNKNOWN");
        assert_eq!(ctx.resolve("${UNKNOWN}"), "${UNKNOWN}");
    }

    #[test]
    fn dollar_at_end() {
        let ctx = Context::without_env();
        assert_eq!(ctx.resolve("test$"), "test$");
    }
}
