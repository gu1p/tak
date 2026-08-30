use anyhow::{Result, bail};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use tak_core::v2::ResolvedJob;
use tak_proto::local_daemon::v2::RunEventKind;

use crate::daemon::scheduler::{
    AttemptCompletion, DispatchCommand, ResultAcceptance, UnknownOutcomeResolution,
};

use super::RunStore;
use super::events::{append_job_event, now_ms, sqlite_i64};

mod node_loss;
mod recovery;
mod transitions;
mod unknown;
mod validation;

use validation::validate_digest;

impl RunStore {
    pub fn ack_dispatch(&self, command: &DispatchCommand) -> Result<ResultAcceptance> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(attempt) = load_attempt(&transaction, command)? else {
            return Ok(ResultAcceptance::Stale);
        };
        if !attempt.matches(command) {
            return Ok(ResultAcceptance::Stale);
        }
        if attempt.state == "running" {
            return Ok(ResultAcceptance::Duplicate);
        }
        if attempt.state != "transferring" {
            return Ok(ResultAcceptance::Stale);
        }
        let now = sqlite_i64(now_ms()?, "timestamp")?;
        transaction.execute(
            "UPDATE run_attempts SET state = 'running', accepted_at_ms = ?6 WHERE run_id = ?1 AND job_id = ?2 AND authored_attempt = ?3 AND dispatch_generation = ?4 AND fencing_token = ?5",
            params![command.run_id, command.job_id, command.authored_attempt,
                command.dispatch_generation, command.fencing_token, now],
        )?;
        transaction.execute(
            "UPDATE run_dispatch_outbox SET delivered_at_ms = COALESCE(delivered_at_ms, ?6) WHERE run_id = ?1 AND job_id = ?2 AND authored_attempt = ?3 AND dispatch_generation = ?4 AND fencing_token = ?5",
            params![command.run_id, command.job_id, command.authored_attempt,
                command.dispatch_generation, command.fencing_token, now],
        )?;
        let job = load_job(&transaction, command)?;
        transaction.execute(
            "UPDATE run_jobs SET state = 'running' WHERE run_id = ?1 AND job_id = ?2 AND current_fencing_token = ?3",
            params![command.run_id, command.job_id, command.fencing_token],
        )?;
        append_job_event(
            &transaction,
            &command.run_id,
            RunEventKind::Running,
            &command.job_id,
            &job.task_ids,
            &command.node_id,
            "worker accepted job",
        )?;
        transaction.commit()?;
        Ok(ResultAcceptance::Applied)
    }

    pub fn complete_attempt(
        &self,
        command: &DispatchCommand,
        completion: AttemptCompletion,
    ) -> Result<ResultAcceptance> {
        validate_digest(completion.digest())?;
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(attempt) = load_attempt(&transaction, command)? else {
            return Ok(ResultAcceptance::Stale);
        };
        let outcome = if matches!(completion, AttemptCompletion::Succeeded { .. }) {
            "succeeded"
        } else {
            "failed"
        };
        if !attempt.matches(command) {
            return Ok(ResultAcceptance::Stale);
        }
        if attempt.digest.as_deref() == Some(completion.digest())
            && attempt.outcome.as_deref() == Some(outcome)
        {
            return Ok(ResultAcceptance::Duplicate);
        }
        if !matches!(attempt.state.as_str(), "transferring" | "running") {
            return Ok(ResultAcceptance::Stale);
        }
        let job = load_job(&transaction, command)?;
        let retry = outcome == "failed" && command.authored_attempt < job.retry.max_attempts.get();
        finish_attempt(&transaction, command, outcome, completion.digest(), retry)?;
        if retry {
            transitions::schedule_retry(&transaction, command, &job, "attempt failed; retrying")?;
        } else {
            transitions::finish_job(&transaction, command, &job, outcome == "succeeded")?;
        }
        transaction.commit()?;
        Ok(ResultAcceptance::Applied)
    }

    pub fn resolve_unknown_attempt(
        &self,
        command: &DispatchCommand,
    ) -> Result<UnknownOutcomeResolution> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let resolution = unknown::resolve_unknown_in_transaction(
            &transaction,
            command,
            "outcome unknown; retrying",
        )?;
        transaction.commit()?;
        Ok(resolution)
    }
}

struct StoredAttempt {
    token: String,
    node_id: String,
    state: String,
    outcome: Option<String>,
    digest: Option<String>,
}

impl StoredAttempt {
    fn matches(&self, command: &DispatchCommand) -> bool {
        self.token == command.fencing_token && self.node_id == command.node_id
    }
}

fn load_attempt(
    transaction: &Transaction<'_>,
    command: &DispatchCommand,
) -> Result<Option<StoredAttempt>> {
    transaction.query_row(
        "SELECT fencing_token, node_id, state, outcome, terminal_digest FROM run_attempts WHERE run_id = ?1 AND job_id = ?2 AND authored_attempt = ?3 AND dispatch_generation = ?4",
        params![command.run_id, command.job_id, command.authored_attempt, command.dispatch_generation],
        |row| Ok(StoredAttempt { token: row.get(0)?, node_id: row.get(1)?, state: row.get(2)?, outcome: row.get(3)?, digest: row.get(4)? }),
    ).optional().map_err(Into::into)
}

fn load_job(transaction: &Transaction<'_>, command: &DispatchCommand) -> Result<ResolvedJob> {
    let definition = transaction.query_row(
        "SELECT definition_json FROM run_jobs WHERE run_id = ?1 AND job_id = ?2 AND current_fencing_token = ?3",
        params![command.run_id, command.job_id, command.fencing_token],
        |row| row.get::<_, String>(0),
    ).optional()?.ok_or_else(|| anyhow::anyhow!("attempt fence is no longer current"))?;
    Ok(serde_json::from_str(&definition)?)
}

fn finish_attempt(
    transaction: &Transaction<'_>,
    command: &DispatchCommand,
    outcome: &str,
    digest: &str,
    retry: bool,
) -> Result<()> {
    let now = sqlite_i64(now_ms()?, "timestamp")?;
    let state = if retry { "retrying" } else { outcome };
    transaction.execute(
        "UPDATE run_attempts SET state = ?6, outcome = ?7, terminal_digest = ?8, finished_at_ms = ?9, released_at_ms = ?9 WHERE run_id = ?1 AND job_id = ?2 AND authored_attempt = ?3 AND dispatch_generation = ?4 AND fencing_token = ?5",
        params![command.run_id, command.job_id, command.authored_attempt,
            command.dispatch_generation, command.fencing_token, state, outcome, digest, now],
    )?;
    let settled = transaction.execute(
        "UPDATE run_dispatch_outbox SET delivered_at_ms = COALESCE(delivered_at_ms, ?6) WHERE run_id = ?1 AND job_id = ?2 AND authored_attempt = ?3 AND dispatch_generation = ?4 AND fencing_token = ?5",
        params![command.run_id, command.job_id, command.authored_attempt,
            command.dispatch_generation, command.fencing_token, now],
    )?;
    if settled != 1 {
        bail!("attempt dispatch outbox is missing");
    }
    Ok(())
}

fn release_unknown(transaction: &Transaction<'_>, command: &DispatchCommand) -> Result<()> {
    let now = sqlite_i64(now_ms()?, "timestamp")?;
    let released = transaction.execute(
        "UPDATE run_attempts SET state = 'unknown', outcome = 'unknown', finished_at_ms = ?6, released_at_ms = ?6 WHERE run_id = ?1 AND job_id = ?2 AND authored_attempt = ?3 AND dispatch_generation = ?4 AND fencing_token = ?5",
        params![command.run_id, command.job_id, command.authored_attempt,
            command.dispatch_generation, command.fencing_token, now],
    )?;
    let settled = transaction.execute(
        "UPDATE run_dispatch_outbox SET delivered_at_ms = COALESCE(delivered_at_ms, ?6) WHERE run_id = ?1 AND job_id = ?2 AND authored_attempt = ?3 AND dispatch_generation = ?4 AND fencing_token = ?5",
        params![command.run_id, command.job_id, command.authored_attempt,
            command.dispatch_generation, command.fencing_token, now],
    )?;
    if released != 1 || settled != 1 {
        bail!("unknown attempt is no longer current");
    }
    Ok(())
}
