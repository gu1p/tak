use tak_core::v2::RunSubmission;
use tak_proto::local_daemon::v2::WorkspaceDisposition;
use takd::RunStore;

use crate::support::v2_run::{ARCHIVE, submission};

#[test]
fn idempotency_keys_are_submitter_scoped() {
    let temp = tempfile::tempdir().unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let request = submission("same-key", "secret");

    let first = store.submit(&request, "uid:1").unwrap();
    let second = store.submit(&request, "uid:2").unwrap();

    assert_ne!(first.run_id, second.run_id);
}

#[test]
fn completed_upload_retries_and_fingerprint_reuse_do_not_reupload() {
    let temp = tempfile::tempdir().unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let request = submission("first", "secret");
    let accepted = store.submit(&request, "uid:1").unwrap();
    let fingerprint = &request.run.workspace.manifest.fingerprint;

    let finished = store
        .upload_workspace(
            &accepted.run_id,
            fingerprint,
            ARCHIVE.len() as u64,
            0,
            ARCHIVE.as_slice(),
        )
        .unwrap();
    assert!(finished.complete);
    let repeated = store
        .upload_workspace(
            &accepted.run_id,
            fingerprint,
            ARCHIVE.len() as u64,
            0,
            ARCHIVE.as_slice(),
        )
        .unwrap();
    assert!(repeated.complete && repeated.next_offset == ARCHIVE.len() as u64);

    let mut different_archive = submission("second", "secret");
    different_archive.run.workspace.archive_sha256 = "f".repeat(64);
    different_archive.run.workspace.archive_size += 1;
    let different_archive = RunSubmission::new(
        different_archive.idempotency_key,
        different_archive.run,
        different_archive.environment_values,
    )
    .unwrap();
    let reused = store.submit(&different_archive, "uid:1").unwrap();
    assert_eq!(reused.workspace, WorkspaceDisposition::Present);
}
