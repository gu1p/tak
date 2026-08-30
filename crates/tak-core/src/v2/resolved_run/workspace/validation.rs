use std::path::{Component, Path};

use super::ResolvedRunError;

pub(super) fn validate_digest(value: &str) -> Result<(), ResolvedRunError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Ok(());
    }
    Err(ResolvedRunError::new("expected lowercase SHA-256 digest"))
}

pub(super) fn validate_relative_path(value: &str) -> Result<(), ResolvedRunError> {
    if value.is_empty()
        || value.contains('\\')
        || value.contains('\0')
        || value
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(ResolvedRunError::new("invalid workspace relative path"));
    }
    let path = Path::new(value);
    if path
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        return Ok(());
    }
    Err(ResolvedRunError::new(
        "workspace path must be relative and non-escaping",
    ))
}

pub(super) fn validate_symlink_target(
    path: &str,
    target: Option<&str>,
) -> Result<(), ResolvedRunError> {
    let Some(target) = target else {
        return Err(ResolvedRunError::new("symlink target is required"));
    };
    if target.is_empty() || target.contains('\\') || Path::new(target).is_absolute() {
        return Err(ResolvedRunError::new(
            "symlink target must be safe and relative",
        ));
    }
    let mut depth = Path::new(path)
        .parent()
        .map_or(0, |parent| parent.components().count());
    for component in target.split('/') {
        match component {
            "" => {
                return Err(ResolvedRunError::new(
                    "symlink target contains an empty segment",
                ));
            }
            "." => {}
            ".." if depth > 0 => depth -= 1,
            ".." => {
                return Err(ResolvedRunError::new(
                    "symlink target escapes the workspace",
                ));
            }
            _ => depth += 1,
        }
    }
    Ok(())
}
