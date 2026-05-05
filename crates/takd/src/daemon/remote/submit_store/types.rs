use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveSubmitAttempt {
    pub idempotency_key: String,
    pub task_run_id: String,
    pub attempt: u32,
    pub task_label: String,
    pub selected_node_id: String,
    pub created_at_ms: i64,
}

pub enum SubmitRegistration {
    Created { idempotency_key: String },
    Attached { idempotency_key: String },
}

#[derive(Debug, Clone)]
pub struct SubmitAttemptStore {
    pub(super) db_path: PathBuf,
}
