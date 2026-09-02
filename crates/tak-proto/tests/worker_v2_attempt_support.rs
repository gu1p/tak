use sha2::{Digest, Sha256};
use tak_core::v2::{
    JobContextManifest, ResolvedTaskUnit, ResourceRequest, WorkspaceDescriptor, WorkspaceManifest,
};
use tak_proto::worker_v2::{
    DispatchAttemptRequest, WorkerAttemptIdentity, WorkerAttemptPayload, WorkerWorkspace,
    WorkerWorkspaceReuse, payload_digest,
};

pub fn request(payload: WorkerAttemptPayload) -> DispatchAttemptRequest {
    DispatchAttemptRequest {
        protocol_version: 2,
        identity: WorkerAttemptIdentity {
            run_id: "run-1".into(),
            job_id: "job-1".into(),
            node_id: "worker-a".into(),
            authored_attempt: 1,
            dispatch_generation: 1,
            fencing_token: "fence-1".into(),
        },
        payload_digest: payload_digest(&payload).unwrap(),
        payload,
    }
}

pub fn payload() -> WorkerAttemptPayload {
    let archive = payload_archive();
    WorkerAttemptPayload {
        workspace: WorkerWorkspace {
            descriptor: WorkspaceDescriptor {
                manifest: WorkspaceManifest::new([]).unwrap(),
                archive_sha256: format!("{:x}", Sha256::digest(&archive)),
                archive_size: archive.len() as u64,
            },
            overlays: vec![],
        },
        workspace_reuse: WorkerWorkspaceReuse::Private,
        tasks: vec![ResolvedTaskUnit {
            task_id: "//:check".into(),
            job_id: "job-1".into(),
            dependencies: vec![],
            steps: vec![],
            outputs: vec![],
            pass_env_names: vec![],
            idempotent: true,
            affinity: None,
            timeout_s: None,
            runtime: None,
        }],
        environment_values: vec![],
        resources: ResourceRequest::default(),
        context_manifest: JobContextManifest { paths: vec![] },
    }
}

pub fn payload_archive() -> Vec<u8> {
    let mut archive = Vec::new();
    let mut builder = tar::Builder::new(&mut archive);
    builder.mode(tar::HeaderMode::Deterministic);
    builder.finish().unwrap();
    drop(builder);
    archive
}
