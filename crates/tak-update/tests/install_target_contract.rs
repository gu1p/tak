use std::fs;
use std::path::Path;

use tak_update::install_target::{
    Updatability, is_system_managed_path, sibling_path, updatability,
};

#[test]
fn flags_system_managed_paths() {
    for path in [
        "/usr/bin/takd",
        "/usr/local/bin/takd",
        "/opt/tak/takd",
        "/nix/store/abc-tak/bin/takd",
        "/opt/homebrew/bin/takd",
    ] {
        assert!(is_system_managed_path(Path::new(path)), "{path}");
    }
}

#[test]
fn allows_user_local_paths() {
    assert!(!is_system_managed_path(Path::new(
        "/home/u/.local/bin/takd"
    )));
    assert!(!is_system_managed_path(Path::new("/home/u/bin/takd")));
}

#[test]
fn sibling_shares_directory() {
    assert_eq!(
        sibling_path(Path::new("/home/u/.local/bin/takd"), "tak"),
        Path::new("/home/u/.local/bin/tak"),
    );
}

#[test]
fn writable_non_system_dir_is_updatable() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("takd");
    fs::write(&target, b"x").unwrap();
    assert_eq!(updatability(&target), Updatability::Updatable);
}
