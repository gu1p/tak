use takd::{AttemptCoordinator, RunStore, SchedulerNode};

use crate::support::coordinator_progress::{ControlledTransport, tick};
use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[tokio::test]
async fn stalled_observation_does_not_block_dispatch_or_cancellation() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("origin.sqlite")).unwrap();
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 10)];
    let run = commit(&store, &independent_jobs("observe-progress", 1), "alice");
    let slow = store.reserve_next(&nodes).unwrap().unwrap();
    store.ack_dispatch(&slow).unwrap();
    let transport = ControlledTransport::new("reconcile");
    let mut coordinator = AttemptCoordinator::new(store.clone(), transport.clone());

    tick(&mut coordinator).await;
    commit(&store, &independent_jobs("later-dispatch", 1), "bob");
    let fast = store.reserve_next(&nodes).unwrap().unwrap();
    tick(&mut coordinator).await;
    assert_eq!(transport.calls("dispatch", &fast.fencing_token), 1);
    store.cancel(&run).unwrap();
    tick(&mut coordinator).await;
    assert_eq!(transport.calls("cancel", &slow.fencing_token), 1);
    assert_eq!(transport.calls("reconcile", &slow.fencing_token), 1);
    transport.release.notify_one();
    tick(&mut coordinator).await;
    assert_eq!(
        store.get_run(&run).unwrap().unwrap().jobs[0].state,
        "cancelled"
    );
}
