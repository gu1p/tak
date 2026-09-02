use anyhow::{Result, bail};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use tak_proto::worker_v2::{
    DispatchAttemptRequest, DispatchDisposition, WorkerAttemptIdentity, WorkerAttemptState,
    encode_dispatch_request,
};

use super::{SubmitAttemptStore, unix_epoch_ms};

mod cancellation;
mod lifecycle;
mod management;
mod observation;
mod outputs;
mod retention;
mod schema;

pub(super) fn ensure_schema(connection: &rusqlite::Connection) -> Result<()> {
    schema::ensure_schema(connection)
}

impl SubmitAttemptStore {
    pub fn register_worker_v2_attempt(
        &self,
        request: &DispatchAttemptRequest,
    ) -> Result<DispatchDisposition> {
        Ok(self
            .register_worker_v2_attempt_with(request, || Ok(Some(())))?
            .expect("no-op worker admission is always available")
            .0)
    }

    pub(crate) fn register_worker_v2_attempt_with<T>(
        &self,
        request: &DispatchAttemptRequest,
        reserve: impl FnOnce() -> Result<Option<T>>,
    ) -> Result<Option<(DispatchDisposition, Option<T>)>> {
        let request_json = String::from_utf8(encode_dispatch_request(request)?)?;
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let disposition = register(&transaction, request, &request_json)?;
        let reservation = if disposition == DispatchDisposition::Accepted {
            let Some(reservation) = reserve()? else {
                return Ok(None);
            };
            Some(reservation)
        } else {
            None
        };
        transaction.commit()?;
        Ok(Some((disposition, reservation)))
    }

    pub fn recover_worker_v2_attempts_after_restart(&self) -> Result<()> {
        let connection = self.open_connection()?;
        connection.execute(
            "UPDATE worker_v2_attempts SET state='missing', updated_at_ms=?1 \
             WHERE state IN ('accepted','running','cancelling')",
            [unix_epoch_ms()],
        )?;
        Ok(())
    }
}

fn current_state(connection: &Connection, identity: &WorkerAttemptIdentity) -> Result<String> {
    connection
        .query_row(
            "SELECT attempt.state FROM worker_v2_attempts attempt JOIN worker_v2_heads head \
             ON head.run_id=attempt.run_id AND head.job_id=attempt.job_id AND \
             head.fencing_token=attempt.fencing_token WHERE attempt.run_id=?1 AND \
             attempt.job_id=?2 AND attempt.authored_attempt=?3 AND \
             attempt.dispatch_generation=?4 AND attempt.fencing_token=?5 AND attempt.node_id=?6",
            params![
                identity.run_id,
                identity.job_id,
                identity.authored_attempt,
                identity.dispatch_generation,
                identity.fencing_token,
                identity.node_id
            ],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("worker attempt fence is no longer current"))
}

fn register(
    transaction: &Transaction<'_>,
    request: &DispatchAttemptRequest,
    request_json: &str,
) -> Result<DispatchDisposition> {
    let identity = &request.identity;
    let head = transaction
        .query_row(
            "SELECT authored_attempt,dispatch_generation,fencing_token FROM worker_v2_heads \
             WHERE run_id=?1 AND job_id=?2",
            params![identity.run_id, identity.job_id],
            |row| {
                Ok((
                    row.get::<_, u32>(0)?,
                    row.get::<_, u32>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?;
    if let Some((attempt, generation, fence)) = head {
        let incoming = (identity.authored_attempt, identity.dispatch_generation);
        match incoming.cmp(&(attempt, generation)) {
            std::cmp::Ordering::Less => return Ok(DispatchDisposition::Stale),
            std::cmp::Ordering::Equal => {
                return exact_duplicate(transaction, request, request_json, &fence);
            }
            std::cmp::Ordering::Greater => {
                transaction.execute(
                    "UPDATE worker_v2_attempts SET state='missing',cancellation_requested=1,\
                     updated_at_ms=?2 \
                     WHERE fencing_token=?1 AND state IN ('accepted','running','cancelling')",
                    params![fence, unix_epoch_ms()],
                )?;
            }
        }
    }
    insert_attempt(transaction, request, request_json)?;
    Ok(DispatchDisposition::Accepted)
}

fn exact_duplicate(
    transaction: &Transaction<'_>,
    request: &DispatchAttemptRequest,
    request_json: &str,
    head_fence: &str,
) -> Result<DispatchDisposition> {
    let stored: String = transaction.query_row(
        "SELECT request_json FROM worker_v2_attempts WHERE fencing_token=?1",
        [head_fence],
        |row| row.get(0),
    )?;
    if head_fence == request.identity.fencing_token && stored == request_json {
        return Ok(DispatchDisposition::Duplicate);
    }
    bail!("conflicting worker dispatch for the same authored attempt generation")
}

fn insert_attempt(
    transaction: &Transaction<'_>,
    request: &DispatchAttemptRequest,
    request_json: &str,
) -> Result<()> {
    let identity = &request.identity;
    let now = unix_epoch_ms();
    transaction.execute(
        "INSERT INTO worker_v2_attempts (fencing_token,run_id,job_id,node_id,authored_attempt,\
         dispatch_generation,payload_digest,request_json,state,created_at_ms,updated_at_ms) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'accepted',?9,?9)",
        params![
            identity.fencing_token,
            identity.run_id,
            identity.job_id,
            identity.node_id,
            identity.authored_attempt,
            identity.dispatch_generation,
            request.payload_digest,
            request_json,
            now
        ],
    )?;
    transaction.execute(
        "INSERT INTO worker_v2_heads (run_id,job_id,authored_attempt,dispatch_generation,\
         fencing_token) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(run_id,job_id) DO UPDATE SET \
         authored_attempt=excluded.authored_attempt,dispatch_generation=excluded.dispatch_generation,\
         fencing_token=excluded.fencing_token",
        params![identity.run_id,identity.job_id,identity.authored_attempt,
            identity.dispatch_generation,identity.fencing_token],
    )?;
    Ok(())
}

fn protocol_state(state: &str) -> Result<WorkerAttemptState> {
    match state {
        "accepted" | "running" | "cancelling" => Ok(WorkerAttemptState::Running),
        "succeeded" | "failed" | "cancelled" => Ok(WorkerAttemptState::Completed),
        "missing" => Ok(WorkerAttemptState::Missing),
        other => bail!("invalid persisted worker attempt state `{other}`"),
    }
}
