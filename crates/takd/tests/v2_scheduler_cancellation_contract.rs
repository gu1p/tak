use tak_proto::local_daemon::v2::RunLifecycleState;
use takd::{AttemptCompletion, ResultAcceptance, RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn persisted_cancellation_fences_late_attempt_results() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let run_id = commit(&store, &independent_jobs("cancel-active", 1), "uid:1");
    let command = store
        .reserve_next(&[SchedulerNode::with_execution_slots("worker-a", 1)])
        .unwrap()
        .unwrap();
    assert_eq!(
        store.ack_dispatch(&command).unwrap(),
        ResultAcceptance::Applied
    );

    assert_eq!(
        store.cancel(&run_id).unwrap(),
        RunLifecycleState::Cancelling
    );
    let completion = AttemptCompletion::Succeeded {
        terminal_digest: "c".repeat(64),
    };
    assert_eq!(
        store.complete_attempt(&command, completion).unwrap(),
        ResultAcceptance::Stale
    );
    assert_eq!(
        store.summary(&run_id).unwrap().unwrap().state,
        RunLifecycleState::Cancelling
    );
}
