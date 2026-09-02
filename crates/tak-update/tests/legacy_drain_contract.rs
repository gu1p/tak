use rusqlite::Connection;
use tak_update::legacy_drain::ensure_legacy_attempts_drained;

fn create_legacy_store() -> (tempfile::TempDir, Connection) {
    std::fs::create_dir_all(".tmp").expect("create test temp root");
    let temp = tempfile::tempdir_in(".tmp").expect("tempdir");
    let connection = Connection::open(temp.path().join("agent.sqlite")).expect("legacy db");
    connection
        .execute_batch(
            "CREATE TABLE submit_attempts (idempotency_key TEXT PRIMARY KEY);
             CREATE TABLE submit_results (idempotency_key TEXT PRIMARY KEY);",
        )
        .expect("legacy schema");
    (temp, connection)
}

#[test]
fn active_legacy_attempts_block_binary_replacement() {
    let (temp, connection) = create_legacy_store();
    connection
        .execute("INSERT INTO submit_attempts VALUES ('active')", [])
        .expect("active attempt");

    let error = ensure_legacy_attempts_drained(temp.path())
        .expect_err("active legacy attempt must block replacement");

    assert!(
        error
            .to_string()
            .contains("active legacy attempts must finish")
    );
}

#[test]
fn completed_legacy_history_remains_read_only_without_blocking() {
    let (temp, connection) = create_legacy_store();
    connection
        .execute("INSERT INTO submit_attempts VALUES ('done')", [])
        .expect("attempt");
    connection
        .execute("INSERT INTO submit_results VALUES ('done')", [])
        .expect("result");
    drop(connection);

    ensure_legacy_attempts_drained(temp.path()).expect("completed history is allowed");
}
