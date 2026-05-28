use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize)]
pub(super) struct PeerEligibility {
    pub(super) pool: Option<String>,
    pub(super) tags: Vec<String>,
    pub(super) capabilities: Vec<String>,
    pub(super) transport: Option<String>,
    pub(super) cpu_cores: Option<f64>,
    pub(super) memory_mb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RemoteHeader {
    pub(super) name: String,
    pub(super) value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub(super) enum DaemonRequest {
    PlaceRemote {
        request_id: String,
        requirements: PeerEligibility,
        task_run_id: String,
        submit_body: Vec<u8>,
    },
    ForwardRemoteHttp {
        request_id: String,
        node_id: String,
        method: String,
        path: String,
        headers: Vec<RemoteHeader>,
        body: Vec<u8>,
    },
    StreamTaskEvents {
        request_id: String,
        task_handle: String,
        after_seq: u64,
    },
    CancelTask {
        request_id: String,
        task_handle: String,
        attempt: u32,
    },
    GetTaskResult {
        request_id: String,
        task_handle: String,
    },
    GetOutputRange {
        request_id: String,
        task_handle: String,
        attempt: u32,
        path: String,
        range: Option<String>,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub(super) enum DaemonResponse {
    RemotePlaced {
        task_handle: String,
        peer: DaemonPeerSnapshot,
        status: u16,
        headers: Vec<RemoteHeader>,
        body: Vec<u8>,
    },
    RemoteHttpResponse {
        status: u16,
        headers: Vec<RemoteHeader>,
        body: Vec<u8>,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct DaemonPeerSnapshot {
    pub(super) node_id: String,
    pub(super) endpoint: String,
}
