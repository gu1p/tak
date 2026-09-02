use sha2::{Digest, Sha256};
use tak_core::v2::{RunSubmission, WorkspaceDescriptor};
use tak_proto::local_daemon::v2::{RunLifecycleState, WorkspaceDisposition};
use takd::RunStore;

use super::{shared_run, shell, wait_for};
use crate::support::protocol_server::spawn_protocol_server;

#[path = "context/fixture.rs"]
mod fixture;

#[tokio::test]
async fn local_shared_jobs_receive_their_own_context_and_shared_writes() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("context.sock");
    let server = spawn_protocol_server(db.clone(), socket.clone());
    wait_for(|| socket.exists()).await;
    let store = RunStore::with_db_path(db).unwrap();
    let (request, archive) = context_run();
    let accepted = store.submit(&request, "alice").unwrap();
    assert!(matches!(
        accepted.workspace,
        WorkspaceDisposition::UploadRequired { .. }
    ));
    store
        .upload_workspace(
            &accepted.run_id,
            &request.run.workspace.manifest.fingerprint,
            archive.len() as u64,
            0,
            &archive,
        )
        .unwrap();
    store.commit(&accepted.run_id).unwrap();
    wait_for(|| {
        store
            .summary(&accepted.run_id)
            .unwrap()
            .is_some_and(|run| run.state.is_terminal())
    })
    .await;
    assert_eq!(
        store.summary(&accepted.run_id).unwrap().unwrap().state,
        RunLifecycleState::Succeeded
    );
    server.abort();
}

fn context_run() -> (RunSubmission, Vec<u8>) {
    let mut request = shared_run();
    let archive = fixture::archive();
    request.run.workspace = WorkspaceDescriptor {
        manifest: fixture::manifest(),
        archive_sha256: format!("{:x}", Sha256::digest(&archive)),
        archive_size: archive.len() as u64,
    };
    request.run.jobs[0].context_manifest.paths = vec!["producer.txt".into()];
    request.run.jobs[1].context_manifest.paths = vec!["consumer.txt".into()];
    request.run.tasks[0].steps = vec![shell(
        "test -f producer.txt; test ! -e consumer.txt; printf shared > shared.txt",
    )];
    request.run.tasks[1].steps = vec![shell(
        "test -f consumer.txt; test ! -e producer.txt; test \"$(cat shared.txt)\" = shared",
    )];
    let request = RunSubmission::new(
        request.idempotency_key,
        request.run,
        request.environment_values,
    )
    .unwrap();
    (request, archive)
}
