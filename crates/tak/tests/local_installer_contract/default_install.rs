use std::fs;

use super::{InstallerFixture, PathMode};

/// Verifies local installer builds and installs both binaries into `~/.local/bin` by default.
#[test]
fn installs_tak_and_takd_into_default_local_bin() {
    let fixture = InstallerFixture::new();
    fixture.run("v1", "/bin/zsh", PathMode::WithoutInstallDirInPath);

    let install_dir = fixture.home_dir().join(".local/bin");
    assert!(install_dir.join("tak").exists(), "tak should be installed");
    assert!(
        install_dir.join("takd").exists(),
        "takd should be installed"
    );

    let tak_body = fs::read_to_string(install_dir.join("tak")).expect("read installed tak");
    assert!(
        tak_body.contains("v1"),
        "installed tak should come from latest local build output"
    );
}

/// Verifies rerunning installer replaces existing binaries with newly built outputs.
#[test]
fn rerun_replaces_existing_binaries_with_new_build() {
    let fixture = InstallerFixture::new();
    fixture.run("old", "/bin/bash", PathMode::WithoutInstallDirInPath);
    fixture.run("new", "/bin/bash", PathMode::WithoutInstallDirInPath);

    let install_dir = fixture.home_dir().join(".local/bin");
    let tak_body = fs::read_to_string(install_dir.join("tak")).expect("read installed tak");
    assert!(
        tak_body.contains("new"),
        "new build should replace old binary"
    );
    assert!(
        !tak_body.contains("old"),
        "old build content should not remain after replacement"
    );
}

/// Verifies installer falls back to `~/bin` when `~/.local/bin` cannot be created.
#[test]
fn falls_back_to_home_bin_when_dot_local_bin_is_unavailable() {
    let fixture = InstallerFixture::new();
    fs::write(fixture.home_dir().join(".local"), "blocker").expect("write blocker file");

    fixture.run("fallback", "/bin/bash", PathMode::WithoutInstallDirInPath);

    assert!(
        fixture.home_dir().join("bin/tak").exists(),
        "tak should be installed to ~/bin fallback"
    );
    assert!(
        fixture.home_dir().join("bin/takd").exists(),
        "takd should be installed to ~/bin fallback"
    );
}
