use tak_proto::local_daemon::v2::RunLifecycleState;
use takd::{AttemptCompletion, ResultAcceptance, RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn active_cancellation_replays_fenced_commands_and_releases_on_ack() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let run_id = commit(&store, &independent_jobs("cancel-two", 2), "uid:1");
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 2)];
    let transferring = store.reserve_next(&nodes).unwrap().unwrap();
    let running = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(
        store.ack_dispatch(&running).unwrap(),
        ResultAcceptance::Applied
    );

    assert_eq!(
        store.cancel(&run_id).unwrap(),
        RunLifecycleState::Cancelling
    );
    assert!(store.pending_dispatches().unwrap().is_empty());
    let cancellations = store.pending_cancellations().unwrap();
    assert_eq!(cancellations.len(), 2);
    assert!(
        cancellations
            .iter()
            .any(|cancel| cancel.fencing_token == transferring.fencing_token)
    );
    assert!(
        cancellations
            .iter()
            .any(|cancel| cancel.fencing_token == running.fencing_token)
    );
    assert_eq!(
        store.complete_attempt(&running, success()).unwrap(),
        ResultAcceptance::Stale
    );
    drop(store);

    let restored = RunStore::with_db_path(db).unwrap();
    assert_eq!(restored.pending_cancellations().unwrap(), cancellations);
    assert_eq!(
        restored.ack_cancellation(&cancellations[0]).unwrap(),
        ResultAcceptance::Applied
    );
    assert_eq!(
        restored.summary(&run_id).unwrap().unwrap().state,
        RunLifecycleState::Cancelling
    );
    assert_eq!(
        restored.ack_cancellation(&cancellations[1]).unwrap(),
        ResultAcceptance::Applied
    );
    assert_eq!(
        restored.summary(&run_id).unwrap().unwrap().state,
        RunLifecycleState::Cancelled
    );
    assert_eq!(
        restored.ack_cancellation(&cancellations[1]).unwrap(),
        ResultAcceptance::Duplicate
    );
    assert!(restored.pending_cancellations().unwrap().is_empty());
}

fn success() -> AttemptCompletion {
    AttemptCompletion::Succeeded {
        terminal_digest: "f".repeat(64),
    }
}
