use takd::{ResultAcceptance, RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn reflected_worker_claim_counts_once_while_an_unaccepted_transfer_stays_additive() {
    let temp = tempfile::tempdir().unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let request = independent_jobs("worker-snapshot-accounting", 3);
    commit(&store, &request, "alice");
    let idle = SchedulerNode::with_execution_slots("worker-a", 2);
    let first = store.reserve_next(&[idle]).unwrap().unwrap();
    assert_eq!(
        store.ack_dispatch(&first).unwrap(),
        ResultAcceptance::Applied
    );

    let reflected = SchedulerNode::with_execution_slots("worker-a", 2).with_execution_usage(1);
    let second = store
        .reserve_next(std::slice::from_ref(&reflected))
        .unwrap()
        .expect("worker claim and accepted origin reservation are the same usage");
    assert_eq!(second.node_id, "worker-a");
    assert!(store.reserve_next(&[reflected]).unwrap().is_none());
}
