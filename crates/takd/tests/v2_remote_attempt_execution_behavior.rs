use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use tak_proto::{
    local_daemon::v2::{RunEventKind, RunLifecycleState},
    worker_v2::WorkerAttemptIdentity,
};
use takd::{AttemptCoordinator, RemoteAttemptTransport, RunStore, TorBroker};

use crate::support::{v2_remote_origin, v2_run::scheduler::commit, worker_http::start_server};

#[tokio::test]
async fn origin_restart_resumes_remote_v2_logs_outputs_and_terminal_acknowledgement() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let worker = start_server().await;
    let peers = v2_remote_origin::peers(worker.addr);
    let db = temp.path().join("origin.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let run_id = commit(&store, &v2_remote_origin::submission(), "alice");
    assert_eq!(peers.scheduler_nodes().len(), 1);
    let command = store.reserve_next(&peers.scheduler_nodes()).unwrap().unwrap();
    let identity = WorkerAttemptIdentity {
        run_id: command.run_id.clone(), job_id: command.job_id.clone(),
        node_id: command.node_id.clone(), authored_attempt: command.authored_attempt,
        dispatch_generation: command.dispatch_generation,
        fencing_token: command.fencing_token.clone(),
    };
    let broker = TorBroker::new();
    let transport = Arc::new(RemoteAttemptTransport::new(
        store.clone(), broker.clone(), peers.clone(),
    ));
    let mut coordinator = AttemptCoordinator::new(store.clone(), transport);
    assert_eq!(coordinator.drive_once().await.unwrap().dispatched, 1);
    assert_eq!(
        store.get_run(&run_id).unwrap().unwrap().jobs[0]
            .cache
            .as_deref(),
        Some("miss")
    );
    assert!(
        store
            .events_after(&run_id, 0)
            .unwrap()
            .iter()
            .any(|event| event.message == "workspace cache miss")
    );
    drop(coordinator);
    drop(store);

    let store = RunStore::with_db_path(db).unwrap();
    let transport = Arc::new(RemoteAttemptTransport::new(store.clone(), broker, peers));
    let mut coordinator = AttemptCoordinator::new(store.clone(), transport);
    tokio::time::timeout(Duration::from_secs(5), async {
        while !store.summary(&run_id).unwrap().unwrap().state.is_terminal() {
            coordinator.drive_once().await.unwrap();
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        coordinator.drive_once().await.unwrap();
    }).await.unwrap();
    assert_eq!(store.summary(&run_id).unwrap().unwrap().state, RunLifecycleState::Succeeded);
    let chunks = store.events_after(&run_id, 0).unwrap().into_iter()
        .filter(|event| event.kind == RunEventKind::Stdout)
        .flat_map(|event| base64::engine::general_purpose::STANDARD
            .decode(event.chunk_base64.unwrap()).unwrap()).collect::<Vec<_>>();
    assert_eq!(chunks, b"remote-log\n");
    let outputs = store.output_manifest(&run_id).unwrap().unwrap();
    assert_eq!(outputs[0].path, "result.txt");
    let output = store.output_chunk(&outputs[0].artifact_id, 0, 64).unwrap().unwrap();
    assert!(output.complete);
    assert_eq!(output.bytes, b"remote-output\n");
    assert!(worker.store.worker_v2_terminal_is_acknowledged(&identity).unwrap());
}
