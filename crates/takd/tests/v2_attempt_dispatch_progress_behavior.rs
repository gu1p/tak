use std::num::NonZeroU32;

use takd::{AttemptCoordinator, RunStore, SchedulerNode};

use crate::support::coordinator_progress::{ControlledTransport, tick};
use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[tokio::test]
async fn stalled_dispatch_does_not_block_other_workers_or_duplicate_requests() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("origin.sqlite")).unwrap();
    let nodes = [
        SchedulerNode::with_execution_slots("worker-a", 10),
        SchedulerNode::with_execution_slots("worker-b", 10),
    ];
    let mut submission = independent_jobs("dispatch-progress", 2);
    submission.run.options.max_parallel_jobs = NonZeroU32::new(10).unwrap();
    let run = commit(&store, &submission, "alice");
    let slow = store.reserve_next(&nodes).unwrap().unwrap();
    let fast = store.reserve_next(&nodes).unwrap().unwrap();
    assert_ne!(slow.node_id, fast.node_id);
    let transport = ControlledTransport::new("dispatch");
    let mut coordinator = AttemptCoordinator::new(store.clone(), transport.clone());

    tick(&mut coordinator).await;
    assert_eq!(
        store.get_run(&run).unwrap().unwrap().jobs[1].state,
        "running"
    );
    for _ in 0..3 {
        tick(&mut coordinator).await;
    }
    assert_eq!(transport.calls("dispatch", &slow.fencing_token), 1);
    assert_eq!(transport.calls("dispatch", &fast.fencing_token), 1);
    assert!(transport.calls("reconcile", &fast.fencing_token) > 0);

    transport.release.notify_one();
    tick(&mut coordinator).await;
    assert!(store.pending_dispatches().unwrap().is_empty());
}
