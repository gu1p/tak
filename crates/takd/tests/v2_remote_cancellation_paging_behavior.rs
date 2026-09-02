use std::{sync::Arc, time::Duration};

use tak_proto::worker_v2::{WorkerOutputStream, WorkerTerminalOutcome, payload_digest};
use takd::{AttemptTransport, RemoteAttemptTransport, RunStore, TorBroker};

use crate::support::{
    v2_remote_origin, v2_run::scheduler::commit, v2_worker::dispatch, worker_http::start_server,
};

#[tokio::test]
async fn remote_cancellation_advances_through_pending_event_pages_before_terminal() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let worker = start_server().await;
    let peers = v2_remote_origin::peers(worker.addr);
    let origin = RunStore::with_db_path(temp.path().join("origin.sqlite")).unwrap();
    commit(&origin, &v2_remote_origin::submission(), "alice");
    let command = origin
        .reserve_next(&peers.scheduler_nodes())
        .unwrap()
        .unwrap();
    let mut request = dispatch(
        command.authored_attempt,
        command.dispatch_generation,
        &command.fencing_token,
    );
    request.identity.run_id.clone_from(&command.run_id);
    request.identity.job_id.clone_from(&command.job_id);
    request.identity.node_id.clone_from(&command.node_id);
    request.payload.tasks[0].job_id.clone_from(&command.job_id);
    request.payload_digest = payload_digest(&request.payload).unwrap();
    worker.store.register_worker_v2_attempt(&request).unwrap();
    worker
        .store
        .mark_worker_v2_running(&request.identity)
        .unwrap();
    for _ in 0..129 {
        worker
            .store
            .append_worker_v2_event(
                &request.identity,
                "//:check",
                WorkerOutputStream::Stdout,
                b"x",
            )
            .unwrap();
    }
    let store = worker.store.clone();
    let identity = request.identity.clone();
    let completion = tokio::spawn(async move {
        while !store.worker_v2_cancellation_requested(&identity).unwrap() {
            tokio::task::yield_now().await;
        }
        store
            .complete_worker_v2_attempt(
                &identity,
                WorkerTerminalOutcome::Cancelled,
                "5dceaeceb6d8a49d3665594710ace68dfbc1800e2d663d7febb302b4dbee3d27",
            )
            .unwrap();
    });
    let transport = Arc::new(RemoteAttemptTransport::new(origin, TorBroker::new(), peers));

    tokio::time::timeout(Duration::from_secs(3), transport.cancel_and_wait(&command))
        .await
        .expect("cancellation observation must advance past the first page")
        .unwrap();
    completion.await.unwrap();
}
