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
