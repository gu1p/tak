pub(super) const SCHEMA: &str = r#"
CREATE UNIQUE INDEX IF NOT EXISTS run_attempts_run_fence
    ON run_attempts(run_id, fencing_token);
CREATE TABLE IF NOT EXISTS run_attempt_outputs (
    run_id TEXT NOT NULL,
    fencing_token TEXT NOT NULL,
    producer_task_id TEXT NOT NULL,
    path TEXT NOT NULL,
    artifact_id TEXT NOT NULL UNIQUE,
    entry_json TEXT NOT NULL,
    PRIMARY KEY (run_id, fencing_token, producer_task_id, path),
    FOREIGN KEY (run_id, fencing_token) REFERENCES run_attempts(run_id, fencing_token)
        ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS run_final_outputs (
    run_id TEXT NOT NULL,
    path TEXT NOT NULL,
    artifact_id TEXT NOT NULL,
    producer_task_id TEXT NOT NULL,
    PRIMARY KEY (run_id, path),
    FOREIGN KEY (run_id) REFERENCES runs(run_id) ON DELETE CASCADE,
    FOREIGN KEY (artifact_id) REFERENCES run_attempt_outputs(artifact_id)
        ON DELETE CASCADE
);
"#;
