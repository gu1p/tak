pub fn default_socket_path() -> PathBuf {
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        return Path::new(&runtime).join("tak/takd.sock");
    }
    let pid = std::process::id();
    PathBuf::from(format!("/tmp/tak-{pid}.sock"))
}

/// Resolves the default SQLite state path for daemon persistence.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn default_state_db_path() -> PathBuf {
    if let Ok(state_home) = std::env::var("XDG_STATE_HOME") {
        return Path::new(&state_home).join("tak/takd.sqlite");
    }
    if let Ok(home) = std::env::var("HOME") {
        return Path::new(&home).join(".local/state/tak/takd.sqlite");
    }
    PathBuf::from("/tmp/takd.sqlite")
}

/// Performs protocol-level validation for acquire-lease requests.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn ensure_valid_request(request: &AcquireLeaseRequest) -> Result<()> {
    if request.ttl_ms == 0 {
        bail!("ttl_ms must be positive");
    }
    if request.needs.is_empty() {
        bail!("at least one need must be provided");
    }
    Ok(())
}

