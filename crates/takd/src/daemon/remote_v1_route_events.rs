fn handle_remote_events_route(
    store: &SubmitAttemptStore,
    method: &str,
    path_only: &str,
    query: Option<&str>,
) -> Result<Option<RemoteV1Response>> {
    let Some(task_run_id) = remote_task_path_arg(path_only, "/events") else {
        return Ok(None);
    };
    if method != "GET" {
        return Ok(None);
    }

    let after_seq = query_param_u64(query, "after_seq").unwrap_or(0);
    let key = resolve_submit_idempotency_key_for_task_run(store, task_run_id, query)?;
    let Some(key) = key else {
        return Ok(Some(json_response(
            404,
            serde_json::json!({"error":"task_not_found"}),
        )));
    };

    let events = store.events(&key)?;
    let mut lines = Vec::new();
    for event in events.into_iter().filter(|event| event.seq > after_seq) {
        let payload_value = serde_json::from_str::<serde_json::Value>(&event.payload_json)
            .unwrap_or_else(|_| serde_json::json!({ "raw": event.payload_json }));
        let event_type = payload_value
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("EVENT");
        let timestamp_ms = payload_value
            .get("timestamp_ms")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0);
        lines.push(
            serde_json::json!({
                "seq": event.seq,
                "task_run_id": task_run_id,
                "type": event_type,
                "timestamp_ms": timestamp_ms,
                "payload": payload_value,
            })
            .to_string(),
        );
    }

    let mut body = lines.join("\n");
    if !body.is_empty() {
        body.push('\n');
    }
    Ok(Some(RemoteV1Response {
        status_code: 200,
        content_type: "application/x-ndjson".to_string(),
        body,
    }))
}
