use takd::daemon::remote::{SubmitAttemptStore, SubmitRegistration};

#[test]
fn submit_store_preserves_idempotency_events_and_result_contract() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("nested/state/takd.sqlite");
    let store = SubmitAttemptStore::with_db_path(db_path.clone()).expect("submit attempt store");
    let execution_root_base = temp.path().join("exec-root");

    let first = store
        .register_submit("task-run-1", Some(1), "node-a", &execution_root_base)
        .expect("register first submit");
    let key = match first {
        SubmitRegistration::Created { idempotency_key } => idempotency_key,
        SubmitRegistration::Attached { .. } => panic!("first submit must create attempt"),
    };

    let duplicate = store
        .register_submit("task-run-1", Some(1), "node-a", &execution_root_base)
        .expect("register duplicate submit");
    match duplicate {
        SubmitRegistration::Attached { idempotency_key } => {
            assert_eq!(idempotency_key, key, "duplicate should attach to same key");
        }
        SubmitRegistration::Created { .. } => panic!("duplicate submit must attach"),
    }

    store
        .append_event(&key, 2, r#"{"type":"second"}"#)
        .expect("append second event");
    store
        .append_event(&key, 1, r#"{"type":"first"}"#)
        .expect("append first event");
    store
        .set_result_payload(&key, r#"{"status":"ok"}"#)
        .expect("set result payload");

    let events = store.events(&key).expect("load events");
    let event_seqs: Vec<u64> = events.iter().map(|event| event.seq).collect();
    assert_eq!(
        event_seqs,
        vec![1, 2],
        "events should be ordered by sequence"
    );
    assert_eq!(
        store.result_payload(&key).expect("load result").as_deref(),
        Some(r#"{"status":"ok"}"#)
    );
    assert!(db_path.exists(), "store should create parent sqlite path");
}
