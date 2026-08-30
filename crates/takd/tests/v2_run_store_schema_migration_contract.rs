use rusqlite::Connection;
use takd::RunStore;

use crate::support::v2_run::submission;

#[test]
fn the_pre_scheduler_v2_schema_upgrades_in_place_before_submission() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let connection = Connection::open(&db).unwrap();
    connection.execute_batch(OLD_SCHEMA).unwrap();
    drop(connection);

    let store = RunStore::with_db_path(db.clone()).unwrap();
    store
        .submit(&submission("after-upgrade", "secret"), "uid:1")
        .unwrap();

    let connection = Connection::open(db).unwrap();
    for (table, column) in [
        ("runs", "max_parallel_jobs"),
        ("runs", "keep_going"),
        ("runs", "dispatch_stopped"),
        ("run_jobs", "current_fencing_token"),
    ] {
        let count: i64 = connection
            .query_row(
                &format!("SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name = ?1"),
                [column],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "missing {table}.{column}");
    }
}

pub(crate) const OLD_SCHEMA: &str = r#"
CREATE TABLE runs (
 run_id TEXT PRIMARY KEY, submitter_id TEXT NOT NULL, idempotency_key TEXT NOT NULL,
 request_digest TEXT NOT NULL, state TEXT NOT NULL, project_id TEXT NOT NULL,
 targets_json TEXT NOT NULL, resolved_json TEXT NOT NULL, workspace_fingerprint TEXT NOT NULL,
 archive_sha256 TEXT NOT NULL, archive_size INTEGER NOT NULL, upload_offset INTEGER NOT NULL DEFAULT 0,
 created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL,
 UNIQUE (submitter_id, idempotency_key)
);
CREATE TABLE run_jobs (
 run_id TEXT NOT NULL, job_id TEXT NOT NULL, ordinal INTEGER NOT NULL, state TEXT NOT NULL,
 definition_json TEXT NOT NULL, node_id TEXT, attempt INTEGER NOT NULL DEFAULT 0,
 PRIMARY KEY (run_id, job_id)
);
CREATE TABLE run_attempts (
 run_id TEXT NOT NULL, job_id TEXT NOT NULL, authored_attempt INTEGER NOT NULL,
 dispatch_generation INTEGER NOT NULL, fencing_token TEXT NOT NULL UNIQUE, node_id TEXT NOT NULL,
 state TEXT NOT NULL, cpu_millis INTEGER NOT NULL, memory_bytes INTEGER NOT NULL,
 execution_slots INTEGER NOT NULL, reserved_at_ms INTEGER NOT NULL, released_at_ms INTEGER,
 PRIMARY KEY (run_id, job_id, authored_attempt, dispatch_generation)
);
"#;
