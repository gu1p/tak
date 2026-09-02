use tak_proto::local_daemon::v2::RunLifecycleState;
use takd::{RunStore, SchedulerNode};

use crate::support::v2_run::{
    final_outputs::{assert_survivor, failed, seed, succeeded},
    local_outputs::failed_run,
    scheduler::commit,
};

#[test]
fn fail_fast_retains_outputs_from_an_already_active_successful_branch() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let request = failed_run("failed-inflight-output", true, false);
    let run_id = commit(&store, &request, "alice");
    let nodes = [SchedulerNode::with_execution_slots("local", 2)];
    let first = store.reserve_next(&nodes).unwrap().unwrap();
    let second = store.reserve_next(&nodes).unwrap().unwrap();
    let (producer, failure) = if first.job_id == "job-0" {
        (first, second)
    } else {
        (second, first)
    };

    store.complete_attempt(&failure, failed()).unwrap();
    seed(&db, &producer);
    store.complete_attempt(&producer, succeeded()).unwrap();

    assert_eq!(
        store.summary(&run_id).unwrap().unwrap().state,
        RunLifecycleState::Failed
    );
    assert_survivor(&store, &run_id);
}
