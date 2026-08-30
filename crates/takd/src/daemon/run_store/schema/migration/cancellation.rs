use anyhow::Result;
use rusqlite::Transaction;

pub(super) fn backfill(transaction: &Transaction<'_>) -> Result<()> {
    transaction.execute(
        "INSERT OR IGNORE INTO run_cancel_outbox (run_id, job_id, authored_attempt, \
         dispatch_generation, fencing_token, node_id) SELECT run_id, job_id, authored_attempt, \
         dispatch_generation, fencing_token, node_id FROM run_attempts \
         WHERE state = 'cancelling' AND released_at_ms IS NULL",
        [],
    )?;
    transaction.execute("DELETE FROM run_outbox WHERE kind = 'cancel_run'", [])?;
    Ok(())
}
