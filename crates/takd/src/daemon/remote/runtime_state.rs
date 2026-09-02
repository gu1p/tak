use std::path::PathBuf;

use super::runtime::RemoteRuntimeConfig;

#[derive(Debug)]
pub(crate) struct RemoteRuntimeState {
    pub(crate) config: RemoteRuntimeConfig,
    execution_root: PathBuf,
}

impl RemoteRuntimeState {
    pub(crate) fn new(config: RemoteRuntimeConfig) -> Self {
        let selected_root = config.initial_execution_root_base();
        Self {
            config,
            execution_root: selected_root,
        }
    }

    pub(crate) fn execution_root_base(&self) -> &PathBuf {
        &self.execution_root
    }
}
