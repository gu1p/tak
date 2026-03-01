use super::*;

pub enum AcquireLeaseResponse {
    LeaseGranted { lease: LeaseInfo },
    LeasePending { pending: PendingInfo },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(super) struct LimiterKey {
    pub(super) name: String,
    pub(super) scope: Scope,
    pub(super) scope_key: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct LeaseRecord {
    pub(super) needs: Vec<NeedRequest>,
    pub(super) expires_at: Instant,
    pub(super) ttl_ms: u64,
    pub(super) request_id: String,
    pub(super) task_label: String,
    pub(super) user_name: String,
    pub(super) pid: u32,
}

#[derive(Debug, Default)]
pub struct LeaseManager {
    pub(super) capacities: HashMap<LimiterKey, f64>,
    pub(super) usage: HashMap<LimiterKey, f64>,
    pub(super) leases: HashMap<String, LeaseRecord>,
    pub(super) pending: VecDeque<AcquireLeaseRequest>,
    pub(super) db_path: Option<PathBuf>,
}
