use tak_proto::local_daemon::v2::RunLifecycleState;

use crate::support::{v2_mixed_outputs::V2MixedOutputCluster, v2_run};

#[tokio::test]
async fn identical_local_and_remote_branch_outputs_coalesce_for_remote_consumption() {
    let cluster = V2MixedOutputCluster::start().await;
    let run_id = cluster
        .run(&v2_run::mixed_outputs::identical("mixed-identical"))
        .await;

    let run = cluster.store.get_run(&run_id).unwrap().unwrap();
    assert_eq!(run.summary.state, RunLifecycleState::Succeeded);
    assert_eq!(
        run.jobs
            .iter()
            .map(|job| (job.job_id.as_str(), job.node_id.as_deref()))
            .collect::<Vec<_>>(),
        [
            ("job-left", Some("local")),
            ("job-right", Some("builder-a")),
            ("job-consumer", Some("builder-a")),
        ]
    );
    assert_eq!(cluster.output(&run_id, "shared/value.txt"), b"same");
    assert_eq!(cluster.output(&run_id, "dist/result.txt"), b"consumed");
    assert_eq!(cluster.remote_attempt_count("job-right"), 1);
    assert_eq!(cluster.remote_attempt_count("job-consumer"), 1);
}
