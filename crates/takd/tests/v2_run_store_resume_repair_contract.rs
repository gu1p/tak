use tak_proto::local_daemon::v2::WorkspaceDisposition;
use takd::RunStore;

use crate::support::v2_run::{ARCHIVE, submission};

#[test]
fn a_missing_partial_upload_resets_the_same_run_to_offset_zero() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let request = submission("resume-partial", "secret");
    let accepted = store.submit(&request, "uid:1").unwrap();
    store
        .upload_workspace(
            &accepted.run_id,
            &request.run.workspace.manifest.fingerprint,
            ARCHIVE.len() as u64,
            0,
            &ARCHIVE[..16],
        )
        .unwrap();
    std::fs::remove_file(
        db.with_extension("v2-blobs")
            .join("uploads")
            .join(format!("{}.part", accepted.run_id)),
    )
    .unwrap();

    let resumed = store.submit(&request, "uid:1").unwrap();
    assert_eq!(
        resumed.workspace,
        WorkspaceDisposition::UploadRequired { next_offset: 0 }
    );
    upload_and_commit(&store, &accepted.run_id, &request);
}

#[test]
fn a_missing_completed_blob_reopens_the_same_run_for_upload() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let request = submission("resume-complete", "secret");
    let accepted = store.submit(&request, "uid:1").unwrap();
    upload_and_commit_workspace(&store, &accepted.run_id, &request);
    std::fs::remove_file(
        db.with_extension("v2-blobs")
            .join("workspaces")
            .join(format!(
                "{}.tar",
                request.run.workspace.manifest.fingerprint
            )),
    )
    .unwrap();

    let resumed = store.submit(&request, "uid:1").unwrap();
    assert_eq!(
        resumed.workspace,
        WorkspaceDisposition::UploadRequired { next_offset: 0 }
    );
    upload_and_commit(&store, &accepted.run_id, &request);
}

fn upload_and_commit(store: &RunStore, run_id: &str, request: &tak_core::v2::RunSubmission) {
    upload_and_commit_workspace(store, run_id, request);
    assert_eq!(store.commit(run_id).unwrap().state.as_str(), "queued");
}

fn upload_and_commit_workspace(
    store: &RunStore,
    run_id: &str,
    request: &tak_core::v2::RunSubmission,
) {
    store
        .upload_workspace(
            run_id,
            &request.run.workspace.manifest.fingerprint,
            ARCHIVE.len() as u64,
            0,
            ARCHIVE.as_slice(),
        )
        .unwrap();
}
