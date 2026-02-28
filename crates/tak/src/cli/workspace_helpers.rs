use super::*;

/// Loads a workspace from the current working directory using default loader options.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(super) fn load_workspace_from_cwd() -> Result<WorkspaceSpec> {
    let cwd = std::env::current_dir().context("failed to resolve current directory")?;
    load_workspace(&cwd, &LoadOptions::default())
}

/// Parses a user-provided CLI label into a fully validated `TaskLabel`.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(super) fn parse_input_label(value: &str) -> Result<TaskLabel> {
    parse_label(value, "//").map_err(|e| anyhow!("invalid label {value}: {e}"))
}

/// Resolves daemon socket path from environment override or default path logic.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(super) fn resolve_daemon_socket_path() -> PathBuf {
    std::env::var("TAKD_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|_| takd::default_socket_path())
}

/// Reads a `u64` value from an environment variable with a fallback default.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(super) fn env_u64(var_name: &str, default: u64) -> u64 {
    std::env::var(var_name)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(default)
}
