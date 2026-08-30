use std::collections::BTreeSet;

use anyhow::{Result, bail};
use rusqlite::{OptionalExtension, Transaction, params};
use tak_core::v2::{RemoteSelection, ResolvedJob};

use crate::daemon::scheduler::SchedulerNode;

pub(super) fn validate_nodes(nodes: &[SchedulerNode]) -> Result<()> {
    let mut ids = BTreeSet::new();
    for node in nodes {
        if node.node_id.trim().is_empty()
            || !ids.insert(node.node_id.as_str())
            || node.execution_used > node.execution_capacity
            || node.cpu_used_millis > node.cpu_capacity_millis
            || node.memory_used_bytes > node.memory_capacity_bytes
        {
            bail!("invalid scheduler node snapshot");
        }
    }
    Ok(())
}

pub(super) fn ready_jobs(
    transaction: &Transaction<'_>,
) -> Result<Vec<(String, String, String, u64)>> {
    let mut statement = transaction.prepare(
        "SELECT j.run_id, j.job_id, j.definition_json, r.max_parallel_jobs \
         FROM run_jobs j JOIN runs r ON r.run_id = j.run_id \
         WHERE j.state = 'ready' AND r.state IN ('queued', 'running') \
         ORDER BY r.created_at_ms, r.run_id, j.ordinal, j.job_id",
    )?;
    statement
        .query_map([], |row| {
            let max_parallel = row.get::<_, i64>(3)?;
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                u64::try_from(max_parallel)
                    .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(3, max_parallel))?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub(super) fn active_run_attempts(transaction: &Transaction<'_>, run_id: &str) -> Result<u64> {
    let count = transaction.query_row(
        "SELECT COUNT(*) FROM run_attempts WHERE run_id = ?1 AND released_at_ms IS NULL",
        [run_id],
        |row| row.get::<_, i64>(0),
    )?;
    u64::try_from(count).map_err(|_| anyhow::anyhow!("invalid active attempt count"))
}

pub(super) fn policy_cursor(
    transaction: &Transaction<'_>,
    run_id: &str,
    job: &ResolvedJob,
) -> Result<u64> {
    if job.placement_policy.selection != RemoteSelection::RoundRobin {
        return Ok(0);
    }
    let cursor = transaction
        .query_row(
            "SELECT next_assignment FROM run_policy_cursors WHERE run_id = ?1 AND policy_id = ?2",
            params![run_id, job.placement_policy.policy_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .unwrap_or(0);
    u64::try_from(cursor).map_err(|_| anyhow::anyhow!("invalid round-robin cursor"))
}
