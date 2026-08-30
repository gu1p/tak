use tak_proto::local_daemon::v2::WorkspaceDisposition;
use takd::RunStore;

use crate::support::v2_run::{ARCHIVE, submission};

#[test]
fn a_missing_blob_file_is_repaired_by_the_replacement_upload() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let first = submission("first", "secret");
    upload_all(&store, &first, "uid:1");
    let blob = db
        .with_extension("v2-blobs")
        .join("workspaces")
        .join(format!("{}.tar", first.run.workspace.manifest.fingerprint));
    std::fs::remove_file(blob).unwrap();

    let replacement = submission("replacement", "secret");
    let accepted = store.submit(&replacement, "uid:1").unwrap();
    assert!(matches!(
        accepted.workspace,
        WorkspaceDisposition::UploadRequired { .. }
    ));
    store
        .upload_workspace(
            &accepted.run_id,
            &replacement.run.workspace.manifest.fingerprint,
            ARCHIVE.len() as u64,
            0,
            ARCHIVE.as_slice(),
        )
        .unwrap();
    assert_eq!(
        store.commit(&accepted.run_id).unwrap().state.as_str(),
        "queued"
    );
}

#[test]
fn a_partial_run_adopts_a_concurrently_published_matching_blob() {
    let temp = tempfile::tempdir().unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let partial = submission("partial", "secret");
    let first = store.submit(&partial, "uid:1").unwrap();
    store
        .upload_workspace(
            &first.run_id,
            &partial.run.workspace.manifest.fingerprint,
            ARCHIVE.len() as u64,
            0,
            &ARCHIVE[..16],
        )
        .unwrap();
    upload_all(&store, &submission("publisher", "secret"), "uid:1");

    let adopted = store.submit(&partial, "uid:1").unwrap();
    assert_eq!(adopted.workspace, WorkspaceDisposition::Present);
    assert_eq!(
        store.commit(&first.run_id).unwrap().state.as_str(),
        "queued"
    );
}

fn upload_all(store: &RunStore, request: &tak_core::v2::RunSubmission, owner: &str) {
    let accepted = store.submit(request, owner).unwrap();
    store
        .upload_workspace(
            &accepted.run_id,
            &request.run.workspace.manifest.fingerprint,
            ARCHIVE.len() as u64,
            0,
            ARCHIVE.as_slice(),
        )
        .unwrap();
}
