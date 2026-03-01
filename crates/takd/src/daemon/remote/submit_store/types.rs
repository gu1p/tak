use std::path::PathBuf;

pub enum SubmitRegistration {
    Created { idempotency_key: String },
    Attached { idempotency_key: String },
}

#[derive(Debug, Clone)]
pub struct SubmitAttemptStore {
    pub(super) db_path: PathBuf,
}
