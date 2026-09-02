use anyhow::Result;
use rusqlite::Transaction;
use sha2::{Digest, Sha256};
use tak_core::v2::ResolvedJob;

use crate::daemon::scheduler::{AttemptRuntimeMetadata, DispatchCommand};

use super::super::output_artifacts::{self, FinalPublication};
use super::{finish_attempt, transitions};

pub(super) fn finish_successful_attempt(
    transaction: &Transaction<'_>,
    command: &DispatchCommand,
    job: &ResolvedJob,
    terminal_digest: &str,
    runtime: Option<&AttemptRuntimeMetadata>,
) -> Result<()> {
    if !would_finish_successfully(transaction, command)? {
        finish_attempt(
            transaction,
            command,
            "succeeded",
            terminal_digest,
            Some(0),
            false,
        )?;
        return transitions::finish_job(transaction, command, job, true, Some(0), runtime);
    }
    match output_artifacts::publish_final(
        transaction,
        &command.run_id,
        Some(&command.fencing_token),
    )? {
        FinalPublication::Published => {
            finish_attempt(
                transaction,
                command,
                "succeeded",
                terminal_digest,
                Some(0),
                false,
            )?;
            transitions::finish_job(transaction, command, job, true, Some(0), runtime)
        }
        FinalPublication::Conflict(error) => {
            let message = format!("declared output commit failed: {error}");
            finish_attempt(
                transaction,
                command,
                "failed",
                &format!("{:x}", Sha256::digest(message.as_bytes())),
                None,
                false,
            )?;
            transitions::fail_job(transaction, command, job, &message, runtime)
        }
    }
}

fn would_finish_successfully(
    transaction: &Transaction<'_>,
    command: &DispatchCommand,
) -> Result<bool> {
    transaction
        .query_row(
            "SELECT NOT EXISTS(SELECT 1 FROM run_jobs WHERE run_id=?1 AND job_id<>?2 AND state<>'succeeded')",
            [&command.run_id, &command.job_id],
            |row| row.get(0),
        )
        .map_err(Into::into)
}
