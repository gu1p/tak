use std::{
    fs,
    path::Path,
    thread,
    time::{Duration, Instant, SystemTime},
};

use super::*;

#[path = "completion_root_tests_fixture.rs"]
mod fixture;

use fixture::blocking_payload;

#[test]
fn final_refresh_uses_resolved_root_after_terminal_persistence() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db = temp.path().join("agent.sqlite");
    let fallback = temp.path().join("fallback");
    let resolved = temp.path().join("resolved");
    let fallback_parent = fallback.join(SESSION_WORKSPACES_DIR_NAME);
    let resolved_parent = resolved.join(SESSION_WORKSPACES_DIR_NAME);
    fs::create_dir_all(&fallback_parent).expect("create fallback sessions");
    fs::create_dir_all(&resolved_parent).expect("create resolved sessions");
    let fallback_baseline = backdate(&fallback_parent);
    let store = SubmitAttemptStore::with_db_path(db.clone()).expect("store");
    let run = "resolved-root-run";
    store
        .register_submit_with_execution_root_base(
            run,
            Some(1),
            "//:check",
            None,
            "builder-a",
            &resolved,
        )
        .expect("register submit");
    let key = build_submit_idempotency_key(run, Some(1)).expect("submit key");
    let context = RemoteNodeContext::isolated_for_test();
    let cancellation = context
        .register_active_execution(key.clone(), run, 1)
        .expect("register active execution");
    let execution = RemoteWorkerSubmitExecution {
        store,
        context: context.clone(),
        idempotency_key: key,
        execution_root_base: fallback,
        selected_node_id: "builder-a".into(),
        transport_kind: "direct".into(),
        image_cache: None,
        cancellation,
        payload: blocking_payload(run),
        admission: PreparedResourceAdmission::Admitted { started_at: 1 },
    };
    let workspace = resolved.join("sessions/shared");
    let worker = thread::spawn(move || run_remote_worker_submit_execution(&execution));
    wait_until(|| workspace.join("ready").exists());
    let early_baseline = backdate(&resolved_parent);
    let gate = rusqlite::Connection::open(db).expect("open database gate");
    gate.busy_timeout(Duration::from_secs(45))
        .expect("configure gate timeout");
    gate.execute_batch("BEGIN IMMEDIATE")
        .expect("lock database");
    fs::write(workspace.join("release"), []).expect("release task");
    wait_until(|| modified(&resolved_parent) > early_baseline);
    let final_baseline = backdate(&resolved_parent);
    gate.execute_batch("COMMIT").expect("unlock database");
    worker.join().expect("worker thread");

    assert!(modified(&resolved_parent) > final_baseline);
    assert_eq!(modified(&fallback_parent), fallback_baseline);
    assert!(!context.cancel_active_task(run, Some(1)).unwrap());
}

fn backdate(path: &Path) -> SystemTime {
    fs::File::open(path)
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH)
        .unwrap();
    modified(path)
}

fn modified(path: &Path) -> SystemTime {
    fs::metadata(path).unwrap().modified().unwrap()
}

fn wait_until(condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(45);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for fixture");
        thread::sleep(Duration::from_millis(2));
    }
}
