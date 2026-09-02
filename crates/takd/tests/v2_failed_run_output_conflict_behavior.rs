use std::num::NonZeroU32;

use tak_core::v2::RunSubmission;
use tak_proto::local_daemon::v2::RunLifecycleState;
use takd::{RunStore, SchedulerNode};

use crate::support::v2_run::{
    final_outputs::{failed, seed_value, succeeded},
    output_conflicts::final_sink,
    scheduler::commit,
};

#[test]
fn failed_run_output_conflicts_are_persisted_and_reported() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let mut request = final_sink("failed-output-conflict");
    let mut failure = request.run.tasks[0].clone();
    failure.task_id = "//:fail".into();
    failure.job_id = "job-fail".into();
    failure.outputs.clear();
    let mut failure_job = request.run.jobs[0].clone();
    failure_job.job_id = failure.job_id.clone();
    failure_job.task_ids = vec![failure.task_id.clone()];
    failure_job.retry.max_attempts = NonZeroU32::MIN;
    request.run.tasks.push(failure);
    request.run.jobs.push(failure_job);
    request.run.targets.push("//:fail".into());
    request.run.options.keep_going = true;
    request.run.options.max_parallel_jobs = NonZeroU32::new(3).unwrap();
    let request = RunSubmission::new(
        request.idempotency_key,
        request.run,
        request.environment_values,
    )
    .unwrap();
    let run_id = commit(&store, &request, "alice");
    let nodes = [SchedulerNode::with_execution_slots("local", 3)];
    let commands = (0..3)
        .map(|_| store.reserve_next(&nodes).unwrap().unwrap())
        .collect::<Vec<_>>();
    for command in commands.iter().filter(|item| item.job_id != "job-fail") {
        let producer = request
            .run
            .jobs
            .iter()
            .find(|job| job.job_id == command.job_id)
            .unwrap()
            .task_ids[0]
            .as_str();
        seed_value(
            &db,
            command,
            producer,
            "shared/value.txt",
            producer.as_bytes(),
        );
        store.complete_attempt(command, succeeded()).unwrap();
    }
    let failure = commands
        .iter()
        .find(|item| item.job_id == "job-fail")
        .unwrap();
    store.complete_attempt(failure, failed()).unwrap();

    assert_eq!(
        store.summary(&run_id).unwrap().unwrap().state,
        RunLifecycleState::Failed
    );
    let error = store.output_manifest(&run_id).unwrap_err().to_string();
    assert!(error.contains("independent producers conflict"), "{error}");
}
