use serde_json::{Value, json};

pub(super) fn submission_response(
    request_id: &str,
    request: &Value,
    fail: bool,
    cancelled: bool,
) -> Value {
    let operation = &request["operation"];
    match operation["type"].as_str().unwrap() {
        "SubmitRun" => json!({
            "protocol_version": 2, "type": "RunSubmitted", "request_id": request_id,
            "run_id": "run-123", "workspace": {"status": "upload_required", "next_offset": 0},
        }),
        "UploadWorkspace" => json!({
            "protocol_version": 2, "type": "WorkspaceUploadProgress", "request_id": request_id,
            "run_id": "run-123", "workspace_fingerprint": operation["workspace_fingerprint"],
            "chunk_accepted": true, "next_offset": operation["archive_size"], "complete": true,
        }),
        "CommitRun" => json!({
            "protocol_version": 2, "type": "RunCommitted", "request_id": request_id,
            "run_id": "run-123", "state": "queued",
        }),
        "CancelRun" if cancelled => json!({
            "protocol_version": 2, "type": "CancellationAccepted", "request_id": request_id,
            "run_id": "run-123", "state": "cancelled",
        }),
        "AttachRun" if cancelled => json!({
            "protocol_version": 2, "type": "RunEvents", "request_id": request_id,
            "run_id": "run-123", "next_event": 0, "state": "cancelled", "terminal": true,
            "events": [],
        }),
        "AttachRun" if fail => failed_attachment(request_id, operation),
        "AttachRun" => json!({
            "protocol_version": 2, "type": "RunEvents", "request_id": request_id,
            "run_id": "run-123", "next_event": 4, "state": "succeeded", "terminal": true,
            "events": [
                event(1, "queued", "//:dep"), event(2, "running", "//:dep"),
                event(3, "succeeded", "//:dep"), event(4, "succeeded", "//:target")
            ],
        }),
        other => panic!("unexpected submission operation {other}"),
    }
}

fn failed_attachment(request_id: &str, operation: &Value) -> Value {
    if operation["after_event"].as_u64() == Some(0) {
        return json!({
            "protocol_version": 2, "type": "RunEvents", "request_id": request_id,
            "run_id": "run-123", "next_event": 1, "state": "running", "terminal": false,
            "events": [event(1, "failed", "//:target")],
        });
    }
    json!({
        "protocol_version": 2, "type": "RunEvents", "request_id": request_id,
        "run_id": "run-123", "next_event": 1, "state": "failed", "terminal": true,
        "events": [],
    })
}

fn event(seq: u64, kind: &str, task: &str) -> Value {
    json!({"seq": seq, "kind": kind, "job_id": "job-0", "task_ids": [task],
        "node_id": "local", "message": kind})
}
