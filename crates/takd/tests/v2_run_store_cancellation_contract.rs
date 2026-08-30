use tak_proto::local_daemon::v2::{RunEventKind, RunLifecycleState};
use takd::RunStore;

use crate::support::v2_run::{ARCHIVE, submission};

#[test]
fn cancelling_precommit_work_is_immediately_terminal_and_discards_its_partial_upload() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let request = submission("cancel", "secret");
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

    assert_eq!(
        store.cancel(&accepted.run_id).unwrap(),
        RunLifecycleState::Cancelled
    );
    assert_eq!(
        store.summary(&accepted.run_id).unwrap().unwrap().state,
        RunLifecycleState::Cancelled
    );
    assert_eq!(
        store
            .events_after(&accepted.run_id, 0)
            .unwrap()
            .last()
            .unwrap()
            .kind,
        RunEventKind::Cancelled
    );
    let partial = db
        .with_extension("v2-blobs")
        .join("uploads")
        .join(format!("{}.part", accepted.run_id));
    assert!(!partial.exists());
}
