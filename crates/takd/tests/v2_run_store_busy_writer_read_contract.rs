use rusqlite::{Connection, TransactionBehavior};
use takd::RunStore;

use crate::support::v2_run::submission;

#[test]
fn summary_remains_available_while_a_wal_writer_is_active() {
    let temp = tempfile::tempdir().unwrap();
    let database = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(database.clone()).unwrap();
    let run_id = store
        .submit(&submission("busy-writer-read", "secret"), "alice")
        .unwrap()
        .run_id;
    let mut writer = Connection::open(database).unwrap();
    let transaction = writer
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    transaction
        .execute(
            "UPDATE runs SET updated_at_ms = updated_at_ms + 1 WHERE run_id = ?1",
            [&run_id],
        )
        .unwrap();

    let summary = store
        .summary(&run_id)
        .expect("a WAL writer must not block opening a summary connection");
    let attachment = store
        .attachment_snapshot(&run_id, 0)
        .expect("a WAL writer must not block opening an attachment connection");

    assert!(summary.is_some());
    assert!(attachment.is_some());
}
