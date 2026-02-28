use std::collections::HashSet;

use anyhow::{Context, Result, anyhow, bail};
use tak_core::model::{RemoteRuntimeSpec, ResolvedTask};

use crate::{
    ParsedRemoteEvents, RemoteLogChunk, RemoteWorkspaceStage, StrictRemoteTarget, SyncedOutput,
};

pub(crate) fn build_remote_submit_payload(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    attempt: u32,
    task_label: &str,
    task: &ResolvedTask,
    remote_workspace: &RemoteWorkspaceStage,
    include_workspace_archive: bool,
) -> Result<serde_json::Value> {
    if !include_workspace_archive {
        return Ok(serde_json::json!({
            "task_run_id": task_run_id,
            "attempt": attempt,
            "task_label": task_label,
            "selected_node_id": target.node_id,
            "workspace": {
                "mode": "REPO_ZIP_SNAPSHOT",
                "manifest_hash": remote_workspace.manifest_hash,
            },
        }));
    }

    let runtime = target
        .runtime
        .as_ref()
        .map(remote_runtime_submit_value)
        .unwrap_or(serde_json::Value::Null);

    let steps = serde_json::to_value(&task.steps)
        .context("failed serializing task steps for remote submit payload")?;

    Ok(serde_json::json!({
        "task_run_id": task_run_id,
        "attempt": attempt,
        "task_label": task_label,
        "selected_node_id": target.node_id,
        "workspace": {
            "mode": "REPO_ZIP_SNAPSHOT",
            "archive_zip_base64": remote_workspace.archive_zip_base64,
            "manifest_hash": remote_workspace.manifest_hash,
        },
        "execution": {
            "steps": steps,
            "timeout_s": task.timeout_s,
            "runtime": runtime,
        },
        "result": {
            "sync_mode": "OUTPUTS_AND_LOGS",
        },
    }))
}

fn remote_runtime_submit_value(runtime: &RemoteRuntimeSpec) -> serde_json::Value {
    match runtime {
        RemoteRuntimeSpec::Containerized { image } => serde_json::json!({
            "kind": "containerized",
            "image": image,
        }),
    }
}
