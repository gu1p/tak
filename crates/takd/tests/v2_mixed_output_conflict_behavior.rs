use tak_proto::local_daemon::v2::{RunEventKind, RunLifecycleState};

use crate::support::{v2_mixed_outputs::V2MixedOutputCluster, v2_run};

#[tokio::test]
async fn differing_local_and_remote_branch_outputs_fail_before_remote_consumption() {
    let cluster = V2MixedOutputCluster::start().await;
    let run_id = cluster
        .run(&v2_run::mixed_outputs::conflicting("mixed-conflict"))
        .await;

    let run = cluster.store.get_run(&run_id).unwrap().unwrap();
    assert_eq!(run.summary.state, RunLifecycleState::Failed);
    assert_eq!(
        run.jobs
            .iter()
            .map(|job| (job.job_id.as_str(), job.state.as_str()))
            .collect::<Vec<_>>(),
        [
            ("job-left", "succeeded"),
            ("job-right", "succeeded"),
            ("job-consumer", "failed"),
        ]
    );
    assert_eq!(cluster.remote_attempt_count("job-right"), 1);
    assert_eq!(cluster.remote_attempt_count("job-consumer"), 0);
    let conflicts = cluster
        .store
        .events_after(&run_id, 0)
        .unwrap()
        .into_iter()
        .filter(|event| {
            event.kind == RunEventKind::Failed
                && event.message.contains("independent producers conflict")
        })
        .count();
    assert_eq!(conflicts, 1);
}
