use anyhow::{Result, bail};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use tak_core::v2::ResolvedJob;
use tak_proto::local_daemon::v2::RunEventKind;

use crate::daemon::scheduler::{DispatchCommand, ResultAcceptance};

use super::super::RunStore;
use super::super::events::{append_event, append_job_event, now_ms, sqlite_i64};

impl RunStore {
    pub fn pending_cancellations(&self) -> Result<Vec<DispatchCommand>> {
        let connection = self.open_connection()?;
        let mut statement = connection.prepare(
            "SELECT outbox.run_id, outbox.job_id, outbox.node_id, outbox.authored_attempt, outbox.dispatch_generation, outbox.fencing_token \
             FROM run_cancel_outbox outbox JOIN run_attempts attempt USING (run_id, job_id, authored_attempt, dispatch_generation) \
             WHERE outbox.delivered_at_ms IS NULL AND attempt.state = 'cancelling' AND attempt.released_at_ms IS NULL ORDER BY outbox.rowid",
        )?;
        statement
            .query_map([], command_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn ack_cancellation(&self, command: &DispatchCommand) -> Result<ResultAcceptance> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let attempt = transaction
            .query_row(
                "SELECT fencing_token, node_id, state FROM run_attempts WHERE run_id = ?1 AND job_id = ?2 AND authored_attempt = ?3 AND dispatch_generation = ?4",
                params![command.run_id, command.job_id, command.authored_attempt, command.dispatch_generation],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)),
            )
            .optional()?;
        let Some((token, node_id, state)) = attempt else {
            return Ok(ResultAcceptance::Stale);
        };
        if token != command.fencing_token || node_id != command.node_id {
            return Ok(ResultAcceptance::Stale);
        }
        if state == "cancelled" {
            return Ok(ResultAcceptance::Duplicate);
        }
        if state != "cancelling" {
            return Ok(ResultAcceptance::Stale);
        }
        settle_cancellation(&transaction, command)?;
        transaction.commit()?;
        Ok(ResultAcceptance::Applied)
    }
}

fn settle_cancellation(
    transaction: &rusqlite::Transaction<'_>,
    command: &DispatchCommand,
) -> Result<()> {
    let now = sqlite_i64(now_ms()?, "timestamp")?;
    let released = transaction.execute(
        "UPDATE run_attempts SET state = 'cancelled', outcome = 'cancelled', finished_at_ms = ?6, released_at_ms = ?6 WHERE run_id = ?1 AND job_id = ?2 AND authored_attempt = ?3 AND dispatch_generation = ?4 AND fencing_token = ?5 AND state = 'cancelling'",
        params![command.run_id, command.job_id, command.authored_attempt,
            command.dispatch_generation, command.fencing_token, now],
    )?;
    let delivered = transaction.execute(
        "UPDATE run_cancel_outbox SET delivered_at_ms = COALESCE(delivered_at_ms, ?6) WHERE run_id = ?1 AND job_id = ?2 AND authored_attempt = ?3 AND dispatch_generation = ?4 AND fencing_token = ?5",
        params![command.run_id, command.job_id, command.authored_attempt,
            command.dispatch_generation, command.fencing_token, now],
    )?;
    if released != 1 || delivered != 1 {
        bail!("cancellation action is no longer current");
    }
    let definition = transaction.query_row(
        "SELECT definition_json FROM run_jobs WHERE run_id = ?1 AND job_id = ?2",
        params![command.run_id, command.job_id],
        |row| row.get::<_, String>(0),
    )?;
    let job: ResolvedJob = serde_json::from_str(&definition)?;
    transaction.execute(
        "UPDATE run_jobs SET state = 'cancelled' WHERE run_id = ?1 AND job_id = ?2 AND state = 'cancelling'",
        params![command.run_id, command.job_id],
    )?;
    append_job_event(
        transaction,
        &command.run_id,
        RunEventKind::Cancelled,
        &command.job_id,
        &job.task_ids,
        &command.node_id,
        "job cancelled",
    )?;
    finish_run_if_settled(transaction, &command.run_id, now)
}

fn finish_run_if_settled(
    transaction: &rusqlite::Transaction<'_>,
    run_id: &str,
    now: i64,
) -> Result<()> {
    let active = transaction.query_row(
        "SELECT COUNT(*) FROM run_attempts WHERE run_id = ?1 AND released_at_ms IS NULL",
        [run_id],
        |row| row.get::<_, i64>(0),
    )?;
    if active != 0 {
        return Ok(());
    }
    transaction.execute(
        "UPDATE runs SET state = 'cancelled', updated_at_ms = ?2 WHERE run_id = ?1 AND state = 'cancelling'",
        params![run_id, now],
    )?;
    append_event(
        transaction,
        run_id,
        RunEventKind::Cancelled,
        "run cancelled",
    )?;
    Ok(())
}

fn command_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DispatchCommand> {
    let attempt = row.get::<_, i64>(3)?;
    let generation = row.get::<_, i64>(4)?;
    Ok(DispatchCommand {
        run_id: row.get(0)?,
        job_id: row.get(1)?,
        node_id: row.get(2)?,
        authored_attempt: u32::try_from(attempt)
            .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(3, attempt))?,
        dispatch_generation: u32::try_from(generation)
            .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(4, generation))?,
        fencing_token: row.get(5)?,
    })
}
