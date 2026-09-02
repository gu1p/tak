use super::RunStore;
use anyhow::Result;
use rusqlite::TransactionBehavior;
mod migration;
mod output;
mod remote;
mod upload;

impl RunStore {
    pub(super) fn ensure_schema(&self) -> Result<()> {
        let mut connection = self.open_connection()?;
        migration::reject_newer_schema(&connection)?;
        connection.execute_batch(SCHEMA)?;
        connection.execute_batch(output::SCHEMA)?;
        connection.execute_batch(remote::SCHEMA)?;
        connection.execute_batch(upload::SCHEMA)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        migration::apply(&transaction)?;
        transaction.execute(
            "INSERT INTO run_schema_version (singleton, version) VALUES (1, 14) \
             ON CONFLICT(singleton) DO UPDATE SET version = excluded.version",
            [],
        )?;
        transaction.commit()?;
        Ok(())
    }
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS run_schema_version (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    version INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS runs (
    run_id TEXT PRIMARY KEY,
    submitter_id TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    request_digest TEXT NOT NULL,
    state TEXT NOT NULL,
    project_id TEXT NOT NULL,
    targets_json TEXT NOT NULL,
    resolved_json TEXT NOT NULL,
    max_parallel_jobs INTEGER NOT NULL,
    keep_going INTEGER NOT NULL,
    dispatch_stopped INTEGER NOT NULL DEFAULT 0,
    exit_code INTEGER,
    logs_expired INTEGER NOT NULL DEFAULT 0,
    outputs_expired INTEGER NOT NULL DEFAULT 0,
    output_error TEXT,
    last_scheduled_turn INTEGER NOT NULL DEFAULT 0,
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
    cache TEXT,
    dispatch_generation INTEGER NOT NULL DEFAULT 0,
    current_fencing_token TEXT,
    next_eligible_at_ms INTEGER NOT NULL DEFAULT 0,
    ready_order INTEGER NOT NULL DEFAULT 0,
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
CREATE TABLE IF NOT EXISTS run_policy_cursors (
    run_id TEXT NOT NULL,
    policy_id TEXT NOT NULL,
    next_assignment INTEGER NOT NULL,
    PRIMARY KEY (run_id, policy_id),
    FOREIGN KEY (run_id) REFERENCES runs(run_id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS run_affinity_bindings (
    run_id TEXT NOT NULL,
    affinity_group TEXT NOT NULL,
    node_id TEXT NOT NULL,
    bound_at_ms INTEGER NOT NULL,
    PRIMARY KEY (run_id, affinity_group),
    FOREIGN KEY (run_id) REFERENCES runs(run_id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS scheduler_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    next_turn INTEGER NOT NULL
);
INSERT OR IGNORE INTO scheduler_state (singleton, next_turn) VALUES (1, 1);
CREATE TABLE IF NOT EXISTS scheduler_submitters (
    submitter_id TEXT PRIMARY KEY,
    last_scheduled_turn INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS scheduler_node_losses (
    node_id TEXT PRIMARY KEY,
    declared_at_ms INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS scheduler_rate_buckets (
    limiter_name TEXT NOT NULL,
    scope TEXT NOT NULL CHECK(scope IN ('run','submitter','project','worktree','node')),
    owner_identity TEXT NOT NULL,
    scope_key_present INTEGER NOT NULL CHECK(scope_key_present IN (0,1)),
    scope_key TEXT NOT NULL,
    burst INTEGER NOT NULL CHECK(burst > 0),
    refill_millis_per_second INTEGER NOT NULL CHECK(refill_millis_per_second > 0),
    available_micros INTEGER NOT NULL CHECK(available_micros >= 0),
    refilled_at_ms INTEGER NOT NULL CHECK(refilled_at_ms >= 0),
    PRIMARY KEY (limiter_name, scope, owner_identity, scope_key_present, scope_key)
);
CREATE TABLE IF NOT EXISTS run_attempts (
    run_id TEXT NOT NULL,
    job_id TEXT NOT NULL,
    authored_attempt INTEGER NOT NULL,
    dispatch_generation INTEGER NOT NULL,
    fencing_token TEXT NOT NULL UNIQUE,
    node_id TEXT NOT NULL,
    transport TEXT,
    worker_event_cursor INTEGER NOT NULL DEFAULT 0,
    state TEXT NOT NULL,
    cpu_millis INTEGER NOT NULL,
    memory_bytes INTEGER NOT NULL,
    execution_slots INTEGER NOT NULL,
    reserved_at_ms INTEGER NOT NULL,
    dispatch_started_at_ms INTEGER,
    accepted_at_ms INTEGER,
    finished_at_ms INTEGER,
    outcome TEXT,
    terminal_digest TEXT,
    exit_code INTEGER,
    released_at_ms INTEGER,
    PRIMARY KEY (run_id, job_id, authored_attempt, dispatch_generation),
    FOREIGN KEY (run_id, job_id) REFERENCES run_jobs(run_id, job_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS run_attempts_active_node
    ON run_attempts(node_id) WHERE released_at_ms IS NULL;
CREATE TABLE IF NOT EXISTS run_dispatch_outbox (
    run_id TEXT NOT NULL,
    job_id TEXT NOT NULL,
    authored_attempt INTEGER NOT NULL,
    dispatch_generation INTEGER NOT NULL,
    fencing_token TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    delivered_at_ms INTEGER,
    PRIMARY KEY (run_id, job_id, authored_attempt, dispatch_generation),
    FOREIGN KEY (run_id, job_id, authored_attempt, dispatch_generation)
        REFERENCES run_attempts(run_id, job_id, authored_attempt, dispatch_generation)
        ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS run_cancel_outbox (
    run_id TEXT NOT NULL,
    job_id TEXT NOT NULL,
    authored_attempt INTEGER NOT NULL,
    dispatch_generation INTEGER NOT NULL,
    fencing_token TEXT NOT NULL,
    node_id TEXT NOT NULL,
    delivered_at_ms INTEGER,
    PRIMARY KEY (run_id, job_id, authored_attempt, dispatch_generation),
    FOREIGN KEY (run_id, job_id, authored_attempt, dispatch_generation)
        REFERENCES run_attempts(run_id, job_id, authored_attempt, dispatch_generation)
        ON DELETE CASCADE
);
"#;
