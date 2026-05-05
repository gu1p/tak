use takd::daemon::remote::SubmitAttemptStore;

#[test]
fn active_attempts_exclude_completed_submits() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let root = temp.path().join("exec");

    store
        .register_submit_with_task_label("active-run", Some(1), "//apps/web:build", "node-a", &root)
        .expect("register active");
    store
        .register_submit_with_task_label("done-run", Some(1), "//apps/web:test", "node-a", &root)
        .expect("register done");
    let done_key = store
        .latest_submit_idempotency_key_for_task_run("done-run")
        .expect("done key")
        .expect("done key exists");
    store
        .set_result_payload(&done_key, r#"{"success":true}"#)
        .expect("complete done");

    let active = store.active_attempts().expect("active attempts");

    assert_eq!(active.len(), 1);
    assert_eq!(active[0].task_run_id, "active-run");
    assert_eq!(active[0].task_label, "//apps/web:build");
}

#[test]
fn unfinished_attempts_can_be_marked_abandoned_after_restart() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let root = temp.path().join("exec");
    store
        .register_submit_with_task_label("stale-run", Some(1), "//apps/web:build", "node-a", &root)
        .expect("register stale");

    let marked = store
        .mark_unfinished_attempts_abandoned()
        .expect("mark abandoned");

    assert_eq!(marked, 1);
    assert!(store.active_attempts().expect("active attempts").is_empty());
    let key = store
        .latest_submit_idempotency_key_for_task_run("stale-run")
        .expect("key")
        .expect("key exists");
    let result = store.result_payload(&key).expect("result").expect("result");
    assert!(result.contains("abandoned"));
}
