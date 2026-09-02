use takd::{RunStore, RunStoreMaintenanceConfig};

#[test]
fn workspace_gc_is_lru_with_a_configurable_budget() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let config = RunStoreMaintenanceConfig {
        workspace_path_blob_budget_bytes: 4,
        ..RunStoreMaintenanceConfig::default()
    };
    let store = RunStore::with_db_path_and_maintenance(db.clone(), config, 100).unwrap();
    let old = seed_blob(&db, "old", 1, b"old!");
    let recent = seed_blob(&db, "recent", 2, b"new!");

    let report = store.run_maintenance_at(100).unwrap();

    assert_eq!(report.evicted_workspace_path_blobs, 1);
    assert!(!old.exists());
    assert!(recent.exists());
}

fn seed_blob(db: &std::path::Path, key: &str, accessed: u64, bytes: &[u8]) -> std::path::PathBuf {
    let path = db
        .with_extension("v2-blobs")
        .join("workspaces")
        .join(format!("{key}.tar"));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, bytes).unwrap();
    rusqlite::Connection::open(db)
        .unwrap()
        .execute(
            "INSERT INTO workspace_blobs VALUES (?1,?1,?2,?3,?4)",
            rusqlite::params![
                key,
                bytes.len() as i64,
                path.display().to_string(),
                accessed as i64
            ],
        )
        .unwrap();
    path
}
