use std::collections::BTreeMap;

use tak_core::v2::{PlacementCandidate, PlacementKind, RunSubmission};
use tak_proto::local_daemon::v2::RunLifecycleState;

use crate::support::{
    v2_cluster::{Origin, WorkerSpec, attempt_count, cluster_lock, peers},
    v2_run::{scheduler::commit, scheduler::independent_jobs},
    worker_http::start_server_for_node,
};

const ONION: &str = "http://pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd.onion";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn daemon_executes_direct_and_tor_labelled_jobs_through_worker_v2() {
    let _cluster = cluster_lock().await;
    let direct = start_server_for_node("worker-a").await;
    let tor = start_server_for_node("worker-b").await;
    let workers = [
        WorkerSpec::direct("worker-a", direct.addr, 1),
        WorkerSpec::tor("worker-b", ONION, 1),
    ];
    let broker = takd::TorBroker::for_direct_dial(tor.addr.to_string());
    let origin = Origin::start(peers(&workers), broker).await;
    let mut request = independent_jobs("cluster-transports", 2);
    for (job, worker) in request.run.jobs.iter_mut().zip(&workers) {
        job.placement_candidates = vec![PlacementCandidate {
            node_id: worker.node_id.clone(),
            kind: PlacementKind::Remote,
            transport: Some(worker.transport.clone()),
            reason: "healthy protocol-v2 worker".into(),
            tier: 0,
            requirements: None,
        }];
    }
    let request = RunSubmission::new(
        request.idempotency_key,
        request.run,
        request.environment_values,
    )
    .unwrap();
    let run_id = commit(&origin.store, &request, "alice");

    origin.wait_for_terminal(&run_id).await;

    let details = origin.store.get_run(&run_id).unwrap().unwrap();
    let nodes = details
        .jobs
        .iter()
        .map(|job| (job.job_id.as_str(), job.node_id.as_deref().unwrap()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(details.summary.state, RunLifecycleState::Succeeded);
    assert_eq!(
        nodes,
        BTreeMap::from([("job-0", "worker-a"), ("job-1", "worker-b")])
    );
    assert_eq!(attempt_count(&direct), 1);
    assert_eq!(attempt_count(&tor), 1);
}
