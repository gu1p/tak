use anyhow::Result;
use rusqlite::TransactionBehavior;

use crate::daemon::scheduler::{DispatchCommand, ResultAcceptance};

use super::super::RunStore;
use super::{finish_attempt, load_attempt, load_job, transitions, validate_digest};

impl RunStore {
    pub(in crate::daemon) fn fail_attempt_permanently(
        &self,
        command: &DispatchCommand,
        terminal_digest: &str,
        message: &str,
    ) -> Result<ResultAcceptance> {
        validate_digest(terminal_digest)?;
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(attempt) = load_attempt(&transaction, command)? else {
            return Ok(ResultAcceptance::Stale);
        };
        if !attempt.matches(command) {
            return Ok(ResultAcceptance::Stale);
        }
        if attempt.digest.as_deref() == Some(terminal_digest)
            && attempt.outcome.as_deref() == Some("failed")
        {
            return Ok(ResultAcceptance::Duplicate);
        }
        if !matches!(
            attempt.state.as_str(),
            "transferring" | "running" | "output_committing"
        ) {
            return Ok(ResultAcceptance::Stale);
        }
        let job = load_job(&transaction, command)?;
        finish_attempt(
            &transaction,
            command,
            "failed",
            terminal_digest,
            None,
            false,
        )?;
        transitions::fail_job(&transaction, command, &job, message, None)?;
        transaction.commit()?;
        Ok(ResultAcceptance::Applied)
    }
}
