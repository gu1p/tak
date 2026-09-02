use serde_json::{Value, json};

use super::submission::submission_response;

pub(super) fn present(request_id: &str, request: &Value) -> Value {
    if request["operation"]["type"] != "SubmitRun" {
        return submission_response(request_id, request, false, None);
    }
    json!({
        "protocol_version": 2, "type": "RunSubmitted", "request_id": request_id,
        "run_id": "run-123", "workspace": {"status": "present"},
    })
}

pub(super) fn resumable(request_id: &str, request: &Value, next_offset: u64) -> Value {
    if request["operation"]["type"] != "SubmitRun" {
        return submission_response(request_id, request, false, None);
    }
    json!({
        "protocol_version": 2, "type": "RunSubmitted", "request_id": request_id,
        "run_id": "run-123",
        "workspace": {"status": "upload_required", "next_offset": next_offset},
    })
}

pub(super) fn attach_cancellation(
    request_id: &str,
    request: &Value,
    request_number: usize,
) -> Value {
    match request["operation"]["type"].as_str().unwrap() {
        "CancelRun" => json!({
            "protocol_version": 2, "type": "CancellationAccepted",
            "request_id": request_id, "run_id": "run-123", "state": "cancelling",
        }),
        "GetOutputManifest" => json!({
            "protocol_version": 2, "type": "OutputManifest", "request_id": request_id,
            "run_id": "run-123", "expired": false, "artifacts": [],
        }),
        "AttachRun" if request_number < 3 => event_page(
            request_id,
            if request_number < 2 {
                "running"
            } else {
                "cancelling"
            },
            false,
        ),
        "AttachRun" => event_page(request_id, "cancelled", true),
        other => panic!("unexpected attach-cancellation operation {other}"),
    }
}

fn event_page(request_id: &str, state: &str, terminal: bool) -> Value {
    json!({
        "protocol_version": 2, "type": "RunEvents", "request_id": request_id,
        "run_id": "run-123", "next_event": 0, "state": state,
        "terminal": terminal, "events": [],
    })
}
