use rusqlite::{Connection, params};
use tak_proto::local_daemon::v2::{RunEvent, RunEventKind, RunLifecycleState};
use takd::RunStore;

use crate::support::v2_run::submission;

#[test]
fn attachment_pages_are_bounded_and_terminal_only_after_the_last_event() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let accepted = store
        .submit(&submission("event-pages", "secret"), "uid:1")
        .unwrap();
    let connection = Connection::open(db).unwrap();
    connection
        .execute(
            "UPDATE runs SET state = 'succeeded' WHERE run_id = ?1",
            [&accepted.run_id],
        )
        .unwrap();
    for seq in 2..=302_u64 {
        let event = RunEvent {
            seq,
            kind: RunEventKind::Running,
            job_id: None,
            task_ids: vec![],
            node_id: None,
            message: "event".into(),
            chunk_base64: None,
        };
        connection
            .execute(
                "INSERT INTO run_events (run_id, seq, payload_json, created_at_ms) VALUES (?1, ?2, ?3, 1)",
                params![
                    accepted.run_id,
                    i64::try_from(seq).unwrap(),
                    serde_json::to_string(&event).unwrap()
                ],
            )
            .unwrap();
    }

    let (summary, first, has_more) = store
        .attachment_snapshot(&accepted.run_id, 0)
        .unwrap()
        .unwrap();
    assert_eq!(summary.state, RunLifecycleState::Succeeded);
    assert_eq!(first.len(), 256);
    assert!(has_more);
    let (_, second, has_more) = store
        .attachment_snapshot(&accepted.run_id, first.last().unwrap().seq)
        .unwrap()
        .unwrap();
    assert_eq!(second.len(), 46);
    assert!(!has_more);
}
