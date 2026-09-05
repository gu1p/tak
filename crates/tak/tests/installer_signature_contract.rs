#![cfg(unix)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn missing_verifier_stops_with_an_actionable_error() {
    for script in ["get-tak.sh", "get-takd.sh"] {
        let (temp, output) = run(script, "no_verifier");
        assert!(!output.status.success());
        assert!(!temp.path().join("extracted").exists());
        assert!(String::from_utf8_lossy(&output.stderr).contains("minisign is required"));
    }
}

#[test]
fn rejected_release_signatures_stop_before_archive_extraction() {
    for script in ["get-tak.sh", "get-takd.sh"] {
        let (temp, output) = run(script, "invalid");
        assert!(!output.status.success());
        assert!(
            !temp.path().join("extracted").exists(),
            "{script} extracted an unverified archive"
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("signature"));
    }
}

#[test]
fn missing_release_signatures_stop_before_verification_or_extraction() {
    for script in ["get-tak.sh", "get-takd.sh"] {
        let (temp, output) = run(script, "missing");
        assert!(!output.status.success());
        assert!(!temp.path().join("verified").exists());
        assert!(!temp.path().join("extracted").exists());
        assert!(String::from_utf8_lossy(&output.stderr).contains("signature"));
    }
}

#[test]
fn installers_verify_prehashed_signatures_with_the_updaters_pinned_key() {
    let key = fs::read_to_string(root().join("crates/tak-update/keys/release.pub")).unwrap();
    let key = key.lines().nth(1).unwrap();
    for script in ["get-tak.sh", "get-takd.sh"] {
        let (temp, output) = run(script, "valid");
        assert_eq!(output.status.code(), Some(86), "{script}: {output:?}");
        let arguments = fs::read_to_string(temp.path().join("verified")).unwrap();
        let arguments = arguments.lines().collect::<Vec<_>>();
        assert!(arguments.contains(&"-H"), "must reject legacy signatures");
        let key_flag = arguments.iter().position(|value| *value == "-P").unwrap();
        assert_eq!(arguments[key_flag + 1], key);
        let archive = arguments.iter().position(|value| *value == "-m").unwrap();
        let signature = arguments.iter().position(|value| *value == "-x").unwrap();
        assert_eq!(
            arguments[signature + 1],
            format!("{}.minisig", arguments[archive + 1])
        );
        assert!(temp.path().join("extracted").exists());
    }
}

fn run(script: &str, mode: &str) -> (tempfile::TempDir, std::process::Output) {
    let temp = tempfile::tempdir().unwrap();
    let source = fs::read_to_string(root().join(script)).unwrap();
    let definitions = source.strip_suffix("main \"$@\"\n").unwrap();
    let fixture = include_str!("support/installer_signature.sh");
    let output = Command::new("/bin/bash")
        .args(["-c", &format!("{definitions}\n{fixture}\nmain")])
        .env("TAK_VERSION", "1.2.3")
        .env("SIGNATURE_MODE", mode)
        .env("SIGNATURE_TEST_ROOT", temp.path())
        .output()
        .unwrap();
    (temp, output)
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
