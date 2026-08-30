use tak_core::v2::Affinity;
use takd::{AttemptCompletion, RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn hard_affinity_home_survives_completion_and_daemon_restart() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let mut request = independent_jobs("hard-affinity", 2);
    let affinity = Affinity::RequireSameNode {
        group: "build".into(),
    };
    for task in &mut request.run.tasks {
        task.affinity = Some(affinity.clone());
    }
    for job in &mut request.run.jobs {
        job.affinity = Some(affinity.clone());
    }
    commit(&store, &request, "alice");
    let nodes = [
        SchedulerNode::with_execution_slots("worker-a", 2),
        SchedulerNode::with_execution_slots("worker-b", 2),
    ];
    let first = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(first.node_id, "worker-a");
    let completion = AttemptCompletion::Succeeded {
        terminal_digest: "3".repeat(64),
    };
    store.complete_attempt(&first, completion).unwrap();
    drop(store);

    let restored = RunStore::with_db_path(db).unwrap();
    let second = restored.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(second.node_id, "worker-a");
}
