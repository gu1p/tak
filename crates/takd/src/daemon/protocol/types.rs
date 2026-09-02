use super::*;

#[path = "types/response.rs"]
mod response;
pub use response::{LeaseInfo, LimiterUsage, PendingInfo, StatusSnapshot};

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
