use std::fs;
use std::time::SystemTime;

use super::*;

#[test]
fn shared_workspace_activity_refreshes_its_storage_parent() {
    assert_storage_parent_refreshed(
        RemoteWorkerSessionReuse::ShareWorkspace,
        SESSION_WORKSPACES_DIR_NAME,
    );
}

#[test]
fn shared_paths_activity_refreshes_its_storage_parent() {
    assert_storage_parent_refreshed(
        RemoteWorkerSessionReuse::SharePaths { paths: Vec::new() },
        SESSION_PATHS_DIR_NAME,
    );
}

fn assert_storage_parent_refreshed(reuse: RemoteWorkerSessionReuse, directory_name: &str) {
    let temp = tempfile::tempdir().expect("tempdir");
    let storage_parent = temp.path().join(directory_name);
    fs::create_dir(&storage_parent).expect("create storage parent");
    fs::File::open(&storage_parent)
        .expect("open storage parent")
        .set_modified(SystemTime::UNIX_EPOCH)
        .expect("backdate storage parent");
    let baseline = fs::metadata(&storage_parent)
        .expect("backdated storage parent metadata")
        .modified()
        .expect("backdated storage parent mtime");
    let session = RemoteWorkerSession {
        key: "run-session".to_string(),
        reuse,
    };

    refresh_session_storage_parent(temp.path(), Some(&session)).expect("refresh storage parent");

    let modified = fs::metadata(&storage_parent)
        .expect("storage parent metadata")
        .modified()
        .expect("storage parent mtime");
    assert!(modified > baseline);
}

#[test]
fn completion_refreshes_only_the_resolved_root_and_unregisters() {
    let temp = tempfile::tempdir().expect("tempdir");
    let resolved = temp.path().join("stored-root");
    let fallback = temp.path().join("fallback-root");
    let resolved_parent = resolved.join(SESSION_WORKSPACES_DIR_NAME);
    let fallback_parent = fallback.join(SESSION_WORKSPACES_DIR_NAME);
    fs::create_dir_all(&resolved_parent).expect("create resolved session parent");
    fs::create_dir_all(&fallback_parent).expect("create fallback session parent");
    let resolved_baseline = backdate(&resolved_parent);
    let fallback_baseline = backdate(&fallback_parent);
    let context = RemoteNodeContext::isolated_for_test();
    let _cancellation = context
        .register_active_execution("submit-1".into(), "run-1", 1)
        .expect("register active execution");
    let session = RemoteWorkerSession {
        key: "run-session".to_string(),
        reuse: RemoteWorkerSessionReuse::ShareWorkspace,
    };

    finish_remote_worker_submit(&context, "submit-1", Some(&resolved), Some(&session));

    assert!(mtime(&resolved_parent) > resolved_baseline);
    assert_eq!(mtime(&fallback_parent), fallback_baseline);
    assert!(!context.cancel_active_task("run-1", Some(1)).unwrap());
}

fn backdate(path: &Path) -> SystemTime {
    fs::File::open(path)
        .expect("open path")
        .set_modified(SystemTime::UNIX_EPOCH)
        .expect("backdate path");
    mtime(path)
}

fn mtime(path: &Path) -> SystemTime {
    fs::metadata(path).unwrap().modified().unwrap()
}
