#[derive(Debug)]
struct StoredLeaseRow {
    lease_id: String,
    request_id: String,
    task_label: String,
    user_name: String,
    pid: u32,
    needs_json: String,
    ttl_ms: i64,
    expires_at_ms: i64,
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
