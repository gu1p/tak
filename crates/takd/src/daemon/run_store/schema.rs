use anyhow::Result;

use super::RunStore;

impl RunStore {
    pub(super) fn ensure_schema(&self) -> Result<()> {
        let connection = self.open_connection()?;
        connection.execute_batch(SCHEMA)?;
        Ok(())
    }
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS runs (
    run_id TEXT PRIMARY KEY,
    submitter_id TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    request_digest TEXT NOT NULL,
    state TEXT NOT NULL,
    project_id TEXT NOT NULL,
    targets_json TEXT NOT NULL,
    resolved_json TEXT NOT NULL,
    workspace_fingerprint TEXT NOT NULL,
    archive_sha256 TEXT NOT NULL,
    archive_size INTEGER NOT NULL,
    upload_offset INTEGER NOT NULL DEFAULT 0,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    UNIQUE (submitter_id, idempotency_key)
);
CREATE TABLE IF NOT EXISTS run_environment (
    run_id TEXT NOT NULL,
    name TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (run_id, name),
    FOREIGN KEY (run_id) REFERENCES runs(run_id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS run_jobs (
    run_id TEXT NOT NULL,
    job_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    state TEXT NOT NULL,
    definition_json TEXT NOT NULL,
    node_id TEXT,
    attempt INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (run_id, job_id),
    FOREIGN KEY (run_id) REFERENCES runs(run_id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS run_dependencies (
    run_id TEXT NOT NULL,
    dependency_job_id TEXT NOT NULL,
    dependent_job_id TEXT NOT NULL,
    PRIMARY KEY (run_id, dependency_job_id, dependent_job_id),
    FOREIGN KEY (run_id) REFERENCES runs(run_id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS run_events (
    run_id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    payload_json TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    PRIMARY KEY (run_id, seq),
    FOREIGN KEY (run_id) REFERENCES runs(run_id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS run_outbox (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    delivered_at_ms INTEGER,
    UNIQUE (run_id, kind),
    FOREIGN KEY (run_id) REFERENCES runs(run_id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS workspace_blobs (
    fingerprint TEXT PRIMARY KEY,
    archive_sha256 TEXT NOT NULL,
    archive_size INTEGER NOT NULL,
    path TEXT NOT NULL,
    last_accessed_ms INTEGER NOT NULL
);
"#;
