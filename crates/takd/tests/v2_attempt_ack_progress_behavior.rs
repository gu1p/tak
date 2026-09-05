use takd::{AttemptCompletion, AttemptCoordinator, RunStore, SchedulerNode};

use crate::support::coordinator_progress::{ControlledTransport, tick};
use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[tokio::test]
async fn stalled_terminal_acknowledgement_does_not_block_later_work() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("origin.sqlite")).unwrap();
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 10)];
    commit(&store, &independent_jobs("ack-progress", 1), "alice");
    let slow = store.reserve_next(&nodes).unwrap().unwrap();
    store.ack_dispatch(&slow).unwrap();
    store
        .complete_attempt(
            &slow,
            AttemptCompletion::Succeeded {
                terminal_digest: "a".repeat(64),
            },
        )
        .unwrap();
    let transport = ControlledTransport::new("ack");
    let mut coordinator = AttemptCoordinator::new(store.clone(), transport.clone());

    tick(&mut coordinator).await;
    commit(&store, &independent_jobs("later-work", 1), "bob");
    let fast = store.reserve_next(&nodes).unwrap().unwrap();
    tick(&mut coordinator).await;
    assert_eq!(transport.calls("dispatch", &fast.fencing_token), 1);
    assert_eq!(transport.calls("ack", &slow.fencing_token), 1);
    transport.release.notify_one();
    tick(&mut coordinator).await;
    tick(&mut coordinator).await;
    assert_eq!(transport.calls("ack", &slow.fencing_token), 1);
}
