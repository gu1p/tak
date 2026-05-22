use anyhow::{Result, anyhow};

use super::AttemptExecutionContext;
use crate::engine::SyncedOutput;
use crate::engine::workspace_sync::{sync_remote_outputs, sync_remote_outputs_from_remote};

pub(super) async fn sync_attempt_outputs(
    context: &AttemptExecutionContext<'_>,
    synced_outputs: &[SyncedOutput],
    run_local_attempt: bool,
) -> Result<()> {
    if run_local_attempt {
        sync_local_attempt_outputs(context, synced_outputs)?;
        return Ok(());
    }

    let target = context
        .placement
        .strict_remote_target
        .as_ref()
        .ok_or_else(|| {
            anyhow!(
                "infra error: missing strict remote target during output sync for task {}",
                context.task.label
            )
        })?;
    sync_remote_outputs_from_remote(
        target,
        context.task_run_id,
        context.attempt,
        context.workspace_root,
        synced_outputs,
    )
    .await
}

fn sync_local_attempt_outputs(
    context: &AttemptExecutionContext<'_>,
    synced_outputs: &[SyncedOutput],
) -> Result<()> {
    if context.run_root != context.workspace_root {
        sync_remote_outputs(context.run_root, context.workspace_root, synced_outputs)?;
    } else if let Some(staged_workspace) = context.remote_workspace {
        sync_remote_outputs(
            staged_workspace.temp_dir.path(),
            context.workspace_root,
            synced_outputs,
        )?;
    }
    Ok(())
}
