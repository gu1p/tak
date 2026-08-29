use std::cell::Cell;
use std::fs;
use std::io::{Error, ErrorKind};

use super::super::{build_submit_idempotency_key, sanitize_submit_idempotency_key};
use super::{CLEANUP_TOMBSTONE_PREFIX, quarantine};

#[test]
fn tombstone_namespace_cannot_be_authored_by_a_submit_id() {
    let submit_key =
        build_submit_idempotency_key(CLEANUP_TOMBSTONE_PREFIX, Some(1)).expect("submit key");
    let storage_name = sanitize_submit_idempotency_key(&submit_key);

    assert!(
        !storage_name.starts_with(CLEANUP_TOMBSTONE_PREFIX),
        "live submit storage was classified as a cleanup tombstone: {storage_name}"
    );
}

#[test]
fn missing_source_is_a_benign_quarantine_miss() {
    let temp = tempfile::tempdir().expect("tempdir");
    let missing = temp.path().join("sessions");

    let result = quarantine::quarantine_stale_remote_entry(&missing).expect("quarantine miss");

    assert!(result.is_none());
}

#[test]
fn quarantine_uses_one_rename_and_preserves_source_on_failure() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source = temp.path().join("sessions");
    fs::create_dir(&source).expect("create source");
    let rename_calls = Cell::new(0);

    let error = quarantine::quarantine_stale_remote_entry_with(&source, |from, to| {
        rename_calls.set(rename_calls.get() + 1);
        assert_eq!(from, source);
        assert_eq!(to.parent(), source.parent());
        assert!(
            to.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(CLEANUP_TOMBSTONE_PREFIX)
        );
        Err(Error::new(ErrorKind::PermissionDenied, "rename denied"))
    })
    .expect_err("rename must fail");

    assert_eq!(rename_calls.get(), 1);
    assert!(format!("{error:#}").contains("quarantine"));
    assert!(source.exists());
}
