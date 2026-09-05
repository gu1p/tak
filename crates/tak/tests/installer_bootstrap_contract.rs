#![cfg(unix)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn installers_reach_extraction_without_minisign() {
    for script in ["get-tak.sh", "get-takd.sh"] {
        let (temp, output) = run(script, "valid");
        assert_eq!(output.status.code(), Some(86), "{script}: {output:?}");
        assert!(temp.path().join("extracted").exists());
        let downloads = fs::read_to_string(temp.path().join("downloads")).unwrap();
        assert_eq!(downloads.lines().count(), 1);
        assert!(downloads.trim_end().ends_with(".tar.gz"));
    }
}

#[test]
fn failed_downloads_stop_before_archive_extraction() {
    for script in ["get-tak.sh", "get-takd.sh"] {
        let (temp, output) = run(script, "missing");
        assert!(!output.status.success());
        assert!(!temp.path().join("extracted").exists());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("failed to download release artifact")
        );
    }
}

fn run(script: &str, mode: &str) -> (tempfile::TempDir, std::process::Output) {
    let temp = tempfile::tempdir().unwrap();
    let source = fs::read_to_string(root().join(script)).unwrap();
    let definitions = source.strip_suffix("main \"$@\"\n").unwrap();
    let fixture = include_str!("support/installer_bootstrap.sh");
    let output = Command::new("/bin/bash")
        .args(["-c", &format!("{definitions}\n{fixture}\nmain")])
        .env("TAK_VERSION", "1.2.3")
        .env("DOWNLOAD_MODE", mode)
        .env("INSTALLER_TEST_ROOT", temp.path())
        .output()
        .unwrap();
    (temp, output)
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
