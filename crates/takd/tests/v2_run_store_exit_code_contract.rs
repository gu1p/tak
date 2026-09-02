use tak_proto::local_daemon::v2::{RunEventKind, RunLifecycleState};
use tak_proto::worker_v2::{WorkerTerminalOutcome, payload_digest};
use takd::{AttemptCompletion, RunStore, SchedulerNode, SubmitAttemptStore};

use crate::support::v2_run::{scheduler::commit, submission};
use crate::support::v2_worker::dispatch;

#[path = "v2_run_store_exit_code_contract/terminal_metadata.rs"]
mod terminal_metadata;

#[test]
fn failed_process_exit_code_is_durable_in_run_summary_and_terminal_events() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let run_id = commit(&store, &submission("exit-code", "redacted"), "alice");
    let command = store
        .reserve_next(&[SchedulerNode::with_execution_slots("local", 1)])
        .unwrap()
        .unwrap();

    store
        .complete_attempt(
            &command,
            AttemptCompletion::Failed {
                terminal_digest: "a".repeat(64),
                exit_code: Some(7),
            },
        )
        .unwrap();

    drop(store);
    let restored = RunStore::with_db_path(db).unwrap();
    let details = restored.get_run(&run_id).unwrap().unwrap();
    assert_eq!(details.summary.state, RunLifecycleState::Failed);
    assert_eq!(details.summary.exit_code, Some(7));
    let events = restored.events_after(&run_id, 0).unwrap();
    assert!(
        events
            .iter()
            .any(|event| { event.kind == RunEventKind::Failed && event.exit_code == Some(7) })
    );
    let attached = restored.attachment_snapshot(&run_id, 0).unwrap().unwrap();
    assert_eq!(attached.summary.exit_code, Some(7));
}

#[test]
fn worker_terminal_exit_code_survives_restart_and_observation() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("worker.sqlite");
    let store = SubmitAttemptStore::with_db_path(db.clone()).unwrap();
    let mut request = dispatch(1, 1, "fence-exit");
    request.payload_digest = payload_digest(&request.payload).unwrap();
    store.register_worker_v2_attempt(&request).unwrap();
    store.mark_worker_v2_running(&request.identity).unwrap();
    store
        .complete_worker_v2_attempt_with_runtime(
            &request.identity,
            WorkerTerminalOutcome::Failed,
            &"b".repeat(64),
            Some(7),
            Some("containerized".into()),
            Some("podman".into()),
        )
        .unwrap();
    drop(store);

    let terminal = SubmitAttemptStore::with_db_path(db)
        .unwrap()
        .observe_worker_v2_attempt(&request.identity, 0)
        .unwrap()
        .terminal
        .unwrap();
    assert_eq!(terminal.exit_code, Some(7));
    assert_eq!(terminal.runtime_kind.as_deref(), Some("containerized"));
    assert_eq!(terminal.runtime_engine.as_deref(), Some("podman"));
}
