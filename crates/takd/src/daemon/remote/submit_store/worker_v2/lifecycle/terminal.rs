use super::super::{SubmitAttemptStore, current_state, unix_epoch_ms};
use super::{require_active, retry_transient_lock};
use anyhow::{Result, bail};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use tak_proto::worker_v2::{WorkerAttemptIdentity, WorkerTerminal, WorkerTerminalOutcome};

use super::terminal_support::{
    event_watermark, existing_terminal, terminal_outputs, terminal_state, validate_terminal,
};

impl SubmitAttemptStore {
    pub fn complete_worker_v2_attempt(
        &self,
        identity: &WorkerAttemptIdentity,
        outcome: WorkerTerminalOutcome,
        terminal_digest: &str,
    ) -> Result<WorkerTerminal> {
        self.complete_worker_v2_attempt_with_exit_code(identity, outcome, terminal_digest, None)
    }

    pub fn complete_worker_v2_attempt_with_exit_code(
        &self,
        identity: &WorkerAttemptIdentity,
        outcome: WorkerTerminalOutcome,
        terminal_digest: &str,
        exit_code: Option<i32>,
    ) -> Result<WorkerTerminal> {
        self.complete_worker_v2_attempt_with_runtime(
            identity,
            outcome,
            terminal_digest,
            exit_code,
            None,
            None,
        )
    }

    pub fn complete_worker_v2_attempt_with_runtime(
        &self,
        identity: &WorkerAttemptIdentity,
        outcome: WorkerTerminalOutcome,
        terminal_digest: &str,
        exit_code: Option<i32>,
        runtime_kind: Option<String>,
        runtime_engine: Option<String>,
    ) -> Result<WorkerTerminal> {
        retry_transient_lock(|| {
            self.complete_worker_v2_attempt_with_runtime_once(
                identity,
                outcome,
                terminal_digest,
                exit_code,
                runtime_kind.clone(),
                runtime_engine.clone(),
            )
        })
    }

    fn complete_worker_v2_attempt_with_runtime_once(
        &self,
        identity: &WorkerAttemptIdentity,
        outcome: WorkerTerminalOutcome,
        terminal_digest: &str,
        exit_code: Option<i32>,
        runtime_kind: Option<String>,
        runtime_engine: Option<String>,
    ) -> Result<WorkerTerminal> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let state = current_state(&transaction, identity)?;
        let outcome = if matches!(state.as_str(), "cancelling" | "cancelled") {
            WorkerTerminalOutcome::Cancelled
        } else {
            outcome
        };
        let exit_code = (outcome != WorkerTerminalOutcome::Cancelled)
            .then_some(exit_code)
            .flatten();
        if matches!(state.as_str(), "succeeded" | "failed" | "cancelled") {
            return existing_terminal(&transaction, identity, outcome, terminal_digest, exit_code);
        }
        require_active(state)?;
        if outcome == WorkerTerminalOutcome::Cancelled {
            transaction.execute(
                "DELETE FROM worker_v2_outputs WHERE fencing_token=?1",
                [&identity.fencing_token],
            )?;
        }
        let outputs = terminal_outputs(&transaction, &identity.fencing_token)?;
        if outcome != WorkerTerminalOutcome::Succeeded && !outputs.is_empty() {
            bail!("only a successful worker attempt may publish outputs");
        }
        let terminal = WorkerTerminal {
            outcome,
            terminal_digest: terminal_digest.to_owned(),
            event_watermark: event_watermark(&transaction, &identity.fencing_token)?,
            outputs,
            exit_code,
            runtime_kind,
            runtime_engine,
        };
        validate_terminal(identity, &terminal)?;
        transaction.execute(
            "UPDATE worker_v2_attempts SET state=?2,terminal_json=?3,updated_at_ms=?4 \
             WHERE fencing_token=?1",
            params![
                identity.fencing_token,
                terminal_state(outcome),
                serde_json::to_string(&terminal)?,
                unix_epoch_ms()
            ],
        )?;
        transaction.commit()?;
        Ok(terminal)
    }

    pub fn acknowledge_worker_v2_terminal(
        &self,
        identity: &WorkerAttemptIdentity,
        terminal_digest: &str,
    ) -> Result<()> {
        self.acknowledge_worker_v2_terminal_for_run(identity, terminal_digest, false)
    }

    pub(crate) fn acknowledge_worker_v2_terminal_for_run(
        &self,
        identity: &WorkerAttemptIdentity,
        terminal_digest: &str,
        run_terminal: bool,
    ) -> Result<()> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (state, terminal) = transaction.query_row(
            "SELECT state,terminal_json FROM worker_v2_attempts WHERE run_id=?1 AND job_id=?2 \
             AND authored_attempt=?3 AND dispatch_generation=?4 AND fencing_token=?5 AND node_id=?6",
            params![identity.run_id, identity.job_id, identity.authored_attempt,
                identity.dispatch_generation, identity.fencing_token, identity.node_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )?;
        if !matches!(state.as_str(), "succeeded" | "failed" | "cancelled") {
            bail!("worker terminal cannot be acknowledged before completion");
        }
        let terminal: WorkerTerminal = serde_json::from_str(&terminal)?;
        if terminal.terminal_digest != terminal_digest {
            bail!("worker terminal acknowledgement digest mismatch");
        }
        let changed = transaction.execute(
            "UPDATE worker_v2_attempts SET acknowledged=1,updated_at_ms=?2 \
             WHERE fencing_token=?1 AND run_id=?3 AND job_id=?4 AND authored_attempt=?5 AND \
             dispatch_generation=?6 AND node_id=?7",
            params![
                identity.fencing_token,
                unix_epoch_ms(),
                identity.run_id,
                identity.job_id,
                identity.authored_attempt,
                identity.dispatch_generation,
                identity.node_id
            ],
        )?;
        if changed != 1 {
            bail!("worker terminal acknowledgement identity mismatch");
        }
        if run_terminal {
            transaction.execute(
                "INSERT INTO worker_v2_terminal_runs (run_id,released_at_ms) VALUES (?1,?2) \
                 ON CONFLICT(run_id) DO NOTHING",
                params![identity.run_id, unix_epoch_ms()],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn worker_v2_terminal_is_acknowledged(
        &self,
        identity: &WorkerAttemptIdentity,
    ) -> Result<bool> {
        let connection = self.open_connection()?;
        connection
            .query_row(
                "SELECT acknowledged FROM worker_v2_attempts WHERE run_id=?1 AND job_id=?2 \
                 AND authored_attempt=?3 AND dispatch_generation=?4 AND fencing_token=?5 \
                 AND node_id=?6",
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
            .optional()
            .map(|value| value.unwrap_or(false))
            .map_err(Into::into)
    }
}
