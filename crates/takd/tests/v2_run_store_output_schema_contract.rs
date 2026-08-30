use rusqlite::Connection;
use takd::RunStore;

use super::v2_run_store_schema_migration_contract::OLD_SCHEMA;

#[test]
fn v6_store_upgrades_to_durable_attempt_and_final_output_tables() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let connection = Connection::open(&db).unwrap();
    connection.execute_batch(OLD_SCHEMA).unwrap();
    connection.execute_batch(
        "CREATE TABLE run_schema_version (singleton INTEGER PRIMARY KEY, version INTEGER NOT NULL); \
         INSERT INTO run_schema_version VALUES (1, 6);",
    )
    .unwrap();
    drop(connection);

    RunStore::with_db_path(db.clone()).unwrap();
    let connection = Connection::open(db).unwrap();
    let version: i64 = connection
        .query_row("SELECT version FROM run_schema_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 7);
    for table in ["run_attempt_outputs", "run_final_outputs"] {
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                [table],
                |row| row.get(0),
            )
            .unwrap();
        assert!(exists, "missing {table}");
    }
}
