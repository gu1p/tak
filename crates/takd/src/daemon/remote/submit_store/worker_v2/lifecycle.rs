use anyhow::{Result, bail};
use base64::Engine;
use rusqlite::{TransactionBehavior, params};
use sha2::{Digest, Sha256};
use tak_proto::worker_v2::{
    ObserveAttemptResponse, WorkerAttemptEvent, WorkerAttemptIdentity, WorkerAttemptState,
    WorkerOutputStream, encode_observe_response,
};

use super::{SubmitAttemptStore, current_state, unix_epoch_ms};

mod terminal;
mod terminal_support;

impl SubmitAttemptStore {
    pub fn mark_worker_v2_running(&self, identity: &WorkerAttemptIdentity) -> Result<()> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let state = current_state(&transaction, identity)?;
        if state == "running" {
            return Ok(());
        }
        if state != "accepted" {
            bail!("worker attempt cannot start from state `{state}`");
        }
        let changed = transaction.execute(
            "UPDATE worker_v2_attempts SET state='running',updated_at_ms=?2 \
             WHERE fencing_token=?1 AND state='accepted'",
            params![identity.fencing_token, unix_epoch_ms()],
        )?;
        if changed != 1 {
            bail!("worker attempt state changed before it started");
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn append_worker_v2_event(
        &self,
        identity: &WorkerAttemptIdentity,
        task_id: &str,
        stream: WorkerOutputStream,
        chunk: &[u8],
    ) -> Result<WorkerAttemptEvent> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_active(current_state(&transaction, identity)?)?;
        let seq_sql: i64 = transaction.query_row(
            "SELECT COALESCE(MAX(seq),0)+1 FROM worker_v2_events WHERE fencing_token=?1",
            [&identity.fencing_token],
            |row| row.get(0),
        )?;
        let seq = u64::try_from(seq_sql)?;
        let event = WorkerAttemptEvent {
            seq,
            task_id: task_id.to_owned(),
            stream,
            chunk_base64: base64::engine::general_purpose::STANDARD.encode(chunk),
            chunk_sha256: format!("{:x}", Sha256::digest(chunk)),
        };
        validate_event(identity, &event)?;
        transaction.execute(
            "INSERT INTO worker_v2_events (fencing_token,seq,event_json) VALUES (?1,?2,?3)",
            params![
                identity.fencing_token,
                seq_sql,
                serde_json::to_string(&event)?
            ],
        )?;
        transaction.commit()?;
        Ok(event)
    }
}

pub(super) fn require_active(state: String) -> Result<()> {
    if matches!(state.as_str(), "accepted" | "running" | "cancelling") {
        return Ok(());
    }
    bail!("worker attempt is not active")
}

fn validate_event(identity: &WorkerAttemptIdentity, event: &WorkerAttemptEvent) -> Result<()> {
    let response = ObserveAttemptResponse {
        protocol_version: 2,
        fencing_token: identity.fencing_token.clone(),
        state: WorkerAttemptState::Running,
        events: vec![event.clone()],
        next_event: event.seq,
        terminal: None,
    };
    encode_observe_response(&response).map(|_| ())
}
