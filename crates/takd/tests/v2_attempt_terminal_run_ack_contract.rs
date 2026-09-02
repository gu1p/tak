use std::sync::Arc;

use takd::{AttemptCompletion, AttemptCoordinator, ResultAcceptance, RunStore, SchedulerNode};

use crate::support::{
    v2_run::scheduler::commit,
    v2_terminal_ack::{AckRecorder, sequential_shared},
};

#[tokio::test]
async fn terminal_transition_durably_replays_remote_acks_for_shared_cleanup() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    commit(&store, &sequential_shared("terminal-ack"), "alice");
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 1)];
    let transport = Arc::new(AckRecorder::default());
    let mut coordinator = AttemptCoordinator::new(store.clone(), transport.clone());

    let first = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(
        store.ack_dispatch(&first).unwrap(),
        ResultAcceptance::Applied
    );
    succeed(&store, &first);
    assert_eq!(coordinator.drive_once().await.unwrap().acknowledged, 1);

    let second = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(
        store.ack_dispatch(&second).unwrap(),
        ResultAcceptance::Applied
    );
    succeed(&store, &second);
    assert_eq!(coordinator.drive_once().await.unwrap().acknowledged, 2);
    assert_eq!(
        *transport.acks.lock().unwrap(),
        [
            ("job-0".into(), false),
            ("job-0".into(), true),
            ("job-1".into(), true)
        ]
    );
}

fn succeed(store: &RunStore, command: &takd::DispatchCommand) {
    store
        .complete_attempt(
            command,
            AttemptCompletion::Succeeded {
                terminal_digest: "a".repeat(64),
            },
        )
        .unwrap();
}

#[tokio::test]
async fn terminal_transition_during_an_ack_keeps_the_release_replay_pending() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    commit(&store, &sequential_shared("racing-terminal-ack"), "alice");
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 1)];
    let first = store.reserve_next(&nodes).unwrap().unwrap();
    store.ack_dispatch(&first).unwrap();
    succeed(&store, &first);
    let second = store.reserve_next(&nodes).unwrap().unwrap();
    store.ack_dispatch(&second).unwrap();
    let transport = Arc::new(AckRecorder::completing(store.clone(), second));
    let mut coordinator = AttemptCoordinator::new(store, transport.clone());

    assert_eq!(coordinator.drive_once().await.unwrap().acknowledged, 1);
    assert_eq!(coordinator.drive_once().await.unwrap().acknowledged, 2);
    assert_eq!(
        *transport.acks.lock().unwrap(),
        [
            ("job-0".into(), false),
            ("job-0".into(), true),
            ("job-1".into(), true)
        ]
    );
}
