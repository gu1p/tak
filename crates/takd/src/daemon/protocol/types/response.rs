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
