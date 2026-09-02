use std::env;
use std::path::{Path, PathBuf};

use super::types::RemoteNodeContext;

const REMOTE_ARTIFACT_ROOT_DIR: &str = "takd-remote-artifacts";

pub(super) fn remote_execution_root_base(context: &RemoteNodeContext) -> PathBuf {
    context.runtime_state().execution_root_base().to_path_buf()
}

pub(super) fn artifact_root_base_for_execution_root_base(execution_root_base: &Path) -> PathBuf {
    execution_root_base
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(env::temp_dir)
        .join(REMOTE_ARTIFACT_ROOT_DIR)
}
