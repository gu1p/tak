pub enum SubmitRegistration {
    Created { idempotency_key: String },
    Attached { idempotency_key: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitEventRecord {
    pub seq: u64,
    pub payload_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteV1Response {
    pub status_code: u16,
    pub content_type: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct SubmitAttemptStore {
    db_path: PathBuf,
}

