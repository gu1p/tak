use takd::{ResultAcceptance, RunStore, SchedulerNode};

use crate::support::v2_run::{
    constraints::project_queue,
    scheduler::{commit, independent_jobs},
};

#[test]
fn cancelling_attempts_hold_constraints_until_worker_acknowledgement() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let first = project_queue(independent_jobs("cancel-cap-a", 1), 1);
    let second = project_queue(independent_jobs("cancel-cap-b", 1), 1);
    let first_id = commit(&store, &first, "alice");
    commit(&store, &second, "bob");
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 2)];
    store.reserve_next(&nodes).unwrap().unwrap();
    store.cancel(&first_id).unwrap();
    assert!(store.reserve_next(&nodes).unwrap().is_none());

    let cancellation = store.pending_cancellations().unwrap().pop().unwrap();
    assert_eq!(
        store.ack_cancellation(&cancellation).unwrap(),
        ResultAcceptance::Applied
    );
    assert!(store.reserve_next(&nodes).unwrap().is_some());
}
