use rusqlite::{Connection, params};
use tak_core::v2::Affinity;
use takd::{RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn v3_upgrade_refuses_to_guess_a_conflicting_active_hard_home() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let mut request = independent_jobs("affinity-conflict", 2);
    let affinity = Affinity::require_same_node("build").unwrap();
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
    store.reserve_next(&nodes).unwrap().unwrap();
    let conflicting = store.reserve_next(&nodes).unwrap().unwrap();
    drop(store);
    let connection = Connection::open(&db).unwrap();
    connection
        .execute(
            "UPDATE run_attempts SET node_id = 'worker-b' \
             WHERE run_id = ?1 AND job_id = ?2",
            params![conflicting.run_id, conflicting.job_id],
        )
        .unwrap();
    connection
        .execute_batch(
            "DROP TABLE run_affinity_bindings; \
             UPDATE run_schema_version SET version = 3",
        )
        .unwrap();
    drop(connection);

    let error = match RunStore::with_db_path(db) {
        Ok(_) => panic!("conflicting v3 hard home was guessed"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("conflicting hard affinity"));
}
