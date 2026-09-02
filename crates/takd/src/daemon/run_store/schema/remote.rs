pub(super) const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS run_worker_ack_outbox (
    run_id TEXT NOT NULL,
    job_id TEXT NOT NULL,
    authored_attempt INTEGER NOT NULL,
    dispatch_generation INTEGER NOT NULL,
    fencing_token TEXT NOT NULL,
    terminal_digest TEXT NOT NULL,
    delivered_at_ms INTEGER,
    PRIMARY KEY (run_id, job_id, authored_attempt, dispatch_generation),
    FOREIGN KEY (run_id, job_id, authored_attempt, dispatch_generation)
        REFERENCES run_attempts(run_id, job_id, authored_attempt, dispatch_generation)
        ON DELETE CASCADE
);
"#;
