use tak_core::v2::{OutputSelector, WorkspaceEntry};
use tak_proto::worker_v2::{WorkerTerminalOutcome, payload_digest};
use takd::SubmitAttemptStore;

use crate::support::v2_worker::dispatch;

#[test]
fn output_chunks_require_the_full_attempt_identity_and_a_valid_offset() {
    let (_temp, store, request) = setup();
    store.mark_worker_v2_running(&request.identity).unwrap();
    let artifact = store
        .publish_worker_v2_output(&request.identity, "//:check", entry(), b"ok\n")
        .unwrap();
    let mut identities = Vec::new();
    for field in 0..5 {
        let mut identity = request.identity.clone();
        match field {
            0 => identity.run_id.push('x'),
            1 => identity.job_id.push('x'),
            2 => identity.node_id.push('x'),
            3 => identity.authored_attempt += 1,
            _ => identity.dispatch_generation += 1,
        }
        identities.push(identity);
    }
    for identity in identities {
        assert!(
            store
                .worker_v2_output_chunk(&identity, &artifact.artifact_id, 0, 8)
                .is_err()
        );
    }
    assert!(
        store
            .worker_v2_output_chunk(&request.identity, &artifact.artifact_id, 4, 8)
            .is_err()
    );
}

#[test]
fn completed_terminal_ack_remains_replayable_after_a_newer_generation() {
    let (_temp, store, first) = setup();
    store
        .complete_worker_v2_attempt(&first.identity, WorkerTerminalOutcome::Failed, &"b".repeat(64))
        .unwrap();
    let second = dispatch(2, 1, "fence-new");
    store.register_worker_v2_attempt(&second).unwrap();

    store
        .acknowledge_worker_v2_terminal(&first.identity, &"b".repeat(64))
        .unwrap();
    assert!(store.worker_v2_terminal_is_acknowledged(&first.identity).unwrap());
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
    request.payload.tasks[0].outputs = vec![OutputSelector::Path {
        value: "result.txt".into(),
    }];
    request.payload_digest = payload_digest(&request.payload).unwrap();
    store.register_worker_v2_attempt(&request).unwrap();
    (temp, store, request)
}

fn entry() -> WorkspaceEntry {
    WorkspaceEntry::file(
        "result.txt",
        false,
        3,
        "dc51b8c96c2d745df3bd5590d990230a482fd247123599548e0632fdbf97fc22",
    )
    .unwrap()
}
