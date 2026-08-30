use anyhow::{Result, bail};
use rusqlite::{OptionalExtension, Transaction, params};
use tak_core::v2::{Affinity, ResolvedJob};

pub(super) fn backfill(transaction: &Transaction<'_>) -> Result<()> {
    transaction.execute(
        "UPDATE run_attempts SET accepted_at_ms = reserved_at_ms \
         WHERE state = 'running' AND accepted_at_ms IS NULL",
        [],
    )?;
    let attempts = {
        let mut statement = transaction.prepare(
            "SELECT attempt.run_id, attempt.node_id, attempt.reserved_at_ms, \
             attempt.released_at_ms, job.definition_json \
             FROM run_attempts attempt JOIN run_jobs job USING (run_id, job_id) \
             WHERE NOT EXISTS (SELECT 1 FROM scheduler_node_losses loss \
                 WHERE loss.node_id = attempt.node_id) \
             ORDER BY attempt.reserved_at_ms, attempt.run_id, attempt.job_id, \
             attempt.authored_attempt, attempt.dispatch_generation",
        )?;
        statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (run_id, node_id, reserved_at, released_at, serialized) in attempts {
        let job: ResolvedJob = serde_json::from_str(&serialized)?;
        let Some(affinity) = job.affinity else {
            continue;
        };
        let (group, required) = match affinity {
            Affinity::PreferSameNode { group } => (group, false),
            Affinity::RequireSameNode { group } => (group, true),
        };
        let home = transaction
            .query_row(
                "SELECT node_id FROM run_affinity_bindings \
                 WHERE run_id = ?1 AND affinity_group = ?2",
                params![run_id, group],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if required && released_at.is_none() && home.as_deref().is_some_and(|home| home != node_id)
        {
            bail!("conflicting hard affinity homes in protocol-v3 run `{run_id}`");
        }
        transaction.execute(
            "INSERT OR IGNORE INTO run_affinity_bindings \
             (run_id, affinity_group, node_id, bound_at_ms) VALUES (?1, ?2, ?3, ?4)",
            params![run_id, group, node_id, reserved_at],
        )?;
    }
    Ok(())
}
