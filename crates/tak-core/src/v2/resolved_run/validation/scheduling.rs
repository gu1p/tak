use std::collections::{BTreeMap, BTreeSet};

use super::super::{PlacementKind, QueueDiscipline, ResolvedRun, ResolvedRunError};
use super::validate_identifier;
use crate::v2::Affinity;

pub(super) fn validate(run: &ResolvedRun) -> Result<(), ResolvedRunError> {
    if run
        .queue_definitions
        .iter()
        .any(|queue| queue.discipline == QueueDiscipline::Priority)
    {
        return Err(ResolvedRunError::new(
            "priority queues require resolved job priorities",
        ));
    }
    let mut policies = BTreeMap::new();
    for job in &run.jobs {
        let policy = &job.placement_policy;
        validate_identifier("placement policy", &policy.policy_id)?;
        if policies
            .insert(policy.policy_id.as_str(), policy.selection)
            .is_some_and(|selection| selection != policy.selection)
        {
            return Err(ResolvedRunError::new(format!(
                "placement policy `{}` has conflicting definitions",
                policy.policy_id
            )));
        }
        validate_candidates(job)?;
        if job.resources.cpu_millis > i64::MAX as u64
            || job.resources.memory_bytes > i64::MAX as u64
        {
            return Err(ResolvedRunError::new(format!(
                "job `{}` resource request exceeds durable range",
                job.job_id
            )));
        }
    }
    validate_hard_affinity(run)?;
    Ok(())
}

fn validate_hard_affinity(run: &ResolvedRun) -> Result<(), ResolvedRunError> {
    let mut candidates = BTreeMap::<&str, BTreeSet<&str>>::new();
    for job in &run.jobs {
        let Some(Affinity::RequireSameNode { group }) = &job.affinity else {
            continue;
        };
        let job_nodes = job
            .placement_candidates
            .iter()
            .map(|candidate| candidate.node_id.as_str())
            .collect::<BTreeSet<_>>();
        candidates
            .entry(group)
            .and_modify(|common| common.retain(|node| job_nodes.contains(node)))
            .or_insert(job_nodes);
    }
    if let Some(group) = candidates
        .into_iter()
        .find_map(|(group, nodes)| nodes.is_empty().then_some(group))
    {
        return Err(ResolvedRunError::new(format!(
            "hard affinity group `{group}` has no common placement candidate"
        )));
    }
    Ok(())
}

fn validate_candidates(job: &super::super::ResolvedJob) -> Result<(), ResolvedRunError> {
    let mut nodes = BTreeSet::new();
    for candidate in &job.placement_candidates {
        validate_identifier("placement node", &candidate.node_id)?;
        if !nodes.insert(candidate.node_id.as_str()) {
            return Err(ResolvedRunError::new(format!(
                "job `{}` has duplicate placement candidate `{}`",
                job.job_id, candidate.node_id
            )));
        }
        let transport_valid = match candidate.kind {
            PlacementKind::Local => candidate.transport.is_none(),
            PlacementKind::Remote => {
                matches!(candidate.transport.as_deref(), Some("direct" | "tor"))
            }
        };
        if !transport_valid || candidate.reason.trim().is_empty() {
            return Err(ResolvedRunError::new(format!(
                "job `{}` has an invalid placement candidate",
                job.job_id
            )));
        }
    }
    Ok(())
}
