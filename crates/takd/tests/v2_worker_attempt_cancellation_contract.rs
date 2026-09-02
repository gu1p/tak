use tak_proto::worker_v2::{CancelDisposition, WorkerTerminalOutcome};
use takd::{SubmitAttemptStore, worker_v2_cancellation_poll_requests_cancel};

use crate::support::v2_worker::dispatch;

#[test]
fn worker_cancellation_is_persisted_idempotent_and_generation_fenced() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).unwrap();
    let first = dispatch(1, 1, "fence-1");
    store.register_worker_v2_attempt(&first).unwrap();
    assert_eq!(
        store.cancel_worker_v2_attempt(&first.identity).unwrap(),
        CancelDisposition::Requested
    );
    assert!(
        store
            .worker_v2_cancellation_requested(&first.identity)
            .unwrap()
    );
    assert_eq!(
        store.cancel_worker_v2_attempt(&first.identity).unwrap(),
        CancelDisposition::Duplicate
    );
    store
        .complete_worker_v2_attempt(
            &first.identity,
            WorkerTerminalOutcome::Cancelled,
            &"c".repeat(64),
        )
        .unwrap();
    assert_eq!(
        store.cancel_worker_v2_attempt(&first.identity).unwrap(),
        CancelDisposition::AlreadyTerminal
    );

    let second = dispatch(1, 2, "fence-2");
    store.register_worker_v2_attempt(&second).unwrap();
    assert_eq!(
        store.cancel_worker_v2_attempt(&first.identity).unwrap(),
        CancelDisposition::Stale
    );
}

#[test]
fn transient_cancellation_poll_errors_do_not_cancel_running_attempts() {
    assert!(!worker_v2_cancellation_poll_requests_cancel(Ok(false)));
    assert!(!worker_v2_cancellation_poll_requests_cancel(Err(
        anyhow::anyhow!("database is temporarily busy")
    )));
    assert!(worker_v2_cancellation_poll_requests_cancel(Ok(true)));
}
