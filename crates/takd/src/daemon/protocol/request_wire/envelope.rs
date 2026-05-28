use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(in crate::daemon::protocol) struct RequestEnvelope {
    #[serde(rename = "type")]
    pub(super) request_type: RequestType,
    pub(super) request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) client: Option<ClientInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) task: Option<TaskInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) needs: Option<Vec<NeedRequest>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) ttl_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) lease_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) requirements: Option<PeerEligibility>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) task_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) submit_body: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) headers: Option<Vec<RemoteResponseHeader>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) body: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) task_handle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) after_seq: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) attempt: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) range: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(in crate::daemon::protocol) enum RequestType {
    AcquireLease,
    RenewLease,
    ReleaseLease,
    Status,
    PeersList,
    PeersEligible,
    PlaceRemote,
    ForwardRemoteHttp,
    StreamTaskEvents,
    CancelTask,
    GetTaskResult,
    GetOutputRange,
}
