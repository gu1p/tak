use rusqlite::Connection;
use takd::{RunStore, SchedulerNode};

use crate::support::v2_run::{
    constraints::project_rate_limit,
    scheduler::{commit, independent_jobs},
};

#[test]
fn failed_attempt_reservation_rolls_back_its_rate_token() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    commit(
        &store,
        &project_rate_limit(independent_jobs("rate-rollback", 1)),
        "alice",
    );
    let connection = Connection::open(&db).unwrap();
    connection
        .execute_batch(
            "CREATE TRIGGER abort_attempt BEFORE INSERT ON run_attempts \
             BEGIN SELECT RAISE(ABORT, 'forced attempt failure'); END;",
        )
        .unwrap();
    let nodes = [SchedulerNode::with_execution_slots("worker-a", 1)];
    let error = store.reserve_next(&nodes).unwrap_err().to_string();
    assert!(error.contains("forced attempt failure"), "{error}");

    connection
        .execute_batch("DROP TRIGGER abort_attempt")
        .unwrap();
    assert!(store.reserve_next(&nodes).unwrap().is_some());
}
