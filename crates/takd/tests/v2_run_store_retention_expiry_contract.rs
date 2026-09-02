use rusqlite::params;
use tak_core::v2::WorkspaceEntry;
use tak_proto::local_daemon::v2::RunEventKind;
use takd::{AttemptCompletion, AttemptOutputStream, RunStore, SchedulerNode};

use crate::support::v2_run::{scheduler::commit, submission};

#[test]
fn expired_run_payloads_keep_structured_state_without_serving_logs_or_outputs() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let run_id = commit(&store, &submission("retention-expiry", "secret"), "alice");
    let command = store
        .reserve_next(&[SchedulerNode::with_execution_slots("local", 1)])
        .unwrap()
        .unwrap();
    assert!(store.expire_run_payloads(&run_id).is_err());
    store
        .append_attempt_output(
            &command,
            "//:check",
            AttemptOutputStream::Stdout,
            b"secret-log",
        )
        .unwrap();
    store
        .complete_attempt(
            &command,
            AttemptCompletion::Succeeded {
                terminal_digest: "a".repeat(64),
            },
        )
        .unwrap();
    seed_output(&db, &run_id, &command.fencing_token);
    assert_eq!(
        store
            .output_manifest_status(&run_id)
            .unwrap()
            .unwrap()
            .artifacts
            .len(),
        1
    );

    store.expire_run_payloads(&run_id).unwrap();

    let details = store.get_run(&run_id).unwrap().unwrap();
    assert!(details.logs_expired && details.outputs_expired);
    assert_eq!(details.jobs.len(), 1);
    let events = store.events_after(&run_id, 0).unwrap();
    assert!(
        events
            .iter()
            .all(|event| !matches!(event.kind, RunEventKind::Stdout | RunEventKind::Stderr))
    );
    let attached = store.attachment_snapshot(&run_id, 0).unwrap().unwrap();
    assert!(attached.logs_expired);
    assert!(
        attached
            .events
            .iter()
            .all(|event| event.chunk_base64.is_none())
    );
    assert!(attached.next_event > 0);
    let outputs = store.output_manifest_status(&run_id).unwrap().unwrap();
    assert!(outputs.expired);
    assert!(outputs.artifacts.is_empty());
    assert!(store.output_chunk("artifact", 0, 8).unwrap().is_none());
    let connection = rusqlite::Connection::open(db).unwrap();
    let retained: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM run_attempt_outputs WHERE run_id=?1",
            [&run_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        retained, 0,
        "expired output rows must be physically removed"
    );
}

fn seed_output(db: &std::path::Path, run_id: &str, fence: &str) {
    let entry = WorkspaceEntry::file("result.txt", false, 6, &"a".repeat(64)).unwrap();
    let connection = rusqlite::Connection::open(db).unwrap();
    connection
        .execute(
            "INSERT INTO run_attempt_outputs VALUES (?1,?2,'//:check','result.txt','artifact',?3)",
            params![run_id, fence, serde_json::to_string(&entry).unwrap()],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO run_final_outputs VALUES (?1,'result.txt','artifact','//:check')",
            [run_id],
        )
        .unwrap();
}
