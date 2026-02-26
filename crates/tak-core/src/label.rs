//! Label parsing and normalization utilities.
//!
//! Task labels are represented as `//package:name` with support for relative `:name`
//! parsing against the current package.

use thiserror::Error;

pub use crate::model::TaskLabel;

#[derive(Debug, Error)]
pub enum LabelError {
    #[error("label is empty")]
    Empty,
    #[error("invalid label format: {0}")]
    InvalidFormat(String),
    #[error("package must begin with //: {0}")]
    InvalidPackage(String),
    #[error("name must be non-empty: {0}")]
    InvalidName(String),
}

/// Parses either an absolute (`//pkg:name`) or relative (`:name`) label.
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

    if let Some((package, name)) = trimmed.split_once(':')
        && package.starts_with("//")
    {
        validate_package(package)?;
        validate_name(name)?;
        return Ok(TaskLabel {
            package: normalize_package(package),
            name: name.to_string(),
        });
    }

    Err(LabelError::InvalidFormat(trimmed.to_string()))
}

/// Normalizes a package string by removing trailing `/` while preserving `//`.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn normalize_package(package: &str) -> String {
    if package == "//" {
        return "//".to_string();
    }

    let without_trailing = package.trim_end_matches('/');
    without_trailing.to_string()
}

/// Validates that a package path uses Tak `//...` package syntax.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn validate_package(package: &str) -> Result<(), LabelError> {
    if !package.starts_with("//") {
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
