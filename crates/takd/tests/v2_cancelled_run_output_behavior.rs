use tak_proto::local_daemon::v2::RunLifecycleState;
use takd::{RunStore, SchedulerNode};

use crate::support::v2_run::{
    final_outputs::{assert_survivor, seed, succeeded},
    local_outputs::failed_run,
    scheduler::commit,
};

#[test]
fn cancelled_runs_retain_outputs_from_jobs_that_already_succeeded() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let request = failed_run("cancelled-output", true, true);
    let run_id = commit(&store, &request, "alice");
    let nodes = [SchedulerNode::with_execution_slots("local", 2)];
    let first = store.reserve_next(&nodes).unwrap().unwrap();
    let second = store.reserve_next(&nodes).unwrap().unwrap();
    let (producer, active) = if first.job_id == "job-0" {
        (first, second)
    } else {
        (second, first)
    };
    seed(&db, &producer);
    store.complete_attempt(&producer, succeeded()).unwrap();

    assert_eq!(store.cancel(&run_id).unwrap(), RunLifecycleState::Cancelling);
    let cancellation = store.pending_cancellations().unwrap().remove(0);
    assert_eq!(cancellation.fencing_token, active.fencing_token);
    store.ack_cancellation(&cancellation).unwrap();

    assert_eq!(
        store.summary(&run_id).unwrap().unwrap().state,
        RunLifecycleState::Cancelled
    );
    assert_survivor(&store, &run_id);
}
