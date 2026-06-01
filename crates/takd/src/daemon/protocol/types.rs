use super::*;
use crate::daemon::peer_manager::{PeerEligibility, PeerPlacementSelection, PeerSnapshot};

#[path = "types/response.rs"]
mod response;
pub use response::{LeaseInfo, LimiterUsage, PendingInfo, Response, StatusSnapshot};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub user: String,
    pub pid: u32,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub label: String,
    pub attempt: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeedRequest {
    pub name: String,
    pub scope: Scope,
    pub scope_key: Option<String>,
    pub slots: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquireLeaseRequest {
    pub request_id: String,
    pub client: ClientInfo,
    pub task: TaskInfo,
    pub needs: Vec<NeedRequest>,
    pub ttl_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenewLeaseRequest {
    pub request_id: String,
    pub lease_id: String,
    pub ttl_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseLeaseRequest {
    pub request_id: String,
    pub lease_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusRequest {
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeersListRequest {
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeersEligibleRequest {
    pub request_id: String,
    #[serde(default)]
    pub requirements: PeerEligibility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceRemoteRequest {
    pub request_id: String,
    #[serde(default)]
    pub requirements: PeerEligibility,
    #[serde(default)]
    pub selection: PeerPlacementSelection,
    #[serde(default)]
    pub preferred_node_id: Option<String>,
    pub task_run_id: String,
    #[serde(default = "default_place_remote_attempt")]
    pub attempt: u32,
    #[serde(default)]
    pub submit_body: Vec<u8>,
}

fn default_place_remote_attempt() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteResponseHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardRemoteHttpRequest {
    pub request_id: String,
    pub node_id: String,
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub headers: Vec<RemoteResponseHeader>,
    #[serde(default)]
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamTaskEventsRequest {
    pub request_id: String,
    pub task_handle: String,
    pub after_seq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelTaskRequest {
    pub request_id: String,
    pub task_handle: String,
    pub attempt: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTaskResultRequest {
    pub request_id: String,
    pub task_handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOutputRangeRequest {
    pub request_id: String,
    pub task_handle: String,
    pub attempt: u32,
    pub path: String,
    pub range: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    try_from = "super::request_wire::RequestEnvelope",
    into = "super::request_wire::RequestEnvelope"
)]
pub enum Request {
    AcquireLease(AcquireLeaseRequest),
    RenewLease(RenewLeaseRequest),
    ReleaseLease(ReleaseLeaseRequest),
    Status(StatusRequest),
    PeersList(PeersListRequest),
    PeersEligible(PeersEligibleRequest),
    PlaceRemote(PlaceRemoteRequest),
    ForwardRemoteHttp(ForwardRemoteHttpRequest),
    StreamTaskEvents(StreamTaskEventsRequest),
    CancelTask(CancelTaskRequest),
    GetTaskResult(GetTaskResultRequest),
    GetOutputRange(GetOutputRangeRequest),
}
