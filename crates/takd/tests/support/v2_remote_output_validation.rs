use std::path::PathBuf;
use std::sync::Arc;

use sha2::{Digest, Sha256};
use tak_core::v2::{OutputSelector, WorkspaceEntry};
use tak_proto::worker_v2::{WorkerAttemptIdentity, WorkerTerminalOutcome, payload_digest};
use takd::{DispatchCommand, RemoteAttemptTransport, RunStore, TorBroker};

use super::{v2_remote_origin, v2_run, v2_worker, worker_http};

pub struct MaliciousOutputCase {
    _origin: tempfile::TempDir,
    _worker: worker_http::RunningServer,
    pub command: DispatchCommand,
    pub transport: Arc<RemoteAttemptTransport>,
    pub blob_path: PathBuf,
}

pub async fn completed(producer: &str, path: &str) -> MaliciousOutputCase {
    std::fs::create_dir_all(".tmp").unwrap();
    let origin = tempfile::tempdir_in(".tmp").unwrap();
    let worker = worker_http::start_server().await;
    let peers = v2_remote_origin::peers(worker.addr);
    let db = origin.path().join("origin.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let submission = v2_remote_origin::submission();
    v2_run::scheduler::commit(&store, &submission, "alice");
    let command = store
        .reserve_next(&peers.scheduler_nodes())
        .unwrap()
        .unwrap();
    let mut request = v2_worker::dispatch(
        command.authored_attempt,
        command.dispatch_generation,
        &command.fencing_token,
    );
    request.identity = identity(&command);
    request.payload.tasks[0].task_id = producer.into();
    request.payload.tasks[0].job_id.clone_from(&command.job_id);
    request.payload.tasks[0].outputs = vec![OutputSelector::Path { value: path.into() }];
    request.payload_digest = payload_digest(&request.payload).unwrap();
    let bytes = b"malicious\n";
    let digest = format!("{:x}", Sha256::digest(bytes));
    let entry = WorkspaceEntry::file(path, false, bytes.len() as u64, &digest).unwrap();
    worker.store.register_worker_v2_attempt(&request).unwrap();
    worker
        .store
        .mark_worker_v2_running(&request.identity)
        .unwrap();
    worker
        .store
        .publish_worker_v2_output(&request.identity, producer, entry, bytes)
        .unwrap();
    worker
        .store
        .complete_worker_v2_attempt(
            &request.identity,
            WorkerTerminalOutcome::Succeeded,
            &"a".repeat(64),
        )
        .unwrap();
    let transport = Arc::new(RemoteAttemptTransport::new(
        store.clone(),
        TorBroker::new(),
        peers,
    ));
    MaliciousOutputCase {
        _origin: origin,
        _worker: worker,
        command,
        transport,
        blob_path: db.with_extension("v2-blobs").join("outputs").join(digest),
    }
}

fn identity(command: &DispatchCommand) -> WorkerAttemptIdentity {
    WorkerAttemptIdentity {
        run_id: command.run_id.clone(),
        job_id: command.job_id.clone(),
        node_id: command.node_id.clone(),
        authored_attempt: command.authored_attempt,
        dispatch_generation: command.dispatch_generation,
        fencing_token: command.fencing_token.clone(),
    }
}
