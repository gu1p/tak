use std::collections::BTreeMap;

use tak_proto::local_daemon::v2::RunLifecycleState;

use crate::support::{
    v2_cluster::{Origin, WorkerSpec, attempt_count, cluster_lock, peers},
    v2_run::{scheduler::commit, scheduler::independent_jobs},
    worker_http::start_server_for_node,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn daemon_executes_ten_round_robin_jobs_five_per_v2_worker() {
    let _cluster = cluster_lock().await;
    let first = start_server_for_node("worker-a").await;
    let second = start_server_for_node("worker-b").await;
    let workers = [
        WorkerSpec::direct("worker-a", first.addr, 5),
        WorkerSpec::direct("worker-b", second.addr, 5),
    ];
    let origin = Origin::start(peers(&workers), takd::TorBroker::new()).await;
    let request = independent_jobs("cluster-round-robin", 10);
    let run_id = commit(&origin.store, &request, "alice");

    origin.wait_for_terminal(&run_id).await;

    let details = origin.store.get_run(&run_id).unwrap().unwrap();
    let counts = details
        .jobs
        .iter()
        .fold(BTreeMap::new(), |mut counts, job| {
            *counts.entry(job.node_id.as_deref().unwrap()).or_insert(0) += 1;
            counts
        });
    assert_eq!(details.summary.state, RunLifecycleState::Succeeded);
    assert_eq!(counts, BTreeMap::from([("worker-a", 5), ("worker-b", 5)]));
    assert_eq!(attempt_count(&first), 5);
    assert_eq!(attempt_count(&second), 5);
}
