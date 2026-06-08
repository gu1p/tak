//! Shared in-memory fixtures for the `tak-update` test suite.

use flate2::Compression;
use flate2::write::GzEncoder;
use tak_update::release_client::{ReleaseClient, ReleaseCoordinates};

/// Build an in-memory `.tar.gz` holding the given `(name, bytes)` members at root,
/// matching how `scripts/package_release_target.sh` packages a release.
pub fn make_targz(members: &[(&str, &[u8])]) -> Vec<u8> {
    let encoder = GzEncoder::new(Vec::new(), Compression::default());
    let mut builder = tar::Builder::new(encoder);
    for &(name, bytes) in members {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        builder
            .append_data(&mut header, name, bytes)
            .expect("append fixture member");
    }
    let encoder = builder.into_inner().expect("finish tar");
    encoder.finish().expect("finish gzip")
}

/// A minimal executable shell-script "binary" whose `--version` prints
/// `<name> <version>` — enough for the installer's pre-swap validation.
pub fn fake_binary(name: &str, version: &str) -> Vec<u8> {
    format!("#!/bin/sh\necho \"{name} {version}\"\n").into_bytes()
}

/// An in-memory [`ReleaseClient`] serving canned release artifacts.
pub struct FakeReleaseClient {
    /// Tag returned by `resolve_latest_tag`.
    pub latest: String,
    /// Bytes returned by `download_archive`.
    pub archive: Vec<u8>,
    /// Line returned by `download_sha256`.
    pub sha256_line: String,
    /// Content returned by `download_signature`.
    pub signature: String,
}

impl ReleaseClient for FakeReleaseClient {
    fn resolve_latest_tag(&self, _repo: &str) -> anyhow::Result<String> {
        Ok(self.latest.clone())
    }

    fn download_archive(&self, _coordinates: &ReleaseCoordinates) -> anyhow::Result<Vec<u8>> {
        Ok(self.archive.clone())
    }

    fn download_sha256(&self, _coordinates: &ReleaseCoordinates) -> anyhow::Result<String> {
        Ok(self.sha256_line.clone())
    }

    fn download_signature(&self, _coordinates: &ReleaseCoordinates) -> anyhow::Result<String> {
        Ok(self.signature.clone())
    }
}
