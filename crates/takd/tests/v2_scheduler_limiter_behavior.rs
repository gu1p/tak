use tak_core::v2::HoldMode;
use takd::{AttemptCompletion, RunStore, SchedulerNode};

use crate::support::v2_run::{
    constraints::project_lock,
    scheduler::{commit, independent_jobs},
};

#[test]
fn at_start_releases_on_acceptance_while_during_waits_for_completion() {
    assert_release_boundary(HoldMode::AtStart, true);
    assert_release_boundary(HoldMode::During, false);
}

fn assert_release_boundary(hold: HoldMode, released_on_ack: bool) {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    for (key, owner) in [("limit-a", "alice"), ("limit-b", "bob")] {
        let request = project_lock(independent_jobs(key, 1), hold);
        commit(&store, &request, owner);
    }
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 2)];
    let first = store.reserve_next(&nodes).unwrap().unwrap();
    assert!(store.reserve_next(&nodes).unwrap().is_none());
    store.ack_dispatch(&first).unwrap();
    if released_on_ack {
        assert!(store.reserve_next(&nodes).unwrap().is_some());
        return;
    }
    assert!(store.reserve_next(&nodes).unwrap().is_none());
    let completion = AttemptCompletion::Succeeded {
        terminal_digest: "2".repeat(64),
    };
    store.complete_attempt(&first, completion).unwrap();
    assert!(store.reserve_next(&nodes).unwrap().is_some());
}
