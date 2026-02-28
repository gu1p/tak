//! Integration contract tests for canonical remote V1 endpoint routing.

use serde_json::Value;
use takd::{SubmitAttemptStore, build_submit_idempotency_key, handle_remote_v1_request};

#[test]
fn serves_required_v1_endpoints_with_stable_contracts() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("takd.sqlite");
    let store = SubmitAttemptStore::with_db_path(db_path).expect("submit attempt store");

    let capabilities = handle_remote_v1_request(&store, "GET", "/v1/node/capabilities", None)
        .expect("capabilities request");
    assert_eq!(capabilities.status_code, 200);
    let capabilities_json: Value =
        serde_json::from_str(&capabilities.body).expect("capabilities json");
    assert_eq!(
        capabilities_json.get("compatible").and_then(Value::as_bool),
        Some(true)
    );

    let status =
        handle_remote_v1_request(&store, "GET", "/v1/node/status", None).expect("status request");
    assert_eq!(status.status_code, 200);
    let status_json: Value = serde_json::from_str(&status.body).expect("status json");
    assert_eq!(
        status_json.get("healthy").and_then(Value::as_bool),
        Some(true)
    );

    let submit = handle_remote_v1_request(
        &store,
        "POST",
        "/v1/tasks/submit",
        Some(r#"{"task_run_id":"task-run-42","attempt":1,"selected_node_id":"remote-a"}"#),
    )
    .expect("submit request");
    assert_eq!(submit.status_code, 200);
    let submit_json: Value = serde_json::from_str(&submit.body).expect("submit json");
    assert_eq!(
        submit_json.get("accepted").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        submit_json.get("attached").and_then(Value::as_bool),
        Some(false)
    );

    let key = build_submit_idempotency_key("task-run-42", Some(1)).expect("idempotency key");
    store
        .append_event(
            &key,
            1,
            r#"{"kind":"TASK_LOG_CHUNK","chunk":"first\n","timestamp_ms":1700000000000}"#,
        )
        .expect("event 1");
    store
        .append_event(
            &key,
            2,
            r#"{"kind":"TASK_LOG_CHUNK","chunk":"second\n","timestamp_ms":1700000000010}"#,
        )
        .expect("event 2");
    store
        .set_result_payload(
            &key,
            r#"{"success":true,"exit_code":0,"outputs":[{"path":"dist/app.bin","digest":"sha256:abc","size":11}]}"#,
        )
        .expect("result payload");

    let events_from_zero = handle_remote_v1_request(
        &store,
        "GET",
        "/v1/tasks/task-run-42/events?after_seq=0",
        None,
    )
    .expect("events request from zero");
    assert_eq!(events_from_zero.status_code, 200);
    assert_eq!(events_from_zero.content_type, "application/x-ndjson");
    let lines_from_zero = events_from_zero
        .body
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    assert_eq!(
        lines_from_zero.len(),
        2,
        "after_seq=0 should stream all events"
    );
    let first_event: Value = serde_json::from_str(lines_from_zero[0]).expect("first event json");
    let second_event: Value = serde_json::from_str(lines_from_zero[1]).expect("second event json");
    assert_eq!(first_event.get("seq").and_then(Value::as_u64), Some(1));
    assert_eq!(second_event.get("seq").and_then(Value::as_u64), Some(2));

    let events_from_one = handle_remote_v1_request(
        &store,
        "GET",
        "/v1/tasks/task-run-42/events?after_seq=1",
        None,
    )
    .expect("events request from seq=1");
    assert_eq!(events_from_one.status_code, 200);
    assert_eq!(events_from_one.content_type, "application/x-ndjson");
    let lines_from_one = events_from_one
        .body
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    assert_eq!(
        lines_from_one.len(),
        1,
        "after_seq should resume without duplicates"
    );
    let event_json: Value = serde_json::from_str(lines_from_one[0]).expect("resumed event json");
    assert_eq!(event_json.get("seq").and_then(Value::as_u64), Some(2));
    assert_eq!(
        event_json.get("task_run_id").and_then(Value::as_str),
        Some("task-run-42")
    );
    assert_eq!(
        event_json.get("type").and_then(Value::as_str),
        Some("TASK_LOG_CHUNK")
    );

    let result = handle_remote_v1_request(&store, "GET", "/v1/tasks/task-run-42/result", None)
        .expect("result request");
    assert_eq!(result.status_code, 200);
    let result_json: Value = serde_json::from_str(&result.body).expect("result json");
    assert_eq!(
        result_json.get("status").and_then(Value::as_str),
        Some("success")
    );
    assert_eq!(
        result_json.get("success").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        result_json.get("exit_code").and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        result_json.get("node_id").and_then(Value::as_str),
        Some("remote-a")
    );
    assert!(
        result_json.get("started_at").is_some(),
        "result envelope should include started_at field"
    );
    assert!(
        result_json.get("finished_at").is_some(),
        "result envelope should include finished_at field"
    );
    assert!(
        result_json.get("duration_ms").is_some(),
        "result envelope should include duration_ms field"
    );
    assert!(
        result_json.get("transport_kind").is_some(),
        "result envelope should include placement transport_kind"
    );
    assert!(
        result_json.get("log_artifact_uri").is_some(),
        "result envelope should include log artifact metadata field"
    );
    assert!(
        result_json.get("outputs").is_some_and(Value::is_array),
        "result envelope should include output metadata list"
    );
    assert!(
        result_json.get("stdout_tail").is_some(),
        "result envelope should include bounded stdout tail field"
    );
    assert!(
        result_json.get("stderr_tail").is_some(),
        "result envelope should include bounded stderr tail field"
    );

    let cancel = handle_remote_v1_request(&store, "POST", "/v1/tasks/task-run-42/cancel", None)
        .expect("cancel request");
    assert_eq!(cancel.status_code, 202);
    let cancel_json: Value = serde_json::from_str(&cancel.body).expect("cancel json");
    assert_eq!(
        cancel_json.get("cancelled").and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn submit_endpoint_attaches_duplicate_attempt() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("takd.sqlite");
    let store = SubmitAttemptStore::with_db_path(db_path).expect("submit attempt store");

    let first = handle_remote_v1_request(
        &store,
        "POST",
        "/v1/tasks/submit",
        Some(r#"{"task_run_id":"task-run-attach","attempt":1,"selected_node_id":"remote-a"}"#),
    )
    .expect("first submit");
    assert_eq!(first.status_code, 200);
    let first_json: Value = serde_json::from_str(&first.body).expect("first submit json");
    assert_eq!(
        first_json.get("attached").and_then(Value::as_bool),
        Some(false)
    );

    let second = handle_remote_v1_request(
        &store,
        "POST",
        "/v1/tasks/submit",
        Some(r#"{"task_run_id":"task-run-attach","attempt":1,"selected_node_id":"remote-a"}"#),
    )
    .expect("second submit");
    assert_eq!(second.status_code, 200);
    let second_json: Value = serde_json::from_str(&second.body).expect("second submit json");
    assert_eq!(
        second_json.get("attached").and_then(Value::as_bool),
        Some(true)
    );
}
