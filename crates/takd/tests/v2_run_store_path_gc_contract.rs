use takd::{RunStore, RunStoreMaintenanceConfig};

#[test]
fn path_blob_gc_uses_the_same_lru_budget_as_workspace_blobs() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let config = RunStoreMaintenanceConfig {
        workspace_path_blob_budget_bytes: 4,
        ..RunStoreMaintenanceConfig::default()
    };
    let store = RunStore::with_db_path_and_maintenance(db.clone(), config, 100).unwrap();
    let root = db.with_extension("v2-blobs").join("path-caches");
    let old = seed_path(&root, "old", 1, b"old!");
    let recent = seed_path(&root, "recent", 2, b"new!");

    let report = store.run_maintenance_at(100).unwrap();

    assert_eq!(report.evicted_workspace_path_blobs, 1);
    assert!(!old.exists());
    assert!(recent.exists());
}

fn seed_path(root: &std::path::Path, key: &str, accessed: u64, data: &[u8]) -> std::path::PathBuf {
    let path = root.join(key);
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(path.join("value"), data).unwrap();
    std::fs::write(path.join(".last-accessed-ms"), accessed.to_string()).unwrap();
    path
}
