use rusqlite::Connection;
use takd::{AttemptCompletion, RunStore, SchedulerNode};

use crate::support::v2_run::{
    constraints::{project_process_cap, project_rate_limit, project_resource},
    scheduler::{commit, independent_jobs},
};

#[test]
fn resource_and_process_cap_leases_release_on_completion() {
    assert_completion_release(project_resource, 600);
    assert_completion_release(|request, _| project_process_cap(request), 1_000);
}

#[test]
fn a_token_bucket_survives_completion_and_restart_until_refilled() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    for (key, owner) in [("rate-a", "alice"), ("rate-b", "bob")] {
        commit(&store, &project_rate_limit(independent_jobs(key, 1)), owner);
    }
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 2)];
    let first = store.reserve_next(&nodes).unwrap().unwrap();
    finish(&store, &first);
    Connection::open(&db)
        .unwrap()
        .execute("DELETE FROM run_attempts", [])
        .unwrap();
    drop(store);
    let restored = RunStore::with_db_path(db.clone()).unwrap();
    assert!(restored.reserve_next(&nodes).unwrap().is_none());
    Connection::open(&db)
        .unwrap()
        .execute(
            "UPDATE scheduler_rate_buckets SET refilled_at_ms = 0, available_micros = 0",
            [],
        )
        .unwrap();
    assert!(restored.reserve_next(&nodes).unwrap().is_some());
}

fn assert_completion_release(
    decorate: impl Fn(tak_core::v2::RunSubmission, u64) -> tak_core::v2::RunSubmission,
    amount: u64,
) {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    for (key, owner) in [("kind-a", "alice"), ("kind-b", "bob")] {
        commit(&store, &decorate(independent_jobs(key, 1), amount), owner);
    }
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 2)];
    let first = store.reserve_next(&nodes).unwrap().unwrap();
    assert!(store.reserve_next(&nodes).unwrap().is_none());
    finish(&store, &first);
    assert!(store.reserve_next(&nodes).unwrap().is_some());
}

fn finish(store: &RunStore, command: &takd::DispatchCommand) {
    store
        .complete_attempt(
            command,
            AttemptCompletion::Succeeded {
                terminal_digest: "4".repeat(64),
            },
        )
        .unwrap();
}
