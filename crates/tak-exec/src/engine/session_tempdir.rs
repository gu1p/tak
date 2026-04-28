use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub(super) fn session_tempdir(workspace_root: &Path, purpose: &str) -> Result<tempfile::TempDir> {
    if let Some(base) = explicit_session_tmpdir() {
        return tempdir_in(&base, purpose);
    }
    let workspace_tmp = workspace_root.join(".tmp");
    if workspace_tmp.is_dir() {
        return tempdir_in(&workspace_tmp.join("tak-sessions"), purpose);
    }
    tempfile::Builder::new()
        .prefix(&format!("tak-session-{purpose}-"))
        .tempdir()
        .context("failed to allocate session temp directory")
}

fn explicit_session_tmpdir() -> Option<PathBuf> {
    let value = std::env::var_os("TAK_SESSION_TMPDIR")?;
    if value.is_empty() {
        return None;
    }
    Some(PathBuf::from(value))
}

fn tempdir_in(base: &Path, purpose: &str) -> Result<tempfile::TempDir> {
    fs::create_dir_all(base)
        .with_context(|| format!("failed to create session temp base {}", base.display()))?;
    tempfile::Builder::new()
        .prefix(&format!("tak-session-{purpose}-"))
        .tempdir_in(base)
        .with_context(|| {
            format!(
                "failed to allocate session temp directory in {}",
                base.display()
            )
        })
}
