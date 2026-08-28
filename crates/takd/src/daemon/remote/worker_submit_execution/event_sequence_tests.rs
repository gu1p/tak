use std::sync::{Arc, Barrier};

use super::*;

#[test]
fn shared_writer_persists_concurrent_events_with_contiguous_sequences() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let registration = store
        .register_submit_with_execution_root_base(
            "sequence-run",
            Some(1),
            "//:check",
            None,
            "node-a",
            temp.path(),
        )
        .expect("register");
    let key = match registration {
        super::SubmitRegistration::Created { idempotency_key }
        | super::SubmitRegistration::Attached { idempotency_key } => idempotency_key,
    };
    let writer = RemoteWorkerEventWriter::new(store.clone(), key.clone(), 1);
    let barrier = Arc::new(Barrier::new(9));
    let mut threads = Vec::new();
    for index in 0..8 {
        let writer = writer.clone();
        let barrier = barrier.clone();
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            writer.append(serde_json::json!({ "kind": format!("EVENT_{index}") }))
        }));
    }
    barrier.wait();
    for thread in threads {
        thread.join().expect("thread").expect("append event");
    }

    let sequences = store
        .events(&key)
        .expect("events")
        .into_iter()
        .map(|event| event.seq)
        .collect::<Vec<_>>();
    assert_eq!(sequences, (1_u64..=8).collect::<Vec<_>>());
}
