use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) fn response(request_id: &str, request: &Value) -> Value {
    match request["operation"]["type"].as_str().unwrap() {
        "GetOutputManifest" => json!({
            "protocol_version": 2, "type": "OutputManifest", "request_id": request_id,
            "run_id": "run-123", "expired": false, "artifacts": [{
                "path": "generated.txt", "entry_type": "file", "executable": false,
                "symlink_target": null, "size": 8,
                "sha256": format!("{:x}", Sha256::digest(b"artifact")),
                "artifact_id": "artifact-1",
            }],
        }),
        "GetOutputChunk" => json!({
            "protocol_version": 2, "type": "OutputChunk", "request_id": request_id,
            "artifact_id": "artifact-1", "offset": 0,
            "chunk_base64": "YXJ0aWZhY3Q=", "complete": true,
        }),
        _ => super::submission::submission_response(request_id, request, false, None),
    }
}
