use serde_json::{Value, json};

#[path = "dashboard_attach/events.rs"]
mod events;

pub(super) fn response(request_id: &str, request: &Value) -> Value {
    response_with_log_retention(request_id, request, false)
}

pub(super) fn expired_response(request_id: &str, request: &Value) -> Value {
    response_with_log_retention(request_id, request, true)
}

fn response_with_log_retention(request_id: &str, request: &Value, logs_expired: bool) -> Value {
    match request["operation"]["type"].as_str().unwrap() {
        "GetRun" => details(request_id),
        "AttachRun" => events::response(request_id, request, logs_expired),
        "GetOutputManifest" => json!({
            "protocol_version": 2, "type": "OutputManifest", "request_id": request_id,
            "run_id": "run-dashboard", "expired": false, "artifacts": [],
        }),
        other => panic!("unexpected dashboard operation {other}"),
    }
}

pub(super) fn interactive_response(
    request_id: &str,
    request: &Value,
    request_number: usize,
) -> Value {
    match request["operation"]["type"].as_str().unwrap() {
        "GetRun" => interactive_details(request_id),
        "CancelRun" => json!({
            "protocol_version": 2, "type": "CancellationAccepted",
            "request_id": request_id, "run_id": "run-dashboard", "state": "cancelling",
        }),
        "AttachRun" if request_number == 1 => events::terminal(request_id, "succeeded"),
        "AttachRun" => events::terminal(request_id, "cancelled"),
        "GetOutputManifest" => json!({
            "protocol_version": 2, "type": "OutputManifest", "request_id": request_id,
            "run_id": "run-dashboard", "expired": false, "artifacts": [],
        }),
        other => panic!("unexpected interactive dashboard operation {other}"),
    }
}

fn interactive_details(request_id: &str) -> Value {
    let jobs = (0..30)
        .map(|index| {
            json!({
                "job_id": format!("job-{index:02}"),
                "task_ids": [if index == 29 {
                    "FINAL-TASK-REACHED".to_owned()
                } else {
                    format!("//:task-{index:02}")
                }], "state": "ready",
                "node_id": null, "attempt": 0, "cache": null, "queue": "builds",
                "placement_candidate_node_ids": [format!("worker-{index:02}")],
            })
        })
        .collect::<Vec<_>>();
    json!({
        "protocol_version": 2, "type": "RunDetails", "request_id": request_id,
        "run": {"summary": {
            "run_id": "run-dashboard", "state": "running", "created_at_ms": 1,
            "updated_at_ms": 2, "targets": ["//:check"], "total_jobs": 30,
            "terminal_jobs": 0
        }, "max_parallel_jobs": 4, "jobs": jobs}
    })
}

fn details(request_id: &str) -> Value {
    json!({
        "protocol_version": 2, "type": "RunDetails", "request_id": request_id,
        "run": {"summary": {
            "run_id": "run-dashboard", "state": "running", "created_at_ms": 1,
            "updated_at_ms": 2, "targets": ["//:check"], "total_jobs": 3,
            "terminal_jobs": 0
        }, "max_parallel_jobs": 3, "jobs": [
            {"job_id": "build", "task_ids": ["//:build"], "state": "running",
             "node_id": "worker-a", "attempt": 1, "cache": "miss", "queue": "builds",
             "placement_candidate_node_ids": ["worker-a", "worker-b", "worker-c"]},
            {"job_id": "test", "task_ids": ["//:test"], "state": "transferring",
             "node_id": "worker-b", "attempt": 1, "cache": "hit", "queue": "builds",
             "placement_candidate_node_ids": ["worker-a", "worker-b", "worker-c"]},
            {"job_id": "lint", "task_ids": ["//:lint"], "state": "ready",
             "node_id": null, "attempt": 0, "cache": null, "queue": "builds",
             "placement_candidate_node_ids": ["worker-a", "worker-b", "worker-c"]}
        ]}
    })
}
