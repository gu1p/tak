use tak_core::v2::{OutputSelector, WorkspaceEntry};
use tak_proto::worker_v2::{WorkerAttemptState, WorkerTerminalOutcome, payload_digest};
use takd::SubmitAttemptStore;

use crate::support::v2_worker::dispatch;

#[test]
fn persisted_cancellation_beats_late_running_output_and_success_transitions() {
    let (_temp, store, request) = setup();
    store.mark_worker_v2_running(&request.identity).unwrap();
    store
        .publish_worker_v2_output(&request.identity, "//:check", entry("one.txt"), b"ok\n")
        .unwrap();
    store.cancel_worker_v2_attempt(&request.identity).unwrap();

    assert!(store.mark_worker_v2_running(&request.identity).is_err());
    assert!(
        store
            .publish_worker_v2_output(
                &request.identity,
                "//:check",
                entry("late.txt"),
                b"ok\n",
            )
            .is_err()
    );
    let terminal = store
        .complete_worker_v2_attempt(
            &request.identity,
            WorkerTerminalOutcome::Succeeded,
            &"a".repeat(64),
        )
        .unwrap();
    assert_eq!(terminal.outcome, WorkerTerminalOutcome::Cancelled);
    assert!(terminal.outputs.is_empty());
    assert_eq!(
        store
            .observe_worker_v2_attempt(&request.identity, 0)
            .unwrap()
            .state,
        WorkerAttemptState::Completed
    );
}
fn setup() -> (
    tempfile::TempDir,
    SubmitAttemptStore,
    tak_proto::worker_v2::DispatchAttemptRequest,
) {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).unwrap();
    let mut request = dispatch(1, 1, "fence-1");
    request.payload.tasks[0].outputs = ["one.txt", "late.txt"]
        .into_iter()
        .map(|value| OutputSelector::Path {
            value: value.into(),
        })
        .collect();
    request.payload_digest = payload_digest(&request.payload).unwrap();
    store.register_worker_v2_attempt(&request).unwrap();
    (temp, store, request)
}

fn entry(path: &str) -> WorkspaceEntry {
    WorkspaceEntry::file(
        path,
        false,
        3,
        "dc51b8c96c2d745df3bd5590d990230a482fd247123599548e0632fdbf97fc22",
    )
    .unwrap()
}
