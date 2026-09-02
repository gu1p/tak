use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use tak_core::v2::{Affinity, Step, WorkspaceEntry, WorkspaceManifest};
use tak_proto::worker_v2::{DispatchAttemptRequest, WorkerWorkspaceReuse, payload_digest};

use crate::support::v2_worker::dispatch;

pub fn dispatch_with_context(
    generation: u32,
    fence: &str,
    context_path: &str,
    script: &str,
) -> DispatchAttemptRequest {
    let archive = context_archive();
    let mut request = dispatch(1, generation, fence);
    request.payload.workspace.descriptor.manifest = WorkspaceManifest::new([
        entry("producer.txt", b"producer"),
        entry("consumer.txt", b"consumer"),
    ])
    .unwrap();
    request.payload.workspace.descriptor.archive_sha256 = format!("{:x}", Sha256::digest(&archive));
    request.payload.workspace.descriptor.archive_size = archive.len() as u64;
    request.payload.context_manifest.paths = vec![context_path.into()];
    request.payload.workspace_reuse = WorkerWorkspaceReuse::Shared {
        session_id: "session-a".into(),
        affinity_group: "shared-group".into(),
    };
    request.payload.tasks[0].affinity = Some(Affinity::require_same_node("shared-group").unwrap());
    request.payload.tasks[0].steps = vec![Step::Cmd {
        argv: vec!["/bin/sh".into(), "-c".into(), script.into()],
        cwd: None,
        env: BTreeMap::new(),
    }];
    request.payload_digest = payload_digest(&request.payload).unwrap();
    request
}

pub fn context_archive() -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut builder = tar::Builder::new(&mut bytes);
    for (name, body) in [
        ("producer.txt", b"producer".as_slice()),
        ("consumer.txt", b"consumer".as_slice()),
    ] {
        let mut header = tar::Header::new_gnu();
        header.set_mode(0o644);
        header.set_size(body.len() as u64);
        header.set_cksum();
        builder.append_data(&mut header, name, body).unwrap();
    }
    builder.finish().unwrap();
    drop(builder);
    bytes
}

fn entry(name: &str, body: &[u8]) -> WorkspaceEntry {
    WorkspaceEntry::file(
        name,
        false,
        body.len() as u64,
        &format!("{:x}", Sha256::digest(body)),
    )
    .unwrap()
}
