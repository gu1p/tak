use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use tak_core::v2::{ResolvedJob, SessionReuse};

#[derive(Debug)]
pub(super) struct FusedJobs {
    pub(super) jobs: Vec<ResolvedJob>,
    pub(super) job_ids: BTreeMap<String, String>,
}

pub(super) fn fuse_jobs(jobs: Vec<ResolvedJob>) -> Result<FusedJobs> {
    let mut fused = Vec::<ResolvedJob>::new();
    let mut containers = BTreeMap::<String, usize>::new();
    let mut job_ids = BTreeMap::new();
    for job in jobs {
        let old_id = job.job_id.clone();
        let container = job.session.as_ref().and_then(|session| {
            matches!(session.reuse, SessionReuse::Container).then(|| session.id.clone())
        });
        if let Some(session_id) = container.as_ref()
            && let Some(index) = containers.get(session_id).copied()
        {
            let destination = &mut fused[index];
            require_compatible(destination, &job, session_id)?;
            job_ids.insert(old_id, destination.job_id.clone());
            merge(destination, job);
            continue;
        }
        let new_id = job.job_id.clone();
        if let Some(session_id) = container {
            containers.insert(session_id, fused.len());
        }
        job_ids.insert(old_id, new_id);
        fused.push(job);
    }
    Ok(FusedJobs {
        jobs: fused,
        job_ids,
    })
}

fn require_compatible(first: &ResolvedJob, next: &ResolvedJob, session_id: &str) -> Result<()> {
    let compatible = first.placement_policy == next.placement_policy
        && first.placement_candidates == next.placement_candidates
        && first.resources == next.resources
        && first.retry == next.retry
        && first.queue == next.queue
        && first.queue_slots == next.queue_slots
        && first.queue_priority == next.queue_priority
        && first.limiter_claims == next.limiter_claims
        && first.affinity == next.affinity
        && first.session == next.session;
    if !compatible {
        bail!("container session `{session_id}` has incompatible scheduling policy")
    }
    Ok(())
}

fn merge(destination: &mut ResolvedJob, source: ResolvedJob) {
    destination.task_ids.extend(source.task_ids);
    destination.idempotent &= source.idempotent;
    destination.pass_env_names = destination
        .pass_env_names
        .iter()
        .chain(&source.pass_env_names)
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    destination.context_manifest.paths = destination
        .context_manifest
        .paths
        .iter()
        .chain(&source.context_manifest.paths)
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
}
