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
        Some("PlaceRemote") => super::placement::place_remote(&value, state).await,
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
        &super::placement::excluded_node_ids(request),
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

async fn result(state: Arc<Mutex<State>>) -> serde_json::Value {
    let state = state.lock().await;
    responses::result(&state.selected_node, state.failover_results)
}
