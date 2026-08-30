use std::num::NonZeroU32;

use tak_core::v2::{
    JobContextManifest, PlacementCandidate, PlacementKind, PlacementPolicy, RemoteSelection,
    ResolvedJob, ResolvedRun, ResolvedRunOptions, ResolvedTaskUnit, ResourceRequest, RetryPolicy,
    WorkspaceDescriptor, WorkspaceEntry, WorkspaceManifest,
};

pub(super) fn sample_run() -> ResolvedRun {
    let manifest = WorkspaceManifest::new(vec![
        WorkspaceEntry::file("TASKS.py", false, 4, &"a".repeat(64)).unwrap(),
    ])
    .unwrap();
    ResolvedRun {
        project_id: "project".into(),
        targets: vec!["//:check".into()],
        options: ResolvedRunOptions {
            max_parallel_jobs: NonZeroU32::new(1).unwrap(),
            keep_going: false,
        },
        workspace: WorkspaceDescriptor {
            manifest,
            archive_sha256: "b".repeat(64),
            archive_size: 1,
        },
        tasks: vec![ResolvedTaskUnit {
            task_id: "//:check".into(),
            job_id: "job-0".into(),
            dependencies: vec![],
            steps: vec![],
            outputs: vec![],
            pass_env_names: vec!["TOKEN".into()],
            idempotent: false,
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
                reason: "local execution".into(),
            }],
            resources: ResourceRequest::default(),
            retry: RetryPolicy::default(),
            idempotent: false,
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
    }
}
