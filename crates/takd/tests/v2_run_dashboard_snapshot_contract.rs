use crate::support::v2_run::{
    constraints::project_queue,
    scheduler::{commit, independent_jobs},
};
use tak_proto::local_daemon::v2::RunEventKind;
use takd::{RunStore, SchedulerNode};

#[test]
fn persisted_run_details_and_events_project_truthful_dashboard_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let database = temp.path().join("takd.sqlite");
    let mut request = project_queue(independent_jobs("dashboard-snapshot", 2), 1);
    for job in &mut request.run.jobs {
        job.queue = Some("shared".into());
    }
    let store = RunStore::with_db_path(database.clone()).unwrap();
    let run_id = commit(&store, &request, "alice");
    drop(store);

    let restored = RunStore::with_db_path(database).unwrap();
    let command = restored
        .reserve_next(&[SchedulerNode::with_execution_slots("worker-a", 1)])
        .unwrap()
        .unwrap();
    let details = restored.get_run(&run_id).unwrap().unwrap();
    let reservation = restored
        .events_after(&run_id, 0)
        .unwrap()
        .into_iter()
        .find(|event| event.kind == RunEventKind::Transferring)
        .unwrap();

    assert_eq!(details.jobs.len(), 2);
    assert_eq!(details.max_parallel_jobs, 2);
    for job in &details.jobs {
        assert_eq!(job.queue.as_deref(), Some("shared"));
        assert_eq!(job.placement_candidate_node_ids, ["worker-a", "worker-b"]);
    }
    assert_eq!(reservation.authored_attempt, Some(command.authored_attempt));
    assert_eq!(
        details
            .jobs
            .iter()
            .find(|job| job.job_id == command.job_id)
            .unwrap()
            .attempt,
        command.authored_attempt
    );
}
