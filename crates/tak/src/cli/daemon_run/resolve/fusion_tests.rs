use tak_core::v2::{
    JobContextManifest, PlacementCandidate, PlacementKind, PlacementPolicy, RemoteSelection,
    ResolvedJob, ResourceRequest, RetryPolicy, Session, SessionReuse,
};

use super::fusion::fuse_jobs;

#[test]
fn container_jobs_merge_in_order_with_conservative_job_policy() {
    let first = job("job-0", "//:dep", &["B"], true);
    let second = job("job-1", "//:target", &["A"], false);
    let fused = fuse_jobs(vec![first, second]).unwrap();

    assert_eq!(fused.jobs.len(), 1);
    assert_eq!(fused.jobs[0].task_ids, ["//:dep", "//:target"]);
    assert_eq!(fused.jobs[0].pass_env_names, ["A", "B"]);
    assert!(!fused.jobs[0].idempotent);
    assert_eq!(fused.job_ids["job-1"], "job-0");
}

#[test]
fn container_jobs_reject_incompatible_scheduling_policy() {
    let first = job("job-0", "//:dep", &[], true);
    let mut second = job("job-1", "//:target", &[], true);
    second.retry.max_attempts = std::num::NonZeroU32::new(2).unwrap();
    let error = fuse_jobs(vec![first, second]).unwrap_err().to_string();
    assert!(error.contains("incompatible scheduling"), "{error}");
}

fn job(id: &str, task_id: &str, pass_env: &[&str], idempotent: bool) -> ResolvedJob {
    let mut session = Session::new("build", SessionReuse::Container, None).unwrap();
    session.id = "container-session".into();
    ResolvedJob {
        job_id: id.into(),
        task_ids: vec![task_id.into()],
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
        idempotent,
        queue: None,
        limiter_claims: vec![],
        affinity: None,
        session: Some(session),
        context_manifest: JobContextManifest { paths: vec![] },
        pass_env_names: pass_env.iter().map(|item| (*item).into()).collect(),
    }
}
