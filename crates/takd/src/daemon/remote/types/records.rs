#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitEventRecord {
    pub seq: u64,
    pub payload_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteV1Response {
    pub status_code: u16,
    pub content_type: String,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitAttemptSummaryRecord {
    pub task_run_id: String,
    pub attempt: u32,
    pub task_label: String,
    pub execution_label: Option<String>,
    pub selected_node_id: String,
    pub state: String,
    pub created_at_ms: i64,
    pub finished_at_ms: Option<i64>,
}
