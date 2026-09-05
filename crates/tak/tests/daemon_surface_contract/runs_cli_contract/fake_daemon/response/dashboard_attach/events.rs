use serde_json::{Value, json};

pub(super) fn response(request_id: &str, request: &Value, logs_expired: bool) -> Value {
    if request["operation"]["after_event"] == 0 {
        return page(
            request_id,
            3,
            "running",
            false,
            json!([
                {"seq": 1, "kind": "queued", "job_id": "lint", "task_ids": ["//:lint"],
                 "node_id": null, "message": "waiting for capacity"},
                {"seq": 2, "kind": "transferring", "job_id": "test", "task_ids": ["//:test"],
                 "node_id": "worker-b", "message": "cache hit"},
                {"seq": 3, "kind": "running", "job_id": "build", "task_ids": ["//:build"],
                 "node_id": "worker-a", "message": "started"}
            ]),
            logs_expired,
        );
    }
    let terminal_events = if logs_expired {
        json!([
            {"seq": 5, "kind": "succeeded", "job_id": "build", "task_ids": ["//:build"],
             "node_id": "worker-a", "message": "done"},
            {"seq": 6, "kind": "succeeded", "job_id": "test", "task_ids": ["//:test"],
             "node_id": "worker-b", "message": "done"},
            {"seq": 7, "kind": "succeeded", "job_id": "lint", "task_ids": ["//:lint"],
             "node_id": "worker-a", "message": "done"}
        ])
    } else {
        json!([
            {"seq": 4, "kind": "stdout", "job_id": "build", "task_ids": ["//:build"],
             "node_id": "worker-a", "message": "", "chunk_base64": "YnVpbGQgbG9nCg=="},
            {"seq": 5, "kind": "succeeded", "job_id": "build", "task_ids": ["//:build"],
             "node_id": "worker-a", "message": "done"},
            {"seq": 6, "kind": "succeeded", "job_id": "test", "task_ids": ["//:test"],
             "node_id": "worker-b", "message": "done"},
            {"seq": 7, "kind": "succeeded", "job_id": "lint", "task_ids": ["//:lint"],
             "node_id": "worker-a", "message": "done"}
        ])
    };
    page(
        request_id,
        7,
        "succeeded",
        true,
        terminal_events,
        logs_expired,
    )
}

pub(super) fn terminal(request_id: &str, state: &str) -> Value {
    page(request_id, 0, state, true, json!([]), false)
}

fn page(
    request_id: &str,
    next_event: u64,
    state: &str,
    terminal: bool,
    events: Value,
    logs_expired: bool,
) -> Value {
    json!({"protocol_version": 2, "type": "RunEvents", "request_id": request_id,
        "run_id": "run-dashboard", "next_event": next_event, "state": state,
        "terminal": terminal, "events": events, "logs_expired": logs_expired})
}
