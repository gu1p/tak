use takd::{AttemptCoordinator, RunStore, SchedulerNode};

use crate::support::coordinator_progress::{ControlledTransport, tick};
use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[tokio::test]
async fn delayed_dispatch_acceptance_cannot_revive_a_cancelled_run() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("origin.sqlite")).unwrap();
    let run = commit(&store, &independent_jobs("late-acceptance", 1), "alice");
    let command = store
        .reserve_next(&[SchedulerNode::with_execution_slots("worker-a", 1)])
        .unwrap()
        .unwrap();
    let transport = ControlledTransport::new("dispatch");
    let mut coordinator = AttemptCoordinator::new(store.clone(), transport.clone());
    tick(&mut coordinator).await;
    store.cancel(&run).unwrap();
    tick(&mut coordinator).await;
    transport.release.notify_one();
    tick(&mut coordinator).await;

    assert_eq!(
        store.get_run(&run).unwrap().unwrap().jobs[0].state,
        "cancelled"
    );
    assert_eq!(transport.calls("dispatch", &command.fencing_token), 1);
    assert!(store.pending_dispatches().unwrap().is_empty());
}

#[tokio::test]
async fn dropping_coordinator_replays_pending_dispatch_with_the_same_fence() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("origin.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    commit(&store, &independent_jobs("restart-inflight", 1), "alice");
    let command = store
        .reserve_next(&[SchedulerNode::with_execution_slots("worker-a", 1)])
        .unwrap()
        .unwrap();
    let transport = ControlledTransport::new("dispatch");
    let mut coordinator = AttemptCoordinator::new(store.clone(), transport.clone());
    tick(&mut coordinator).await;
    drop(coordinator);
    drop(store);

    let restored = RunStore::with_db_path(db).unwrap();
    let mut coordinator = AttemptCoordinator::new(restored.clone(), transport.clone());
    transport.release.notify_one();
    tick(&mut coordinator).await;
    assert_eq!(transport.calls("dispatch", &command.fencing_token), 2);
    assert!(restored.pending_dispatches().unwrap().is_empty());
}
