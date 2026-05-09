use std::fs;
use std::path::Path;

use rusqlite::{Connection, params};

#[allow(dead_code)]
pub fn write_active_container_run(state_root: &Path) {
    write_active_container_run_for(state_root, "task-run-1", "//apps/web:build", "local", "");
}

#[allow(dead_code)]
pub fn write_active_remote_container_run(state_root: &Path) {
    write_active_container_run_for(
        state_root,
        "remote-task-run-1",
        "//apps/remote:build",
        "remote",
        "builder-a",
    );
}

fn write_active_container_run_for(
    state_root: &Path,
    task_run_id: &str,
    task_label: &str,
    placement: &str,
    remote_node_id: &str,
) {
    let db_path = state_root.join("tak/tasks.sqlite");
    fs::create_dir_all(db_path.parent().expect("task history parent")).expect("create state dir");
    let conn = Connection::open(&db_path).expect("open task history");
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS task_runs (
            task_run_id TEXT PRIMARY KEY,
            task_label TEXT NOT NULL,
            attempts INTEGER NOT NULL,
            state TEXT NOT NULL,
            placement TEXT NOT NULL DEFAULT 'unknown',
            remote_node_id TEXT NOT NULL DEFAULT '',
            origin TEXT NOT NULL DEFAULT 'task',
            runtime TEXT NOT NULL DEFAULT '',
            runtime_source TEXT NOT NULL DEFAULT '',
            command TEXT NOT NULL DEFAULT '',
            started_at_ms INTEGER NOT NULL,
            finished_at_ms INTEGER
        );
        ",
    )
    .expect("create task history schema");
    conn.execute(
        "
        INSERT INTO task_runs (
            task_run_id, task_label, attempts, state, placement, remote_node_id,
            origin, runtime, runtime_source, command, started_at_ms, finished_at_ms
        )
        VALUES (?1, ?2, 1, 'active', ?3, ?4, 'task', 'containerized', ?5, ?6, ?7, NULL)
        ",
        params![
            task_run_id,
            task_label,
            placement,
            remote_node_id,
            "image:alpine:3.20",
            "make build",
            1_734_000_000_000_i64
        ],
    )
    .expect("insert active run");
}
