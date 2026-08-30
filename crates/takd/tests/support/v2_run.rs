use std::num::NonZeroU32;
use std::sync::LazyLock;

use sha2::{Digest, Sha256};
use tak_core::v2::{
    EnvironmentValue, JobContextManifest, PlacementCandidate, PlacementKind, PlacementPolicy,
    RemoteSelection, ResolvedJob, ResolvedRun, ResolvedRunOptions, ResolvedTaskUnit,
    ResourceRequest, RetryPolicy, RunSubmission, WorkspaceDescriptor, WorkspaceEntry,
    WorkspaceManifest,
};

pub mod scheduler;

pub static ARCHIVE: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let mut archive = Vec::new();
    let mut builder = tar::Builder::new(&mut archive);
    builder.mode(tar::HeaderMode::Deterministic);
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Regular);
    header.set_mode(0o644);
    header.set_size(4);
    header.set_cksum();
    builder
        .append_data(&mut header, "TASKS.py", &b"spec"[..])
        .unwrap();
    builder.finish().unwrap();
    drop(builder);
    archive
});

pub fn submission(key: &str, secret: &str) -> RunSubmission {
    let manifest = WorkspaceManifest::new(vec![
        WorkspaceEntry::file(
            "TASKS.py",
            false,
            4,
            &format!("{:x}", Sha256::digest(b"spec")),
        )
        .unwrap(),
    ])
    .unwrap();
    let run = ResolvedRun {
        project_id: "project".into(),
        targets: vec!["//:check".into()],
        options: ResolvedRunOptions {
            max_parallel_jobs: NonZeroU32::new(1).unwrap(),
            keep_going: false,
        },
        workspace: WorkspaceDescriptor {
            manifest,
            archive_sha256: format!("{:x}", Sha256::digest(ARCHIVE.as_slice())),
            archive_size: ARCHIVE.len() as u64,
        },
        tasks: vec![ResolvedTaskUnit {
            task_id: "//:check".into(),
            job_id: "job-0".into(),
            dependencies: vec![],
            steps: vec![],
            outputs: vec![],
            pass_env_names: vec!["TOKEN".into()],
            idempotent: true,
            affinity: None,
        }],
        jobs: vec![ResolvedJob {
            job_id: "job-0".into(),
            task_ids: vec!["//:check".into()],
            placement_policy: PlacementPolicy {
                policy_id: "local".into(),
                selection: RemoteSelection::Sequential,
            },
            placement_candidates: vec![PlacementCandidate {
                node_id: "local".into(),
                kind: PlacementKind::Local,
                transport: None,
                reason: "local".into(),
            }],
            resources: ResourceRequest::default(),
            retry: RetryPolicy::default(),
            idempotent: true,
            queue: None,
            limiter_claims: vec![],
            affinity: None,
            session: None,
            context_manifest: JobContextManifest {
                paths: vec!["TASKS.py".into()],
            },
            pass_env_names: vec!["TOKEN".into()],
        }],
        job_edges: vec![],
        limiter_definitions: vec![],
        queue_definitions: vec![],
    };
    RunSubmission::new(
        key,
        run,
        vec![EnvironmentValue::new("TOKEN", secret).unwrap()],
    )
    .unwrap()
}
