use std::collections::BTreeMap;

use anyhow::Result;
use rusqlite::Transaction;
use tak_core::v2::{RemoteSelection, ResolvedJob, ResourceRequest};

use crate::daemon::scheduler::SchedulerNode;

pub(super) fn select_node<'a>(
    transaction: &Transaction<'_>,
    nodes: &'a [SchedulerNode],
    job: &ResolvedJob,
    cursor: u64,
) -> Result<Option<(&'a SchedulerNode, Option<u64>)>> {
    let nodes_by_id = nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let count = job.placement_candidates.len();
    let start = match job.placement_policy.selection {
        RemoteSelection::RoundRobin => usize::try_from(cursor % count as u64)?,
        RemoteSelection::Sequential | RemoteSelection::Balanced => 0,
    };
    for offset in 0..count {
        let index = (start + offset) % count;
        let candidate = &job.placement_candidates[index];
        let Some(node) = nodes_by_id.get(candidate.node_id.as_str()) else {
            continue;
        };
        if has_capacity(transaction, node, job.resources)? {
            let next = (job.placement_policy.selection == RemoteSelection::RoundRobin)
                .then_some(((index + 1) % count) as u64);
            return Ok(Some((node, next)));
        }
    }
    Ok(None)
}

fn has_capacity(
    transaction: &Transaction<'_>,
    node: &SchedulerNode,
    request: ResourceRequest,
) -> Result<bool> {
    let reserved = transaction.query_row(
        "SELECT COALESCE(SUM(cpu_millis), 0), COALESCE(SUM(memory_bytes), 0), \
         COALESCE(SUM(execution_slots), 0) FROM run_attempts \
         WHERE node_id = ?1 AND released_at_ms IS NULL",
        [&node.node_id],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        },
    )?;
    let cpu = node
        .cpu_used_millis
        .saturating_add(u64::try_from(reserved.0)?);
    let memory = node
        .memory_used_bytes
        .saturating_add(u64::try_from(reserved.1)?);
    let slots = u64::from(node.execution_used).saturating_add(u64::try_from(reserved.2)?);
    Ok(
        cpu.saturating_add(request.cpu_millis) <= node.cpu_capacity_millis
            && memory.saturating_add(request.memory_bytes) <= node.memory_capacity_bytes
            && slots.saturating_add(u64::from(request.execution_slots.get()))
                <= u64::from(node.execution_capacity),
    )
}
