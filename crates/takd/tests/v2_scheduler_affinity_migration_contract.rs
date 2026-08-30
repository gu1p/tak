use rusqlite::Connection;
use tak_core::v2::Affinity;
use takd::{RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn v3_upgrade_restores_the_first_hard_affinity_reservation() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let mut request = independent_jobs("affinity-upgrade", 2);
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
    assert_eq!(
        store.reserve_next(&nodes).unwrap().unwrap().node_id,
        "worker-a"
    );
    drop(store);
    let connection = Connection::open(&db).unwrap();
    connection
        .execute_batch(
            "DROP TABLE IF EXISTS run_affinity_bindings; \
             UPDATE run_schema_version SET version = 3",
        )
        .unwrap();
    drop(connection);

    let restored = RunStore::with_db_path(db).unwrap();
    assert_eq!(
        restored.reserve_next(&nodes).unwrap().unwrap().node_id,
        "worker-a"
    );
}
