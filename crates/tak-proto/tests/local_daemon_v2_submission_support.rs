use std::num::NonZeroU32;

use tak_core::v2::{
    EnvironmentValue, JobContextManifest, PlacementCandidate, PlacementKind, ResolvedJob,
    ResolvedRun, ResolvedRunOptions, ResolvedTaskUnit, RetryPolicy, WorkspaceDescriptor,
    WorkspaceEntry, WorkspaceManifest,
};

pub fn sample_run() -> ResolvedRun {
    let manifest = WorkspaceManifest::new(vec![
        WorkspaceEntry::file("TASKS.py", false, 4, &"a".repeat(64)).unwrap(),
    ])
    .unwrap();
    ResolvedRun {
        project_id: "project".into(),
        targets: vec!["//:check".into()],
        options: ResolvedRunOptions {
            max_parallel_jobs: NonZeroU32::new(2).unwrap(),
            keep_going: true,
        },
        workspace: WorkspaceDescriptor {
            manifest,
            archive_sha256: "b".repeat(64),
            archive_size: 4,
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
            placement_candidates: vec![PlacementCandidate {
                node_id: "local".into(),
                kind: PlacementKind::Local,
                transport: None,
                reason: "local".into(),
            }],
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
    }
}

pub fn environment() -> Vec<EnvironmentValue> {
    vec![EnvironmentValue::new("TOKEN", "secret").unwrap()]
}
