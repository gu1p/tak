use takd::{RunStore, RunStoreMaintenanceConfig};

#[test]
fn origin_gc_counts_and_evicts_an_extracted_base_with_its_archive() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let config = RunStoreMaintenanceConfig {
        workspace_path_blob_budget_bytes: 4,
        ..RunStoreMaintenanceConfig::default()
    };
    let store = RunStore::with_db_path_and_maintenance(db.clone(), config, 100).unwrap();
    let blob_root = db.with_extension("v2-blobs");
    let archive = blob_root.join("workspaces/fingerprint.tar");
    std::fs::create_dir_all(archive.parent().unwrap()).unwrap();
    std::fs::write(&archive, b"four").unwrap();
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute(
            "INSERT INTO workspace_blobs VALUES ('fingerprint','digest',4,?1,1)",
            [archive.display().to_string()],
        )
        .unwrap();
    let base = blob_root.join("workspace-bases/fingerprint");
    std::fs::create_dir_all(base.join("data")).unwrap();
    std::fs::write(base.join("data/value"), b"base").unwrap();
    std::fs::write(base.join("ready"), b"v2\n").unwrap();

    let report = store.run_maintenance_at(100).unwrap();

    assert_eq!(report.evicted_workspace_path_blobs, 1);
    assert!(!archive.exists());
    assert!(!base.exists());
}
