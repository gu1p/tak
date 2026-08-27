use std::sync::Arc;

use tokio::sync::Mutex;

use super::super::{State, responses};

pub(super) async fn place_remote(
    request: &serde_json::Value,
    state: Arc<Mutex<State>>,
) -> serde_json::Value {
    let excluded = excluded_node_ids(request);
    if ["builder-a", "builder-b"]
        .iter()
        .all(|node| excluded.iter().any(|value| value == node))
    {
        return responses::classified_error(
            "all Tor peers are unreachable",
            "all_tor_peers_unreachable",
            true,
        );
    }
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
    if state.submit_transport_failover && selected_node == "builder-a" {
        return responses::placement::classified_peer_error(
            selected_node,
            "remote node builder-a unavailable: Tor connection failed",
            "remote_placement_transport_failed",
        );
    }
    let status =
        if state.submit_always_fails || (state.submit_failover && selected_node == "builder-a") {
            503
        } else {
            200
        };
    responses::placed(selected_node, status)
}

pub(super) fn excluded_node_ids(request: &serde_json::Value) -> Vec<String> {
    request
        .get("excluded_node_ids")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect()
}
