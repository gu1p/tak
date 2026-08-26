use std::collections::BTreeMap;

use anyhow::Result;
use tak_core::model::{ResolvedTask, TaskLabel};

use super::PlacementMode;
use super::remote_models::{RemoteWorkspaceStage, RuntimeExecutionMetadata, TaskPlacement};
use super::runtime_metadata::resolve_runtime_execution_metadata;
use super::session_workspaces::PreparedTaskSession;

mod acceptance;
mod resolve;
mod target_refresh;
mod upload_progress;

pub(crate) use resolve::resolve_attempt_submit_state;

pub(crate) struct AttemptSubmitState<'a> {
    /// The eagerly-staged workspace (miss path). `None` on the cache-hit path, where staging
    /// was skipped up front; it is staged lazily here only if an upload turns out to be needed.
    pub(crate) remote_workspace: Option<&'a RemoteWorkspaceStage>,
    /// Deterministic content hash of the staged workspace, keying the per-job upload cache.
    /// `None` for non-remote placements.
    pub(crate) workspace_content_hash: Option<&'a str>,
    pub(crate) task_run_id: &'a str,
    pub(crate) attempt: u32,
    pub(crate) session: Option<&'a PreparedTaskSession>,
    pub(crate) fused_members: Option<&'a [ResolvedTask]>,
    pub(crate) execution_label: Option<&'a str>,
    pub(crate) fused_member_execution_labels: Option<&'a BTreeMap<TaskLabel, String>>,
}

pub(crate) async fn resolve_initial_runtime_metadata(
    task: &ResolvedTask,
    placement: &mut TaskPlacement,
) -> Result<Option<RuntimeExecutionMetadata>> {
    if placement.placement_mode == PlacementMode::Remote {
        return Ok(None);
    }
    resolve_runtime_execution_metadata(task, placement)
}
