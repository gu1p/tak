use std::num::NonZeroU32;

use takd::{NodeLossResolution, ResultAcceptance, RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn lost_active_attempts_retry_only_when_idempotent_and_reject_late_results() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut retryable = independent_jobs("lost-retryable", 1);
    retryable.run.jobs[0].retry.max_attempts = NonZeroU32::new(2).unwrap();
    let retry_run = commit(&store, &retryable, "alice");
    let mut unsafe_run = independent_jobs("lost-unsafe", 1);
    unsafe_run.run.tasks[0].idempotent = false;
    unsafe_run.run.jobs[0].idempotent = false;
    unsafe_run.run.jobs[0].retry.max_attempts = NonZeroU32::new(2).unwrap();
    let unsafe_id = commit(&store, &unsafe_run, "bob");
    let only_lost = [SchedulerNode::with_execution_slots("worker-a", 2)];
    let first = store.reserve_next(&only_lost).unwrap().unwrap();
    let second = store.reserve_next(&only_lost).unwrap().unwrap();
    store.ack_dispatch(&first).unwrap();
    store.ack_dispatch(&second).unwrap();

    assert_eq!(
        store.declare_node_lost("worker-a").unwrap(),
        NodeLossResolution::Applied
    );
    assert_eq!(state(&store, &retry_run), "retrying");
    assert_eq!(state(&store, &unsafe_id), "failed");
    let nodes = [
        SchedulerNode::with_execution_slots("worker-a", 2),
        SchedulerNode::with_execution_slots("worker-b", 2),
    ];
    let retry = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(retry.node_id, "worker-b");
    assert_eq!(retry.authored_attempt, 2);
    assert_eq!(
        store.complete_attempt(&first, success()).unwrap(),
        ResultAcceptance::Stale
    );
    assert_eq!(
        store.complete_attempt(&second, success()).unwrap(),
        ResultAcceptance::Stale
    );
}

#[test]
fn fail_fast_node_loss_does_not_strand_later_idempotent_work_in_retrying() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = independent_jobs("lost-fail-fast", 2);
    request.run.tasks[0].idempotent = false;
    request.run.jobs[0].idempotent = false;
    for job in &mut request.run.jobs {
        job.retry.max_attempts = NonZeroU32::new(2).unwrap();
    }
    let run_id = commit(&store, &request, "alice");
    let only_lost = [SchedulerNode::with_execution_slots("worker-a", 2)];
    for _ in 0..2 {
        let command = store.reserve_next(&only_lost).unwrap().unwrap();
        store.ack_dispatch(&command).unwrap();
    }

    store.declare_node_lost("worker-a").unwrap();
    let details = store.get_run(&run_id).unwrap().unwrap();
    assert!(details.jobs.iter().all(|job| job.state != "retrying"));
    assert_eq!(details.summary.state.as_str(), "failed");
    assert!(
        store
            .reserve_next(&[SchedulerNode::with_execution_slots("worker-b", 2)])
            .unwrap()
            .is_none()
    );
}

fn state(store: &RunStore, run_id: &str) -> String {
    store.get_run(run_id).unwrap().unwrap().jobs[0]
        .state
        .clone()
}

fn success() -> takd::AttemptCompletion {
    takd::AttemptCompletion::Succeeded {
        terminal_digest: "c".repeat(64),
    }
}
