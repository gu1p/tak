use rusqlite::Connection;
use takd::{AttemptCompletion, DispatchCommand, ResultAcceptance, RunStore};

use crate::support::v2_run::scheduler::independent_jobs;

use super::v2_run_store_schema_migration_contract::OLD_SCHEMA;

#[test]
fn migration_restores_options_and_the_active_attempt_fence() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let mut request = independent_jobs("active-upgrade", 3);
    request.run.options.keep_going = true;
    let connection = Connection::open(&db).unwrap();
    connection.execute_batch(OLD_SCHEMA).unwrap();
    connection.execute(
        "INSERT INTO runs (run_id, submitter_id, idempotency_key, request_digest, state, project_id, targets_json, resolved_json, workspace_fingerprint, archive_sha256, archive_size, upload_offset, created_at_ms, updated_at_ms) VALUES ('run-old', 'owner', 'key', 'digest', 'running', 'project', '[]', ?1, 'workspace', 'archive', 0, 0, 10, 20)",
        [serde_json::to_string(&request.run).unwrap()],
    ).unwrap();
    connection.execute(
        "INSERT INTO run_jobs (run_id, job_id, ordinal, state, definition_json, node_id, attempt) VALUES ('run-old', 'job-0', 0, 'transferring', ?1, 'worker-a', 1)",
        [serde_json::to_string(&request.run.jobs[0]).unwrap()],
    ).unwrap();
    connection.execute(
        "INSERT INTO run_attempts (run_id, job_id, authored_attempt, dispatch_generation, fencing_token, node_id, state, cpu_millis, memory_bytes, execution_slots, reserved_at_ms) VALUES ('run-old', 'job-0', 1, 7, 'fence', 'worker-a', 'transferring', 0, 0, 1, 20)",
        [],
    ).unwrap();
    drop(connection);

    let store = RunStore::with_db_path(db.clone()).unwrap();
    let connection = Connection::open(db).unwrap();
    let options = connection.query_row(
        "SELECT max_parallel_jobs, keep_going, dispatch_generation, current_fencing_token FROM runs JOIN run_jobs USING (run_id) WHERE run_id = 'run-old'",
        [],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, bool>(1)?, row.get::<_, i64>(2)?, row.get::<_, String>(3)?)),
    ).unwrap();
    assert_eq!(options, (3, true, 7, "fence".into()));
    drop(connection);
    assert_eq!(
        store.ack_dispatch(&command()).unwrap(),
        ResultAcceptance::Applied
    );
    let completion = AttemptCompletion::Succeeded {
        terminal_digest: "e".repeat(64),
    };
    assert_eq!(
        store.complete_attempt(&command(), completion).unwrap(),
        ResultAcceptance::Applied
    );
}

#[test]
fn a_newer_schema_version_is_rejected() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let connection = Connection::open(&db).unwrap();
    connection.execute_batch(OLD_SCHEMA).unwrap();
    connection.execute_batch("CREATE TABLE run_schema_version (singleton INTEGER PRIMARY KEY, version INTEGER NOT NULL); INSERT INTO run_schema_version VALUES (1, 99);").unwrap();
    drop(connection);
    let error = match RunStore::with_db_path(db) {
        Ok(_) => panic!("newer schema was accepted"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("newer"));
}

fn command() -> DispatchCommand {
    DispatchCommand {
        run_id: "run-old".into(),
        job_id: "job-0".into(),
        node_id: "worker-a".into(),
        authored_attempt: 1,
        dispatch_generation: 7,
        fencing_token: "fence".into(),
    }
}
