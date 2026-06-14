use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use tak_core::model::{ResolvedTask, SessionReuseSpec, SessionUseSpec};

mod manager;

#[derive(Debug)]
pub(crate) struct PreparedTaskSession {
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) display_name: String,
    pub(crate) reuse: SessionReuseSpec,
    pub(crate) root: Option<PathBuf>,
    _task_workspace: Option<tempfile::TempDir>,
}

pub(crate) struct ExecutionSessionManager {
    run_id: String,
    workspaces: BTreeMap<String, SessionWorkspace>,
}

#[derive(Clone)]
pub(crate) struct SharedExecutionSessionManager {
    inner: Arc<Mutex<ExecutionSessionManager>>,
}

enum SessionWorkspace {
    ShareWorkspace { workspace: tempfile::TempDir },
    SharePaths { store: tempfile::TempDir },
}

impl SharedExecutionSessionManager {
    pub(crate) fn new(run_id: String) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ExecutionSessionManager::new(run_id))),
        }
    }

    pub(crate) fn prepare_task(
        &self,
        task: &ResolvedTask,
        selected_session: Option<&SessionUseSpec>,
        workspace_root: &Path,
        use_local_workspace: bool,
    ) -> Result<Option<PreparedTaskSession>> {
        self.inner
            .lock()
            .expect("execution session manager lock")
            .prepare_task(task, selected_session, workspace_root, use_local_workspace)
    }

    pub(crate) fn finish_task(
        &self,
        prepared: Option<&PreparedTaskSession>,
        success: bool,
    ) -> Result<()> {
        self.inner
            .lock()
            .expect("execution session manager lock")
            .finish_task(prepared, success)
    }
}
