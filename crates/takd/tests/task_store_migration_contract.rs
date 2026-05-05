use rusqlite::{Connection, params};
use takd::daemon::remote::SubmitAttemptStore;

#[test]
fn submit_store_migrates_legacy_attempt_rows_without_task_labels() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("agent.sqlite");
    let conn = Connection::open(&db_path).expect("open sqlite");
    conn.execute_batch(
        "
        CREATE TABLE submit_attempts (
            idempotency_key TEXT PRIMARY KEY,
            task_run_id TEXT NOT NULL,
            attempt INTEGER NOT NULL,
            selected_node_id TEXT NOT NULL,
            created_at_ms INTEGER NOT NULL
        );
        CREATE TABLE submit_events (
            idempotency_key TEXT NOT NULL,
            seq INTEGER NOT NULL,
            payload_json TEXT NOT NULL,
            PRIMARY KEY (idempotency_key, seq)
        );
        CREATE TABLE submit_results (
            idempotency_key TEXT PRIMARY KEY,
            payload_json TEXT NOT NULL
        );
        ",
    )
    .expect("legacy schema");
    conn.execute(
        "
        INSERT INTO submit_attempts (
            idempotency_key, task_run_id, attempt, selected_node_id, created_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5)
        ",
        params!["legacy-key", "legacy-run", 1_i64, "node-a", 10_i64],
    )
    .expect("legacy row");
    drop(conn);

    let store = SubmitAttemptStore::with_db_path(db_path).expect("migrate store");
    let active = store.active_attempts().expect("active attempts");

    assert_eq!(active.len(), 1);
    assert_eq!(active[0].task_run_id, "legacy-run");
    assert_eq!(active[0].task_label, "");
}
