pub enum AcquireLeaseResponse {
    LeaseGranted { lease: LeaseInfo },
    LeasePending { pending: PendingInfo },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct LimiterKey {
    name: String,
    scope: Scope,
    scope_key: Option<String>,
}

#[derive(Debug, Clone)]
struct LeaseRecord {
    needs: Vec<NeedRequest>,
    expires_at: Instant,
    ttl_ms: u64,
    request_id: String,
    task_label: String,
    user_name: String,
    pid: u32,
}

#[derive(Debug, Default)]
pub struct LeaseManager {
    capacities: HashMap<LimiterKey, f64>,
    usage: HashMap<LimiterKey, f64>,
    leases: HashMap<String, LeaseRecord>,
    pending: VecDeque<AcquireLeaseRequest>,
    db_path: Option<PathBuf>,
}

