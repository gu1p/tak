pub(in super::super) fn classified_peer_error(
    node_id: &str,
    message: &str,
    code: &str,
) -> serde_json::Value {
    serde_json::json!({
        "type": "Error",
        "message": message,
        "code": code,
        "retryable": true,
        "node_id": node_id
    })
}
