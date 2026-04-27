use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tak_core::model::{
    CurrentStateSpec, ExecutionPlacementSpec, OutputSelectorSpec, ResolvedTask, SessionReuseSpec,
    TaskExecutionSpec,
};

use super::session_workspace_files::{
    copy_directory_contents, extract_share_paths, materialize_seed_workspace,
};

#[derive(Debug)]
pub(crate) struct PreparedTaskSession {
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) reuse: SessionReuseSpec,
    pub(crate) root: Option<PathBuf>,
    _task_workspace: Option<tempfile::TempDir>,
}

pub(crate) struct ExecutionSessionManager {
    run_id: String,
    workspaces: BTreeMap<String, SessionWorkspace>,
}

enum SessionWorkspace {
    ShareWorkspace { workspace: tempfile::TempDir },
    SharePaths { store: tempfile::TempDir },
}

impl ExecutionSessionManager {
    pub(crate) fn new(run_id: String) -> Self {
        Self {
            run_id,
            workspaces: BTreeMap::new(),
        }
    }

    pub(crate) fn prepare_task(
        &mut self,
        task: &ResolvedTask,
        workspace_root: &Path,
    ) -> Result<Option<PreparedTaskSession>> {
        let Some(session) = task.session.as_ref() else {
            return Ok(None);
        };
        if !session_uses_local_workspace(&session.execution) {
            return Ok(Some(PreparedTaskSession {
                key: self.session_key(&session.name),
                name: session.name.clone(),
                reuse: session.reuse.clone(),
                root: None,
                _task_workspace: None,
            }));
        }
        let seed = session.context.as_ref().unwrap_or(&task.context);
        match &session.reuse {
            SessionReuseSpec::ShareWorkspace => {
                let root = self.share_workspace_root(&session.name, seed, workspace_root)?;
                Ok(Some(PreparedTaskSession {
                    key: self.session_key(&session.name),
                    name: session.name.clone(),
                    reuse: session.reuse.clone(),
                    root: Some(root),
                    _task_workspace: None,
                }))
            }
            SessionReuseSpec::SharePaths { paths } => {
                let prepared = self.share_paths_task(&session.name, paths, seed, workspace_root)?;
                Ok(Some(PreparedTaskSession {
                    key: self.session_key(&session.name),
                    name: session.name.clone(),
                    reuse: session.reuse.clone(),
                    root: Some(prepared.path().to_path_buf()),
                    _task_workspace: Some(prepared),
                }))
            }
        }
    }

    fn session_key(&self, name: &str) -> String {
        format!("{}-{name}", self.run_id)
    }

    pub(crate) fn finish_task(
        &self,
        prepared: Option<&PreparedTaskSession>,
        success: bool,
    ) -> Result<()> {
        let Some(prepared) = prepared else {
            return Ok(());
        };
        if !success {
            return Ok(());
        }
        let (SessionReuseSpec::SharePaths { paths }, Some(root)) =
            (&prepared.reuse, &prepared.root)
        else {
            return Ok(());
        };
        let Some(SessionWorkspace::SharePaths { store }) = self.workspaces.get(&prepared.name)
        else {
            return Ok(());
        };
        extract_share_paths(root, store.path(), paths)
    }

    fn share_workspace_root(
        &mut self,
        name: &str,
        seed: &CurrentStateSpec,
        workspace_root: &Path,
    ) -> Result<PathBuf> {
        if !self.workspaces.contains_key(name) {
            let workspace = session_tempdir(workspace_root, "workspace")
                .context("failed to create session workspace")?;
            materialize_seed_workspace(workspace_root, workspace.path(), seed)?;
            self.workspaces.insert(
                name.to_string(),
                SessionWorkspace::ShareWorkspace { workspace },
            );
        }
        match self.workspaces.get(name).expect("session workspace") {
            SessionWorkspace::ShareWorkspace { workspace } => Ok(workspace.path().to_path_buf()),
            SessionWorkspace::SharePaths { .. } => unreachable!("session reuse is stable"),
        }
    }

    fn share_paths_task(
        &mut self,
        name: &str,
        _paths: &[OutputSelectorSpec],
        seed: &CurrentStateSpec,
        workspace_root: &Path,
    ) -> Result<tempfile::TempDir> {
        if !self.workspaces.contains_key(name) {
            let store = session_tempdir(workspace_root, "paths-store")
                .context("failed to create session path store")?;
            self.workspaces
                .insert(name.to_string(), SessionWorkspace::SharePaths { store });
        }
        let workspace = session_tempdir(workspace_root, "paths-task")
            .context("failed to create session task workspace")?;
        materialize_seed_workspace(workspace_root, workspace.path(), seed)?;
        if let Some(SessionWorkspace::SharePaths { store }) = self.workspaces.get(name) {
            copy_directory_contents(store.path(), workspace.path())?;
        }
        Ok(workspace)
    }
}

fn session_uses_local_workspace(execution: &TaskExecutionSpec) -> bool {
    match execution {
        TaskExecutionSpec::LocalOnly(_) => true,
        TaskExecutionSpec::ByExecutionPolicy { placements, .. } => placements
            .iter()
            .all(|placement| matches!(placement, ExecutionPlacementSpec::Local(_))),
        _ => false,
    }
}

fn session_tempdir(workspace_root: &Path, purpose: &str) -> Result<tempfile::TempDir> {
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
