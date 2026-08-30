use rusqlite::Connection;
use tak_proto::local_daemon::v2::RunLifecycleState;
use takd::{ResultAcceptance, RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn schema_upgrade_backfills_each_unreleased_cancelling_attempt() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let run_id = commit(&store, &independent_jobs("cancel-upgrade", 1), "uid:1");
    store
        .reserve_next(&[SchedulerNode::with_execution_slots("worker-a", 1)])
        .unwrap()
        .unwrap();
    store.cancel(&run_id).unwrap();
    drop(store);
    let connection = Connection::open(&db).unwrap();
    connection
        .execute("DROP TABLE run_cancel_outbox", [])
        .unwrap();
    connection
        .execute(
            "INSERT OR REPLACE INTO run_outbox (run_id, kind, payload_json) VALUES (?1, 'cancel_run', '{}')",
            [&run_id],
        )
        .unwrap();
    connection
        .execute("UPDATE run_schema_version SET version = 2", [])
        .unwrap();
    drop(connection);

    let restored = RunStore::with_db_path(db).unwrap();
    let commands = restored.pending_cancellations().unwrap();
    assert_eq!(commands.len(), 1);
    let connection = Connection::open(temp.path().join("takd.sqlite")).unwrap();
    let legacy: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM run_outbox WHERE kind = 'cancel_run'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(legacy, 0);
    drop(connection);
    assert_eq!(
        restored.ack_cancellation(&commands[0]).unwrap(),
        ResultAcceptance::Applied
    );
    assert_eq!(
        restored.summary(&run_id).unwrap().unwrap().state,
        RunLifecycleState::Cancelled
    );
}
