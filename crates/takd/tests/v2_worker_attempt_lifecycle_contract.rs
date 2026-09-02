use tak_core::v2::{OutputSelector, WorkspaceEntry};
use tak_proto::worker_v2::{
    DispatchDisposition, WorkerAttemptState, WorkerOutputStream, WorkerTerminalOutcome,
    payload_digest,
};
use takd::SubmitAttemptStore;

use crate::support::v2_worker::dispatch;

#[test]
fn worker_attempt_lifecycle_durably_observes_events_outputs_terminal_and_ack() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("agent.sqlite");
    let store = SubmitAttemptStore::with_db_path(db.clone()).unwrap();
    let mut request = dispatch(1, 1, "fence-1");
    request.payload.tasks[0].outputs = vec![OutputSelector::Path {
        value: "result.txt".into(),
    }];
    request.payload_digest = payload_digest(&request.payload).unwrap();
    assert_eq!(
        store.register_worker_v2_attempt(&request).unwrap(),
        DispatchDisposition::Accepted
    );
    store.mark_worker_v2_running(&request.identity).unwrap();
    let event = store
        .append_worker_v2_event(
            &request.identity,
            "//:check",
            WorkerOutputStream::Stdout,
            b"ok\n",
        )
        .unwrap();
    assert_eq!(event.seq, 1);
    let entry = WorkspaceEntry::file(
        "result.txt",
        false,
        3,
        "dc51b8c96c2d745df3bd5590d990230a482fd247123599548e0632fdbf97fc22",
    )
    .unwrap();
    let artifact = store
        .publish_worker_v2_output(&request.identity, "//:check", entry, b"ok\n")
        .unwrap();
    store
        .complete_worker_v2_attempt(
            &request.identity,
            WorkerTerminalOutcome::Succeeded,
            "5dceaeceb6d8a49d3665594710ace68dfbc1800e2d663d7febb302b4dbee3d27",
        )
        .unwrap();

    let observed = store
        .observe_worker_v2_attempt(&request.identity, 0)
        .unwrap();
    assert_eq!(observed.state, WorkerAttemptState::Completed);
    assert_eq!(observed.events, vec![event]);
    assert_eq!(observed.terminal.unwrap().outputs, vec![artifact.clone()]);
    assert_eq!(
        store
            .worker_v2_output_chunk(&request.identity, &artifact.artifact_id, 0, 8)
            .unwrap(),
        b"ok\n"
    );
    store
        .acknowledge_worker_v2_terminal(
            &request.identity,
            "5dceaeceb6d8a49d3665594710ace68dfbc1800e2d663d7febb302b4dbee3d27",
        )
        .unwrap();
    drop(store);
    assert_eq!(
        SubmitAttemptStore::with_db_path(db)
            .unwrap()
            .observe_worker_v2_attempt(&request.identity, 1)
            .unwrap()
            .state,
        WorkerAttemptState::Completed
    );
}
