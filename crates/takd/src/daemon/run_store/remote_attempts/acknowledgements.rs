use anyhow::Result;
use rusqlite::{Transaction, TransactionBehavior, params};

use crate::daemon::scheduler::{DispatchCommand, ResultAcceptance};

use super::super::RunStore;
use super::super::events::{now_ms, sqlite_i64};

pub(in crate::daemon) struct WorkerTerminalAck {
    pub(in crate::daemon) command: DispatchCommand,
    pub(in crate::daemon) terminal_digest: String,
    pub(in crate::daemon) run_terminal: bool,
}

pub(in crate::daemon::run_store) fn queue(
    transaction: &Transaction<'_>,
    command: &DispatchCommand,
    terminal_digest: &str,
) -> Result<()> {
    if command.transport.is_none() {
        return Ok(());
    }
    transaction.execute(
        "INSERT INTO run_worker_ack_outbox (run_id,job_id,authored_attempt,dispatch_generation,\
         fencing_token,terminal_digest) VALUES (?1,?2,?3,?4,?5,?6) \
         ON CONFLICT(run_id,job_id,authored_attempt,dispatch_generation) DO UPDATE SET \
         terminal_digest=excluded.terminal_digest,delivered_at_ms=NULL",
        params![
            command.run_id,
            command.job_id,
            command.authored_attempt,
            command.dispatch_generation,
            command.fencing_token,
            terminal_digest
        ],
    )?;
    Ok(())
}

pub(in crate::daemon::run_store) fn rearm_terminal_run(
    transaction: &Transaction<'_>,
    run_id: &str,
) -> Result<()> {
    transaction.execute(
        "UPDATE run_worker_ack_outbox SET delivered_at_ms=NULL WHERE run_id=?1",
        [run_id],
    )?;
    Ok(())
}

impl RunStore {
    pub(in crate::daemon) fn pending_worker_terminal_acks(&self) -> Result<Vec<WorkerTerminalAck>> {
        let connection = self.open_connection()?;
        let mut statement = connection.prepare(
            "SELECT attempt.run_id,attempt.job_id,attempt.node_id,attempt.transport,\
             attempt.authored_attempt,attempt.dispatch_generation,attempt.fencing_token,\
             outbox.terminal_digest,run.state IN ('succeeded','failed','cancelled') \
             FROM run_worker_ack_outbox outbox JOIN run_attempts attempt \
             USING (run_id,job_id,authored_attempt,dispatch_generation) \
             JOIN runs run ON run.run_id=outbox.run_id \
             WHERE outbox.delivered_at_ms IS NULL ORDER BY outbox.rowid",
        )?;
        statement
            .query_map([], |row| {
                let attempt = row.get::<_, i64>(4)?;
                let generation = row.get::<_, i64>(5)?;
                Ok(WorkerTerminalAck {
                    command: DispatchCommand {
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
                    terminal_digest: row.get(7)?,
                    run_terminal: row.get(8)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()
            .map_err(Into::into)
    }

    pub(in crate::daemon) fn mark_worker_terminal_acknowledged(
        &self,
        ack: &WorkerTerminalAck,
    ) -> Result<ResultAcceptance> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let updated = transaction.execute(
            "UPDATE run_worker_ack_outbox SET delivered_at_ms=?7 WHERE run_id=?1 AND job_id=?2 \
             AND authored_attempt=?3 AND dispatch_generation=?4 AND fencing_token=?5 \
             AND terminal_digest=?6 AND delivered_at_ms IS NULL AND ?8=(SELECT state IN \
             ('succeeded','failed','cancelled') FROM runs WHERE run_id=?1)",
            params![
                ack.command.run_id,
                ack.command.job_id,
                ack.command.authored_attempt,
                ack.command.dispatch_generation,
                ack.command.fencing_token,
                ack.terminal_digest,
                sqlite_i64(now_ms()?, "terminal acknowledgement timestamp")?,
                ack.run_terminal
            ],
        )?;
        transaction.commit()?;
        Ok(if updated == 1 {
            ResultAcceptance::Applied
        } else {
            ResultAcceptance::Duplicate
        })
    }
}
