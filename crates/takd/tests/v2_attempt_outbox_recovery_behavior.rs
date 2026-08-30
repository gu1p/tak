use takd::{ResultAcceptance, RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn restart_replays_transfers_and_reconciles_accepted_attempts_without_retrying() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    commit(&store, &independent_jobs("recover-attempts", 2), "uid:1");
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 2)];
    let transferring = store.reserve_next(&nodes).unwrap().unwrap();
    let running = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(
        store.ack_dispatch(&running).unwrap(),
        ResultAcceptance::Applied
    );
    drop(store);

    let restored = RunStore::with_db_path(db).unwrap();
    assert_eq!(restored.pending_dispatches().unwrap(), [transferring]);
    assert_eq!(
        restored.running_attempts_for_reconciliation().unwrap(),
        [running]
    );
}
