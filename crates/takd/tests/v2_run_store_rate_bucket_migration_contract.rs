use rusqlite::Connection;
use takd::RunStore;

#[test]
fn v5_store_upgrades_to_durable_rate_bucket_schema_v6() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let connection = Connection::open(&db).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE run_schema_version (singleton INTEGER PRIMARY KEY, version INTEGER);\
             INSERT INTO run_schema_version VALUES (1, 5);",
        )
        .unwrap();
    drop(connection);

    let _store = RunStore::with_db_path(db.clone()).unwrap();
    let connection = Connection::open(db).unwrap();
    let version: i64 = connection
        .query_row(
            "SELECT version FROM run_schema_version WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(version, 6);
    let columns: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('scheduler_rate_buckets') \
             WHERE name IN ('available_micros','refilled_at_ms','refill_millis_per_second')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(columns, 3);
}
