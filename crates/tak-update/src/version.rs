//! Release version parsing and ordering.
//!
//! Release tags are strict SemVer (`vX.Y.Z`, all-numeric) produced by
//! `scripts/compute_release_version.sh`, so a `(major, minor, patch)` tuple is a
//! faithful, dependency-free model. Field order matches the derived [`Ord`], so
//! `v0.1.7 > v0.1.0` and `v0.2.0 > v0.1.99` compare correctly without pulling in
//! a full SemVer dependency.

use std::fmt;

/// A parsed release version: `major.minor.patch`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Error returned when a string is not a strict `X.Y.Z` / `vX.Y.Z` version.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VersionParseError {
    /// The input was empty or whitespace-only.
    #[error("version `{0}` is empty")]
    Empty(String),
    /// The input did not have exactly three `.`-separated components.
    #[error("version `{0}` must have exactly three numeric components (X.Y.Z)")]
    Shape(String),
    /// A component was not a base-10 unsigned integer.
    #[error("version `{0}` has a non-numeric component")]
    NonNumeric(String),
}

/// Parse a `vX.Y.Z` or `X.Y.Z` version string.
///
/// A single leading `v` is tolerated: release tags carry it (`v0.1.7`) while
/// `--version` output does not (`0.1.7`), mirroring the installer's `resolve_tag`.
///
/// ```rust
/// use tak_update::version::parse_version;
/// assert_eq!(parse_version("v0.1.7").unwrap(), parse_version("0.1.7").unwrap());
/// assert!(parse_version("0.2.0").unwrap() > parse_version("0.1.99").unwrap());
/// assert!(parse_version("v1.2").is_err());
/// ```
pub fn parse_version(text: &str) -> Result<Version, VersionParseError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(VersionParseError::Empty(text.to_string()));
    }
    let body = trimmed.strip_prefix('v').unwrap_or(trimmed);
    let mut parts = body.split('.');
    let triple = (parts.next(), parts.next(), parts.next(), parts.next());
    let (major, minor, patch) = match triple {
        (Some(major), Some(minor), Some(patch), None) => (major, minor, patch),
        _ => return Err(VersionParseError::Shape(text.to_string())),
    };
    Ok(Version {
        major: parse_component(major, text)?,
        minor: parse_component(minor, text)?,
        patch: parse_component(patch, text)?,
    })
}

fn parse_component(component: &str, original: &str) -> Result<u64, VersionParseError> {
    if component.is_empty() || !component.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(VersionParseError::NonNumeric(original.to_string()));
    }
    component
        .parse::<u64>()
        .map_err(|_| VersionParseError::NonNumeric(original.to_string()))
}

/// Render a version as a release tag string (`vX.Y.Z`).
///
/// ```rust
/// use tak_update::version::{parse_version, tag_string};
/// assert_eq!(tag_string(parse_version("0.1.7").unwrap()), "v0.1.7");
/// ```
pub fn tag_string(version: Version) -> String {
    format!("v{version}")
}
