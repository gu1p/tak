use super::*;

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
pub enum Request {
    AcquireLease(AcquireLeaseRequest),
    RenewLease(RenewLeaseRequest),
    ReleaseLease(ReleaseLeaseRequest),
    Status(StatusRequest),
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
