use tak_core::v2::RunSubmission;
use takd::{
    AttemptDispatch, AttemptTransport, RemoteAttemptTransport, RunStore, TorBroker,
};

use crate::support::{v2_remote_origin, v2_run, worker_http::start_server};

#[tokio::test]
async fn concurrent_origin_dispatches_transfer_one_missing_workspace() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let worker = start_server().await;
    let peers = v2_remote_origin::peers(worker.addr);
    let store = RunStore::with_db_path(temp.path().join("origin.sqlite")).unwrap();
    let mut submission = v2_run::scheduler::independent_jobs("cache-singleflight", 2);
    for job in &mut submission.run.jobs {
        job.placement_candidates.truncate(1);
        job.placement_candidates[0].node_id = "builder-a".into();
    }
    let submission = RunSubmission::new(
        submission.idempotency_key,
        submission.run,
        submission.environment_values,
    )
    .unwrap();
    let run_id = v2_run::scheduler::commit(&store, &submission, "alice");
    let first = store.reserve_next(&peers.scheduler_nodes()).unwrap().unwrap();
    let second = store.reserve_next(&peers.scheduler_nodes()).unwrap().unwrap();
    let transport = RemoteAttemptTransport::new(store.clone(), TorBroker::new(), peers);

    let (first_result, second_result) = tokio::join!(
        transport.dispatch(&first),
        transport.dispatch(&second),
    );
    assert_eq!(first_result.unwrap(), AttemptDispatch::Accepted);
    assert_eq!(second_result.unwrap(), AttemptDispatch::Accepted);
    let mut cache_results = store
        .get_run(&run_id)
        .unwrap()
        .unwrap()
        .jobs
        .into_iter()
        .map(|job| job.cache.unwrap())
        .collect::<Vec<_>>();
    cache_results.sort();
    assert_eq!(cache_results, ["hit", "miss"]);
}
