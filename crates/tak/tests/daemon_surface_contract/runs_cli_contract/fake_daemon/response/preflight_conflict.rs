use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) fn response(request_id: &str, request: &Value) -> Value {
    match request["operation"]["type"].as_str().unwrap() {
        "GetOutputManifest" => json!({
            "protocol_version": 2, "type": "OutputManifest", "request_id": request_id,
            "run_id": "run-123", "expired": false, "artifacts": [
                artifact("first.txt", "artifact-first", b"daemon-a"),
                artifact("second.txt", "artifact-second", b"daemon-b"),
            ],
        }),
        "GetOutputChunk" => {
            let artifact_id = request["operation"]["artifact_id"].as_str().unwrap();
            let chunk = match artifact_id {
                "artifact-first" => "ZGFlbW9uLWE=",
                "artifact-second" => "ZGFlbW9uLWI=",
                other => panic!("unexpected artifact {other}"),
            };
            json!({
                "protocol_version": 2, "type": "OutputChunk", "request_id": request_id,
                "artifact_id": artifact_id, "offset": 0,
                "chunk_base64": chunk, "complete": true,
            })
        }
        _ => super::submission::submission_response(request_id, request, false, None),
    }
}

fn artifact(path: &str, artifact_id: &str, contents: &[u8]) -> Value {
    json!({
        "path": path, "entry_type": "file", "executable": false,
        "symlink_target": null, "size": contents.len(),
        "sha256": format!("{:x}", Sha256::digest(contents)),
        "artifact_id": artifact_id,
    })
}
