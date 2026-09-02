use tak_core::v2::{
    JobContextManifest, ResolvedTaskUnit, ResourceRequest, WorkspaceDescriptor, WorkspaceManifest,
};
use tak_proto::worker_v2::{
    DispatchAttemptRequest, WorkerAttemptIdentity, WorkerAttemptPayload, WorkerWorkspace,
    WorkerWorkspaceReuse, payload_digest,
};

pub fn dispatch(authored_attempt: u32, generation: u32, fence: &str) -> DispatchAttemptRequest {
    let payload = WorkerAttemptPayload {
        workspace: WorkerWorkspace {
            descriptor: WorkspaceDescriptor {
                manifest: WorkspaceManifest::new([]).unwrap(),
                archive_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                    .into(),
                archive_size: 0,
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
    };
    DispatchAttemptRequest {
        protocol_version: 2,
        identity: WorkerAttemptIdentity {
            run_id: "run-1".into(),
            job_id: "job-1".into(),
            node_id: "builder-a".into(),
            authored_attempt,
            dispatch_generation: generation,
            fencing_token: fence.into(),
        },
        payload_digest: payload_digest(&payload).unwrap(),
        payload,
    }
}
