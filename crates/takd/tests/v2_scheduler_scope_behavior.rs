use tak_core::v2::{DefinitionScope, HoldMode};
use takd::{RunStore, SchedulerNode};

use crate::support::v2_run::{
    constraints::scoped_lock,
    scheduler::{commit, independent_jobs},
};

#[test]
fn run_scope_isolated_each_run() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    for key in ["run-a", "run-b"] {
        let request = scoped_lock(
            independent_jobs(key, 1),
            DefinitionScope::Run,
            None,
            HoldMode::During,
        );
        commit(&store, &request, "alice");
    }
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 2)];
    assert!(store.reserve_next(&nodes).unwrap().is_some());
    assert!(store.reserve_next(&nodes).unwrap().is_some());
}

#[test]
fn submitter_scope_is_shared_only_by_the_same_submitter() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let alice = submitter_run("submitter-a");
    let alice_waiter = submitter_run("submitter-b");
    let bob = submitter_run("submitter-c");
    commit(&store, &alice, "alice");
    commit(&store, &alice_waiter, "alice");
    let bob_id = commit(&store, &bob, "bob");
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 3)];
    assert!(store.reserve_next(&nodes).unwrap().is_some());
    assert_eq!(store.reserve_next(&nodes).unwrap().unwrap().run_id, bob_id);
}

#[test]
fn node_scope_can_use_a_different_node_without_overbooking_the_first() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    for (key, owner) in [("node-a", "alice"), ("node-b", "bob")] {
        let request = scoped_lock(
            independent_jobs(key, 1),
            DefinitionScope::Node,
            None,
            HoldMode::During,
        );
        commit(&store, &request, owner);
    }
    let nodes = [
        SchedulerNode::with_execution_slots("worker-a", 2),
        SchedulerNode::with_execution_slots("worker-b", 2),
    ];
    assert_eq!(
        store.reserve_next(&nodes).unwrap().unwrap().node_id,
        "worker-a"
    );
    assert_eq!(
        store.reserve_next(&nodes).unwrap().unwrap().node_id,
        "worker-b"
    );
}

fn submitter_run(key: &str) -> tak_core::v2::RunSubmission {
    scoped_lock(
        independent_jobs(key, 1),
        DefinitionScope::Submitter,
        None,
        HoldMode::During,
    )
}
