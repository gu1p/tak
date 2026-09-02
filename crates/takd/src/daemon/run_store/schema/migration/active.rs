use anyhow::Result;
use rusqlite::{Transaction, params};

use crate::daemon::scheduler::DispatchCommand;

pub(super) fn backfill(transaction: &Transaction<'_>) -> Result<()> {
    transaction.execute(
        "UPDATE run_jobs SET \
         attempt = (SELECT authored_attempt FROM run_attempts a WHERE a.run_id = run_jobs.run_id AND a.job_id = run_jobs.job_id AND a.released_at_ms IS NULL ORDER BY authored_attempt DESC, dispatch_generation DESC LIMIT 1), \
         dispatch_generation = (SELECT dispatch_generation FROM run_attempts a WHERE a.run_id = run_jobs.run_id AND a.job_id = run_jobs.job_id AND a.released_at_ms IS NULL ORDER BY authored_attempt DESC, dispatch_generation DESC LIMIT 1), \
         current_fencing_token = (SELECT fencing_token FROM run_attempts a WHERE a.run_id = run_jobs.run_id AND a.job_id = run_jobs.job_id AND a.released_at_ms IS NULL ORDER BY authored_attempt DESC, dispatch_generation DESC LIMIT 1) \
         WHERE EXISTS (SELECT 1 FROM run_attempts a WHERE a.run_id = run_jobs.run_id AND a.job_id = run_jobs.job_id AND a.released_at_ms IS NULL)",
        [],
    )?;
    for (command, state, reserved_at) in active_attempts(transaction)? {
        let delivered_at = (state != "transferring").then_some(reserved_at);
        transaction.execute(
            "INSERT OR IGNORE INTO run_dispatch_outbox (run_id, job_id, authored_attempt, dispatch_generation, fencing_token, payload_json, delivered_at_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![command.run_id, command.job_id, command.authored_attempt,
                command.dispatch_generation, command.fencing_token,
                serde_json::to_string(&command)?, delivered_at],
        )?;
    }
    Ok(())
}

fn active_attempts(transaction: &Transaction<'_>) -> Result<Vec<(DispatchCommand, String, i64)>> {
    let mut statement = transaction.prepare(
        "SELECT run_id, job_id, node_id, transport, authored_attempt, dispatch_generation, \
         fencing_token, state, reserved_at_ms FROM run_attempts WHERE released_at_ms IS NULL",
    )?;
    statement
        .query_map([], |row| {
            let attempt = row.get::<_, i64>(4)?;
            let generation = row.get::<_, i64>(5)?;
            Ok((
                DispatchCommand {
                    run_id: row.get(0)?,
                    job_id: row.get(1)?,
                    node_id: row.get(2)?,
                    transport: row.get(3)?,
                    authored_attempt: u32::try_from(attempt)
                        .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(4, attempt))?,
                    dispatch_generation: u32::try_from(generation)
                        .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(5, generation))?,
                    fencing_token: row.get(6)?,
                },
                row.get(7)?,
                row.get(8)?,
            ))
        })?
        .collect::<rusqlite::Result<_>>()
        .map_err(Into::into)
}
