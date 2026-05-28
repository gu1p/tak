use super::*;

pub fn default_socket_path() -> PathBuf {
    tak_core::runtime_paths::default_daemon_socket_path()
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
