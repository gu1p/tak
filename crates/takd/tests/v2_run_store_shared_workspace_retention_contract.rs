use takd::{RunStore, RunStoreMaintenanceConfig};

use crate::support::v2_shared_retention::{active, age, seed, terminal};

const DAY_MS: u64 = 24 * 60 * 60 * 1_000;
const NOW_MS: u64 = 40 * DAY_MS;

#[test]
fn maintenance_keeps_active_shared_and_unrelated_roots() {
    let (_temp, db) = state();
    let store = RunStore::with_db_path_and_maintenance(
        db.clone(),
        RunStoreMaintenanceConfig::default(),
        NOW_MS,
    )
    .unwrap();
    let blob_root = db.with_extension("v2-blobs");
    let (_run_id, shared) = active(&store, &blob_root, "active-shared-retention");
    let unrelated = blob_root.join("shared-workspaces/unrelated");
    seed(&shared);
    seed(&unrelated);

    store.run_maintenance_at(NOW_MS + 60 * DAY_MS).unwrap();

    assert!(shared.join("value").is_file());
    assert!(unrelated.join("value").is_file());
}

#[test]
fn payload_expiry_reclaims_terminal_shared_roots_on_sweep_and_startup() {
    let (_temp, db) = state();
    let config = RunStoreMaintenanceConfig::default();
    let store = RunStore::with_db_path_and_maintenance(db.clone(), config, NOW_MS).unwrap();
    let blob_root = db.with_extension("v2-blobs");
    let (swept_id, swept) = terminal(&store, &blob_root, "swept-shared-retention");
    let (startup_id, startup) = terminal(&store, &blob_root, "startup-shared-retention");
    let unrelated = blob_root.join("shared-workspaces/unrelated");
    for root in [&swept, &startup, &unrelated] {
        seed(root);
    }
    age(&db, &swept_id, NOW_MS - 8 * DAY_MS);

    store.run_maintenance_at(NOW_MS).unwrap();
    assert!(!swept.exists() && startup.exists() && unrelated.exists());

    age(&db, &startup_id, NOW_MS - 8 * DAY_MS);
    drop(store);
    let _restarted = RunStore::with_db_path_and_maintenance(db, config, NOW_MS).unwrap();
    assert!(!startup.exists() && unrelated.exists());
}

fn state() -> (tempfile::TempDir, std::path::PathBuf) {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    (temp, db)
}
