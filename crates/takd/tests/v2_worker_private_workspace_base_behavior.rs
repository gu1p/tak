use std::collections::BTreeMap;

use sha2::{Digest, Sha256};
use tak_core::v2::{Step, WorkspaceEntry, WorkspaceManifest};
use tak_proto::worker_v2::{WorkerTerminalOutcome, payload_digest};

use crate::support::{
    worker_http::start_server,
    v2_worker::dispatch,
    v2_worker_shared::{send, wait_terminal},
};

#[tokio::test]
async fn ordinary_worker_jobs_clone_one_immutable_base_without_leaking_writes() {
    let server = start_server().await;
    let archive = seed_archive();
    let first = request(
        1,
        "private-base-1",
        &archive,
        "printf changed > seed.txt; printf leak > undeclared.txt",
    );
    send(&server, &first, &archive).await;
    assert_eq!(
        outcome(&server, &first).await,
        WorkerTerminalOutcome::Succeeded
    );

    let fingerprint = &first.payload.workspace.descriptor.manifest.fingerprint;
    let base = server
        .state_root
        .join("worker-v2-workspace-bases")
        .join(fingerprint)
        .join("data");
    assert_eq!(std::fs::read(base.join("seed.txt")).unwrap(), b"base");
    assert!(!base.join("undeclared.txt").exists());

    let second = request(
        2,
        "private-base-2",
        &archive,
        "test \"$(cat seed.txt)\" = base; test ! -e undeclared.txt",
    );
    send(&server, &second, &archive).await;
    assert_eq!(
        outcome(&server, &second).await,
        WorkerTerminalOutcome::Succeeded
    );
    assert_eq!(std::fs::read(base.join("seed.txt")).unwrap(), b"base");
}

async fn outcome(
    server: &crate::support::worker_http::RunningServer,
    request: &tak_proto::worker_v2::DispatchAttemptRequest,
) -> WorkerTerminalOutcome {
    wait_terminal(server, request)
        .await
        .terminal
        .unwrap()
        .outcome
}

fn request(
    generation: u32,
    fence: &str,
    archive: &[u8],
    script: &str,
) -> tak_proto::worker_v2::DispatchAttemptRequest {
    let mut request = dispatch(1, generation, fence);
    let digest = format!("{:x}", Sha256::digest(b"base"));
    let entry = WorkspaceEntry::file("seed.txt", false, 4, &digest).unwrap();
    request.payload.workspace.descriptor.manifest = WorkspaceManifest::new([entry]).unwrap();
    request.payload.workspace.descriptor.archive_sha256 = format!("{:x}", Sha256::digest(archive));
    request.payload.workspace.descriptor.archive_size = archive.len() as u64;
    request.payload.context_manifest.paths = vec!["seed.txt".into()];
    request.payload.tasks[0].steps = vec![Step::Cmd {
        argv: vec!["/bin/sh".into(), "-c".into(), script.into()],
        cwd: None,
        env: BTreeMap::new(),
    }];
    request.payload_digest = payload_digest(&request.payload).unwrap();
    request
}

fn seed_archive() -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut archive = tar::Builder::new(&mut bytes);
    let mut header = tar::Header::new_gnu();
    header.set_mode(0o644);
    header.set_size(4);
    header.set_cksum();
    archive
        .append_data(&mut header, "seed.txt", &b"base"[..])
        .unwrap();
    archive.finish().unwrap();
    drop(archive);
    bytes
}
