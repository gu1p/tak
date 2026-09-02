use std::num::NonZeroU32;

use tak_core::v2::{Affinity, RemoteSelection, Session, SessionReuse};
use takd::{AttemptCompletion, NodeLossResolution, ResultAcceptance, RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn lost_shared_workspace_home_fails_the_group_once_and_keeps_unrelated_work_running() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = independent_jobs("hard-node-loss", 4);
    request.run.options.keep_going = true;
    let hard = Affinity::require_same_node("build").unwrap();
    let session = Session::new(
        "shared",
        SessionReuse::shared_workspace(2).unwrap(),
        Some(hard.clone()),
    )
    .unwrap();
    for index in 0..3 {
        request.run.tasks[index].affinity = Some(hard.clone());
        request.run.jobs[index].affinity = Some(hard.clone());
        request.run.jobs[index].session = Some(session.clone());
        request.run.jobs[index].retry.max_attempts = NonZeroU32::new(2).unwrap();
    }
    request.run.jobs[3].placement_policy.policy_id = "unrelated".into();
    request.run.jobs[3].placement_policy.selection = RemoteSelection::Sequential;
    let run_id = commit(&store, &request, "alice");
    let nodes = [
        SchedulerNode::with_execution_slots("worker-a", 3),
        SchedulerNode::with_execution_slots("worker-b", 3),
    ];
    let first = store.reserve_next(&nodes).unwrap().unwrap();
    let second = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(first.node_id, "worker-a");
    assert_eq!(second.node_id, "worker-a");
    store.ack_dispatch(&first).unwrap();
    assert_eq!(store.pending_dispatches().unwrap(), vec![second.clone()]);

    assert_eq!(
        store.declare_node_lost("worker-a").unwrap(),
        NodeLossResolution::Applied
    );
    let event_count = store.events_after(&run_id, 0).unwrap().len();
    assert_eq!(
        store.declare_node_lost("worker-a").unwrap(),
        NodeLossResolution::Duplicate
    );
    assert_eq!(store.events_after(&run_id, 0).unwrap().len(), event_count);
    let details = store.get_run(&run_id).unwrap().unwrap();
    assert_eq!(details.jobs[0].state, "failed");
    assert_eq!(details.jobs[1].state, "failed");
    assert_eq!(details.jobs[2].state, "failed");
    assert_eq!(details.jobs[3].state, "ready");
    assert!(store.pending_dispatches().unwrap().is_empty());
    let survivors = [SchedulerNode::with_execution_slots("worker-b", 3)];
    let unrelated = store.reserve_next(&survivors).unwrap().unwrap();
    assert_eq!(unrelated.node_id, "worker-b");
    assert_eq!(
        store.complete_attempt(&first, success()).unwrap(),
        ResultAcceptance::Stale
    );
    assert_eq!(
        store.complete_attempt(&second, success()).unwrap(),
        ResultAcceptance::Stale
    );
}

fn success() -> AttemptCompletion {
    AttemptCompletion::Succeeded {
        terminal_digest: "a".repeat(64),
    }
}
