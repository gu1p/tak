use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) fn management_response(request_id: &str, request: &Value) -> Value {
    match request["operation"]["type"]
        .as_str()
        .expect("operation type")
    {
        "ListRuns" => json!({
            "protocol_version": 2, "type": "RunList", "request_id": request_id,
            "runs": [summary("running")],
        }),
        "GetRun" => json!({
            "protocol_version": 2, "type": "RunDetails", "request_id": request_id,
            "run": {"summary": summary("running"), "jobs": [{
                "job_id": "job-0", "task_ids": ["//:check"], "state": "running",
                "node_id": "worker-a", "attempt": 1, "cache": "miss",
            }, {
                "job_id": "job-1", "task_ids": ["//:cached"], "state": "running",
                "node_id": "worker-b", "attempt": 1, "cache": "hit",
            }]},
        }),
        "AttachRun" => json!({
            "protocol_version": 2, "type": "RunEvents", "request_id": request_id,
            "run_id": "run-1", "next_event": 1, "state": "succeeded", "terminal": true,
            "events": [{"seq": 1, "kind": "succeeded", "job_id": "job-0",
                "task_ids": ["//:check"], "node_id": "worker-a", "message": "done"}],
        }),
        "CancelRun" => json!({
            "protocol_version": 2, "type": "CancellationAccepted", "request_id": request_id,
            "run_id": "run-1", "state": "cancelling",
        }),
        "GetOutputManifest" => json!({
            "protocol_version": 2, "type": "OutputManifest", "request_id": request_id,
            "run_id": "run-1", "expired": false, "artifacts": [{
                "path": "result.txt", "entry_type": "file", "executable": false,
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
        other => panic!("unexpected management operation {other}"),
    }
}

pub(super) fn failed_attach_response(request_id: &str, request: &Value) -> Value {
    if request["operation"]["type"] == "GetOutputManifest" {
        return json!({
            "protocol_version": 2, "type": "OutputManifest", "request_id": request_id,
            "run_id": "run-1", "expired": false, "artifacts": [],
        });
    }
    let after = request["operation"]["after_event"].as_u64().unwrap();
    let (state, terminal, events) = if after == 0 {
        (
            "running",
            false,
            json!([{"seq": 1, "kind": "failed", "job_id": "job-0",
                "task_ids": ["//:check"], "node_id": "worker-a", "message": "failed"}]),
        )
    } else {
        ("failed", true, json!([]))
    };
    json!({
        "protocol_version": 2, "type": "RunEvents", "request_id": request_id,
        "run_id": "run-1", "next_event": 1, "state": state,
        "terminal": terminal, "events": events,
    })
}

fn summary(state: &str) -> Value {
    json!({
        "run_id": "run-1", "state": state, "created_at_ms": 1, "updated_at_ms": 2,
        "targets": ["//:check"], "total_jobs": 1, "terminal_jobs": 0,
    })
}
