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

pub(super) struct ReadyJob {
    pub(super) run_id: String,
    pub(super) job_id: String,
    pub(super) definition: String,
    pub(super) max_parallel: u64,
    pub(super) workspace_fingerprint: String,
    pub(super) submitter_id: String,
}

pub(super) fn ready_jobs(transaction: &Transaction<'_>, now_ms: u64) -> Result<Vec<ReadyJob>> {
    let mut statement = transaction.prepare(
        "SELECT j.run_id, j.job_id, j.definition_json, r.max_parallel_jobs, \
         r.workspace_fingerprint, r.submitter_id \
         FROM run_jobs j JOIN runs r ON r.run_id = j.run_id \
         LEFT JOIN scheduler_submitters submitter ON submitter.submitter_id = r.submitter_id \
         WHERE (j.state = 'ready' OR (j.state = 'retrying' AND j.next_eligible_at_ms <= ?1)) \
         AND r.state IN ('queued', 'running') AND r.dispatch_stopped = 0 \
         ORDER BY COALESCE(submitter.last_scheduled_turn, 0), \
         (SELECT MIN(first_run.created_at_ms) FROM runs first_run WHERE first_run.submitter_id = r.submitter_id), \
         r.submitter_id, r.last_scheduled_turn, r.created_at_ms, r.run_id, \
         j.next_eligible_at_ms, j.ready_order, j.ordinal, j.job_id",
    )?;
    statement
        .query_map([i64::try_from(now_ms)?], |row| {
            let max_parallel = row.get::<_, i64>(3)?;
            Ok(ReadyJob {
                run_id: row.get(0)?,
                job_id: row.get(1)?,
                definition: row.get(2)?,
                max_parallel: u64::try_from(max_parallel)
                    .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(3, max_parallel))?,
                workspace_fingerprint: row.get(4)?,
                submitter_id: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

impl super::super::RunStore {
    pub fn workspace_fingerprint(&self, run_id: &str) -> Result<Option<String>> {
        let connection = self.open_connection()?;
        connection
            .query_row(
                "SELECT workspace_fingerprint FROM runs WHERE run_id = ?1",
                [run_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }
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
