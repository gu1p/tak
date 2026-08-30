use tak_core::v2::JobEdge;
use takd::{AttemptCompletion, ResultAcceptance, RunStore, SchedulerNode};

use crate::support::v2_run::scheduler::{commit, independent_jobs};

#[test]
fn an_older_ready_job_precedes_a_newly_unblocked_lower_ordinal_job() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = independent_jobs("ready-age", 3);
    request.run.tasks[0].dependencies = vec![request.run.tasks[2].task_id.clone()];
    request.run.job_edges = vec![JobEdge {
        dependency_job_id: request.run.jobs[2].job_id.clone(),
        dependent_job_id: request.run.jobs[0].job_id.clone(),
    }];
    request.run.jobs[0].placement_candidates.truncate(1);
    request.run.jobs[2].placement_candidates.truncate(1);
    request.run.jobs[1].placement_candidates.remove(0);
    commit(&store, &request, "uid:1");

    let worker_a = SchedulerNode::with_execution_slots("worker-a", 1);
    let root = store
        .reserve_next(std::slice::from_ref(&worker_a))
        .unwrap()
        .unwrap();
    assert_eq!(root.job_id, "job-2");
    assert_eq!(
        store
            .complete_attempt(
                &root,
                AttemptCompletion::Succeeded {
                    terminal_digest: "d".repeat(64)
                },
            )
            .unwrap(),
        ResultAcceptance::Applied
    );
    rusqlite::Connection::open(temp.path().join("takd.sqlite"))
        .unwrap()
        .execute(
            "UPDATE run_jobs SET next_eligible_at_ms = (SELECT next_eligible_at_ms FROM run_jobs WHERE job_id = 'job-0') WHERE job_id = 'job-1'",
            [],
        )
        .unwrap();

    let next = store
        .reserve_next(&[worker_a, SchedulerNode::with_execution_slots("worker-b", 1)])
        .unwrap()
        .unwrap();
    assert_eq!(next.job_id, "job-1");
}
