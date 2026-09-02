use rusqlite::Connection;
use tak_core::v2::{Affinity, RemoteSelection};
use takd::{AttemptCompletion, NodeLossResolution, RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn a_soft_home_moves_only_after_declared_loss_and_the_new_home_survives_restart() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let mut request = independent_jobs("soft-node-loss", 3);
    let soft = Affinity::prefer_same_node("build").unwrap();
    for task in &mut request.run.tasks {
        task.affinity = Some(soft.clone());
    }
    for job in &mut request.run.jobs {
        job.affinity = Some(soft.clone());
        job.placement_policy.selection = RemoteSelection::Balanced;
    }
    let run_id = commit(&store, &request, "alice");
    let nodes = [
        SchedulerNode::with_execution_slots("worker-a", 3),
        SchedulerNode::with_execution_slots("worker-b", 3),
    ];
    let first = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(first.node_id, "worker-a");
    finish(&store, &first, '1');
    assert_eq!(
        store.declare_node_lost("worker-a").unwrap(),
        NodeLossResolution::Applied
    );
    drop(store);

    let restored = RunStore::with_db_path(db.clone()).unwrap();
    let survivors = [SchedulerNode::with_execution_slots("worker-b", 3)];
    let second = restored.reserve_next(&survivors).unwrap().unwrap();
    assert_eq!(second.node_id, "worker-b");
    let stored_home: String = Connection::open(&db)
        .unwrap()
        .query_row(
            "SELECT node_id FROM run_affinity_bindings WHERE run_id = ?1 AND affinity_group = 'build'",
            [&run_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored_home, "worker-b");
    finish(&restored, &second, '2');
    drop(restored);

    let restored = RunStore::with_db_path(db).unwrap();
    let third = restored.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(third.node_id, "worker-b");
    assert_eq!(third.run_id, run_id);
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
