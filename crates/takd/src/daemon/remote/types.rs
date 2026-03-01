use super::*;

#[derive(Debug, Clone)]
pub(super) struct RemoteWorkerSubmitPayload {
    pub(super) workspace_zip_base64: String,
    pub(super) steps: Vec<StepDef>,
    pub(super) timeout_s: Option<u64>,
    pub(super) runtime: Option<RemoteRuntimeSpec>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct RemoteWorkerOutputRecord {
    pub(super) path: String,
    pub(super) digest: String,
    pub(super) size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WorkspaceFileFingerprint {
    pub(super) digest: String,
    pub(super) size: u64,
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
