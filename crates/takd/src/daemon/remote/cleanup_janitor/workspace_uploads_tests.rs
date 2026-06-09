use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use super::{
    WORKSPACE_UPLOADS_DIR_NAME, cleanup_stale_remote_entries_with,
    cleanup_stale_workspace_uploads_with, remove_stale_remote_entry,
    remove_stale_workspace_upload_file,
};

fn write_blob_with_age(path: &Path, age: Duration) {
    fs::write(path, b"zip-bytes").expect("write blob");
    let modified = SystemTime::now().checked_sub(age).expect("backdated mtime");
    let file = fs::OpenOptions::new()
        .write(true)
        .open(path)
        .expect("open blob to set mtime");
    file.set_modified(modified).expect("set blob mtime");
}

#[test]
fn per_blob_sweep_removes_only_stale_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let dir = temp.path();
    let stale = dir.join("stale-id.zip");
    let fresh = dir.join("fresh-id.zip");
    write_blob_with_age(&stale, Duration::from_secs(3600));
    write_blob_with_age(&fresh, Duration::ZERO);

    cleanup_stale_workspace_uploads_with(
        dir,
        Duration::from_secs(900),
        remove_stale_workspace_upload_file,
    )
    .expect("per-blob sweep");

    assert!(!stale.exists(), "stale blob should be reaped");
    assert!(
        fresh.exists(),
        "a recently-touched blob (file mtime fresh) must survive — this is what touch-on-resolve relies on"
    );
}

#[test]
fn per_blob_sweep_is_noop_when_dir_absent() {
    let temp = tempfile::tempdir().expect("tempdir");
    let missing = temp.path().join("no-such-uploads-dir");
    cleanup_stale_workspace_uploads_with(&missing, Duration::ZERO, |_| {
        panic!("remover must not be called for an absent dir")
    })
    .expect("absent upload dir is a no-op (artifact roots have none)");
}

#[test]
fn generic_sweep_skips_workspace_uploads_dir_but_reaps_stale_jobs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    let uploads = root.join(WORKSPACE_UPLOADS_DIR_NAME);
    let blob = uploads.join("any-upload-id.zip");
    let stale_job = root.join("some-job");
    fs::create_dir_all(&uploads).expect("create uploads dir");
    fs::write(&blob, b"zip-bytes").expect("write blob");
    fs::create_dir_all(&stale_job).expect("create stale job dir");

    // ttl == ZERO means every entry is "stale" by age — proving the upload dir is
    // skipped by NAME, not by freshness, while a real stale job dir is still reaped.
    cleanup_stale_remote_entries_with(
        root,
        &BTreeSet::new(),
        Duration::ZERO,
        remove_stale_remote_entry,
    )
    .expect("generic per-job sweep");

    assert!(
        uploads.exists() && blob.exists(),
        ".workspace-uploads (and its blobs) must be excluded from the whole-dir sweep"
    );
    assert!(
        !stale_job.exists(),
        "a genuinely stale job dir must still be reaped"
    );
}
