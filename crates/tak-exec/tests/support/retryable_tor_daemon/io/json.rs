use std::sync::Arc;

use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::Mutex;

use super::super::{State, responses};

pub(super) async fn serve(
    mut reader: BufReader<UnixStream>,
    line: String,
    state: Arc<Mutex<State>>,
) {
    let value = serde_json::from_str::<serde_json::Value>(&line).expect("daemon request json");
    let response = match value.get("type").and_then(|value| value.as_str()) {
        Some("PeersEligible") => peers(&value, state).await,
        Some("ForwardRemoteHttp") => upload_status(&value, state).await,
        Some("PlaceRemote") => place_remote(&value, state).await,
        Some("StreamTaskEvents") => responses::events(),
        Some("GetTaskResult") => result(state).await,
        _ => responses::error("unexpected daemon request"),
    };
    let stream = reader.get_mut();
    let _ = stream.write_all(response.to_string().as_bytes()).await;
    let _ = stream.write_all(b"\n").await;
}

async fn peers(request: &serde_json::Value, state: Arc<Mutex<State>>) -> serde_json::Value {
    let mut state = state.lock().await;
    state.peer_requests += 1;
    if state.non_retryable_peers {
        return responses::classified_error(
            "No known remote worker satisfies this task's requirements.",
            "resource_requirements_exceed_worker_capacity",
            false,
        );
    }
    responses::peers(
        state.failover_results || state.upload_failover,
        &excluded_node_ids(request),
    )
}

async fn upload_status(request: &serde_json::Value, state: Arc<Mutex<State>>) -> serde_json::Value {
    let path = request
        .get("path")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let upload_id = path
        .trim_start_matches("/v2/workspaces/uploads/")
        .to_string();
    let state = state.lock().await;
    responses::upload_status(&upload_id, state.committed, state.committed == state.size)
}

async fn place_remote(request: &serde_json::Value, state: Arc<Mutex<State>>) -> serde_json::Value {
    let excluded = excluded_node_ids(request);
    let preferred = request
        .get("preferred_node_id")
        .and_then(|value| value.as_str());
    let selected_node = if preferred.is_some_and(|node| !excluded.iter().any(|value| value == node))
    {
        preferred.expect("checked preferred node")
    } else if excluded.iter().any(|node| node == "builder-a") {
        "builder-b"
    } else if state.lock().await.failover_results {
        "builder-a"
    } else {
        "builder-retry"
    };
    let mut state = state.lock().await;
    if let Some(attempt) = request.get("attempt").and_then(|value| value.as_u64()) {
        state.submit_attempts.push(attempt as u32);
    }
    state.selected_node = selected_node.to_string();
    state.placement_exclusions.push(excluded);
    responses::placed(selected_node)
}

async fn result(state: Arc<Mutex<State>>) -> serde_json::Value {
    let state = state.lock().await;
    responses::result(&state.selected_node, state.failover_results)
}

fn excluded_node_ids(request: &serde_json::Value) -> Vec<String> {
    request
        .get("excluded_node_ids")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect()
}
