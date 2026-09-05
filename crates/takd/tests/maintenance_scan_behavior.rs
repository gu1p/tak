use std::io::Write;

use takd::{AttemptCompletion, RunStore, RunStoreMaintenanceConfig, SchedulerNode};

use crate::support::{
    maintenance_scan::pause_scan,
    v2_run::{scheduler::commit, submission},
};

#[test]
fn slow_cache_scan_allows_cancellation() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let run_id = commit(&store, &submission("cancel-during-scan", "secret"), "alice");
    let (mut barrier, maintenance) = pause_scan(store.clone(), &db);

    let cancelled = store.cancel(&run_id);

    barrier.write_all(b"0").unwrap();
    drop(barrier);
    maintenance.join().unwrap().unwrap();
    assert!(
        cancelled.is_ok(),
        "cache scanning must not block cancellation: {cancelled:?}"
    );
}

#[test]
fn cache_gc_preserves_workspace_reactivated_during_scan() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("takd.sqlite");
    let config = RunStoreMaintenanceConfig {
        workspace_path_blob_budget_bytes: 0,
        ..RunStoreMaintenanceConfig::default()
    };
    let store = RunStore::with_db_path_and_maintenance(db.clone(), config, 100).unwrap();
    commit(&store, &submission("first", "secret"), "alice");
    let attempt = store
        .reserve_next(&[SchedulerNode::with_execution_slots("local", 1)])
        .unwrap()
        .unwrap();
    store
        .complete_attempt(
            &attempt,
            AttemptCompletion::Succeeded {
                terminal_digest: "a".repeat(64),
            },
        )
        .unwrap();
    let request = submission("reactivated", "secret");
    let workspace = db
        .with_extension("v2-blobs")
        .join("workspaces")
        .join(format!(
            "{}.tar",
            request.run.workspace.manifest.fingerprint
        ));
    let (mut barrier, maintenance) = pause_scan(store.clone(), &db);

    let submitted = store.submit(&request, "alice");

    barrier.write_all(b"0").unwrap();
    drop(barrier);
    maintenance.join().unwrap().unwrap();
    assert!(
        submitted.is_ok(),
        "cache scanning must not block submissions: {submitted:?}"
    );
    assert!(
        workspace.is_file(),
        "GC must recheck protection before eviction"
    );
}
