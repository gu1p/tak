//! The release-source port and pure helpers for locating release assets.
//!
//! [`ReleaseClient`] is the seam the update use-case depends on; tests use a fake
//! and the real HTTPS implementation lives in [`crate::http`]. The naming/URL
//! helpers and [`tag_from_latest_url`] are pure so they unit-test without network.

/// The default upstream release repository (`owner/name`).
pub const DEFAULT_REPO: &str = "gu1p/tak";

/// The minisign public key release signatures are verified against. Rotate by
/// replacing `keys/release.pub` and the matching CI signing secret.
pub const RELEASE_PUBLIC_KEY: &str = include_str!("../keys/release.pub");

/// Coordinates identifying one release archive on GitHub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseCoordinates {
    /// `owner/name` of the GitHub repository.
    pub repo: String,
    /// Release tag, e.g. `v0.1.7`.
    pub tag: String,
    /// Target triple, e.g. `x86_64-unknown-linux-musl`.
    pub target: String,
}

impl ReleaseCoordinates {
    /// Build coordinates from any string-like parts.
    ///
    /// ```
    /// use tak_update::release_client::ReleaseCoordinates;
    /// let c = ReleaseCoordinates::new("gu1p/tak", "v0.1.7", "aarch64-apple-darwin");
    /// assert_eq!(c.tag, "v0.1.7");
    /// ```
    pub fn new(repo: impl Into<String>, tag: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            repo: repo.into(),
            tag: tag.into(),
            target: target.into(),
        }
    }

    /// The archive file name, e.g. `tak-v0.1.7-x86_64-unknown-linux-musl.tar.gz`.
    pub fn archive_name(&self) -> String {
        format!("tak-{}-{}.tar.gz", self.tag, self.target)
    }

    /// The archive download URL (itself 302-redirects to the asset CDN).
    pub fn archive_url(&self) -> String {
        format!(
            "https://github.com/{}/releases/download/{}/{}",
            self.repo,
            self.tag,
            self.archive_name(),
        )
    }

    /// The companion checksum URL (`<archive>.sha256`).
    pub fn sha256_url(&self) -> String {
        format!("{}.sha256", self.archive_url())
    }

    /// The companion signature URL (`<archive>.minisig`).
    pub fn signature_url(&self) -> String {
        format!("{}.minisig", self.archive_url())
    }
}

/// The `releases/latest` URL whose redirect reveals the newest tag.
pub fn latest_release_url(repo: &str) -> String {
    format!("https://github.com/{repo}/releases/latest")
}

/// Error returned when a `releases/latest` redirect URL cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TagParseError {
    /// The URL did not contain a `/releases/tag/` segment.
    #[error("`{0}` is not a releases/tag URL")]
    NotATagUrl(String),
    /// The URL had a `/releases/tag/` segment but no tag after it.
    #[error("releases/tag URL `{0}` has no tag segment")]
    MissingTag(String),
}

/// Extract the release tag from a resolved `…/releases/tag/<tag>` URL.
///
/// Mirrors the `resolve_tag` parsing in `get-tak.sh`: take the path segment after
/// `/releases/tag/`, dropping any `?query`/`#fragment` and trailing slash.
///
/// ```
/// use tak_update::release_client::tag_from_latest_url;
/// assert_eq!(
///     tag_from_latest_url("https://github.com/gu1p/tak/releases/tag/v0.1.7").unwrap(),
///     "v0.1.7",
/// );
/// assert!(tag_from_latest_url("https://github.com/gu1p/tak/releases/latest").is_err());
/// ```
pub fn tag_from_latest_url(url: &str) -> Result<String, TagParseError> {
    let marker = "/releases/tag/";
    let index = url
        .find(marker)
        .ok_or_else(|| TagParseError::NotATagUrl(url.to_string()))?;
    let rest = &url[index + marker.len()..];
    let rest = rest.split(['?', '#']).next().unwrap_or(rest);
    let tag = rest.trim_end_matches('/');
    let tag = tag.split('/').next().unwrap_or(tag);
    if tag.is_empty() {
        return Err(TagParseError::MissingTag(url.to_string()));
    }
    Ok(tag.to_string())
}

/// A source of release metadata and artifacts.
///
/// Implementations are synchronous; async callers wrap them in
/// `tokio::task::spawn_blocking`. The real implementation lives in
/// [`crate::http`]; tests use an in-memory fake.
pub trait ReleaseClient: Send + Sync {
    /// Resolve the newest release tag for `repo` (e.g. `v0.1.7`).
    fn resolve_latest_tag(&self, repo: &str) -> anyhow::Result<String>;
    /// Download the release archive bytes.
    fn download_archive(&self, coordinates: &ReleaseCoordinates) -> anyhow::Result<Vec<u8>>;
    /// Download the companion `.sha256` checksum line.
    fn download_sha256(&self, coordinates: &ReleaseCoordinates) -> anyhow::Result<String>;
    /// Download the companion `.minisig` signature file content.
    fn download_signature(&self, coordinates: &ReleaseCoordinates) -> anyhow::Result<String>;
}
