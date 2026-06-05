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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        code: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        retryable: Option<bool>,
    },
}

impl Response {
    pub fn error(request_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error {
            request_id: request_id.into(),
            message: message.into(),
            code: None,
            retryable: Some(false),
        }
    }

    pub fn classified_error(
        request_id: impl Into<String>,
        message: impl Into<String>,
        code: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self::Error {
            request_id: request_id.into(),
            message: message.into(),
            code: Some(code.into()),
            retryable: Some(retryable),
        }
    }
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
