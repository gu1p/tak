use std::fs;

use super::{InstallerFixture, PathMode};

/// Verifies installer resolves build artifacts from `CARGO_TARGET_DIR` when it is configured.
#[test]
fn installs_from_custom_cargo_target_dir() {
    let fixture = InstallerFixture::new();
    let custom_target = fixture.home_dir().join("custom-target");

    fixture.run_with_target_dir(
        "custom",
        "/bin/bash",
        PathMode::WithoutInstallDirInPath,
        &custom_target,
    );

    let install_dir = fixture.home_dir().join(".local/bin");
    let tak_body = fs::read_to_string(install_dir.join("tak")).expect("read installed tak");
    assert!(
        tak_body.contains("custom"),
        "installer should install binary emitted under CARGO_TARGET_DIR"
    );
}

/// Verifies installer follows Cargo metadata target-directory when env override is not set.
#[test]
fn installs_from_metadata_target_directory_without_env_override() {
    let fixture = InstallerFixture::new();
    let metadata_target = fixture.home_dir().join("metadata-target");

    fixture.run_with_metadata_target_no_env(
        "meta",
        "/bin/bash",
        PathMode::WithoutInstallDirInPath,
        &metadata_target,
    );

    let install_dir = fixture.home_dir().join(".local/bin");
    let tak_body = fs::read_to_string(install_dir.join("tak")).expect("read installed tak");
    assert!(
        tak_body.contains("meta"),
        "installer should use cargo metadata target_directory"
    );
}
