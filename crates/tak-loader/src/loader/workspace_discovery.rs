use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};

use super::TASKS_FILE;

pub fn detect_workspace_root(start: &Path) -> Result<PathBuf> {
    resolve_tasks_file(start)?
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("TASKS.py has no parent: {}", start.display()))
}

fn resolve_tasks_file(start: &Path) -> Result<PathBuf> {
    let candidate = if start.is_file() {
        start.to_path_buf()
    } else {
        start.join(TASKS_FILE)
    };

    if candidate.file_name().is_none_or(|name| name != TASKS_FILE) {
        bail!(
            "expected `{TASKS_FILE}` in the current directory, got {}",
            candidate.display()
        );
    }
    if !candidate.is_file() {
        let directory = candidate
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| start.to_path_buf());
        bail!(
            "no `{TASKS_FILE}` found in current directory {}\nRun Tak from a directory that contains `{TASKS_FILE}`.",
            directory.display()
        );
    }

    candidate
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", candidate.display()))
}
