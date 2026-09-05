use std::sync::Arc;

use tak_core::v2::RunSubmission;
use tak_proto::local_daemon::v2::WorkspaceDisposition;
use takd::{AttemptCoordinator, RemoteAttemptTransport, RunStore, TorBroker};

use crate::support::{v2_remote_origin, v2_run::ARCHIVE, worker_http::start_server};

#[tokio::test]
async fn cache_hit_dispatch_uses_the_canonical_stored_archive_descriptor() {
    let temp = tempfile::tempdir().unwrap();
    let store = RunStore::with_db_path(temp.path().join("origin.sqlite")).unwrap();
    seed_workspace_cache(&store);

    let mut reused = v2_remote_origin::submission();
    reused.idempotency_key = "descriptor-reuse".into();
    reused.run.workspace.archive_sha256 = "f".repeat(64);
    reused.run.workspace.archive_size += 1;
    let reused = RunSubmission::new(
        reused.idempotency_key,
        reused.run,
        reused.environment_values,
    )
    .unwrap();
    let submitted = store.submit(&reused, "alice").unwrap();
    assert_eq!(submitted.workspace, WorkspaceDisposition::Present);
    store.commit(&submitted.run_id).unwrap();

    let worker = start_server().await;
    let peers = v2_remote_origin::peers(worker.addr);
    store
        .reserve_next(&peers.scheduler_nodes())
        .unwrap()
        .unwrap();
    let transport = Arc::new(RemoteAttemptTransport::new(
        store.clone(),
        TorBroker::new(),
        peers,
    ));
    let mut coordinator = AttemptCoordinator::new(store.clone(), transport);
    crate::support::coordinator_wait::until(&mut coordinator, || {
        store.pending_dispatches().unwrap().is_empty()
    }).await;
}

fn seed_workspace_cache(store: &RunStore) {
    let request = v2_remote_origin::submission();
    let submitted = store.submit(&request, "alice").unwrap();
    store
        .upload_workspace(
            &submitted.run_id,
            &request.run.workspace.manifest.fingerprint,
            ARCHIVE.len() as u64,
            0,
            &ARCHIVE,
        )
        .unwrap();
}
