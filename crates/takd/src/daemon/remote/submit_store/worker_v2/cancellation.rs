use anyhow::Result;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use tak_proto::worker_v2::{CancelDisposition, WorkerAttemptIdentity};

use super::{SubmitAttemptStore, unix_epoch_ms};

impl SubmitAttemptStore {
    pub fn cancel_worker_v2_attempt(
        &self,
        identity: &WorkerAttemptIdentity,
    ) -> Result<CancelDisposition> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some((state, requested)) = attempt_state(&transaction, identity)? else {
            return Ok(CancelDisposition::Stale);
        };
        if !is_current(&transaction, identity)? {
            return Ok(CancelDisposition::Stale);
        }
        let disposition = if matches!(state.as_str(), "succeeded" | "failed" | "cancelled") {
            CancelDisposition::AlreadyTerminal
        } else if requested {
            CancelDisposition::Duplicate
        } else if matches!(state.as_str(), "accepted" | "running" | "cancelling") {
            transaction.execute(
                "UPDATE worker_v2_attempts SET state='cancelling',cancellation_requested=1,\
                 updated_at_ms=?2 WHERE fencing_token=?1",
                params![identity.fencing_token, unix_epoch_ms()],
            )?;
            CancelDisposition::Requested
        } else {
            CancelDisposition::Stale
        };
        transaction.commit()?;
        Ok(disposition)
    }

    pub fn worker_v2_cancellation_requested(
        &self,
        identity: &WorkerAttemptIdentity,
    ) -> Result<bool> {
        let connection = self.open_connection()?;
        Ok(attempt_state(&connection, identity)?
            .is_some_and(|(_, cancellation_requested)| cancellation_requested))
    }
}

fn attempt_state(
    connection: &rusqlite::Connection,
    identity: &WorkerAttemptIdentity,
) -> Result<Option<(String, bool)>> {
    Ok(connection
        .query_row(
            "SELECT state,cancellation_requested FROM worker_v2_attempts WHERE run_id=?1 AND \
             job_id=?2 AND authored_attempt=?3 AND dispatch_generation=?4 AND \
             fencing_token=?5 AND node_id=?6",
            params![
                identity.run_id,
                identity.job_id,
                identity.authored_attempt,
                identity.dispatch_generation,
                identity.fencing_token,
                identity.node_id
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?)
}

fn is_current(connection: &rusqlite::Connection, identity: &WorkerAttemptIdentity) -> Result<bool> {
    Ok(connection
        .query_row(
            "SELECT 1 FROM worker_v2_heads WHERE run_id=?1 AND job_id=?2 AND fencing_token=?3",
            params![identity.run_id, identity.job_id, identity.fencing_token],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}
