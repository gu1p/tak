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

pub(crate) fn parse_remote_events_response(
    target: &StrictRemoteTarget,
    response_body: &str,
    last_seen_seq: u64,
) -> Result<ParsedRemoteEvents> {
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(response_body)
        && is_wrapped_remote_events_payload(&parsed)
    {
        return parse_wrapped_remote_events(target, &parsed, last_seen_seq);
    }

    parse_ndjson_remote_events(target, response_body, last_seen_seq)
}

fn is_wrapped_remote_events_payload(parsed: &serde_json::Value) -> bool {
    parsed
        .as_object()
        .is_some_and(|object| object.contains_key("events") || object.contains_key("done"))
}

fn parse_wrapped_remote_events(
    target: &StrictRemoteTarget,
    parsed: &serde_json::Value,
    last_seen_seq: u64,
) -> Result<ParsedRemoteEvents> {
    let mut checkpoint = last_seen_seq;
    let mut remote_logs = Vec::new();
    let mut observed_new_log_seqs = HashSet::new();
    if let Some(events) = parsed.get("events") {
        let events = events.as_array().ok_or_else(|| {
            anyhow!(
                "infra error: remote node {} events payload must contain an array",
                target.node_id
            )
        })?;
        for event in events {
            let Some(seq) = event.get("seq").and_then(serde_json::Value::as_u64) else {
                continue;
            };
            if seq > checkpoint {
                checkpoint = seq;
            }
            if seq <= last_seen_seq || !observed_new_log_seqs.insert(seq) {
                continue;
            }

            let is_log_chunk = event
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|kind| kind == "TASK_LOG_CHUNK");
            if !is_log_chunk {
                continue;
            }

            let chunk = event
                .get("chunk")
                .and_then(serde_json::Value::as_str)
                .or_else(|| event.get("message").and_then(serde_json::Value::as_str))
                .unwrap_or_default();
            remote_logs.push(RemoteLogChunk {
                seq,
                chunk: chunk.to_string(),
            });
        }
    }
    remote_logs.sort_unstable_by_key(|chunk| chunk.seq);

    Ok(ParsedRemoteEvents {
        next_seq: checkpoint,
        done: parsed
            .get("done")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true),
        remote_logs,
    })
}

fn parse_ndjson_remote_events(
    target: &StrictRemoteTarget,
    response_body: &str,
    last_seen_seq: u64,
) -> Result<ParsedRemoteEvents> {
    let mut checkpoint = last_seen_seq;
    let mut remote_logs = Vec::new();
    let mut observed_new_log_seqs = HashSet::new();
    let mut done = false;

    for line in response_body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let event: serde_json::Value = serde_json::from_str(line).with_context(|| {
            format!(
                "infra error: remote node {} returned invalid NDJSON event line",
                target.node_id
            )
        })?;
        let Some(seq) = event.get("seq").and_then(serde_json::Value::as_u64) else {
            continue;
        };
        if seq > checkpoint {
            checkpoint = seq;
        }
        if seq <= last_seen_seq || !observed_new_log_seqs.insert(seq) {
            continue;
        }

        let event_type = event
            .get("type")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                event
                    .get("payload")
                    .and_then(|payload| payload.get("kind"))
                    .and_then(serde_json::Value::as_str)
            })
            .unwrap_or_default();
        if event_type == "TASK_LOG_CHUNK" {
            let payload = event.get("payload").unwrap_or(&serde_json::Value::Null);
            let chunk = payload
                .get("chunk")
                .and_then(serde_json::Value::as_str)
                .or_else(|| payload.get("message").and_then(serde_json::Value::as_str))
                .or_else(|| event.get("chunk").and_then(serde_json::Value::as_str))
                .unwrap_or_default();
            remote_logs.push(RemoteLogChunk {
                seq,
                chunk: chunk.to_string(),
            });
        }
        if matches!(
            event_type,
            "TASK_COMPLETED" | "TASK_FAILED" | "TASK_TERMINAL"
        ) {
            done = true;
        }
    }
    remote_logs.sort_unstable_by_key(|chunk| chunk.seq);

    Ok(ParsedRemoteEvents {
        next_seq: checkpoint,
        done,
        remote_logs,
    })
}

pub(crate) fn parse_remote_result_outputs(
    target: &StrictRemoteTarget,
    result: &serde_json::Value,
) -> Result<Vec<SyncedOutput>> {
    let Some(outputs) = result.get("outputs") else {
        return Ok(Vec::new());
    };
    let outputs = outputs.as_array().ok_or_else(|| {
        anyhow!(
            "infra error: remote node {} result outputs field must be an array",
            target.node_id
        )
    })?;

    let mut synced_outputs = Vec::with_capacity(outputs.len());
    for output in outputs {
        let path = output
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                anyhow!(
                    "infra error: remote node {} result output is missing string path",
                    target.node_id
                )
            })?
            .trim()
            .to_string();
        if path.is_empty() {
            bail!(
                "infra error: remote node {} result output path cannot be empty",
                target.node_id
            );
        }

        let digest = output
            .get("digest")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                anyhow!(
                    "infra error: remote node {} result output is missing string digest",
                    target.node_id
                )
            })?
            .trim()
            .to_string();
        if digest.is_empty() {
            bail!(
                "infra error: remote node {} result output digest cannot be empty",
                target.node_id
            );
        }

        let size_bytes = output
            .get("size")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                anyhow!(
                    "infra error: remote node {} result output is missing numeric size",
                    target.node_id
                )
            })?;

        synced_outputs.push(SyncedOutput {
            path,
            digest,
            size_bytes,
        });
    }

    Ok(synced_outputs)
}
