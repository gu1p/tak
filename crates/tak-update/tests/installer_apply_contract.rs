use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use tak_update::fs_installer::FsInstaller;
use tak_update::installer::{BinaryArtifact, InstallError, InstallPlan, Installer};

use crate::fixtures::fake_binary;

fn artifact(dir: &Path, name: &str, version: &str) -> BinaryArtifact {
    BinaryArtifact::for_test(name, dir.join(name), fake_binary(name, version))
}

#[test]
fn installs_both_binaries_and_sets_mode() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("tak"), fake_binary("tak", "0.1.0")).unwrap();
    fs::write(dir.path().join("takd"), fake_binary("takd", "0.1.0")).unwrap();
    let plan = InstallPlan::for_test(
        "v0.1.7",
        vec![
            artifact(dir.path(), "tak", "0.1.7"),
            artifact(dir.path(), "takd", "0.1.7"),
        ],
    );
    let report = FsInstaller.install(&plan).unwrap();
    assert_eq!(
        report.installed,
        vec!["tak".to_string(), "takd".to_string()]
    );
    for name in ["tak", "takd"] {
        let path = dir.path().join(name);
        assert_eq!(fs::read(&path).unwrap(), fake_binary(name, "0.1.7"));
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o755
        );
    }
}

#[test]
fn version_mismatch_aborts_without_swapping() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("takd"), fake_binary("takd", "0.1.0")).unwrap();
    let plan = InstallPlan::for_test("v0.1.7", vec![artifact(dir.path(), "takd", "0.1.5")]);
    let err = FsInstaller.install(&plan).unwrap_err();
    assert!(matches!(err, InstallError::VersionMismatch { .. }));
    assert_eq!(
        fs::read(dir.path().join("takd")).unwrap(),
        fake_binary("takd", "0.1.0"),
    );
}

#[test]
fn second_candidate_invalid_touches_no_live_binary() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("takd"), fake_binary("takd", "0.1.0")).unwrap();
    fs::write(dir.path().join("tak"), fake_binary("tak", "0.1.0")).unwrap();
    // takd validates, but tak reports the wrong version — neither must be swapped.
    let plan = InstallPlan::for_test(
        "v0.1.7",
        vec![
            artifact(dir.path(), "takd", "0.1.7"),
            artifact(dir.path(), "tak", "0.1.5"),
        ],
    );
    assert!(matches!(
        FsInstaller.install(&plan).unwrap_err(),
        InstallError::VersionMismatch { .. },
    ));
    assert_eq!(
        fs::read(dir.path().join("takd")).unwrap(),
        fake_binary("takd", "0.1.0")
    );
    assert_eq!(
        fs::read(dir.path().join("tak")).unwrap(),
        fake_binary("tak", "0.1.0")
    );
}

#[test]
fn installs_into_fresh_dir_without_backup() {
    let dir = tempfile::tempdir().unwrap();
    let plan = InstallPlan::for_test("v0.1.7", vec![artifact(dir.path(), "takd", "0.1.7")]);
    let report = FsInstaller.install(&plan).unwrap();
    assert_eq!(report.installed, vec!["takd".to_string()]);
    assert!(report.backups.is_empty());
    assert!(dir.path().join("takd").exists());
}
