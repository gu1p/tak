use anyhow::{Context, Result};
use rusqlite::{Transaction, params};
use tak_core::v2::ResolvedJob;

use crate::daemon::scheduler::DispatchCommand;

pub(super) fn backfill(transaction: &Transaction<'_>) -> Result<()> {
    let attempts = {
        let mut statement = transaction.prepare(
            "SELECT attempt.run_id, attempt.job_id, attempt.authored_attempt, \
             attempt.dispatch_generation, attempt.node_id, job.definition_json \
             FROM run_attempts attempt JOIN run_jobs job USING (run_id, job_id)",
        )?;
        statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (run_id, job_id, attempt, generation, node_id, definition) in attempts {
        let job: ResolvedJob = serde_json::from_str(&definition)?;
        let transport = job
            .placement_candidates
            .iter()
            .find(|candidate| candidate.node_id == node_id)
            .with_context(|| {
                format!("attempt node `{node_id}` is not a candidate for job `{job_id}`")
            })?
            .transport
            .as_deref();
        transaction.execute(
            "UPDATE run_attempts SET transport=?5 WHERE run_id=?1 AND job_id=?2 \
             AND authored_attempt=?3 AND dispatch_generation=?4",
            params![run_id, job_id, attempt, generation, transport],
        )?;
    }
    rewrite_dispatch_outbox(transaction)
}

fn rewrite_dispatch_outbox(transaction: &Transaction<'_>) -> Result<()> {
    let commands = {
        let mut statement = transaction.prepare(
            "SELECT attempt.run_id, attempt.job_id, attempt.node_id, attempt.transport, \
             attempt.authored_attempt, attempt.dispatch_generation, attempt.fencing_token \
             FROM run_dispatch_outbox outbox JOIN run_attempts attempt \
             USING (run_id, job_id, authored_attempt, dispatch_generation)",
        )?;
        statement
            .query_map([], command_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for command in commands {
        transaction.execute(
            "UPDATE run_dispatch_outbox SET payload_json=?5 WHERE run_id=?1 AND job_id=?2 \
             AND authored_attempt=?3 AND dispatch_generation=?4",
            params![
                command.run_id,
                command.job_id,
                command.authored_attempt,
                command.dispatch_generation,
                serde_json::to_string(&command)?,
            ],
        )?;
    }
    Ok(())
}

fn command_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DispatchCommand> {
    let attempt = row.get::<_, i64>(4)?;
    let generation = row.get::<_, i64>(5)?;
    Ok(DispatchCommand {
        run_id: row.get(0)?,
        job_id: row.get(1)?,
        node_id: row.get(2)?,
        transport: row.get(3)?,
        authored_attempt: u32::try_from(attempt)
            .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(4, attempt))?,
        dispatch_generation: u32::try_from(generation)
            .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(5, generation))?,
        fencing_token: row.get(6)?,
    })
}
