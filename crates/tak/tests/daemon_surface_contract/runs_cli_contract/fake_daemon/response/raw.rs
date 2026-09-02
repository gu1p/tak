use serde_json::{Value, json};

pub(super) fn response(bytes: &[u8], request_id: &str, request: &Value) -> Vec<u8> {
    if request["operation"]["type"] != "GetOutputManifest" || is_output_manifest(bytes) {
        return bytes.to_vec();
    }
    serde_json::to_vec(&json!({
        "protocol_version": 2, "type": "OutputManifest", "request_id": request_id,
        "run_id": request["operation"]["run_id"], "expired": false, "artifacts": [],
    }))
    .unwrap()
    .into_iter()
    .chain(*b"\n")
    .collect()
}

fn is_output_manifest(bytes: &[u8]) -> bool {
    serde_json::from_slice::<Value>(bytes)
        .ok()
        .is_some_and(|value| value["type"] == "OutputManifest")
}
