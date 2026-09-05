use std::fs;
use std::process::Command;

use super::{InstallerFixture, PathMode};

/// Verifies installer adds one PATH line to active shell rc file and does not duplicate it.
#[test]
fn appends_path_to_active_shell_rc_once() {
    let fixture = InstallerFixture::new();
    let rc = fixture.home_dir().join(".zshrc");

    fixture.run("v1", "/bin/zsh", PathMode::WithoutInstallDirInPath);
    fixture.run("v2", "/bin/zsh", PathMode::WithoutInstallDirInPath);

    let loaded = Command::new("/bin/sh")
        .args(["-c", ". \"$TEST_RC\"; printf '%s' \"$PATH\""])
        .env("TEST_RC", &rc)
        .env("PATH", "/usr/bin:/bin")
        .output()
        .expect("load generated shell profile");
    assert!(loaded.status.success(), "{loaded:?}");
    let expected = format!(
        "{}:/usr/bin:/bin",
        fixture.home_dir().join(".local/bin").display()
    );
    assert_eq!(
        String::from_utf8(loaded.stdout).unwrap(),
        expected,
        "install directory should be prepended exactly once"
    );
}

/// Verifies installer does not touch rc file when install directory is already in current PATH.
#[test]
fn does_not_edit_shell_rc_when_install_dir_already_in_path() {
    let fixture = InstallerFixture::new();
    let rc = fixture.home_dir().join(".bashrc");
    fs::write(&rc, "# keep-me\n").expect("seed bashrc");

    fixture.run("v1", "/bin/bash", PathMode::WithInstallDirInPath);

    let rc_content = fs::read_to_string(&rc).expect("bashrc should exist");
    assert_eq!(rc_content, "# keep-me\n", "bashrc should not be modified");
}
