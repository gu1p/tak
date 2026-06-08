use tak_update::fs_installer::FsInstaller;
use tak_update::plan::{Destinations, UpdateOptions, run_update};
use tak_update::version::parse_version;

use crate::fixtures::FakeReleaseClient;

const PUBLIC_KEY: &str = include_str!("data/test_release.pub");
const OTHER_KEY: &str = include_str!("data/other_release.pub");
const SIGNATURE: &str = include_str!("data/test_archive.tar.gz.minisig");
const SHA256: &str = include_str!("data/test_archive.tar.gz.sha256");
const ARCHIVE: &[u8] = include_bytes!("data/test_archive.tar.gz");

fn base_fake() -> FakeReleaseClient {
    FakeReleaseClient {
        latest: "v0.1.7".to_string(),
        archive: ARCHIVE.to_vec(),
        sha256_line: SHA256.to_string(),
        signature: SIGNATURE.to_string(),
    }
}

fn opts(public_key: &'static str) -> UpdateOptions<'static> {
    UpdateOptions {
        repo: "gu1p/tak",
        target: "x86_64-unknown-linux-musl",
        current: parse_version("0.1.0").unwrap(),
        requested_tag: None,
        allow_downgrade: false,
        check_only: false,
        public_key,
    }
}

#[test]
fn rejects_archive_signed_by_unknown_key() {
    let dir = tempfile::tempdir().unwrap();
    let dests = Destinations {
        tak: None,
        takd: Some(dir.path().join("takd")),
    };
    let result = run_update(&base_fake(), &FsInstaller, &dests, &opts(OTHER_KEY));
    assert!(result.is_err());
    assert!(!dir.path().join("takd").exists());
}

#[test]
fn rejects_tampered_archive_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let dests = Destinations {
        tak: None,
        takd: Some(dir.path().join("takd")),
    };
    let mut fake = base_fake();
    fake.archive[20] ^= 0xff;
    let result = run_update(&fake, &FsInstaller, &dests, &opts(PUBLIC_KEY));
    assert!(result.is_err());
    assert!(!dir.path().join("takd").exists());
}

#[test]
fn rejects_valid_signature_with_bad_checksum() {
    let dir = tempfile::tempdir().unwrap();
    let dests = Destinations {
        tak: None,
        takd: Some(dir.path().join("takd")),
    };
    // Real signature over the real archive, but a wrong sha256 line: the signature
    // passes and the checksum fails, so nothing is installed.
    let mut fake = base_fake();
    fake.sha256_line = format!("{}  test_archive.tar.gz", "0".repeat(64));
    let result = run_update(&fake, &FsInstaller, &dests, &opts(PUBLIC_KEY));
    assert!(result.is_err());
    assert!(!dir.path().join("takd").exists());
}
