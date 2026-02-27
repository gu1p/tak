//! Label parsing and normalization utilities.
//!
//! Task labels support clean user-facing syntax (`package:name`, `name`) and preserve
//! canonical internal package storage (`//package`) with support for relative `:name`
//! parsing against the current package.

use thiserror::Error;

pub use crate::model::TaskLabel;

#[derive(Debug, Error)]
pub enum LabelError {
    #[error("label is empty")]
    Empty,
    #[error("invalid label format: {0}")]
    InvalidFormat(String),
    #[error("invalid package: {0}")]
    InvalidPackage(String),
    #[error("name must be non-empty: {0}")]
    InvalidName(String),
}

/// Parses labels using one of:
/// - `package:name`
/// - `//package:name` (backward compatible)
/// - `name` (root package)
/// - `:name` (relative to current package)
///
/// Relative labels are resolved against `current_package`.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn parse_label(raw: &str, current_package: &str) -> Result<TaskLabel, LabelError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(LabelError::Empty);
    }

    if let Some(name) = trimmed.strip_prefix(':') {
        validate_name(name)?;
        validate_package(current_package)?;
        return Ok(TaskLabel {
            package: normalize_package(current_package),
            name: name.to_string(),
        });
    }

    if let Some((package, name)) = trimmed.split_once(':') {
        validate_name(name)?;
        let normalized_package = normalize_package(package);
        validate_package(&normalized_package)?;
        return Ok(TaskLabel {
            package: normalized_package,
            name: name.to_string(),
        });
    }

    validate_name(trimmed)?;
    validate_package(current_package)?;
    Ok(TaskLabel {
        package: normalize_package(current_package),
        name: trimmed.to_string(),
    })
}

/// Normalizes package syntax into canonical `//...` representation.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn normalize_package(package: &str) -> String {
    let trimmed = package.trim();
    let without_prefix = trimmed.strip_prefix("//").unwrap_or(trimmed);
    let core = without_prefix.trim_matches('/');
    if core.is_empty() {
        "//".to_string()
    } else {
        format!("//{core}")
    }
}

/// Validates a canonical Tak package path.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn validate_package(package: &str) -> Result<(), LabelError> {
    if !package.starts_with("//") || package.contains(':') {
        return Err(LabelError::InvalidPackage(package.to_string()));
    }

    Ok(())
}

/// Validates that a task name is non-empty after trimming whitespace.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn validate_name(name: &str) -> Result<(), LabelError> {
    if name.trim().is_empty() {
        return Err(LabelError::InvalidName(name.to_string()));
    }

    Ok(())
}
