use std::fs;

use super::{InstallerFixture, PathMode};

/// Verifies installer adds one PATH line to active shell rc file and does not duplicate it.
#[test]
fn appends_path_to_active_shell_rc_once() {
    let fixture = InstallerFixture::new();
    let rc = fixture.home_dir().join(".zshrc");

    fixture.run("v1", "/bin/zsh", PathMode::WithoutInstallDirInPath);
    fixture.run("v2", "/bin/zsh", PathMode::WithoutInstallDirInPath);

    let rc_content = fs::read_to_string(&rc).expect("zshrc should exist");
    let expected = format!(
        "export PATH=\"{}:$PATH\"",
        fixture.home_dir().join(".local/bin").display()
    );
    let occurrences = rc_content.lines().filter(|line| *line == expected).count();
    assert_eq!(occurrences, 1, "path export should be appended once");
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
