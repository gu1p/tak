use anyhow::Result;
use rusqlite::Connection;

pub(super) fn ensure_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS worker_v2_attempts (
            fencing_token TEXT PRIMARY KEY,
            run_id TEXT NOT NULL,
            job_id TEXT NOT NULL,
            node_id TEXT NOT NULL,
            authored_attempt INTEGER NOT NULL,
            dispatch_generation INTEGER NOT NULL,
            payload_digest TEXT NOT NULL,
            request_json TEXT NOT NULL,
            state TEXT NOT NULL,
            cancellation_requested INTEGER NOT NULL DEFAULT 0,
            terminal_json TEXT,
            acknowledged INTEGER NOT NULL DEFAULT 0,
            created_at_ms INTEGER NOT NULL,
            updated_at_ms INTEGER NOT NULL,
            UNIQUE(run_id, job_id, authored_attempt, dispatch_generation)
        );
        CREATE TABLE IF NOT EXISTS worker_v2_heads (
            run_id TEXT NOT NULL,
            job_id TEXT NOT NULL,
            authored_attempt INTEGER NOT NULL,
            dispatch_generation INTEGER NOT NULL,
            fencing_token TEXT NOT NULL,
            PRIMARY KEY(run_id, job_id),
            FOREIGN KEY(fencing_token) REFERENCES worker_v2_attempts(fencing_token)
        );
        CREATE TABLE IF NOT EXISTS worker_v2_events (
            fencing_token TEXT NOT NULL,
            seq INTEGER NOT NULL,
            event_json TEXT NOT NULL,
            PRIMARY KEY(fencing_token, seq),
            FOREIGN KEY(fencing_token) REFERENCES worker_v2_attempts(fencing_token)
        );
        CREATE TABLE IF NOT EXISTS worker_v2_outputs (
            fencing_token TEXT NOT NULL,
            artifact_id TEXT NOT NULL UNIQUE,
            producer_task_id TEXT NOT NULL,
            path TEXT NOT NULL,
            entry_json TEXT NOT NULL,
            content BLOB NOT NULL,
            PRIMARY KEY(fencing_token, producer_task_id, path),
            FOREIGN KEY(fencing_token) REFERENCES worker_v2_attempts(fencing_token)
        );
        CREATE TABLE IF NOT EXISTS worker_v2_terminal_runs (
            run_id TEXT PRIMARY KEY,
            released_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_worker_v2_attempt_identity
        ON worker_v2_attempts(run_id, job_id, authored_attempt, dispatch_generation);
        ",
    )?;
    Ok(())
}
