use serde_json::{Value, json};

pub(super) fn response(request_id: &str, request: &Value) -> Value {
    if request["operation"]["type"] == "ResolveRemoteCandidates" {
        return json!({
            "protocol_version": 2, "type": "RemoteCandidates", "request_id": request_id,
            "candidates": [],
        });
    }
    super::submission::remote_submission_response(request_id, request)
}
