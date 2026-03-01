use super::*;

#[derive(Debug)]
pub(super) struct StoredLeaseRow {
    pub(super) lease_id: String,
    pub(super) request_id: String,
    pub(super) task_label: String,
    pub(super) user_name: String,
    pub(super) pid: u32,
    pub(super) needs_json: String,
    pub(super) ttl_ms: i64,
    pub(super) expires_at_ms: i64,
}

pub type SharedLeaseManager = Arc<Mutex<LeaseManager>>;

/// Creates a shared in-memory lease manager.
#[must_use]
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn new_shared_manager() -> SharedLeaseManager {
    Arc::new(Mutex::new(LeaseManager::new()))
}

/// Creates a shared SQLite-backed lease manager.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn new_shared_manager_with_db(db_path: PathBuf) -> Result<SharedLeaseManager> {
    let manager = LeaseManager::with_db_path(db_path)?;
    Ok(Arc::new(Mutex::new(manager)))
}
