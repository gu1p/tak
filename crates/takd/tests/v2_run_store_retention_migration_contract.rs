use rusqlite::Connection;
use takd::RunStore;

use super::v2_run_store_schema_migration_contract::OLD_SCHEMA;

#[test]
fn existing_runs_gain_nonexpired_payload_state_during_schema_upgrade() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let connection = Connection::open(&db).unwrap();
    connection.execute_batch(OLD_SCHEMA).unwrap();
    connection
        .execute(
            "ALTER TABLE runs ADD COLUMN max_parallel_jobs INTEGER NOT NULL DEFAULT 1",
            [],
        )
        .unwrap();
    drop(connection);

    RunStore::with_db_path(db.clone()).unwrap();

    let connection = Connection::open(db).unwrap();
    for column in ["logs_expired", "outputs_expired"] {
        let declaration: (i64, String) = connection
            .query_row(
                "SELECT \"notnull\",dflt_value FROM pragma_table_info('runs') WHERE name=?1",
                [column],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(declaration, (1, "0".into()));
    }
    let version: i64 = connection
        .query_row("SELECT version FROM run_schema_version", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(version, 14);
}
