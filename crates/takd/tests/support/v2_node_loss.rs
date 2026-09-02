use std::num::NonZeroU32;

use tak_core::v2::{Affinity, Session, SessionReuse};
use takd::{ResultAcceptance, RunStore, SchedulerNode};

use super::v2_run::scheduler::{commit, independent_jobs};

mod peers;
pub use peers::{peer_manager, restarted_peer_manager, snapshot};

pub struct SeededRuns {
    pub retry: String,
    pub unsafe_run: String,
    pub hard: String,
    pub soft: String,
}

pub fn seed(store: &RunStore) -> SeededRuns {
    let retry = retryable("runtime-loss-retry");
    let retry = reserve_run(store, &retry, "retry");
    let mut unsafe_work = retryable("runtime-loss-unsafe");
    unsafe_work.run.tasks[0].idempotent = false;
    unsafe_work.run.jobs[0].idempotent = false;
    let unsafe_run = reserve_run(store, &unsafe_work, "unsafe");
    let mut hard_work = retryable("runtime-loss-hard");
    let hard_affinity = Affinity::require_same_node("hard").unwrap();
    hard_work.run.tasks[0].affinity = Some(hard_affinity.clone());
    hard_work.run.jobs[0].affinity = Some(hard_affinity.clone());
    hard_work.run.jobs[0].session = Some(
        Session::new(
            "shared",
            SessionReuse::shared_workspace(1).unwrap(),
            Some(hard_affinity),
        )
        .unwrap(),
    );
    let hard = reserve_run(store, &hard_work, "hard");
    let mut soft_work = retryable("runtime-loss-soft");
    let soft_affinity = Affinity::prefer_same_node("soft").unwrap();
    soft_work.run.tasks[0].affinity = Some(soft_affinity.clone());
    soft_work.run.jobs[0].affinity = Some(soft_affinity);
    let soft = reserve_run(store, &soft_work, "soft");
    SeededRuns {
        retry,
        unsafe_run,
        hard,
        soft,
    }
}

fn retryable(key: &str) -> tak_core::v2::RunSubmission {
    let mut work = independent_jobs(key, 1);
    work.run.jobs[0].retry.max_attempts = NonZeroU32::new(2).unwrap();
    work
}

fn reserve_run(store: &RunStore, work: &tak_core::v2::RunSubmission, owner: &str) -> String {
    let run_id = commit(store, work, owner);
    let command = store
        .reserve_next(&[SchedulerNode::with_execution_slots("worker-a", 8)])
        .unwrap()
        .unwrap();
    assert_eq!(
        store.ack_dispatch(&command).unwrap(),
        ResultAcceptance::Applied
    );
    run_id
}
