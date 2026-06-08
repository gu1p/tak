use serde::{Deserialize, Serialize};
use tak_core::model::Scope;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ClientInfo {
    pub(super) user: String,
    pub(super) pid: u32,
    pub(super) session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct TaskInfo {
    pub(super) label: String,
    pub(super) attempt: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct NeedRequest {
    pub(super) name: String,
    pub(super) scope: Scope,
    pub(super) scope_key: Option<String>,
    pub(super) slots: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct AcquireLeaseRequest {
    pub(super) request_id: String,
    pub(super) client: ClientInfo,
    pub(super) task: TaskInfo,
    pub(super) needs: Vec<NeedRequest>,
    pub(super) ttl_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ReleaseLeaseRequest {
    pub(super) request_id: String,
    pub(super) lease_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RenewLeaseRequest {
    pub(super) request_id: String,
    pub(super) lease_id: String,
    pub(super) ttl_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct LeaseInfo {
    pub(super) lease_id: String,
    pub(super) ttl_ms: u64,
    pub(super) renew_after_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct PendingInfo {
    pub(super) queue_position: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub(super) enum Request {
    #[serde(rename = "AcquireLease")]
    Acquire(AcquireLeaseRequest),
    #[serde(rename = "RenewLease")]
    Renew(RenewLeaseRequest),
    #[serde(rename = "ReleaseLease")]
    Release(ReleaseLeaseRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub(super) enum Response {
    LeaseGranted {
        request_id: String,
        lease: LeaseInfo,
    },
    LeasePending {
        request_id: String,
        pending: PendingInfo,
    },
    LeaseReleased {
        request_id: String,
    },
    LeaseRenewed {
        request_id: String,
        ttl_ms: u64,
    },
    Error {
        request_id: String,
        message: String,
    },
}
