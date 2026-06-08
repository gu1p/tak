use std::process::Command;

use tak_update::fs_installer::FsInstaller;
use tak_update::plan::{Destinations, UpdateAction, UpdateOptions, run_update};
use tak_update::version::parse_version;

use crate::fixtures::FakeReleaseClient;

const PUBLIC_KEY: &str = include_str!("data/test_release.pub");
const SIGNATURE: &str = include_str!("data/test_archive.tar.gz.minisig");
const SHA256: &str = include_str!("data/test_archive.tar.gz.sha256");
const ARCHIVE: &[u8] = include_bytes!("data/test_archive.tar.gz");

fn fake() -> FakeReleaseClient {
    FakeReleaseClient {
        latest: "v0.1.7".to_string(),
        archive: ARCHIVE.to_vec(),
        sha256_line: SHA256.to_string(),
        signature: SIGNATURE.to_string(),
    }
}

fn opts(current: &str, check_only: bool) -> UpdateOptions<'static> {
    UpdateOptions {
        repo: "gu1p/tak",
        target: "x86_64-unknown-linux-musl",
        current: parse_version(current).unwrap(),
        requested_tag: None,
        allow_downgrade: false,
        check_only,
        public_key: PUBLIC_KEY,
    }
}

#[test]
fn installs_newer_release_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let dests = Destinations {
        tak: Some(dir.path().join("tak")),
        takd: Some(dir.path().join("takd")),
    };
    let outcome = run_update(&fake(), &FsInstaller, &dests, &opts("0.1.0", false)).unwrap();
    assert_eq!(outcome.tag, "v0.1.7");
    assert!(matches!(outcome.action, UpdateAction::Installed(_)));
    for name in ["tak", "takd"] {
        let path = dir.path().join(name);
        let out = Command::new(&path).arg("--version").output().unwrap();
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            format!("{name} 0.1.7")
        );
    }
}

#[test]
fn check_only_reports_without_installing() {
    let dir = tempfile::tempdir().unwrap();
    let dests = Destinations {
        tak: None,
        takd: Some(dir.path().join("takd")),
    };
    let outcome = run_update(&fake(), &FsInstaller, &dests, &opts("0.1.0", true)).unwrap();
    assert!(matches!(outcome.action, UpdateAction::Available));
    assert!(!dir.path().join("takd").exists());
}

#[test]
fn up_to_date_when_current_is_latest() {
    let dir = tempfile::tempdir().unwrap();
    let dests = Destinations {
        tak: None,
        takd: Some(dir.path().join("takd")),
    };
    let outcome = run_update(&fake(), &FsInstaller, &dests, &opts("0.1.7", false)).unwrap();
    assert!(matches!(outcome.action, UpdateAction::UpToDate));
    assert!(!dir.path().join("takd").exists());
}
