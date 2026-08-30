use anyhow::Result;
use rusqlite::Transaction;

use crate::daemon::scheduler::{DispatchCommand, UnknownOutcomeResolution};

use super::{load_attempt, load_job, release_unknown, transitions};

pub(super) fn resolve_unknown_in_transaction(
    transaction: &Transaction<'_>,
    command: &DispatchCommand,
    retry_message: &str,
) -> Result<UnknownOutcomeResolution> {
    let Some(attempt) = load_attempt(transaction, command)? else {
        return Ok(UnknownOutcomeResolution::Stale);
    };
    if !attempt.matches(command) || !matches!(attempt.state.as_str(), "transferring" | "running") {
        return Ok(UnknownOutcomeResolution::Stale);
    }
    let job = load_job(transaction, command)?;
    let dispatch_stopped = transaction.query_row(
        "SELECT dispatch_stopped FROM runs WHERE run_id = ?1",
        [&command.run_id],
        |row| row.get::<_, bool>(0),
    )?;
    let retry = !dispatch_stopped
        && job.idempotent
        && command.authored_attempt < job.retry.max_attempts.get();
    release_unknown(transaction, command)?;
    if retry {
        transitions::schedule_retry(transaction, command, &job, retry_message)?;
        Ok(UnknownOutcomeResolution::Retrying)
    } else {
        transitions::finish_job(transaction, command, &job, false)?;
        Ok(UnknownOutcomeResolution::Failed)
    }
}
