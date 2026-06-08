//! The production HTTPS [`ReleaseClient`], backed by `ureq` (rustls + bundled
//! WebPKI roots).
//!
//! The latest tag is read from the `releases/latest` 302 redirect's `Location`
//! header (mirroring `get-tak.sh`); archives, checksums, and signatures are
//! downloaded as release assets, following the GitHub→CDN redirect. Transient
//! 5xx/connection failures are retried (GitHub's asset CDN occasionally 504s).
//! The client is synchronous; async callers wrap it in `spawn_blocking`.

use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use ureq::Agent;

use crate::release_client::{
    ReleaseClient, ReleaseCoordinates, latest_release_url, tag_from_latest_url,
};

/// Cap on a downloaded archive (binaries are tens of MB; this is generous).
const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
/// Cap on small text assets (`.sha256`, `.minisig`).
const MAX_TEXT_BYTES: u64 = 64 * 1024;
/// Per-request wall-clock timeout.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
/// Attempts for a transient (5xx / connection) failure before giving up.
const MAX_ATTEMPTS: u32 = 3;

/// A [`ReleaseClient`] that fetches release assets from GitHub over HTTPS.
pub struct UreqReleaseClient {
    /// Follows redirects (GitHub asset → CDN) for downloads.
    download: Agent,
    /// Does not follow redirects, so `resolve_latest_tag` can read `Location`.
    probe: Agent,
}

impl UreqReleaseClient {
    /// Build a client with sensible timeouts and bundled WebPKI roots.
    ///
    /// ```no_run
    /// use tak_update::http::UreqReleaseClient;
    /// use tak_update::release_client::ReleaseClient;
    /// let client = UreqReleaseClient::new();
    /// let tag = client.resolve_latest_tag("gu1p/tak")?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn new() -> Self {
        Self {
            download: build_agent(10),
            probe: build_agent(0),
        }
    }

    /// GET `url` with retry on transient failures, returning the body bytes.
    fn fetch_bytes(&self, url: &str, limit: u64) -> Result<Vec<u8>> {
        let mut attempt = 0;
        loop {
            attempt += 1;
            match self.download.get(url).call() {
                Ok(mut response) if response.status().is_success() => {
                    return response
                        .body_mut()
                        .with_config()
                        .limit(limit)
                        .read_to_vec()
                        .with_context(|| format!("read body of {url}"));
                }
                Ok(response) if response.status().is_server_error() && attempt < MAX_ATTEMPTS => {}
                Ok(response) => bail!("GET {url} returned status {}", response.status()),
                Err(err) if attempt < MAX_ATTEMPTS => {
                    let _ = err;
                }
                Err(err) => return Err(anyhow!("GET {url}: {err}")),
            }
            std::thread::sleep(Duration::from_millis(400 * u64::from(attempt)));
        }
    }

    fn fetch_text(&self, url: &str) -> Result<String> {
        let bytes = self.fetch_bytes(url, MAX_TEXT_BYTES)?;
        String::from_utf8(bytes).with_context(|| format!("decode {url} as UTF-8"))
    }
}

impl Default for UreqReleaseClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ReleaseClient for UreqReleaseClient {
    fn resolve_latest_tag(&self, repo: &str) -> Result<String> {
        let url = latest_release_url(repo);
        let response = self
            .probe
            .get(&url)
            .call()
            .with_context(|| format!("GET {url}"))?;
        let location = response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                anyhow!(
                    "{url} did not redirect to a release tag (status {})",
                    response.status()
                )
            })?;
        tag_from_latest_url(location)
            .map_err(|err| anyhow!("parse latest tag from `{location}`: {err}"))
    }

    fn download_archive(&self, coordinates: &ReleaseCoordinates) -> Result<Vec<u8>> {
        self.fetch_bytes(&coordinates.archive_url(), MAX_ARCHIVE_BYTES)
    }

    fn download_sha256(&self, coordinates: &ReleaseCoordinates) -> Result<String> {
        self.fetch_text(&coordinates.sha256_url())
    }

    fn download_signature(&self, coordinates: &ReleaseCoordinates) -> Result<String> {
        self.fetch_text(&coordinates.signature_url())
    }
}

fn build_agent(max_redirects: u32) -> Agent {
    Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .max_redirects(max_redirects)
        .http_status_as_error(false)
        .build()
        .new_agent()
}
