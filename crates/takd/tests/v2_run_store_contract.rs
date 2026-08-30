use rusqlite::Connection;
use tak_proto::local_daemon::v2::WorkspaceDisposition;
use takd::RunStore;

use crate::support::v2_run::{ARCHIVE, submission};

#[test]
fn submission_is_idempotent_content_checked_and_secret_safe() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let first = store
        .submit(&submission("idem", "secret-a"), "uid:1")
        .unwrap();
    let second = store
        .submit(&submission("idem", "secret-a"), "uid:1")
        .unwrap();

    assert_eq!(first.run_id, second.run_id);
    assert!(matches!(
        first.workspace,
        WorkspaceDisposition::UploadRequired { next_offset: 0 }
    ));
    let error = store
        .submit(&submission("idem", "secret-b"), "uid:1")
        .unwrap_err();
    assert!(
        error.to_string().contains("idempotency conflict"),
        "{error}"
    );
    assert!(!error.to_string().contains("secret-a") && !error.to_string().contains("secret-b"));
    drop(store);

    let restored = RunStore::with_db_path(db).unwrap();
    let listed = restored.list_runs().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].run_id, first.run_id);
    assert_eq!(listed[0].state.as_str(), "awaiting_workspace");
    assert!(!format!("{listed:?}").contains("secret"));
}

#[test]
fn resumable_upload_commit_and_events_survive_restart_atomically() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("takd.sqlite");
    let request = submission("resume", "secret");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let accepted = store.submit(&request, "uid:1").unwrap();
    let fingerprint = &request.run.workspace.manifest.fingerprint;
    let progress = store
        .upload_workspace(
            &accepted.run_id,
            fingerprint,
            ARCHIVE.len() as u64,
            0,
            &ARCHIVE[..2],
        )
        .unwrap();
    assert_eq!(progress.next_offset, 2);
    drop(store);

    let restored = RunStore::with_db_path(db.clone()).unwrap();
    let attached = restored.submit(&request, "uid:1").unwrap();
    assert!(matches!(
        attached.workspace,
        WorkspaceDisposition::UploadRequired { next_offset: 2 }
    ));
    restored
        .upload_workspace(
            &attached.run_id,
            fingerprint,
            ARCHIVE.len() as u64,
            2,
            &ARCHIVE[2..],
        )
        .unwrap();
    restored.commit(&attached.run_id).unwrap();
    let details = restored.get_run(&attached.run_id).unwrap().unwrap();
    assert_eq!(details.summary.state.as_str(), "queued");
    let events = restored.events_after(&attached.run_id, 0).unwrap();
    assert_eq!(
        events.iter().map(|event| event.seq).collect::<Vec<_>>(),
        [1, 2, 3]
    );

    let connection = Connection::open(db).unwrap();
    let outbox: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM run_outbox WHERE run_id = ?1 AND kind = 'scheduler_wakeup'",
            [&attached.run_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(outbox, 1);
}
