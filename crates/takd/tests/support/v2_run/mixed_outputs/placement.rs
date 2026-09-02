use tak_core::v2::{PlacementCandidate, PlacementKind, ResolvedJob};

pub(super) fn local_job(template: &ResolvedJob, job_id: &str, task_id: &str) -> ResolvedJob {
    scheduled(
        template,
        job_id,
        task_id,
        PlacementCandidate {
            node_id: "local".into(),
            kind: PlacementKind::Local,
            transport: None,
            reason: "local".into(),
            tier: 0,
            requirements: None,
        },
    )
}

pub(super) fn remote_job(template: &ResolvedJob, job_id: &str, task_id: &str) -> ResolvedJob {
    scheduled(
        template,
        job_id,
        task_id,
        PlacementCandidate {
            node_id: "builder-a".into(),
            kind: PlacementKind::Remote,
            transport: Some("direct".into()),
            reason: "healthy protocol-v2 worker".into(),
            tier: 0,
            requirements: None,
        },
    )
}

fn scheduled(
    template: &ResolvedJob,
    job_id: &str,
    task_id: &str,
    candidate: PlacementCandidate,
) -> ResolvedJob {
    ResolvedJob {
        job_id: job_id.into(),
        task_ids: vec![task_id.into()],
        placement_candidates: vec![candidate],
        ..template.clone()
    }
}
