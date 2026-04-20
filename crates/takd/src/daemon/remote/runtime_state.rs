use std::path::PathBuf;
use std::sync::Mutex;

use super::runtime::RemoteRuntimeConfig;

#[derive(Debug, Clone)]
pub(crate) struct RemoteExecutionRootSelection {
    pub(crate) selected_root: PathBuf,
    pub(crate) probed: bool,
}

#[derive(Debug)]
pub(crate) struct RemoteRuntimeState {
    pub(crate) config: RemoteRuntimeConfig,
    execution_root: Mutex<RemoteExecutionRootSelection>,
}

impl RemoteRuntimeState {
    pub(crate) fn new(config: RemoteRuntimeConfig) -> Self {
        let selected_root = config.initial_execution_root_base();
        let probed = config.explicit_remote_exec_root().is_some();
        Self {
            config,
            execution_root: Mutex::new(RemoteExecutionRootSelection {
                selected_root,
                probed,
            }),
        }
    }

    pub(crate) fn execution_root_selection(&self) -> RemoteExecutionRootSelection {
        match self.execution_root.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    pub(crate) fn update_execution_root_selection(&self, selected_root: PathBuf) {
        let mut guard = match self.execution_root.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.selected_root = selected_root;
        guard.probed = true;
    }
}
