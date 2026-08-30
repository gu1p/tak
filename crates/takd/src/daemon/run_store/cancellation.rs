use anyhow::{Result, bail};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use tak_proto::local_daemon::v2::{RunEventKind, RunLifecycleState};

use super::RunStore;
use super::events::{append_event, now_ms, sqlite_i64};

impl RunStore {
    pub fn cancel(&self, run_id: &str) -> Result<RunLifecycleState> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let state = transaction
            .query_row(
                "SELECT state FROM runs WHERE run_id = ?1",
                [run_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| anyhow::anyhow!("run not found"))?;
        let (result, remove_partial) = match state.as_str() {
            "succeeded" => (RunLifecycleState::Succeeded, false),
            "failed" => (RunLifecycleState::Failed, false),
            "cancelled" => (RunLifecycleState::Cancelled, true),
            "awaiting_workspace" | "awaiting_commit" | "queued" => {
                cancel_before_dispatch(&transaction, run_id)?;
                (RunLifecycleState::Cancelled, true)
            }
            "running" => {
                request_active_cancellation(&transaction, run_id)?;
                (RunLifecycleState::Cancelling, false)
            }
            "cancelling" => (RunLifecycleState::Cancelling, false),
            other => bail!("stored run has unknown state `{other}`"),
        };
        transaction.commit()?;
        if remove_partial {
            remove_upload(&self.upload_path(run_id));
        }
        Ok(result)
    }
}

fn cancel_before_dispatch(transaction: &rusqlite::Transaction<'_>, run_id: &str) -> Result<()> {
    let now = sqlite_i64(now_ms()?, "timestamp")?;
    transaction.execute(
        "UPDATE runs SET state = 'cancelled', updated_at_ms = ?2 WHERE run_id = ?1",
        params![run_id, now],
    )?;
    transaction.execute(
        "UPDATE run_jobs SET state = 'cancelled' WHERE run_id = ?1 AND state NOT IN ('succeeded', 'failed', 'cancelled', 'skipped')",
        [run_id],
    )?;
    transaction.execute(
        "DELETE FROM run_outbox WHERE run_id = ?1 AND kind = 'scheduler_wakeup'",
        [run_id],
    )?;
    append_event(
        transaction,
        run_id,
        RunEventKind::Cancelled,
        "run cancelled before dispatch",
    )?;
    Ok(())
}

fn request_active_cancellation(
    transaction: &rusqlite::Transaction<'_>,
    run_id: &str,
) -> Result<()> {
    let now = sqlite_i64(now_ms()?, "timestamp")?;
    transaction.execute(
        "UPDATE runs SET state = 'cancelling', dispatch_stopped = 1, updated_at_ms = ?2 WHERE run_id = ?1",
        params![run_id, now],
    )?;
    transaction.execute(
        "UPDATE run_jobs SET state = 'cancelled' WHERE run_id = ?1 AND state IN ('ready', 'blocked', 'retrying')",
        [run_id],
    )?;
    transaction.execute(
        "UPDATE run_jobs SET state = 'cancelling', current_fencing_token = NULL WHERE run_id = ?1 AND state IN ('transferring', 'running', 'output_committing')",
        [run_id],
    )?;
    transaction.execute(
        "UPDATE run_attempts SET state = 'cancelling' WHERE run_id = ?1 AND released_at_ms IS NULL",
        [run_id],
    )?;
    transaction.execute(
        "UPDATE run_dispatch_outbox SET delivered_at_ms = COALESCE(delivered_at_ms, ?2) WHERE run_id = ?1 AND delivered_at_ms IS NULL",
        params![run_id, now],
    )?;
    append_event(
        transaction,
        run_id,
        RunEventKind::Cancelling,
        "cancellation requested",
    )?;
    transaction.execute(
        "INSERT OR IGNORE INTO run_outbox (run_id, kind, payload_json) VALUES (?1, 'cancel_run', '{}')",
        [run_id],
    )?;
    Ok(())
}

fn remove_upload(path: &std::path::Path) {
    if let Err(error) = std::fs::remove_file(path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!("could not remove cancelled v2 workspace upload: {error}");
    }
}
