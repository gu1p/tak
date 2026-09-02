use std::num::NonZeroU32;

use takd::{
    AttemptCompletion, NodeLossResolution, ResultAcceptance, RunStore, SchedulerNode,
    UnknownOutcomeResolution,
};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn missing_idempotent_output_commit_retries_and_fences_the_late_result() {
    let (_temp, store, run_id) = committed("missing-output-commit", 1);
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 1)];
    let first = store.reserve_next(&nodes).unwrap().unwrap();
    store.begin_output_commit(&first).unwrap();

    assert_eq!(
        store.resolve_unknown_attempt(&first).unwrap(),
        UnknownOutcomeResolution::Retrying
    );
    let second = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(second.authored_attempt, 2);
    assert_ne!(second.fencing_token, first.fencing_token);
    assert_eq!(
        store.complete_attempt(&first, success("a")).unwrap(),
        ResultAcceptance::Stale
    );
    assert_eq!(state(&store, &run_id, &second.job_id), "transferring");
}

#[test]
fn node_loss_settles_every_output_committing_attempt() {
    let (_temp, store, run_id) = committed("lost-output-commit", 2);
    let only_lost = [SchedulerNode::with_execution_slots("worker-a", 2)];
    let retryable = store.reserve_next(&only_lost).unwrap().unwrap();
    let unsafe_attempt = store.reserve_next(&only_lost).unwrap().unwrap();
    store.begin_output_commit(&retryable).unwrap();
    store.begin_output_commit(&unsafe_attempt).unwrap();

    assert_eq!(
        store.declare_node_lost("worker-a").unwrap(),
        NodeLossResolution::Applied
    );
    assert_eq!(state(&store, &run_id, &retryable.job_id), "retrying");
    assert_eq!(state(&store, &run_id, &unsafe_attempt.job_id), "failed");
    let retry = store
        .reserve_next(&[SchedulerNode::with_execution_slots("worker-b", 1)])
        .unwrap()
        .unwrap();
    assert_eq!(retry.job_id, retryable.job_id);
    assert_eq!(retry.authored_attempt, 2);
    assert_eq!(
        store.complete_attempt(&retryable, success("b")).unwrap(),
        ResultAcceptance::Stale
    );
    assert_eq!(
        store.complete_attempt(&unsafe_attempt, success("c")).unwrap(),
        ResultAcceptance::Stale
    );
}

fn committed(key: &str, count: usize) -> (tempfile::TempDir, RunStore, String) {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = independent_jobs(key, count);
    request.run.options.keep_going = true;
    for job in &mut request.run.jobs {
        job.retry.max_attempts = NonZeroU32::new(2).unwrap();
    }
    if count > 1 {
        request.run.tasks[1].idempotent = false;
        request.run.jobs[1].idempotent = false;
    }
    let run_id = commit(&store, &request, "alice");
    (temp, store, run_id)
}

fn state(store: &RunStore, run_id: &str, job_id: &str) -> String {
    store
        .get_run(run_id)
        .unwrap()
        .unwrap()
        .jobs
        .into_iter()
        .find(|job| job.job_id == job_id)
        .unwrap()
        .state
}

fn success(seed: &str) -> AttemptCompletion {
    AttemptCompletion::Succeeded {
        terminal_digest: seed.repeat(64),
    }
}
