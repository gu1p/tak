use std::collections::BTreeMap;

use sha2::{Digest, Sha256};
use tak_core::v2::{OutputSelector, Step, WorkspaceManifest};
use tak_proto::worker_v2::{DispatchAttemptRequest, payload_digest};

use super::v2_worker::dispatch;

pub fn output_dispatch() -> DispatchAttemptRequest {
    let archive = output_archive();
    let mut request = dispatch(1, 1, "fence-1");
    request.payload.workspace.descriptor.manifest = WorkspaceManifest::new([]).unwrap();
    request.payload.workspace.descriptor.archive_sha256 = format!("{:x}", Sha256::digest(&archive));
    request.payload.workspace.descriptor.archive_size = archive.len() as u64;
    request.payload.tasks[0].steps = vec![Step::Cmd {
        argv: vec![
            "/bin/sh".into(),
            "-c".into(),
            "printf 'hello\\n'; printf 'ok\\n' > result.txt".into(),
        ],
        cwd: None,
        env: BTreeMap::new(),
    }];
    request.payload.tasks[0].outputs = vec![OutputSelector::Path {
        value: "result.txt".into(),
    }];
    request.payload_digest = payload_digest(&request.payload).unwrap();
    request
}

pub fn output_archive() -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut builder = tar::Builder::new(&mut bytes);
    builder.mode(tar::HeaderMode::Deterministic);
    builder.finish().unwrap();
    drop(builder);
    bytes
}
