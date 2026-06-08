use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use tak_update::swap::{back_up, restore, swap_binary_atomically};

#[test]
fn replaces_contents_and_sets_mode() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("takd");
    fs::write(&target, b"OLD").unwrap();
    swap_binary_atomically(&target, b"NEW-BYTES", 0o755).unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"NEW-BYTES");
    assert_eq!(
        fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o755
    );
}

#[test]
fn creates_target_when_absent() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("takd");
    swap_binary_atomically(&target, b"FRESH", 0o755).unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"FRESH");
}

#[test]
fn back_up_then_restore_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("takd");
    fs::write(&target, b"ORIGINAL").unwrap();
    let backup = back_up(&target).unwrap().expect("backup created");
    swap_binary_atomically(&target, b"BROKEN", 0o755).unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"BROKEN");
    restore(&backup).unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"ORIGINAL");
}

#[test]
fn back_up_absent_target_is_none() {
    let dir = tempfile::tempdir().unwrap();
    assert!(back_up(&dir.path().join("missing")).unwrap().is_none());
}

#[test]
fn leaves_original_intact_when_dir_readonly() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("takd");
    fs::write(&target, b"OLD").unwrap();
    set_mode(dir.path(), 0o555);
    if fs::write(dir.path().join(".probe"), b"x").is_ok() {
        // Running as root (directory perms bypassed); guarantee under test n/a.
        let _ = fs::remove_file(dir.path().join(".probe"));
        set_mode(dir.path(), 0o755);
        return;
    }
    let result = swap_binary_atomically(&target, b"NEW", 0o755);
    set_mode(dir.path(), 0o755);
    assert!(result.is_err());
    assert_eq!(fs::read(&target).unwrap(), b"OLD");
}

fn set_mode(path: &Path, mode: u32) {
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(mode);
    fs::set_permissions(path, perms).unwrap();
}
