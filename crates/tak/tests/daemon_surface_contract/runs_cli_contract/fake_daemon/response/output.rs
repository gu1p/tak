use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) fn unsafe_output_response(request_id: &str, request: &Value) -> Value {
    match request["operation"]["type"].as_str().unwrap() {
        "GetOutputManifest" => json!({
            "protocol_version": 2, "type": "OutputManifest", "request_id": request_id,
            "run_id": "run-1", "expired": false, "artifacts": [
                {"path": "a//link", "entry_type": "symlink", "executable": false,
                 "symlink_target": ".", "size": 1,
                 "sha256": format!("{:x}", Sha256::digest(b".")), "artifact_id": "link"},
                {"path": "a/link/escaped.txt", "entry_type": "file", "executable": false,
                 "symlink_target": null, "size": 6,
                 "sha256": format!("{:x}", Sha256::digest(b"escape")), "artifact_id": "file"}
            ],
        }),
        "GetOutputChunk" => json!({
            "protocol_version": 2, "type": "OutputChunk", "request_id": request_id,
            "artifact_id": "file", "offset": 0, "chunk_base64": "ZXNjYXBl", "complete": true,
        }),
        other => panic!("unexpected unsafe-output operation {other}"),
    }
}

pub(super) fn huge_output_response(request_id: &str, request: &Value) -> Value {
    match request["operation"]["type"].as_str().unwrap() {
        "GetOutputManifest" => json!({
            "protocol_version": 2, "type": "OutputManifest", "request_id": request_id,
            "run_id": "run-1", "expired": false, "artifacts": [{
                "path": "huge.bin", "entry_type": "file", "executable": false,
                "symlink_target": null, "size": u64::MAX,
                "sha256": "0".repeat(64), "artifact_id": "huge"
            }],
        }),
        "GetOutputChunk" => json!({
            "protocol_version": 2, "type": "OutputChunk", "request_id": request_id,
            "artifact_id": "huge", "offset": 0, "chunk_base64": "", "complete": false,
        }),
        other => panic!("unexpected huge-output operation {other}"),
    }
}

pub(super) fn symlink_chain_output_response(request_id: &str, request: &Value) -> Value {
    assert_eq!(request["operation"]["type"], "GetOutputManifest");
    let empty = format!("{:x}", Sha256::digest([]));
    json!({
        "protocol_version": 2, "type": "OutputManifest", "request_id": request_id,
        "run_id": "run-1", "expired": false, "artifacts": [
            {"path": "inside", "entry_type": "directory", "executable": false,
             "symlink_target": null, "size": 0, "sha256": empty, "artifact_id": "inside"},
            {"path": "a", "entry_type": "directory", "executable": false,
             "symlink_target": null, "size": 0,
             "sha256": format!("{:x}", Sha256::digest([])), "artifact_id": "a"},
            {"path": "a/pivot", "entry_type": "symlink", "executable": false,
             "symlink_target": "../inside", "size": 9,
             "sha256": format!("{:x}", Sha256::digest(b"../inside")), "artifact_id": "pivot"},
            {"path": "a/link", "entry_type": "symlink", "executable": false,
             "symlink_target": "pivot/../..", "size": 11,
             "sha256": format!("{:x}", Sha256::digest(b"pivot/../..")), "artifact_id": "link"}
        ]
    })
}
