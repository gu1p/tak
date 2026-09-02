use tak_proto::local_daemon::v2::RunLifecycleState;

use crate::support::{v2_mixed_outputs::V2MixedOutputCluster, v2_run};

#[tokio::test]
async fn remote_local_remote_chain_receives_all_transitive_declared_outputs() {
    let cluster = V2MixedOutputCluster::start().await;
    let run_id = cluster
        .run(&v2_run::mixed_outputs::transitive("mixed-transitive"))
        .await;

    let run = cluster.store.get_run(&run_id).unwrap().unwrap();
    assert_eq!(run.summary.state, RunLifecycleState::Succeeded);
    assert_eq!(
        run.jobs
            .iter()
            .map(|job| (job.job_id.as_str(), job.node_id.as_deref()))
            .collect::<Vec<_>>(),
        [
            ("job-ancestor", Some("builder-a")),
            ("job-middle", Some("local")),
            ("job-consumer", Some("builder-a")),
        ]
    );
    assert_eq!(cluster.output(&run_id, "graph/ancestor.txt"), b"ancestor");
    assert_eq!(cluster.output(&run_id, "graph/middle.txt"), b"middle");
    assert_eq!(
        cluster.output(&run_id, "dist/result.txt"),
        b"ancestor+middle"
    );
    assert_eq!(cluster.remote_attempt_count("job-ancestor"), 1);
    assert_eq!(cluster.remote_attempt_count("job-consumer"), 1);
}
