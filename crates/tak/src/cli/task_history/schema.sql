CREATE TABLE IF NOT EXISTS task_runs (
    task_run_id TEXT PRIMARY KEY,
    task_label TEXT NOT NULL,
    attempts INTEGER NOT NULL,
    state TEXT NOT NULL,
    placement TEXT NOT NULL DEFAULT 'unknown',
    remote_node_id TEXT NOT NULL DEFAULT '',
    started_at_ms INTEGER NOT NULL,
    finished_at_ms INTEGER
);

CREATE TABLE IF NOT EXISTS task_outputs (
    task_run_id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    attempt INTEGER NOT NULL,
    stream TEXT NOT NULL,
    bytes BLOB NOT NULL,
    PRIMARY KEY (task_run_id, seq)
);
