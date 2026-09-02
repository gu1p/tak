use std::num::NonZeroU32;

use tak_core::v2::Affinity;
use takd::{AttemptCompletion, NodeLossResolution, ResultAcceptance, RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn a_fresh_healthy_snapshot_recovers_the_node_without_unfencing_old_work() {
    let (temp, store) = store();
    let mut work = independent_jobs("recover-node", 1);
    work.run.jobs[0].retry.max_attempts = NonZeroU32::new(2).unwrap();
    let run_id = commit(&store, &work, "alice");
    let healthy = [SchedulerNode::with_execution_slots("worker-a", 1)];
    let lost_attempt = store.reserve_next(&healthy).unwrap().unwrap();
    store.ack_dispatch(&lost_attempt).unwrap();

    assert_eq!(
        store.declare_node_lost("worker-a").unwrap(),
        NodeLossResolution::Applied
    );
    assert_eq!(
        store.declare_node_lost("worker-a").unwrap(),
        NodeLossResolution::Duplicate
    );
    let recovered = store.reserve_next(&healthy).unwrap().unwrap();
    assert_eq!(recovered.authored_attempt, 2);
    assert_ne!(recovered.fencing_token, lost_attempt.fencing_token);
    assert_eq!(
        store.complete_attempt(&lost_attempt, success("a")).unwrap(),
        ResultAcceptance::Stale
    );
    store.complete_attempt(&recovered, success("b")).unwrap();
    assert_eq!(state(&store, &run_id), "succeeded");

    drop(store);
    let restored = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    assert_eq!(
        restored.declare_node_lost("worker-a").unwrap(),
        NodeLossResolution::Applied
    );
    assert_eq!(
        restored.declare_node_lost("worker-a").unwrap(),
        NodeLossResolution::Duplicate
    );
    let later_id = commit(&restored, &independent_jobs("later-run", 1), "bob");
    let later = restored.reserve_next(&healthy).unwrap().unwrap();
    assert_eq!(later.run_id, later_id);
    assert_eq!(later.node_id, "worker-a");
}

#[test]
fn recovery_does_not_revive_a_failed_hard_affinity_group() {
    let (_temp, store) = store();
    let mut hard = independent_jobs("failed-hard-group", 1);
    let affinity = Affinity::require_same_node("shared").unwrap();
    hard.run.tasks[0].affinity = Some(affinity.clone());
    hard.run.jobs[0].affinity = Some(affinity);
    let hard_id = commit(&store, &hard, "alice");
    let healthy = [SchedulerNode::with_execution_slots("worker-a", 1)];
    let attempt = store.reserve_next(&healthy).unwrap().unwrap();
    store.ack_dispatch(&attempt).unwrap();
    store.declare_node_lost("worker-a").unwrap();
    assert_eq!(state(&store, &hard_id), "failed");

    let later_id = commit(&store, &independent_jobs("after-hard-loss", 1), "bob");
    let later = store.reserve_next(&healthy).unwrap().unwrap();
    assert_eq!(later.run_id, later_id);
    assert_eq!(state(&store, &hard_id), "failed");
}

fn store() -> (tempfile::TempDir, RunStore) {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    (temp, store)
}

fn state(store: &RunStore, run_id: &str) -> String {
    store.get_run(run_id).unwrap().unwrap().jobs[0]
        .state
        .clone()
}

fn success(seed: &str) -> AttemptCompletion {
    AttemptCompletion::Succeeded {
        terminal_digest: seed.repeat(64),
    }
}
