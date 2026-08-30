use tak_core::v2::{Affinity, RemoteSelection};
use takd::{AttemptCompletion, RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn soft_home_survives_fallback_completion_and_restart() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let mut request = independent_jobs("soft-home", 3);
    let affinity = Affinity::prefer_same_node("build").unwrap();
    for task in &mut request.run.tasks {
        task.affinity = Some(affinity.clone());
    }
    for job in &mut request.run.jobs {
        job.affinity = Some(affinity.clone());
        job.placement_policy.selection = RemoteSelection::Balanced;
    }
    request.run.jobs[1].placement_candidates.reverse();
    request.run.jobs[2].placement_candidates.reverse();
    commit(&store, &request, "alice");
    let available = [
        SchedulerNode::with_execution_slots("worker-a", 10),
        SchedulerNode::with_execution_slots("worker-b", 10),
    ];
    let first = store.reserve_next(&available).unwrap().unwrap();
    assert_eq!(first.node_id, "worker-a");
    finish(&store, &first, '6');

    let saturated_home = [
        SchedulerNode::with_execution_slots("worker-a", 10).with_execution_usage(9),
        SchedulerNode::with_execution_slots("worker-b", 10),
    ];
    let fallback = store.reserve_next(&saturated_home).unwrap().unwrap();
    assert_eq!(fallback.node_id, "worker-b");
    finish(&store, &fallback, '7');
    drop(store);

    let restored = RunStore::with_db_path(db).unwrap();
    let third = restored.reserve_next(&available).unwrap().unwrap();
    assert_eq!(third.node_id, "worker-a");
}

fn finish(store: &RunStore, command: &takd::DispatchCommand, digest: char) {
    store
        .complete_attempt(
            command,
            AttemptCompletion::Succeeded {
                terminal_digest: digest.to_string().repeat(64),
            },
        )
        .unwrap();
}
