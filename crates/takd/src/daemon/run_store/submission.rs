use anyhow::{Result, bail};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use tak_core::v2::RunSubmission;
use tak_proto::local_daemon::v2::{RunEventKind, WorkspaceDisposition};

use super::events::{append_event, now_ms, sqlite_i64};
use super::workspace_uploads;
use super::{RunStore, SubmitRunResult};

mod definition_conflicts;
mod persistence;

use definition_conflicts::reject_active_conflicts;
use persistence::{insert_edges, insert_environment, insert_jobs, insert_run};

impl RunStore {
    pub fn submit(
        &self,
        submission: &RunSubmission,
        submitter_id: &str,
    ) -> Result<SubmitRunResult> {
        submission.validate()?;
        if submitter_id.trim().is_empty() {
            bail!("submitter identity is required");
        }
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(result) = existing_submission(self, &transaction, submission, submitter_id)? {
            transaction.commit()?;
            return Ok(result);
        }
        reject_active_conflicts(&transaction, submission, submitter_id)?;
        let run_id = format!("run-{}", uuid::Uuid::new_v4());
        let workspace = workspace_uploads::disposition(
            self,
            &transaction,
            &run_id,
            &submission.run.workspace,
            0,
        )?;
        insert_run(&transaction, &run_id, submission, submitter_id, &workspace)?;
        insert_environment(&transaction, &run_id, submission)?;
        insert_jobs(&transaction, &run_id, submission)?;
        insert_edges(&transaction, &run_id, submission)?;
        append_event(
            &transaction,
            &run_id,
            RunEventKind::Submitted,
            "run submitted",
        )?;
        transaction.commit()?;
        Ok(SubmitRunResult { run_id, workspace })
    }
}

fn existing_submission(
    store: &RunStore,
    transaction: &Transaction<'_>,
    submission: &RunSubmission,
    submitter_id: &str,
) -> Result<Option<SubmitRunResult>> {
    let existing = transaction
        .query_row(
            "SELECT run_id, request_digest, submitter_id, upload_offset, state FROM runs WHERE submitter_id = ?1 AND idempotency_key = ?2",
            params![submitter_id, submission.idempotency_key],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, i64>(3)?, row.get::<_, String>(4)?)),
        )
        .optional()?;
    let Some((run_id, digest, owner, offset, state)) = existing else {
        return Ok(None);
    };
    if digest != submission.request_digest() || owner != submitter_id {
        bail!("run submission idempotency conflict");
    }
    let offset =
        u64::try_from(offset).map_err(|_| anyhow::anyhow!("stored upload offset is invalid"))?;
    let workspace = if is_post_commit(&state) {
        WorkspaceDisposition::Present
    } else {
        workspace_uploads::disposition(
            store,
            transaction,
            &run_id,
            &submission.run.workspace,
            offset,
        )?
    };
    if matches!(workspace, WorkspaceDisposition::Present) && state == "awaiting_workspace" {
        transaction.execute(
            "UPDATE runs SET state = 'awaiting_commit', upload_offset = archive_size, updated_at_ms = ?2 WHERE run_id = ?1",
            params![run_id, sqlite_i64(now_ms()?, "timestamp")?],
        )?;
        append_event(
            transaction,
            &run_id,
            RunEventKind::WorkspaceUploading,
            "workspace cache hit",
        )?;
    } else if let WorkspaceDisposition::UploadRequired { next_offset } = &workspace
        && matches!(state.as_str(), "awaiting_workspace" | "awaiting_commit")
    {
        transaction.execute(
            "UPDATE runs SET state = 'awaiting_workspace', upload_offset = ?2, updated_at_ms = ?3 WHERE run_id = ?1",
            params![
                run_id,
                sqlite_i64(*next_offset, "upload offset")?,
                sqlite_i64(now_ms()?, "timestamp")?
            ],
        )?;
    }
    Ok(Some(SubmitRunResult { run_id, workspace }))
}

fn is_post_commit(state: &str) -> bool {
    matches!(
        state,
        "queued" | "running" | "cancelling" | "succeeded" | "failed" | "cancelled"
    )
}
