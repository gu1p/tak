use anyhow::{Result, anyhow};

use super::types::DaemonRequest;
use crate::engine::StrictRemoteTarget;

pub(super) fn lifecycle_request(
    target: &StrictRemoteTarget,
    method: &str,
    path: &str,
    extra_headers: &[(&str, String)],
) -> Result<Option<DaemonRequest>> {
    let Some(task_run_id) = task_run_id_from_path(path) else {
        return Ok(None);
    };
    let task_handle = task_handle(target, &task_run_id);
    if method == "GET" && path.contains("/events") {
        return Ok(Some(DaemonRequest::StreamTaskEvents {
            request_id: request_id("events", target, path),
            task_handle,
            after_seq: query_u64(path, "after_seq").unwrap_or(0),
        }));
    }
    if method == "POST" && path.contains("/cancel") {
        return Ok(Some(DaemonRequest::CancelTask {
            request_id: request_id("cancel", target, path),
            task_handle,
            attempt: query_u32(path, "attempt").unwrap_or(1),
        }));
    }
    result_or_output_request(target, method, path, extra_headers, task_handle)
}

fn result_or_output_request(
    target: &StrictRemoteTarget,
    method: &str,
    path: &str,
    extra_headers: &[(&str, String)],
    task_handle: String,
) -> Result<Option<DaemonRequest>> {
    if method == "GET" && path.ends_with("/result") {
        return Ok(Some(DaemonRequest::GetTaskResult {
            request_id: request_id("result", target, path),
            task_handle,
        }));
    }
    if method == "GET" && path.contains("/outputs") {
        return Ok(Some(DaemonRequest::GetOutputRange {
            request_id: request_id("outputs", target, path),
            task_handle,
            attempt: query_u32(path, "attempt").unwrap_or(1),
            path: query_string(path, "path")?
                .ok_or_else(|| anyhow!("output request missing path query"))?,
            range: range_header(extra_headers),
        }));
    }
    Ok(None)
}

fn range_header(headers: &[(&str, String)]) -> Option<String> {
    headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("range"))
        .map(|(_, value)| value.clone())
}

fn task_handle(target: &StrictRemoteTarget, task_run_id: &str) -> String {
    target
        .daemon_task_handle
        .clone()
        .unwrap_or_else(|| format!("remote:{}:{task_run_id}", target.node_id))
}

fn task_run_id_from_path(path: &str) -> Option<String> {
    let path = path.split('?').next().unwrap_or(path);
    let parts = path
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    (parts.len() == 4
        && parts[0] == "v1"
        && parts[1] == "tasks"
        && matches!(parts[3], "events" | "cancel" | "result" | "outputs"))
    .then(|| parts[2].to_string())
}

fn query_u64(path: &str, key: &str) -> Option<u64> {
    query_string(path, key).ok().flatten()?.parse().ok()
}

fn query_u32(path: &str, key: &str) -> Option<u32> {
    query_string(path, key).ok().flatten()?.parse().ok()
}

fn query_string(path: &str, key: &str) -> Result<Option<String>> {
    let Some((_, query)) = path.split_once('?') else {
        return Ok(None);
    };
    Ok(url::form_urlencoded::parse(query.as_bytes())
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.into_owned()))
}

pub(super) fn request_id(prefix: &str, target: &StrictRemoteTarget, path: &str) -> String {
    format!("{prefix}:{}:{:x}", target.node_id, stable_hash(path))
}

fn stable_hash(value: &str) -> u64 {
    value
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325, |hash, byte| {
            hash.wrapping_mul(0x100000001b3)
                .wrapping_add(u64::from(*byte))
        })
}
