use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseInfo {
    pub lease_id: String,
    pub ttl_ms: u64,
    pub renew_after_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingInfo {
    pub queue_position: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Response {
    LeaseGranted {
        request_id: String,
        lease: LeaseInfo,
    },
    LeasePending {
        request_id: String,
        pending: PendingInfo,
    },
    LeaseRenewed {
        request_id: String,
        ttl_ms: u64,
    },
    LeaseReleased {
        request_id: String,
    },
    StatusSnapshot {
        request_id: String,
        status: StatusSnapshot,
    },
    PeersSnapshot {
        request_id: String,
        peers: Vec<PeerSnapshot>,
    },
    RemotePlaced {
        request_id: String,
        task_handle: String,
        peer: Box<PeerSnapshot>,
        status: u16,
        headers: Vec<RemoteResponseHeader>,
        body: Vec<u8>,
    },
    RemoteHttpResponse {
        request_id: String,
        status: u16,
        headers: Vec<RemoteResponseHeader>,
        body: Vec<u8>,
    },
    Error {
        request_id: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusSnapshot {
    pub active_leases: usize,
    pub pending_requests: usize,
    pub usage: Vec<LimiterUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimiterUsage {
    pub name: String,
    pub scope: Scope,
    pub scope_key: Option<String>,
    pub used: f64,
    pub capacity: f64,
}
