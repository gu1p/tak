use std::fs;
use std::path::Path;

use tak_update::fs_installer::FsInstaller;
use tak_update::installer::{BinaryArtifact, InstallPlan, Installer};

use crate::fixtures::fake_binary;

fn artifact(name: &str, dest: &Path, version: &str) -> BinaryArtifact {
    BinaryArtifact::for_test(name, dest, fake_binary(name, version))
}

#[test]
fn rolls_back_first_binary_when_second_commit_fails() {
    let dir = tempfile::tempdir().unwrap();
    let takd = dir.path().join("takd");
    fs::write(&takd, fake_binary("takd", "0.1.0")).unwrap();

    // `tak`'s destination is a directory, so its commit fails after `takd` swapped.
    let tak_dir = dir.path().join("tak");
    fs::create_dir(&tak_dir).unwrap();

    let plan = InstallPlan::for_test(
        "v0.1.7",
        vec![
            artifact("takd", &takd, "0.1.7"),
            artifact("tak", &tak_dir, "0.1.7"),
        ],
    );
    assert!(FsInstaller.install(&plan).is_err());
    assert_eq!(fs::read(&takd).unwrap(), fake_binary("takd", "0.1.0"));
}

#[test]
fn removes_freshly_created_binary_when_later_commit_fails() {
    let dir = tempfile::tempdir().unwrap();
    // `takd` does not pre-exist, so its commit creates it fresh.
    let takd = dir.path().join("takd");
    // `tak`'s destination is a directory, so its commit fails after `takd` swapped.
    let tak_dir = dir.path().join("tak");
    fs::create_dir(&tak_dir).unwrap();

    let plan = InstallPlan::for_test(
        "v0.1.7",
        vec![
            artifact("takd", &takd, "0.1.7"),
            artifact("tak", &tak_dir, "0.1.7"),
        ],
    );
    assert!(FsInstaller.install(&plan).is_err());
    // The freshly-created `takd` must be removed by rollback, not left behind.
    assert!(!takd.exists(), "fresh takd should be rolled back");
}
