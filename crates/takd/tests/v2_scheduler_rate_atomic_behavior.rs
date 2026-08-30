use std::num::{NonZeroU32, NonZeroU64};
use std::sync::{Arc, Barrier};

use tak_core::v2::{DefinitionScope, LimiterClaim, LimiterDefinition, RunSubmission};
use takd::{AttemptCompletion, RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn exhausted_second_bucket_does_not_partially_consume_the_first() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    commit(&store, &rate_run("exhaust-b", 1, &["b"]), "alice");
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 3)];
    let first = store.reserve_next(&nodes).unwrap().unwrap();
    finish(&store, &first);
    commit(&store, &rate_run("both", 1, &["a", "b"]), "bob");
    let a_only = commit(&store, &rate_run("a-only", 1, &["a"]), "carol");
    assert_eq!(store.reserve_next(&nodes).unwrap().unwrap().run_id, a_only);
}

#[test]
fn concurrent_reservations_consume_one_burst_token_exactly_once() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    commit(&store, &rate_run("race", 4, &["a"]), "alice");
    let nodes = Arc::new([SchedulerNode::with_execution_slots("worker-a", 4)]);
    let barrier = Arc::new(Barrier::new(4));
    let threads = (0..4)
        .map(|_| {
            let store = store.clone();
            let nodes = Arc::clone(&nodes);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                store.reserve_next(nodes.as_slice()).unwrap()
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        threads
            .into_iter()
            .filter_map(|thread| thread.join().unwrap())
            .count(),
        1
    );
}

fn rate_run(key: &str, jobs: usize, claims: &[&str]) -> RunSubmission {
    let mut request = independent_jobs(key, jobs);
    request.run.limiter_definitions = ["a", "b"]
        .map(|name| LimiterDefinition::RateLimit {
            name: name.into(),
            scope: DefinitionScope::Project,
            scope_key: None,
            burst: NonZeroU32::MIN,
            refill_millis_per_second: NonZeroU64::MIN,
        })
        .into();
    for job in &mut request.run.jobs {
        job.limiter_claims = claims
            .iter()
            .map(|name| LimiterClaim {
                name: (*name).into(),
                amount_millis: NonZeroU64::new(1_000).unwrap(),
            })
            .collect();
    }
    request
}

fn finish(store: &RunStore, command: &takd::DispatchCommand) {
    store
        .complete_attempt(
            command,
            AttemptCompletion::Succeeded {
                terminal_digest: "a".repeat(64),
            },
        )
        .unwrap();
}
