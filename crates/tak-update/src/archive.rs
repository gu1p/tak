//! In-memory extraction of release archives.
//!
//! A release archive is a gzipped tarball holding exactly the `tak` and `takd`
//! binaries at its root (`tar -C <dir> -czf <archive> tak takd`, see
//! `scripts/package_release_target.sh`). Extraction happens fully in memory —
//! nothing is written to disk here — and only after the archive's signature and
//! checksum have been verified by the caller. Only regular-file members named
//! exactly `tak`/`takd` are read, each bounded by [`MAX_MEMBER_BYTES`].

use std::collections::HashMap;
use std::io::Read;

use flate2::read::GzDecoder;
use tar::Archive;

use crate::verify::VerifiedArchive;

/// Defensive upper bound on a single archive member (binaries are tens of MB).
const MAX_MEMBER_BYTES: u64 = 512 * 1024 * 1024;

/// The two binaries shipped in every release archive.
#[derive(Debug, Clone)]
pub struct Binaries {
    /// Bytes of the `tak` CLI binary.
    pub tak: Vec<u8>,
    /// Bytes of the `takd` daemon binary.
    pub takd: Vec<u8>,
}

/// Error returned while decompressing or reading a release archive.
#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    /// The gzip/tar stream could not be read.
    #[error("failed to read release archive: {0}")]
    Io(String),
    /// A member exceeded [`MAX_MEMBER_BYTES`].
    #[error("release archive member `{0}` exceeds the {1}-byte limit")]
    MemberTooLarge(String, u64),
    /// A wanted binary appeared more than once.
    #[error("release archive contains a duplicate `{0}` member")]
    DuplicateMember(String),
    /// A required binary was absent from the archive.
    #[error("release archive is missing the `{0}` binary")]
    MissingMember(String),
}

/// Decompress a verified `.tar.gz` release archive in memory and return `tak` +
/// `takd`.
///
/// Requires a [`VerifiedArchive`] (signature + checksum already verified), so this
/// can only run on authentic bytes. Errors if either binary is missing,
/// duplicated, or implausibly large. Members are matched by their root path
/// (`tak`/`takd`), tolerating a leading `./`.
///
/// ```no_run
/// # // Reason: requires a VerifiedArchive (signature + checksum already verified).
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn extract_binaries(archive: &VerifiedArchive) -> Result<Binaries, ArchiveError> {
    let mut members = collect_members(archive.as_bytes(), &["tak", "takd"])?;
    let tak = members
        .remove("tak")
        .ok_or_else(|| ArchiveError::MissingMember("tak".to_string()))?;
    let takd = members
        .remove("takd")
        .ok_or_else(|| ArchiveError::MissingMember("takd".to_string()))?;
    Ok(Binaries { tak, takd })
}

fn collect_members(
    targz: &[u8],
    wanted: &[&str],
) -> Result<HashMap<String, Vec<u8>>, ArchiveError> {
    let mut found: HashMap<String, Vec<u8>> = HashMap::new();
    let mut archive = Archive::new(GzDecoder::new(targz));
    for entry in archive.entries().map_err(io_err)? {
        let entry = entry.map_err(io_err)?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let raw = entry.path().map_err(io_err)?.to_string_lossy().into_owned();
        let name = raw.strip_prefix("./").unwrap_or(&raw).to_string();
        if !wanted.contains(&name.as_str()) {
            continue;
        }
        if found.contains_key(&name) {
            return Err(ArchiveError::DuplicateMember(name));
        }
        if entry.header().size().map_err(io_err)? > MAX_MEMBER_BYTES {
            return Err(ArchiveError::MemberTooLarge(name, MAX_MEMBER_BYTES));
        }
        let mut bytes = Vec::new();
        let read = entry
            .take(MAX_MEMBER_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(io_err)?;
        if read as u64 > MAX_MEMBER_BYTES {
            return Err(ArchiveError::MemberTooLarge(name, MAX_MEMBER_BYTES));
        }
        found.insert(name, bytes);
    }
    Ok(found)
}

fn io_err(err: std::io::Error) -> ArchiveError {
    ArchiveError::Io(err.to_string())
}
