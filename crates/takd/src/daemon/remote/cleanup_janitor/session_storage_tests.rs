use std::collections::BTreeSet;
use std::fs;
use std::time::Duration;

use super::quarantine::{cleanup_quarantined_remote_entries, quarantine_stale_remote_entry};
use super::storage::{cleanup_stale_remote_entries_with, remove_stale_remote_entry};
use super::{CLEANUP_TOMBSTONE_PREFIX, protect_session_storage_roots};

#[test]
fn active_work_protects_both_session_storage_roots() {
    let mut protected = BTreeSet::from(["active-job_1".to_string()]);

    protect_session_storage_roots(&mut protected);

    assert_eq!(
        protected,
        BTreeSet::from([
            "active-job_1".to_string(),
            "session-paths".to_string(),
            "sessions".to_string(),
        ])
    );
}

#[test]
fn idle_worker_does_not_keep_session_storage_roots_alive() {
    let mut protected = BTreeSet::new();

    protect_session_storage_roots(&mut protected);

    assert!(protected.is_empty());
}

#[test]
fn generic_sweep_skips_direct_tombstones_then_reaps_them() {
    let temp = tempfile::tempdir().expect("tempdir");
    let session_root = temp.path().join("sessions");
    fs::create_dir(&session_root).expect("create session root");
    let tombstone = quarantine_stale_remote_entry(&session_root)
        .expect("quarantine session")
        .expect("session existed");
    assert_eq!(tombstone.parent(), Some(temp.path()));

    cleanup_stale_remote_entries_with(
        temp.path(),
        &BTreeSet::new(),
        Duration::ZERO,
        remove_stale_remote_entry,
    )
    .expect("generic cleanup");
    assert!(tombstone.exists(), "generic sweep removed a tombstone");

    cleanup_quarantined_remote_entries(temp.path()).expect("reap tombstones");
    assert!(!tombstone.exists());
}

#[cfg(unix)]
#[test]
fn cleanup_unlinks_a_direct_tombstone_symlink_without_following_it() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("outside tempdir");
    let outside_entry = outside.path().join("must-survive");
    fs::write(&outside_entry, b"present").expect("write outside entry");
    let tombstone = temp
        .path()
        .join(format!("{CLEANUP_TOMBSTONE_PREFIX}swapped"));
    symlink(outside.path(), &tombstone).expect("symlink tombstone");

    cleanup_quarantined_remote_entries(temp.path()).expect("reap tombstone link");

    assert!(!tombstone.exists());
    assert!(outside_entry.exists());
}
