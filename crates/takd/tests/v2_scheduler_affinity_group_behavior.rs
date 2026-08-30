use tak_core::v2::Affinity;
use takd::{AttemptCompletion, RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn a_soft_first_member_cannot_bind_outside_its_hard_group_intersection() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = independent_jobs("mixed-affinity", 2);
    request.run.tasks[0].affinity = Some(Affinity::prefer_same_node("build").unwrap());
    request.run.jobs[0].affinity = request.run.tasks[0].affinity.clone();
    request.run.jobs[0].placement_candidates.reverse();
    request.run.tasks[1].affinity = Some(Affinity::require_same_node("build").unwrap());
    request.run.jobs[1].affinity = request.run.tasks[1].affinity.clone();
    request.run.jobs[1].placement_candidates.truncate(1);
    commit(&store, &request, "alice");
    let nodes = [
        SchedulerNode::with_execution_slots("worker-a", 2),
        SchedulerNode::with_execution_slots("worker-b", 2),
    ];
    let first = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(first.node_id, "worker-a");
    finish(&store, &first);
    assert_eq!(
        store.reserve_next(&nodes).unwrap().unwrap().node_id,
        "worker-a"
    );
}

#[test]
fn affinity_group_homes_are_scoped_to_the_run() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    for (key, reverse, owner) in [("home-a", false, "alice"), ("home-b", true, "bob")] {
        let mut request = independent_jobs(key, 1);
        let hard = Affinity::require_same_node("build").unwrap();
        request.run.tasks[0].affinity = Some(hard.clone());
        request.run.jobs[0].affinity = Some(hard);
        if reverse {
            request.run.jobs[0].placement_candidates.reverse();
        }
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

fn finish(store: &RunStore, command: &takd::DispatchCommand) {
    store
        .complete_attempt(
            command,
            AttemptCompletion::Succeeded {
                terminal_digest: "9".repeat(64),
            },
        )
        .unwrap();
}
