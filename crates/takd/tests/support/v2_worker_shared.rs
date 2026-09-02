use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use tak_core::v2::{Affinity, Step, WorkspaceEntry, WorkspaceManifest};
use tak_proto::worker_v2::{
    DispatchAttemptRequest, WorkerWorkspaceReuse, encode_dispatch_request, payload_digest,
};

use super::v2_worker::dispatch;
use super::v2_worker_cache::ensure;
use super::v2_worker_http::{post, status};
use super::worker_http::RunningServer;

mod context;
mod observation;

pub use context::{context_archive, dispatch_with_context};
pub use observation::wait_terminal;

pub fn dispatch_with_seed(
    generation: u32,
    fence: &str,
    seed: &str,
    matching_manifest: bool,
) -> DispatchAttemptRequest {
    let archive = seed_archive(seed);
    let mut request = dispatch(1, generation, fence);
    let entry = WorkspaceEntry::file(
        "seed.txt",
        false,
        seed.len() as u64,
        &format!("{:x}", Sha256::digest(seed.as_bytes())),
    )
    .unwrap();
    let declared = if matching_manifest {
        entry
    } else {
        WorkspaceEntry::file(
            "seed.txt",
            false,
            0,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        )
        .unwrap()
    };
    request.payload.workspace.descriptor.manifest = WorkspaceManifest::new([declared]).unwrap();
    request.payload.workspace.descriptor.archive_sha256 = format!("{:x}", Sha256::digest(&archive));
    request.payload.workspace.descriptor.archive_size = archive.len() as u64;
    request.payload.context_manifest.paths = vec!["seed.txt".into()];
    request.payload.workspace_reuse = WorkerWorkspaceReuse::Shared {
        session_id: "session-a".into(),
        affinity_group: "shared-group".into(),
    };
    request.payload.tasks[0].affinity = Some(Affinity::require_same_node("shared-group").unwrap());
    request.payload.tasks[0].steps = vec![Step::Cmd {
        argv: vec![
            "/bin/sh".into(),
            "-c".into(),
            format!("printf 'executed\\n'; test \"$(cat seed.txt)\" = '{seed}'"),
        ],
        cwd: None,
        env: BTreeMap::new(),
    }];
    request.payload_digest = payload_digest(&request.payload).unwrap();
    request
}

pub async fn send(server: &RunningServer, request: &DispatchAttemptRequest, archive: &[u8]) {
    ensure(server, &request.payload.workspace.descriptor, archive).await;
    let response = post(
        server,
        "/v2/attempts/dispatch",
        Some("secret"),
        &["v2"],
        &encode_dispatch_request(request).unwrap(),
    )
    .await;
    assert_eq!(status(&response), 202);
}

pub fn seed_archive(seed: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut builder = tar::Builder::new(&mut bytes);
    let mut header = tar::Header::new_gnu();
    header.set_mode(0o644);
    header.set_size(seed.len() as u64);
    header.set_cksum();
    builder
        .append_data(&mut header, "seed.txt", seed.as_bytes())
        .unwrap();
    builder.finish().unwrap();
    drop(builder);
    bytes
}
