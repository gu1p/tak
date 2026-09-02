use anyhow::{Context, Result, bail};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use tak_core::v2::ResolvedRun;
use tak_proto::local_daemon::v2::{MAX_WORKSPACE_CHUNK_BYTES, RunEventKind, RunSummary};

use super::blob::{
    ensure_private_parent, publish_blob, verified_blob, verify_archive_manifest, verify_file,
};
use super::events::{append_event, now_ms, sqlite_i64};
use super::workspace_uploads;
use super::{RunStore, UploadProgress};

impl RunStore {
    pub fn upload_workspace(
        &self,
        run_id: &str,
        fingerprint: &str,
        declared_archive_size: u64,
        offset: u64,
        chunk: &[u8],
    ) -> Result<UploadProgress> {
        if chunk.is_empty() || chunk.len() > MAX_WORKSPACE_CHUNK_BYTES {
            bail!("invalid workspace upload chunk");
        }
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (expected_fingerprint, archive_size, current_offset, state, resolved_json) =
            transaction
                .query_row(
                    "SELECT workspace_fingerprint, archive_size, upload_offset, state, resolved_json FROM runs WHERE run_id = ?1",
                    [run_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?)),
                )
                .optional()?
                .ok_or_else(|| anyhow::anyhow!("run not found"))?;
        let archive_size = u64::try_from(archive_size)
            .map_err(|_| anyhow::anyhow!("stored archive size is invalid"))?;
        let fallback_offset = u64::try_from(current_offset)
            .map_err(|_| anyhow::anyhow!("stored upload offset is invalid"))?;
        if fingerprint != expected_fingerprint || declared_archive_size != archive_size {
            bail!("workspace upload does not match the pending run");
        }
        if state != "awaiting_workspace"
            && fallback_offset == archive_size
            && verified_blob(self, &transaction, fingerprint)?.is_some()
        {
            transaction.commit()?;
            return Ok(UploadProgress {
                chunk_accepted: false,
                next_offset: archive_size,
                complete: true,
            });
        }
        if state != "awaiting_workspace" {
            bail!("workspace upload does not match the pending run");
        }
        let resolved: ResolvedRun = serde_json::from_str(&resolved_json)
            .map_err(|error| anyhow::anyhow!("stored resolved run is invalid: {error}"))?;
        let upload = workspace_uploads::resolve(
            self,
            &transaction,
            run_id,
            &resolved.workspace,
            fallback_offset,
        )?;
        let current_offset = upload.next_offset;
        if offset != current_offset {
            transaction.commit()?;
            return Ok(UploadProgress {
                chunk_accepted: false,
                next_offset: current_offset,
                complete: false,
            });
        }
        let next_offset = offset.saturating_add(chunk.len() as u64);
        if next_offset > upload.archive_size {
            bail!("workspace upload exceeds declared archive size");
        }
        let upload_path = self.upload_path(&upload.owner_run_id);
        ensure_private_parent(&upload_path)?;
        workspace_uploads::append(&upload_path, offset, chunk)?;
        let complete = next_offset == upload.archive_size;
        if complete {
            let verification =
                verify_file(&upload_path, &upload.archive_sha256, upload.archive_size).and_then(
                    |()| verify_archive_manifest(&upload_path, &resolved.workspace.manifest),
                );
            if let Err(error) = verification {
                workspace_uploads::reset(&transaction, fingerprint)?;
                transaction.commit()?;
                workspace_uploads::remove(&upload_path);
                return Err(error);
            }
            publish_blob(
                self,
                &transaction,
                &upload_path,
                fingerprint,
                &upload.archive_sha256,
                upload.archive_size,
            )?;
        }
        let completed_runs =
            workspace_uploads::record_progress(&transaction, fingerprint, next_offset, complete)?;
        for completed_run in completed_runs {
            append_event(
                &transaction,
                &completed_run,
                RunEventKind::WorkspaceUploading,
                "workspace upload complete",
            )?;
        }
        transaction.commit()?;
        if complete {
            workspace_uploads::remove(&upload_path);
        }
        Ok(UploadProgress {
            chunk_accepted: true,
            next_offset,
            complete,
        })
    }

    pub fn commit(&self, run_id: &str) -> Result<RunSummary> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (state, fingerprint) = transaction
            .query_row(
                "SELECT state, workspace_fingerprint FROM runs WHERE run_id = ?1",
                [run_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
            .ok_or_else(|| anyhow::anyhow!("run not found"))?;
        if matches!(
            state.as_str(),
            "queued" | "running" | "cancelling" | "succeeded" | "failed" | "cancelled"
        ) {
            transaction.commit()?;
            drop(connection);
            return self.summary(run_id)?.context("run not found");
        }
        if state != "awaiting_commit" || verified_blob(self, &transaction, &fingerprint)?.is_none()
        {
            bail!("run workspace is incomplete");
        }
        let now = sqlite_i64(now_ms()?, "timestamp")?;
        transaction.execute(
            "UPDATE run_jobs SET state = CASE WHEN EXISTS (SELECT 1 FROM run_dependencies d WHERE d.run_id = run_jobs.run_id AND d.dependent_job_id = run_jobs.job_id) THEN 'blocked' ELSE 'ready' END, next_eligible_at_ms = CASE WHEN EXISTS (SELECT 1 FROM run_dependencies d WHERE d.run_id = run_jobs.run_id AND d.dependent_job_id = run_jobs.job_id) THEN 0 ELSE ?2 END, ready_order = ordinal WHERE run_id = ?1",
            params![run_id, now],
        )?;
        transaction.execute(
            "UPDATE runs SET state = 'queued', updated_at_ms = ?2 WHERE run_id = ?1",
            params![run_id, now],
        )?;
        append_event(&transaction, run_id, RunEventKind::Queued, "run committed")?;
        transaction.execute(
            "INSERT OR IGNORE INTO run_outbox (run_id, kind, payload_json) VALUES (?1, 'scheduler_wakeup', '{}')",
            [run_id],
        )?;
        transaction.commit()?;
        drop(connection);
        self.summary(run_id)?.context("run not found")
    }
}
