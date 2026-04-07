use super::*;
use tak_proto::NodeInfo;

#[derive(Debug, Clone)]
pub(super) struct RemoteWorkerSubmitPayload {
    pub(super) workspace_zip: Vec<u8>,
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
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct RemoteNodeContext {
    pub node: NodeInfo,
    pub bearer_token: String,
}

impl RemoteNodeContext {
    pub fn new(node: NodeInfo, bearer_token: String) -> Self {
        Self { node, bearer_token }
    }
}
