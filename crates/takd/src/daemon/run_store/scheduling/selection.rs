use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rusqlite::Transaction;
use tak_core::v2::{RemoteSelection, ResolvedJob, ResourceRequest};

use crate::daemon::scheduler::SchedulerNode;

use super::constraints::{self, Context};

#[cfg(test)]
mod tests;

pub(super) struct AffinitySelection<'a> {
    pub(super) eligible_nodes: Option<&'a BTreeSet<String>>,
    pub(super) preferred_node: Option<&'a str>,
}

pub(super) fn select_node<'a>(
    transaction: &Transaction<'_>,
    nodes: &'a [SchedulerNode],
    job: &ResolvedJob,
    cursor: u64,
    workspace_fingerprint: &str,
    affinity: &AffinitySelection<'_>,
    constraint_context: &Context<'_>,
) -> Result<Option<(&'a SchedulerNode, Option<u64>)>> {
    let nodes_by_id = nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    if job.placement_policy.selection == RemoteSelection::Balanced {
        return select_balanced(
            transaction,
            &nodes_by_id,
            job,
            workspace_fingerprint,
            affinity,
            constraint_context,
        );
    }
    let count = job.placement_candidates.len();
    let start = match job.placement_policy.selection {
        RemoteSelection::RoundRobin => usize::try_from(cursor % count as u64)?,
        RemoteSelection::Sequential => 0,
        RemoteSelection::Balanced => unreachable!("balanced handled above"),
    };
    for offset in 0..count {
        let index = (start + offset) % count;
        let candidate = &job.placement_candidates[index];
        if affinity
            .eligible_nodes
            .is_some_and(|eligible| !eligible.contains(&candidate.node_id))
        {
            continue;
        }
        let Some(node) = nodes_by_id.get(candidate.node_id.as_str()) else {
            continue;
        };
        let usage = reserved_usage(transaction, node)?;
        if has_capacity(node, usage, job.resources)
            && constraints::can_acquire(transaction, constraint_context, job, &node.node_id)?
        {
            let next = (job.placement_policy.selection == RemoteSelection::RoundRobin)
                .then_some(((index + 1) % count) as u64);
            return Ok(Some((node, next)));
        }
    }
    Ok(None)
}

fn select_balanced<'a>(
    transaction: &Transaction<'_>,
    nodes: &BTreeMap<&str, &'a SchedulerNode>,
    job: &ResolvedJob,
    workspace_fingerprint: &str,
    affinity: &AffinitySelection<'_>,
    constraint_context: &Context<'_>,
) -> Result<Option<(&'a SchedulerNode, Option<u64>)>> {
    let mut best = None;
    for (index, candidate) in job.placement_candidates.iter().enumerate() {
        if affinity
            .eligible_nodes
            .is_some_and(|eligible| !eligible.contains(&candidate.node_id))
        {
            continue;
        }
        let Some(node) = nodes.get(candidate.node_id.as_str()).copied() else {
            continue;
        };
        let usage = reserved_usage(transaction, node)?;
        if !has_capacity(node, usage, job.resources) {
            continue;
        }
        if !constraints::can_acquire(transaction, constraint_context, job, &node.node_id)? {
            continue;
        }
        let locality = node.cached_content.contains(workspace_fingerprint)
            || affinity.preferred_node == Some(node.node_id.as_str());
        let score = score_node(node, usage, job.resources, locality);
        if best
            .as_ref()
            .is_none_or(|(_, best_score, best_index)| (score, index) < (*best_score, *best_index))
        {
            best = Some((node, score, index));
        }
    }
    Ok(best.map(|(node, _, _)| (node, None)))
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Usage {
    cpu_millis: u64,
    memory_bytes: u64,
    execution_slots: u64,
    attempt_count: u64,
}

fn reserved_usage(transaction: &Transaction<'_>, node: &SchedulerNode) -> Result<Usage> {
    let mut statement = transaction.prepare(
        "SELECT cpu_millis, memory_bytes, execution_slots FROM run_attempts \
         WHERE node_id = ?1 AND released_at_ms IS NULL",
    )?;
    let rows = statement.query_map([&node.node_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;
    let mut usage = Usage::default();
    for row in rows {
        let (cpu, memory, slots) = row?;
        usage.cpu_millis = usage
            .cpu_millis
            .checked_add(u64::try_from(cpu)?)
            .ok_or_else(|| anyhow::anyhow!("CPU reservation total overflow"))?;
        usage.memory_bytes = usage
            .memory_bytes
            .checked_add(u64::try_from(memory)?)
            .ok_or_else(|| anyhow::anyhow!("memory reservation total overflow"))?;
        usage.execution_slots = usage
            .execution_slots
            .checked_add(u64::try_from(slots)?)
            .ok_or_else(|| anyhow::anyhow!("slot reservation total overflow"))?;
        usage.attempt_count = usage
            .attempt_count
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("reservation count overflow"))?;
    }
    Ok(usage)
}

fn has_capacity(node: &SchedulerNode, reserved: Usage, request: ResourceRequest) -> bool {
    let Some(cpu) = node
        .cpu_used_millis
        .checked_add(reserved.cpu_millis)
        .and_then(|used| used.checked_add(request.cpu_millis))
    else {
        return false;
    };
    let Some(memory) = node
        .memory_used_bytes
        .checked_add(reserved.memory_bytes)
        .and_then(|used| used.checked_add(request.memory_bytes))
    else {
        return false;
    };
    let Some(slots) = u64::from(node.execution_used)
        .checked_add(reserved.execution_slots)
        .and_then(|used| used.checked_add(u64::from(request.execution_slots.get())))
    else {
        return false;
    };
    cpu <= node.cpu_capacity_millis
        && memory <= node.memory_capacity_bytes
        && slots <= u64::from(node.execution_capacity)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct PlacementScore {
    pub(super) dominant_pressure: u128,
    queue_pressure: u64,
}

pub(super) fn score_node(
    node: &SchedulerNode,
    reserved: Usage,
    request: ResourceRequest,
    locality: bool,
) -> PlacementScore {
    let current = dominant_pressure(node, reserved, request, true);
    let projected = dominant_pressure(node, reserved, request, false);
    let increment = projected.saturating_sub(current);
    let credit = if locality {
        increment.saturating_sub(1) / 2
    } else {
        0
    };
    PlacementScore {
        dominant_pressure: projected.saturating_sub(credit),
        queue_pressure: u64::from(node.queue_depth).saturating_add(reserved.attempt_count),
    }
}

fn dominant_pressure(
    node: &SchedulerNode,
    reserved: Usage,
    request: ResourceRequest,
    current: bool,
) -> u128 {
    let request_cpu = if current { 0 } else { request.cpu_millis };
    let request_memory = if current { 0 } else { request.memory_bytes };
    let request_slots = if current {
        0
    } else {
        u64::from(request.execution_slots.get())
    };
    [
        ratio(
            node.cpu_used_millis
                .saturating_add(reserved.cpu_millis)
                .saturating_add(request_cpu),
            node.cpu_capacity_millis,
        ),
        ratio(
            node.memory_used_bytes
                .saturating_add(reserved.memory_bytes)
                .saturating_add(request_memory),
            node.memory_capacity_bytes,
        ),
        ratio(
            u64::from(node.execution_used)
                .saturating_add(reserved.execution_slots)
                .saturating_add(request_slots),
            u64::from(node.execution_capacity),
        ),
    ]
    .into_iter()
    .max()
    .unwrap_or(0)
}

fn ratio(value: u64, capacity: u64) -> u128 {
    const SCALE: u128 = 1_u128 << 64;
    if value == 0 {
        return 0;
    }
    if capacity == 0 {
        return u128::MAX;
    }
    let numerator = u128::from(value) * SCALE;
    let denominator = u128::from(capacity);
    let quotient = numerator / denominator;
    quotient + u128::from(!numerator.is_multiple_of(denominator))
}
