use std::num::NonZeroU32;

use rusqlite::params;
use takd::{ResultAcceptance, RunStore, SchedulerNode, UnknownOutcomeResolution};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn a_new_reservation_clears_the_previous_attempt_cache_result() {
    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db_path.clone()).unwrap();
    let mut request = independent_jobs("cache-reset-on-reservation", 1);
    request.run.tasks[0].idempotent = true;
    request.run.jobs[0].idempotent = true;
    request.run.jobs[0].retry.max_attempts = NonZeroU32::new(2).unwrap();
    let run_id = commit(&store, &request, "uid:1");

    let nodes = [SchedulerNode::with_execution_slots("worker-a", 1)];
    let first = store.reserve_next(&nodes).unwrap().unwrap();
    assert_eq!(
        store.ack_dispatch(&first).unwrap(),
        ResultAcceptance::Applied
    );
    rusqlite::Connection::open(&db_path)
        .unwrap()
        .execute(
            "UPDATE run_jobs SET cache = ?3 WHERE run_id = ?1 AND job_id = ?2",
            params![run_id, first.job_id, "hit"],
        )
        .unwrap();
    assert_eq!(
        store.get_run(&run_id).unwrap().unwrap().jobs[0]
            .cache
            .as_deref(),
        Some("hit")
    );

    assert_eq!(
        store.resolve_unknown_attempt(&first).unwrap(),
        UnknownOutcomeResolution::Retrying
    );
    assert_eq!(store.get_run(&run_id).unwrap().unwrap().jobs[0].cache, None);
    rusqlite::Connection::open(&db_path)
        .unwrap()
        .execute(
            "UPDATE run_jobs SET cache = ?3 WHERE run_id = ?1 AND job_id = ?2",
            params![run_id, first.job_id, "miss"],
        )
        .unwrap();
    let second = store.reserve_next(&nodes).unwrap().unwrap();

    assert_eq!(second.authored_attempt, 2);
    assert_eq!(store.get_run(&run_id).unwrap().unwrap().jobs[0].cache, None);
}
