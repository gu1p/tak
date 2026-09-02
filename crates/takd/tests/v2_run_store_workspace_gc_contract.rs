use sha2::{Digest, Sha256};
use tak_proto::local_daemon::v2::WorkspaceDisposition;
use takd::{RunStore, RunStoreMaintenanceConfig, SchedulerNode};

use crate::support::v2_run::{ARCHIVE, path_cache, submission, submission_with_spec};

#[test]
fn workspace_gc_never_evicts_active_leased_shared_or_in_transfer_data() {
    let (_temp, db) = state();
    let config = RunStoreMaintenanceConfig {
        workspace_path_blob_budget_bytes: 0,
        ..RunStoreMaintenanceConfig::default()
    };
    let store = RunStore::with_db_path_and_maintenance(db.clone(), config, 100).unwrap();
    let partial = store
        .submit(&submission("partial", "secret"), "alice")
        .unwrap();
    store
        .upload_workspace(
            &partial.run_id,
            &submission("x", "s").run.workspace.manifest.fingerprint,
            ARCHIVE.len() as u64,
            0,
            &ARCHIVE[..2],
        )
        .unwrap();
    let (active, active_archive) = submission_with_spec("active", "secret", b"active");
    let accepted = store.submit(&active, "alice").unwrap();
    assert!(matches!(
        accepted.workspace,
        WorkspaceDisposition::UploadRequired { .. }
    ));
    store
        .upload_workspace(
            &accepted.run_id,
            &active.run.workspace.manifest.fingerprint,
            active_archive.len() as u64,
            0,
            &active_archive,
        )
        .unwrap();
    let mut leased = path_cache::dependent_run("leased");
    leased.run.workspace = active.run.workspace.clone();
    let leased_id = store.submit(&leased, "alice").unwrap().run_id;
    store.commit(&leased_id).unwrap();
    store
        .reserve_next(&[SchedulerNode::with_execution_slots("local", 1)])
        .unwrap()
        .unwrap();
    let roots = db.with_extension("v2-blobs");
    let workspace = roots
        .join("workspaces")
        .join(format!("{}.tar", active.run.workspace.manifest.fingerprint));
    let path_key = serde_json::to_vec(&(&leased_id, "compiler", "local")).unwrap();
    let leased_path = roots
        .join("path-caches")
        .join(format!("{:x}", Sha256::digest(path_key)));
    std::fs::create_dir_all(&leased_path).unwrap();
    std::fs::write(leased_path.join("value"), b"leased").unwrap();
    let shared = roots.join("shared-workspaces/session/value");
    std::fs::create_dir_all(shared.parent().unwrap()).unwrap();
    std::fs::write(&shared, b"shared").unwrap();
    let partial_upload = roots
        .join("uploads")
        .join(format!("{}.part", partial.run_id));

    store.run_maintenance_at(100).unwrap();

    assert!(workspace.exists() && leased_path.exists());
    assert!(shared.exists() && partial_upload.exists());
}

fn state() -> (tempfile::TempDir, std::path::PathBuf) {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    (temp, db)
}
